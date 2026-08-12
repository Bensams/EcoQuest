//! PostgreSQL implementation of [`AuthStore`].
//!
//! Uses runtime `sqlx::query` rather than the compile-time macros so the build
//! does not require a live database or checked-in offline metadata.

use chrono::{DateTime, Utc};
use ecoquest_application::{
    auth::{
        models::{AuditEntry, RefreshTokenRecord, RequestContext, User},
        ports::AuthStore,
    },
    AppError, AppResult,
};
use ecoquest_domain::{DomainError, Role, UserStatus};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::PgStore;

/// PostgreSQL-backed identity store.
#[derive(Clone, Debug)]
pub struct PgAuthStore {
    pool: PgPool,
}

impl PgAuthStore {
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

fn user_from_row(row: &PgRow) -> AppResult<User> {
    let role: String = row.try_get("role").map_err(db_err)?;
    let status: String = row.try_get("status").map_err(db_err)?;
    Ok(User {
        id: row.try_get("id").map_err(db_err)?,
        username: row.try_get("username").map_err(db_err)?,
        email: row.try_get("email").map_err(db_err)?,
        password_hash: row.try_get("password_hash").map_err(db_err)?,
        role: Role::from_str(&role).map_err(AppError::Domain)?,
        wallet_address: row.try_get("wallet_address").map_err(db_err)?,
        status: UserStatus::from_str(&status).map_err(AppError::Domain)?,
        eco_points: row.try_get("eco_points").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
        updated_at: row.try_get("updated_at").map_err(db_err)?,
    })
}

/// Columns shared by every user query; `role`/`status` are cast to text so the
/// enums are parsed once, by the domain.
const USER_COLUMNS: &str = "id, username, email, password_hash, role::text AS role, \
     wallet_address, status::text AS status, eco_points, created_at, updated_at";

#[async_trait::async_trait]
impl AuthStore for PgAuthStore {
    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: Role,
    ) -> AppResult<User> {
        let sql = format!(
            "INSERT INTO users (username, email, password_hash, role) \
             VALUES ($1, $2, $3, $4::user_role) RETURNING {USER_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(username)
            .bind(email)
            .bind(password_hash)
            .bind(role.as_str())
            .fetch_one(&self.pool)
            .await;

        match row {
            Ok(row) => user_from_row(&row),
            // 23505 = unique_violation. Uniqueness is enforced by the database so
            // concurrent registrations cannot both succeed.
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
                let field = match e.constraint() {
                    Some("users_username_lower_key") => "username",
                    _ => "email",
                };
                Err(AppError::Domain(DomainError::Conflict(format!(
                    "{field} is already registered"
                ))))
            }
            Err(e) => Err(db_err(e)),
        }
    }

    async fn find_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE lower(email) = lower($1)");
        let row = sqlx::query(&sql)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        row.as_ref().map(user_from_row).transpose()
    }

    async fn find_user_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        row.as_ref().map(user_from_row).transpose()
    }

    async fn insert_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        context: &RequestContext,
    ) -> AppResult<Uuid> {
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at, user_agent, ip_address) \
             VALUES ($1, $2, $3, $4, $5::inet) RETURNING id",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(context.user_agent.as_deref())
        // Bound as text and cast, so sqlx needs no extra IP type feature.
        .bind(context.ip_address.map(|ip| ip.to_string()))
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)
    }

    async fn find_refresh_token(&self, token_hash: &[u8]) -> AppResult<Option<RefreshTokenRecord>> {
        let row = sqlx::query(
            "SELECT id, user_id, expires_at, revoked_at, replaced_by \
             FROM refresh_tokens WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        row.map(|row| {
            Ok(RefreshTokenRecord {
                id: row.try_get("id").map_err(db_err)?,
                user_id: row.try_get("user_id").map_err(db_err)?,
                expires_at: row.try_get("expires_at").map_err(db_err)?,
                revoked_at: row.try_get("revoked_at").map_err(db_err)?,
                replaced_by: row.try_get("replaced_by").map_err(db_err)?,
            })
        })
        .transpose()
    }

    async fn mark_refresh_token_rotated(&self, id: Uuid, replacement: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE refresh_tokens SET replaced_by = $2, revoked_at = COALESCE(revoked_at, now()) \
             WHERE id = $1",
        )
        .bind(id)
        .bind(replacement)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn revoke_refresh_token(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() \
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected())
    }

    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO audit_logs \
             (actor_id, action, entity_type, entity_id, metadata, ip_address) \
             VALUES ($1, $2, $3, $4, $5, $6::inet)",
        )
        .bind(entry.actor_id)
        .bind(&entry.action)
        .bind(&entry.entity_type)
        .bind(entry.entity_id)
        .bind(&entry.metadata)
        .bind(entry.ip_address.map(|ip| ip.to_string()))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
}
