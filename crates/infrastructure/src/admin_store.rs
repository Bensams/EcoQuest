//! PostgreSQL implementation of [`AdminStore`].
//!
//! Uses runtime `sqlx::query` rather than the compile-time macros so the build
//! does not require a live database or checked-in offline metadata.

use ecoquest_application::{
    admin::{AdminEvent, AdminOrganization, AdminStore, AdminUser},
    AppError, AppResult,
};
use ecoquest_domain::{EventStatus, Role, UserStatus, VerificationStatus};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::PgStore;

/// PostgreSQL-backed platform administration store.
#[derive(Clone, Debug)]
pub struct PgAdminStore {
    pool: PgPool,
}

impl PgAdminStore {
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

fn organization_from_row(row: &PgRow) -> AppResult<AdminOrganization> {
    let verification_status: String = row.try_get("verification_status").map_err(db_err)?;
    Ok(AdminOrganization {
        id: row.try_get("id").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        organization_type: row.try_get("organization_type").map_err(db_err)?,
        location: row.try_get("location").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        verification_status: VerificationStatus::from_str(&verification_status)
            .map_err(AppError::Domain)?,
        owner_id: row.try_get("owner_id").map_err(db_err)?,
        owner_username: row.try_get("owner_username").map_err(db_err)?,
        event_count: row.try_get("event_count").map_err(db_err)?,
        reviewed_at: row.try_get("reviewed_at").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

fn user_from_row(row: &PgRow) -> AppResult<AdminUser> {
    let role: String = row.try_get("role").map_err(db_err)?;
    let status: String = row.try_get("status").map_err(db_err)?;
    Ok(AdminUser {
        id: row.try_get("id").map_err(db_err)?,
        username: row.try_get("username").map_err(db_err)?,
        email: row.try_get("email").map_err(db_err)?,
        role: Role::from_str(&role).map_err(AppError::Domain)?,
        status: UserStatus::from_str(&status).map_err(AppError::Domain)?,
        eco_points: row.try_get("eco_points").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

fn event_from_row(row: &PgRow) -> AppResult<AdminEvent> {
    let status: String = row.try_get("status").map_err(db_err)?;
    let activity_type: String = row.try_get("activity_type").map_err(db_err)?;
    Ok(AdminEvent {
        id: row.try_get("id").map_err(db_err)?,
        organization_id: row.try_get("organization_id").map_err(db_err)?,
        organization_name: row.try_get("organization_name").map_err(db_err)?,
        owner_username: row.try_get("owner_username").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        activity_type,
        location: row.try_get("location").map_err(db_err)?,
        starts_at: row.try_get("starts_at").map_err(db_err)?,
        ends_at: row.try_get("ends_at").map_err(db_err)?,
        status: EventStatus::from_str(&status).map_err(AppError::Domain)?,
        registered_count: row.try_get("registered_count").map_err(db_err)?,
        cancelled_by: row.try_get("cancelled_by").map_err(db_err)?,
        cancelled_at: row.try_get("cancelled_at").map_err(db_err)?,
        cancellation_reason: row.try_get("cancellation_reason").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

#[async_trait::async_trait]
impl AdminStore for PgAdminStore {
    async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>> {
        let rows = sqlx::query(
            "SELECT o.id, o.name, o.organization_type, o.location, o.description, \
             o.verification_status::text AS verification_status, o.reviewed_at, o.created_at, \
             o.owner_id, \
             (SELECT username FROM users u WHERE u.id = o.owner_id) AS owner_username, \
             (SELECT count(*) FROM events e WHERE e.organization_id = o.id)::bigint AS event_count \
             FROM organizations o \
             ORDER BY o.created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(organization_from_row).collect()
    }

    async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE organizations SET verification_status=$2::verification_status, \
             reviewed_by=$3, reviewed_at=now() WHERE id=$1",
        )
        .bind(organization_id)
        .bind(status.as_str())
        .bind(reviewer_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
        let rows = sqlx::query(
            "SELECT id, username, email, role::text AS role, status::text AS status, \
             eco_points, created_at FROM users ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(user_from_row).collect()
    }

    async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<bool> {
        let result = sqlx::query("UPDATE users SET role=$2::user_role WHERE id=$1")
            .bind(user_id)
            .bind(role.as_str())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_user_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<bool> {
        let result = sqlx::query("UPDATE users SET status=$2::user_status WHERE id=$1")
            .bind(user_id)
            .bind(status.as_str())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_events(&self) -> AppResult<Vec<AdminEvent>> {
        const EVENT_SELECT: &str =
            "SELECT e.id, e.organization_id, o.name AS organization_name, \
             COALESCE(u.username, '(no owner)') AS owner_username, \
             e.name, e.activity_type::text AS activity_type, e.location, \
             e.starts_at, e.ends_at, e.status::text AS status, \
             (SELECT count(*) FROM participations p WHERE p.event_id=e.id AND p.status <> 'CANCELLED')::bigint AS registered_count, \
             e.cancelled_by, e.cancelled_at, e.cancellation_reason, e.created_at \
             FROM events e JOIN organizations o ON o.id=e.organization_id \
             LEFT JOIN users u ON u.id=o.owner_id";
        let rows = sqlx::query(&format!("{EVENT_SELECT} ORDER BY e.created_at DESC"))
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        rows.iter().map(event_from_row).collect()
    }

    async fn find_event(&self, event_id: Uuid) -> AppResult<Option<AdminEvent>> {
        const EVENT_SELECT: &str =
            "SELECT e.id, e.organization_id, o.name AS organization_name, \
             COALESCE(u.username, '(no owner)') AS owner_username, \
             e.name, e.activity_type::text AS activity_type, e.location, \
             e.starts_at, e.ends_at, e.status::text AS status, \
             (SELECT count(*) FROM participations p WHERE p.event_id=e.id AND p.status <> 'CANCELLED')::bigint AS registered_count, \
             e.cancelled_by, e.cancelled_at, e.cancellation_reason, e.created_at \
             FROM events e JOIN organizations o ON o.id=e.organization_id \
             LEFT JOIN users u ON u.id=o.owner_id WHERE e.id=$1";
        sqlx::query(EVENT_SELECT)
            .bind(event_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(event_from_row)
            .transpose()
    }

    async fn cancel_event(
        &self,
        event_id: Uuid,
        cancelled_by: Uuid,
        reason: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE events SET status='CANCELLED'::event_status, cancelled_by=$2, \
             cancelled_at=now(), cancellation_reason=$3 \
             WHERE id=$1 AND status <> 'CANCELLED'::event_status AND status <> 'COMPLETED'::event_status",
        )
        .bind(event_id)
        .bind(cancelled_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() == 1)
    }
}
