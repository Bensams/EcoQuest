//! Shared handler state.

use std::{net::IpAddr, sync::Arc};

use ecoquest_application::{
    achievements::AchievementService, auth::AuthService, events::EventService, CertificateService,
    HealthProbe, ImpactService,
};

use crate::{auth::rate_limit::RateLimiter, ApiError};

/// State injected into every Axum handler.
#[derive(Clone)]
pub struct AppState {
    /// Readiness probe for the primary datastore.
    pub health_probe: Arc<dyn HealthProbe>,
    /// Authentication use cases.
    pub auth: Arc<AuthService>,
    /// Environmental mission use cases. Optional while Phase 1 test fixtures run.
    pub events: Option<Arc<EventService>>,
    /// Read-only Phase 7 impact dashboards.
    pub impact: Option<Arc<ImpactService>>,
    /// Phase 8 wallet ownership and achievement read use cases.
    pub achievements: Option<Arc<AchievementService>>,
    /// Phase 5 certificate issuance and verification use cases.
    pub certificates: Option<Arc<CertificateService>>,
    /// Rate limiter guarding authentication endpoints.
    pub auth_rate_limiter: Arc<RateLimiter>,
    /// Whether session cookies carry the `Secure` attribute.
    pub cookies_secure: bool,
}

impl AppState {
    /// Builds state from its collaborators.
    #[must_use]
    pub fn new(
        health_probe: Arc<dyn HealthProbe>,
        auth: Arc<AuthService>,
        auth_rate_limiter: Arc<RateLimiter>,
        cookies_secure: bool,
    ) -> Self {
        Self {
            health_probe,
            auth,
            events: None,
            impact: None,
            achievements: None,
            certificates: None,
            auth_rate_limiter,
            cookies_secure,
        }
    }

    /// Attaches Phase 3 mission use cases.
    #[must_use]
    pub fn with_events(mut self, events: Arc<EventService>) -> Self {
        self.events = Some(events);
        self
    }

    /// Attaches Phase 7 impact read use cases.
    #[must_use]
    pub fn with_impact(mut self, impact: Arc<ImpactService>) -> Self {
        self.impact = Some(impact);
        self
    }

    /// Attaches Phase 8 achievement use cases.
    #[must_use]
    pub fn with_achievements(mut self, achievements: Arc<AchievementService>) -> Self {
        self.achievements = Some(achievements);
        self
    }

    /// Attaches Phase 5 certificate use cases.
    #[must_use]
    pub fn with_certificates(mut self, certificates: Arc<CertificateService>) -> Self {
        self.certificates = Some(certificates);
        self
    }

    /// Gets certificate use cases or returns a safe configuration error.
    pub fn certificate_service(&self) -> Result<&Arc<CertificateService>, ApiError> {
        self.certificates
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable("certificates are not configured".into()))
    }

    pub fn achievement_service(&self) -> Result<&Arc<AchievementService>, ApiError> {
        self.achievements
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable("achievements are not configured".into()))
    }

    /// Gets mission use cases or returns a safe configuration error.
    pub fn event_service(&self) -> Result<&Arc<EventService>, ApiError> {
        self.events
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable("missions are not configured".into()))
    }

    /// Gets impact use cases or returns a safe configuration error.
    pub fn impact_service(&self) -> Result<&Arc<ImpactService>, ApiError> {
        self.impact
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable("impact is not configured".into()))
    }

    /// Applies the authentication rate limit.
    ///
    /// # Errors
    /// Returns [`ApiError::TooManyRequests`] when the caller is over the limit.
    pub fn enforce_auth_rate_limit(&self, ip: Option<IpAddr>) -> Result<(), ApiError> {
        if self.auth_rate_limiter.check(ip) {
            Ok(())
        } else {
            tracing::warn!(?ip, "authentication rate limit exceeded");
            Err(ApiError::TooManyRequests(
                "too many attempts, please try again later".into(),
            ))
        }
    }
}
