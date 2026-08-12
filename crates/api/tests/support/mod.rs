//! Shared test doubles: an in-memory [`AuthStore`] and a router builder.
//!
//! Keeps the integration tests free of a live database while still exercising
//! the real service, extractor, cookie and error-mapping code.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use ecoquest_api::{auth::RateLimiter, router, AppState};
use ecoquest_application::{
    auth::{
        models::{AuditEntry, RefreshTokenRecord, RequestContext, User},
        ports::AuthStore,
        AuthService, PasswordHasherService, TokenService,
    },
    AppError, AppResult, HealthProbe,
};
use ecoquest_domain::{DomainError, Role, UserStatus};
use uuid::Uuid;

/// Test JWT secret; long enough to satisfy [`TokenService`].
pub const TEST_SECRET: &[u8] = b"test-secret-that-is-long-enough!!";

/// Health probe stub; `true` reports the database as up.
pub struct StubProbe(pub bool);

#[async_trait::async_trait]
impl HealthProbe for StubProbe {
    async fn check(&self) -> AppResult<()> {
        if self.0 {
            Ok(())
        } else {
            Err(AppError::Infrastructure("database down".into()))
        }
    }
}

#[derive(Default)]
struct Data {
    users: Vec<User>,
    tokens: Vec<(Vec<u8>, RefreshTokenRecord)>,
    audits: Vec<AuditEntry>,
}

/// In-memory identity store mirroring the PostgreSQL semantics used in tests.
#[derive(Default)]
pub struct MemoryStore {
    data: Mutex<Data>,
}

impl MemoryStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Data> {
        self.data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Actions recorded in the audit trail, in order.
    pub fn audit_actions(&self) -> Vec<String> {
        self.lock()
            .audits
            .iter()
            .map(|a| a.action.clone())
            .collect()
    }

    /// Overrides a user's status, simulating an admin suspension.
    pub fn set_status(&self, user_id: Uuid, status: UserStatus) {
        let mut data = self.lock();
        if let Some(user) = data.users.iter_mut().find(|u| u.id == user_id) {
            user.status = status;
        }
    }

    /// Forces every stored refresh token to be expired.
    pub fn expire_all_tokens(&self) {
        let mut data = self.lock();
        for (_, record) in data.tokens.iter_mut() {
            record.expires_at = Utc::now() - chrono::Duration::seconds(1);
        }
    }

    /// Id of the single stored user, for tests that create exactly one.
    pub fn only_user_id(&self) -> Uuid {
        let data = self.lock();
        assert_eq!(data.users.len(), 1, "expected exactly one user");
        data.users[0].id
    }
}

#[async_trait::async_trait]
impl AuthStore for MemoryStore {
    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: Role,
    ) -> AppResult<User> {
        let mut data = self.lock();
        if data
            .users
            .iter()
            .any(|u| u.email.eq_ignore_ascii_case(email))
        {
            return Err(AppError::Domain(DomainError::Conflict(
                "email is already registered".into(),
            )));
        }
        if data
            .users
            .iter()
            .any(|u| u.username.eq_ignore_ascii_case(username))
        {
            return Err(AppError::Domain(DomainError::Conflict(
                "username is already registered".into(),
            )));
        }
        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            role,
            wallet_address: None,
            status: UserStatus::Active,
            eco_points: 0,
            created_at: now,
            updated_at: now,
        };
        data.users.push(user.clone());
        Ok(user)
    }

    async fn find_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        Ok(self
            .lock()
            .users
            .iter()
            .find(|u| u.email.eq_ignore_ascii_case(email))
            .cloned())
    }

    async fn find_user_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        Ok(self.lock().users.iter().find(|u| u.id == id).cloned())
    }

    async fn insert_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        _context: &RequestContext,
    ) -> AppResult<Uuid> {
        let record = RefreshTokenRecord {
            id: Uuid::new_v4(),
            user_id,
            expires_at,
            revoked_at: None,
            replaced_by: None,
        };
        let id = record.id;
        self.lock().tokens.push((token_hash.to_vec(), record));
        Ok(id)
    }

    async fn find_refresh_token(&self, token_hash: &[u8]) -> AppResult<Option<RefreshTokenRecord>> {
        Ok(self
            .lock()
            .tokens
            .iter()
            .find(|(h, _)| h == token_hash)
            .map(|(_, r)| r.clone()))
    }

    async fn mark_refresh_token_rotated(&self, id: Uuid, replacement: Uuid) -> AppResult<()> {
        let mut data = self.lock();
        if let Some((_, record)) = data.tokens.iter_mut().find(|(_, r)| r.id == id) {
            record.replaced_by = Some(replacement);
            record.revoked_at.get_or_insert_with(Utc::now);
        }
        Ok(())
    }

    async fn revoke_refresh_token(&self, id: Uuid) -> AppResult<bool> {
        let mut data = self.lock();
        match data.tokens.iter_mut().find(|(_, r)| r.id == id) {
            Some((_, record)) if record.revoked_at.is_none() => {
                record.revoked_at = Some(Utc::now());
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<u64> {
        let mut data = self.lock();
        let mut count = 0;
        for (_, record) in data.tokens.iter_mut() {
            if record.user_id == user_id && record.revoked_at.is_none() {
                record.revoked_at = Some(Utc::now());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()> {
        self.lock().audits.push(entry);
        Ok(())
    }
}

/// Builds a router plus the store behind it, with the given limits.
pub fn test_app(rate_limit: u32, access_ttl_secs: i64) -> (axum::Router, Arc<MemoryStore>) {
    let (state, store) = test_state(rate_limit, access_ttl_secs);
    (router(state, &["http://localhost:5173".to_string()]), store)
}

/// Builds handler state directly, for tests that need their own routes.
pub fn test_state(rate_limit: u32, access_ttl_secs: i64) -> (AppState, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::default());
    let auth = AuthService::new(
        store.clone(),
        PasswordHasherService::for_tests(),
        TokenService::new(TEST_SECRET, access_ttl_secs, 3600).expect("token service"),
    );
    let state = AppState::new(
        Arc::new(StubProbe(true)),
        Arc::new(auth),
        Arc::new(RateLimiter::new(rate_limit, 60)),
        false,
    );
    (state, store)
}
