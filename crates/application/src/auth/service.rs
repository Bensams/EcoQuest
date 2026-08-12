//! Authentication use cases: register, login, refresh, logout, current user.

use std::sync::Arc;

use chrono::{Duration, Utc};
use ecoquest_domain::{DomainError, Role};
use uuid::Uuid;

use super::{
    models::{
        AuditEntry, AuthOutcome, LoginCommand, RegisterCommand, RequestContext, SessionTokens,
        UserProfile,
    },
    password::PasswordHasherService,
    ports::AuthStore,
    tokens::{hash_refresh_token, AccessClaims, TokenService},
};
use crate::{AppError, AppResult};

/// A valid Argon2id hash of a random value, used to equalize login timing for
/// unknown accounts. It matches no real password.
const DUMMY_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0c2E$H4Wg4c0Ni0mSf6oM3sZ4Tt0BUb7LTC2xVwsMLQNTt1A";

/// Orchestrates authentication against an [`AuthStore`].
#[derive(Clone)]
pub struct AuthService {
    store: Arc<dyn AuthStore>,
    hasher: PasswordHasherService,
    tokens: TokenService,
}

impl AuthService {
    /// Builds the service from its collaborators.
    #[must_use]
    pub fn new(
        store: Arc<dyn AuthStore>,
        hasher: PasswordHasherService,
        tokens: TokenService,
    ) -> Self {
        Self {
            store,
            hasher,
            tokens,
        }
    }

    /// Access-token lifetime in seconds; used to size the access cookie.
    #[must_use]
    pub fn access_ttl_secs(&self) -> i64 {
        self.tokens.access_ttl_secs()
    }

    /// Refresh-token lifetime in seconds; used to size the refresh cookie.
    #[must_use]
    pub fn refresh_ttl_secs(&self) -> i64 {
        self.tokens.refresh_ttl_secs()
    }

    /// Verifies an access token and returns its claims.
    ///
    /// # Errors
    /// Returns [`AppError::Domain`] when the token is invalid or expired.
    pub fn verify_access_token(&self, token: &str) -> AppResult<AccessClaims> {
        self.tokens.verify_access_token(token)
    }

    /// Registers a new account and opens a session for it.
    ///
    /// # Errors
    /// - [`DomainError::Forbidden`] when the requested role is not self-assignable.
    /// - [`DomainError::Conflict`] when email or username is already taken.
    pub async fn register(
        &self,
        command: RegisterCommand,
        context: &RequestContext,
    ) -> AppResult<AuthOutcome> {
        if !command.role.is_self_assignable() {
            return Err(AppError::Domain(DomainError::Forbidden(
                "this role cannot be requested at registration".into(),
            )));
        }

        let password_hash = self.hasher.hash(command.password.expose())?;
        let user = self
            .store
            .create_user(
                command.username.as_str(),
                command.email.as_str(),
                &password_hash,
                command.role,
            )
            .await?;

        self.audit(
            Some(user.id),
            "user.registered",
            Some(user.id),
            serde_json::json!({ "role": user.role.as_str() }),
            context,
        )
        .await;

        let tokens = self.open_session(user.id, user.role, context).await?;
        Ok(AuthOutcome {
            user: user.into(),
            tokens,
        })
    }

    /// Authenticates with email and password.
    ///
    /// # Errors
    /// Returns [`DomainError::Forbidden`] for unknown users, wrong passwords and
    /// non-active accounts alike: the caller must not learn which it was.
    pub async fn login(
        &self,
        command: LoginCommand,
        context: &RequestContext,
    ) -> AppResult<AuthOutcome> {
        let rejected = || AppError::Domain(DomainError::Forbidden("invalid credentials".into()));
        let user = self
            .store
            .find_user_by_email(command.email.as_str())
            .await?;

        let Some(user) = user else {
            // Hash anyway so response time does not reveal account existence.
            let _ = self.hasher.verify(&command.password, DUMMY_HASH);
            self.audit(
                None,
                "user.login_failed",
                None,
                serde_json::json!({ "reason": "unknown_email" }),
                context,
            )
            .await;
            return Err(rejected());
        };

        if !self.hasher.verify(&command.password, &user.password_hash) {
            self.audit(
                Some(user.id),
                "user.login_failed",
                Some(user.id),
                serde_json::json!({ "reason": "bad_password" }),
                context,
            )
            .await;
            return Err(rejected());
        }

        if !user.status.can_authenticate() {
            self.audit(
                Some(user.id),
                "user.login_blocked",
                Some(user.id),
                serde_json::json!({ "status": user.status.as_str() }),
                context,
            )
            .await;
            return Err(rejected());
        }

        self.audit(
            Some(user.id),
            "user.login",
            Some(user.id),
            serde_json::json!({}),
            context,
        )
        .await;

        let tokens = self.open_session(user.id, user.role, context).await?;
        Ok(AuthOutcome {
            user: user.into(),
            tokens,
        })
    }

