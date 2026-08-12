//! EcoQuest domain rules.
//!
//! Pure Rust only: no Axum, SQLx, blockchain or frontend types may appear here.

pub mod auth;
pub mod events;

pub use auth::{EmailAddress, Password, Role, UserStatus, Username, VerificationStatus};
pub use events::{ActivityType, EventStatus, ParticipationStatus};

use serde::{Deserialize, Serialize};

/// Errors produced by domain rules. Transport layers map these to responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum DomainError {
    /// A value failed a domain invariant.
    #[error("invalid input: {0}")]
    Validation(String),
    /// The requested entity does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The operation conflicts with current state (e.g. already verified).
    #[error("conflict: {0}")]
    Conflict(String),
    /// The actor is not allowed to perform the operation.
    #[error("forbidden: {0}")]
    Forbidden(String),
}

/// Result alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_error_renders_message() {
        let err = DomainError::Conflict("participation already verified".into());
        assert_eq!(err.to_string(), "conflict: participation already verified");
    }

    #[test]
    fn domain_error_roundtrips_json() {
        let err = DomainError::NotFound("event".into());
        let json = serde_json::to_string(&err).expect("serialize");
        let back: DomainError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(err, back);
    }
}
