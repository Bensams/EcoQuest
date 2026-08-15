//! EcoQuest use cases.
//!
//! Orchestrates domain rules through ports (traits). Infrastructure supplies the
//! adapters; nothing here may depend on Axum, SQLx or Solana crates.

pub mod achievements;
pub mod admin;
pub mod auth;
pub mod certificates;
pub mod events;
pub mod impact;
pub mod organizations;

pub use achievements::{
    AchievementService, AchievementStore, BlockchainAdapter, MockBlockchainAdapter,
};
pub use admin::{
    AdminEvent, AdminListQuery, AdminOrganization, AdminService, AdminStore, AdminUser,
    EventModeration, Page,
};
pub use auth::{AuthService, AuthStore, PasswordHasherService, TokenService};
pub use certificates::{
    verification_hash, Certificate, CertificateData, CertificateService, CertificateStore,
    OrganizationStatus,
};
pub use events::{EventService, EventStore};
pub use impact::{ImpactService, ImpactStore};
pub use organizations::{
    CreateOrganizationCommand, Organization, OrganizationService, OrganizationStore,
};

use ecoquest_domain::DomainError;

/// Errors returned by use cases.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A domain invariant was violated.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// An outbound port failed (database, queue, external service).
    #[error("infrastructure failure: {0}")]
    Infrastructure(String),
}

/// Result alias for use cases.
pub type AppResult<T> = Result<T, AppError>;

/// Port exposing liveness of backing stores, used by the health endpoint.
#[async_trait::async_trait]
pub trait HealthProbe: Send + Sync {
    /// Returns `Ok(())` when the backing store accepts queries.
    async fn check(&self) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_errors_convert_into_app_errors() {
        let err: AppError = DomainError::Validation("bad email".into()).into();
        assert_eq!(err.to_string(), "invalid input: bad email");
    }
}