    /// Exchanges a refresh token for a new session, rotating the old token.
    ///
    /// Presenting an already-rotated token is treated as theft: every session of
    /// that user is revoked.
    ///
    /// # Errors
    /// Returns [`DomainError::Forbidden`] when the token is unknown, expired,
    /// revoked, already rotated, or its user can no longer authenticate.
    pub async fn refresh(
        &self,
        refresh_token: &str,
        context: &RequestContext,
    ) -> AppResult<AuthOutcome> {
        let rejected = || AppError::Domain(DomainError::Forbidden("invalid session".into()));
        let hash = hash_refresh_token(refresh_token);
        let Some(record) = self.store.find_refresh_token(&hash).await? else {
            return Err(rejected());
        };

        let now = Utc::now();
        if !record.is_usable(now) {
            // Reuse of a rotated token means the token leaked; drop all sessions.
            if record.replaced_by.is_some() {
                let revoked = self.store.revoke_all_user_tokens(record.user_id).await?;
                self.audit(
                    Some(record.user_id),
                    "session.reuse_detected",
                    Some(record.id),
                    serde_json::json!({ "revoked_sessions": revoked }),
                    context,
                )
                .await;
            }
            return Err(rejected());
        }

        let Some(user) = self.store.find_user_by_id(record.user_id).await? else {
            return Err(rejected());
        };
        if !user.status.can_authenticate() {
            self.store.revoke_all_user_tokens(user.id).await?;
            return Err(rejected());
        }

        let tokens = self.open_session(user.id, user.role, context).await?;
        let replacement = self
            .store
            .find_refresh_token(&hash_refresh_token(&tokens.refresh_token))
            .await?
            .ok_or_else(|| AppError::Infrastructure("rotated token vanished".into()))?;
        self.store
            .mark_refresh_token_rotated(record.id, replacement.id)
            .await?;

        self.audit(
            Some(user.id),
            "session.refreshed",
            Some(replacement.id),
            serde_json::json!({}),
            context,
        )
        .await;

        Ok(AuthOutcome {
            user: user.into(),
            tokens,
        })
    }

    /// Revokes a refresh token. Unknown tokens succeed silently so logout is
    /// idempotent and cannot be used to probe for valid tokens.
    ///
    /// # Errors
    /// Returns [`AppError::Infrastructure`] when the store fails.
    pub async fn logout(&self, refresh_token: &str, context: &RequestContext) -> AppResult<()> {
        let hash = hash_refresh_token(refresh_token);
        if let Some(record) = self.store.find_refresh_token(&hash).await? {
            if self.store.revoke_refresh_token(record.id).await? {
                self.audit(
                    Some(record.user_id),
                    "session.logout",
                    Some(record.id),
                    serde_json::json!({}),
                    context,
                )
                .await;
            }
        }
        Ok(())
    }

    /// Loads the profile of an authenticated user.
    ///
    /// # Errors
    /// Returns [`DomainError::NotFound`] when the account no longer exists and
    /// [`DomainError::Forbidden`] when it may no longer authenticate.
    pub async fn current_user(&self, user_id: Uuid) -> AppResult<UserProfile> {
        let user = self
            .store
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        if !user.status.can_authenticate() {
            return Err(AppError::Domain(DomainError::Forbidden(
                "account is not active".into(),
            )));
        }
        Ok(user.into())
    }

    async fn open_session(
        &self,
        user_id: Uuid,
        role: Role,
        context: &RequestContext,
    ) -> AppResult<SessionTokens> {
        let access_token = self.tokens.issue_access_token(user_id, role)?;
        let (refresh_token, refresh_hash) = self.tokens.generate_refresh_token();
        let refresh_expires_at = Utc::now() + Duration::seconds(self.tokens.refresh_ttl_secs());
        self.store
            .insert_refresh_token(user_id, &refresh_hash, refresh_expires_at, context)
            .await?;
        Ok(SessionTokens {
            access_token,
            access_expires_in: self.tokens.access_ttl_secs(),
            refresh_token,
            refresh_expires_at,
        })
    }

    /// Audit writes must never fail an otherwise successful request, so failures
    /// are logged instead of propagated.
    async fn audit(
        &self,
        actor_id: Option<Uuid>,
        action: &str,
        entity_id: Option<Uuid>,
        metadata: serde_json::Value,
        context: &RequestContext,
    ) {
        let entry = AuditEntry {
            actor_id,
            action: action.to_string(),
            entity_type: "user".to_string(),
            entity_id,
            metadata,
            ip_address: context.ip_address,
        };
        if let Err(err) = self.store.record_audit(entry).await {
            tracing::error!(error = %err, action, "failed to write audit entry");
        }
    }
}
