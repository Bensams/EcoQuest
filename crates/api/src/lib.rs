//! EcoQuest HTTP API (Axum).

pub mod achievements;
pub mod auth;
pub mod certificates;
pub mod config;
pub mod error;
pub mod events;
pub mod health;
pub mod impact;
pub mod state;
pub mod telemetry;

use axum::{
    extract::Request,
    http::{header, HeaderName, HeaderValue, Method},
    middleware::Next,
    response::Response,
    routing::get,
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

pub use config::Config;
pub use error::{ApiError, ApiErrorBody, ApiErrorDetail};
pub use state::AppState;

/// Builds the application router.
///
/// `allowed_origins` are matched exactly; malformed entries are skipped with a
/// warning rather than crashing the server.
pub fn router(state: AppState, allowed_origins: &[String]) -> Router {
    let origins = allowed_origins
        .iter()
        .filter_map(|origin| match origin.parse::<header::HeaderValue>() {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!(origin = %origin, "ignoring malformed CORS origin");
                None
            }
        })
        .collect::<Vec<_>>();

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .expose_headers([HeaderName::from_static("x-request-id")])
        // Required for the browser to send and store the session cookies.
        .allow_credentials(true);

    Router::new()
        .route("/api/health", get(health::health))
        .merge(auth::routes())
        .merge(achievements::routes())
        .merge(events::routes())
        .merge(impact::routes())
        .merge(certificates::routes())
        .layer(cors)
        .layer(axum::middleware::from_fn(security_and_request_id))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Adds baseline browser protections and a correlation ID to every response.
async fn security_and_request_id(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map_or_else(|| Uuid::new_v4().to_string(), ToString::to_string);
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let id =
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid"));
    headers.insert(HeaderName::from_static("x-request-id"), id);
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(self), geolocation=()"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
        ),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    tracing::info!(%request_id, %method, %path, status = %response.status(), "http request complete");
    response
}
