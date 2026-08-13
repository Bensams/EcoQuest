//! Organization application use cases.

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, VerificationStatus};
use uuid::Uuid;

use crate::{AppError, AppResult};

/// Organization as owned by its creator and reviewed by admins.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub organization_type: String,
    pub location: String,
    pub description: String,
    pub verification_status: VerificationStatus,
    pub owner_id: Uuid,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User-facing organization application payload.
#[derive(Debug)]
pub struct CreateOrganizationCommand {
    pub name: String,
    pub organization_type: String,
    pub location: String,
    pub description: String,
}

#[async_trait::async_trait]
pub trait OrganizationStore: Send + Sync {
    /// Inserts a new PENDING organization owned by `owner_id`. Fails with an
    /// infrastructure error on duplicate names or other constraint violations.
    async fn create_organization(
        &self,
        command: CreateOrganizationCommand,
        owner_id: Uuid,
    ) -> AppResult<Organization>;
}

#[derive(Clone)]
pub struct OrganizationService {
    store: std::sync::Arc<dyn OrganizationStore>,
}

impl OrganizationService {
    #[must_use]
    pub fn new(store: std::sync::Arc<dyn OrganizationStore>) -> Self {
        Self { store }
    }

    pub async fn create(&self, command: CreateOrganizationCommand, owner_id: Uuid) -> AppResult<Organization> {
        validate(&command)?;
        self.store.create_organization(command, owner_id).await
    }
}

fn validate(command: &CreateOrganizationCommand) -> AppResult<()> {
    if command.name.trim().is_empty() || command.name.len() > 120 {
        return Err(AppError::Domain(DomainError::Validation(
            "organization name must be 1-120 characters".into(),
        )));
    }
    if command.organization_type.trim().is_empty() {
        return Err(AppError::Domain(DomainError::Validation(
            "organization type is required".into(),
        )));
    }
    if command.location.trim().is_empty() {
        return Err(AppError::Domain(DomainError::Validation(
            "location is required".into(),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> CreateOrganizationCommand {
        CreateOrganizationCommand {
            name: "Green Earth Butuan".into(),
            organization_type: "NGO".into(),
            location: "Butuan City".into(),
            description: "Recycling drives".into(),
        }
    }

    #[test]
    fn accepts_a_complete_application() {
        assert!(validate(&command()).is_ok());
    }

    #[test]
    fn trims_whitespace_before_validating() {
        let mut c = command();
        c.name = "   ".into();
        assert!(validate(&c).is_err());
    }

    #[test]
    fn rejects_missing_type_or_location() {
        let mut c = command();
        c.organization_type = "  ".into();
        assert!(validate(&c).is_err());
        let mut c = command();
        c.location = String::new();
        assert!(validate(&c).is_err());
    }

    #[test]
    fn rejects_overlong_names() {
        let mut c = command();
        c.name = "x".repeat(121);
        assert!(validate(&c).is_err());
    }
}