//! PostgreSQL implementation of [`OrganizationStore`].

use ecoquest_application::{
    organizations::{CreateOrganizationCommand, Organization, OrganizationStore},
    AppError, AppResult,
};
use ecoquest_domain::VerificationStatus;
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::PgStore;

/// PostgreSQL-backed organization store.
#[derive(Clone, Debug)]
pub struct PgOrganizationStore {
    pool: PgPool,
}

impl PgOrganizationStore {
    /// Wraps an existing pool.
    #[must_use]
    pub fn new(store: &PgStore) -> Self {
        Self {
            pool: store.pool().clone(),
        }
    }
}

fn db_err(err: sqlx::Error) -> AppError {
    AppError::Infrastructure(err.to_string())
}

fn row_to_organization(row: &PgRow) -> AppResult<Organization> {
    let verification_status: String = row.try_get("verification_status").map_err(db_err)?;
    Ok(Organization {
        id: row.try_get("id").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        organization_type: row.try_get("organization_type").map_err(db_err)?,
        location: row.try_get("location").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        verification_status: VerificationStatus::from_str(&verification_status)
            .map_err(AppError::Domain)?,
        owner_id: row.try_get("owner_id").map_err(db_err)?,
        reviewed_at: row.try_get("reviewed_at").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

#[async_trait::async_trait]
impl OrganizationStore for PgOrganizationStore {
    async fn create_organization(
        &self,
        command: CreateOrganizationCommand,
        owner_id: Uuid,
    ) -> AppResult<Organization> {
        let name = command.name.trim();
        let organization_type = command.organization_type.trim();
        let location = command.location.trim();
        let description = command.description.trim();
        let row = sqlx::query(
            "INSERT INTO organizations (name, organization_type, location, description, owner_id) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, name, organization_type, location, description, \
             verification_status::text AS verification_status, owner_id, reviewed_at, created_at",
        )
        .bind(name)
        .bind(organization_type)
        .bind(location)
        .bind(description)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        row_to_organization(&row)
    }
}