//! Platform administration use cases (organizations and users).

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, Role, UserStatus, VerificationStatus};
use uuid::Uuid;

use crate::{AppError, AppResult};

/// Organization row as listed on the admin dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminOrganization {
    pub id: Uuid,
    pub name: String,
    pub organization_type: String,
    pub location: String,
    pub description: String,
    pub verification_status: VerificationStatus,
    pub member_count: i64,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User row as listed on the admin dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub status: UserStatus,
    pub eco_points: i32,
    pub created_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait AdminStore: Send + Sync {
    /// Lists all organizations with membership counts.
    async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>>;
    /// Sets the verification status of an organization. Returns false when it does not exist.
    async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
    ) -> AppResult<bool>;
    /// Lists all users.
    async fn list_users(&self) -> AppResult<Vec<AdminUser>>;
    /// Sets the platform role of a user. Returns false when it does not exist.
    async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<bool>;
    /// Sets the lifecycle status of a user. Returns false when it does not exist.
    async fn set_user_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<bool>;
}

/// Administration use cases.
#[derive(Clone)]
pub struct AdminService {
    store: std::sync::Arc<dyn AdminStore>,
}

impl AdminService {
    #[must_use]
    pub fn new(store: std::sync::Arc<dyn AdminStore>) -> Self {
        Self { store }
    }

    pub async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>> {
        self.store.list_organizations().await
    }

    pub async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
    ) -> AppResult<()> {
        if !self
            .store
            .set_organization_status(organization_id, status, reviewer_id)
            .await?
        {
            return Err(AppError::Domain(DomainError::NotFound(
                "organization".into(),
            )));
        }
        Ok(())
    }

    pub async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
        self.store.list_users().await
    }

    pub async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<()> {
        if !self.store.set_user_role(user_id, role).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        Ok(())
    }

    pub async fn set_user_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<()> {
        if !self.store.set_user_status(user_id, status).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        Ok(())
    }
}
