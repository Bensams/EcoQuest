//! EcoQuest API binary.

use std::sync::Arc;

use ecoquest_api::{auth::RateLimiter, router, telemetry, AppState, Config};
use ecoquest_application::{
    achievements::{AchievementService, MockBlockchainAdapter},
    admin::AdminService,
    auth::{AuthService, PasswordHasherService, TokenService},
    events::EventService,
    organizations::OrganizationService,
    CertificateService, ImpactService,
};
use ecoquest_infrastructure::{
    PgAchievementStore, PgAdminStore, PgAuthStore, PgCertificateStore, PgEventStore, PgImpactStore,
    PgOrganizationStore, PgStore,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .env is a developer convenience; real deployments use real env vars.
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    telemetry::init(&config.log_filter, config.log_json);

    let store = PgStore::connect_lazy(&config.database_url, config.database_max_connections)?;
    store.run_migrations().await?;

    let auth = AuthService::new(
        Arc::new(PgAuthStore::new(&store)),
        PasswordHasherService::new(),
        TokenService::new(
            config.jwt_secret.as_bytes(),
            config.access_token_ttl_secs,
            config.refresh_token_ttl_secs,
        )?,
    );
    let events = EventService::new(Arc::new(PgEventStore::new(&store)));
    let impact = ImpactService::new(Arc::new(PgImpactStore::new(&store)));
    // ponytail: mock adapter is local test transport; replace with Solana signer adapter before devnet.
    let achievements = Arc::new(AchievementService::new(
        Arc::new(PgAchievementStore::new(&store)),
        Arc::new(MockBlockchainAdapter),
    ));
    let certificates = Arc::new(CertificateService::new(Arc::new(PgCertificateStore::new(
        &store,
    ))));
    let admin = Arc::new(AdminService::new(Arc::new(PgAdminStore::new(&store))));
    let organizations = Arc::new(OrganizationService::new(Arc::new(
        PgOrganizationStore::new(&store),
    )));
    let worker = achievements.clone();
    tokio::spawn(async move {
        loop {
            if let Err(error) = worker.process_one().await {
                tracing::error!(%error, "achievement worker failed");
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    let certificate_worker = certificates.clone();
    tokio::spawn(async move {
        loop {
            // Sleep only when the queue is empty so a backlog drains without delay.
            match certificate_worker.process_one().await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => tracing::error!(%error, "certificate worker failed"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    let state = AppState::new(
        Arc::new(store),
        Arc::new(auth),
        Arc::new(RateLimiter::new(
            config.auth_rate_limit_max,
            config.auth_rate_limit_window_secs,
        )),
        config.cookies_secure,
    )
    .with_events(Arc::new(events))
    .with_impact(Arc::new(impact))
    .with_achievements(achievements)
    .with_certificates(certificates)
    .with_admin(admin)
    .with_organizations(organizations);

    let app = router(state, &config.cors_allowed_origins);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "ecoquest api listening");

    // ConnectInfo is required so the rate limiter and audit log see client IPs.
    let app = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %err, "failed to listen for shutdown signal");
    }
    tracing::info!("shutdown signal received");
}
