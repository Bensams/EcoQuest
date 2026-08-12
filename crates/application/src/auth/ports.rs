//! Outbound ports the authentication use cases depend on.

use chrono::{DateTime, Utc};
use ecoquest_domain::Role;
use uuid::Uuid;

use super::models::{AuditEntry, RefreshTokenRecord, RequestContext, User};
use crate::AppResult;

/// Persistence operations required by authentication.
///
/// One port instead of five: every method is used by the same use cases and the
/// same transaction boundary, so splitting it would only add indirection.
#[async_trait::async_trait]
pub trait AuthStore: Send + Sync {
    /// Inserts a user. Returns [`crate::AppError::Domain`] with
    /// [`ecoquest_domain::DomainError::Conflict`] when email or username is taken.
    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: Role,
    ) -> AppResult<User>;

    /// Looks up a user by normalized email.
    async fn find_user_by_email(&self, email: &str) -> AppResult<Option<User>>;

    /// Looks up a user by id.
    async fn find_user_by_id(&self, id: Uuid) -> AppResult<Option<User>>;

    /// Stores a refresh token by its hash.
    async fn insert_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        context: &RequestContext,
    ) -> AppResult<Uuid>;

    /// Finds a refresh token record by token hash.
    async fn find_refresh_token(&self, token_hash: &[u8]) -> AppResult<Option<RefreshTokenRecord>>;

    /// Marks a token as rotated into `replacement`.
    async fn mark_refresh_token_rotated(&self, id: Uuid, replacement: Uuid) -> AppResult<()>;

    /// Revokes a single token. Returns `true` when it was still active.
    async fn revoke_refresh_token(&self, id: Uuid) -> AppResult<bool>;

    /// Revokes every active token of a user; returns how many were revoked.
    /// Used when refresh-token reuse is detected.
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<u64>;

    /// Appends an audit entry.
    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()>;
}
