//! Shared test doubles: an in-memory [`AuthStore`] and a router builder.
//!
//! Keeps the integration tests free of a live database while still exercising
//! the real service, extractor, cookie and error-mapping code.

// Every integration test binary compiles this whole module, so helpers used by
// only one binary are dead code in the others.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use ecoquest_api::{auth::RateLimiter, router, AppState};
use ecoquest_application::{
    admin::{
        AdminAuditEntry, AdminCertificateSummary, AdminEvent, AdminListQuery, AdminOrgMember,
        AdminOrganization, AdminParticipant, AdminPointTransaction, AdminQrStatus, AdminService,
        AdminStore, AdminUser, AdminUserAchievement, AdminUserActivity, Page,
    },
    auth::{
        models::{AuditEntry, RefreshTokenRecord, RequestContext, User},
        ports::AuthStore,
        AuthService, PasswordHasherService, TokenService,
    },
    events::EventImpact,
    impact::ImpactStats,
    AppError, AppResult, HealthProbe,
};
use ecoquest_domain::{DomainError, EventStatus, Role, UserStatus, VerificationStatus};
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

    /// Inserts a user with an explicit role, used to mint administrator sessions.
    pub fn insert_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: Role,
    ) -> User {
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
        self.lock().users.push(user.clone());
        user
    }
}

/// In-memory administration store for HTTP tests.
#[derive(Default)]
pub struct MemoryAdminStore {
    orgs: Mutex<Vec<AdminOrganization>>,
    users: Mutex<Vec<AdminUser>>,
    events: Mutex<Vec<AdminEvent>>,
    audits: Mutex<Vec<String>>,
}

impl MemoryAdminStore {
    pub fn seed_org(&self, org: AdminOrganization) {
        self.orgs.lock().unwrap().push(org);
    }

    pub fn seed_user(&self, user: AdminUser) {
        self.users.lock().unwrap().push(user);
    }

    pub fn seed_event(&self, event: AdminEvent) {
        self.events.lock().unwrap().push(event);
    }

    pub fn audit_actions(&self) -> Vec<String> {
        self.audits.lock().unwrap().clone()
    }
}

fn empty_impact() -> ImpactStats {
    ImpactStats {
        verified_activities: 0,
        active_participants: 0,
        approved_organizations: 0,
        certificates_issued: 0,
        metrics: vec![],
    }
}

fn page<T: Clone>(items: Vec<T>, query: &AdminListQuery) -> Page<T> {
    let total = items.len() as i64;
    let start = query.offset() as usize;
    let end = start.saturating_add(query.per_page as usize);
    Page {
        items: items
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect(),
        total,
        page: query.page,
        per_page: query.per_page,
    }
}

#[async_trait::async_trait]
impl AdminStore for MemoryAdminStore {
    async fn list_organizations(
        &self,
        query: &AdminListQuery,
    ) -> AppResult<Page<AdminOrganization>> {
        Ok(page(self.orgs.lock().unwrap().clone(), query))
    }
    async fn find_organization(&self, id: Uuid) -> AppResult<Option<AdminOrganization>> {
        Ok(self
            .orgs
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.id == id)
            .cloned())
    }
    async fn set_organization_status(
        &self,
        id: Uuid,
        status: VerificationStatus,
        _reviewer: Uuid,
        reason: &str,
    ) -> AppResult<bool> {
        let mut orgs = self.orgs.lock().unwrap();
        let Some(org) = orgs.iter_mut().find(|o| o.id == id) else {
            return Ok(false);
        };
        org.verification_status = status;
        org.review_reason = Some(reason.to_string());
        Ok(true)
    }
    async fn organization_events(&self, _id: Uuid) -> AppResult<Vec<AdminEvent>> {
        Ok(vec![])
    }
    async fn organization_members(&self, _id: Uuid) -> AppResult<Vec<AdminOrgMember>> {
        Ok(vec![])
    }
    async fn organization_certificates(
        &self,
        _id: Uuid,
    ) -> AppResult<Vec<AdminCertificateSummary>> {
        Ok(vec![])
    }
    async fn organization_impact(&self, _id: Uuid) -> AppResult<ImpactStats> {
        Ok(empty_impact())
    }
    async fn list_users(&self, query: &AdminListQuery) -> AppResult<Page<AdminUser>> {
        Ok(page(self.users.lock().unwrap().clone(), query))
    }
    async fn find_user(&self, id: Uuid) -> AppResult<Option<AdminUser>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned())
    }
    async fn set_user_role(&self, id: Uuid, role: Role) -> AppResult<bool> {
        let mut users = self.users.lock().unwrap();
        let Some(user) = users.iter_mut().find(|u| u.id == id) else {
            return Ok(false);
        };
        user.role = role;
        Ok(true)
    }
    async fn set_user_status(&self, id: Uuid, status: UserStatus, reason: &str) -> AppResult<bool> {
        let mut users = self.users.lock().unwrap();
        let Some(user) = users.iter_mut().find(|u| u.id == id) else {
            return Ok(false);
        };
        user.status = status;
        user.status_reason = Some(reason.to_string());
        Ok(true)
    }
    async fn revoke_all_user_tokens(&self, _id: Uuid) -> AppResult<u64> {
        Ok(0)
    }
    async fn user_activities(&self, _id: Uuid) -> AppResult<Vec<AdminUserActivity>> {
        Ok(vec![])
    }
    async fn user_certificates(&self, _id: Uuid) -> AppResult<Vec<AdminCertificateSummary>> {
        Ok(vec![])
    }
    async fn user_achievements(&self, _id: Uuid) -> AppResult<Vec<AdminUserAchievement>> {
        Ok(vec![])
    }
    async fn user_ledger(&self, _id: Uuid) -> AppResult<Vec<AdminPointTransaction>> {
        Ok(vec![])
    }
    async fn apply_point_correction(
        &self,
        id: Uuid,
        amount: i32,
        _reason: &str,
        _by: Uuid,
    ) -> AppResult<Option<i32>> {
        let mut users = self.users.lock().unwrap();
        let Some(user) = users.iter_mut().find(|u| u.id == id) else {
            return Ok(None);
        };
        let next = user.eco_points + amount;
        if next < 0 {
            return Ok(None);
        }
        user.eco_points = next;
        Ok(Some(next))
    }
    async fn insert_password_reset(
        &self,
        _id: Uuid,
        _hash: &[u8],
        _by: Uuid,
        _exp: DateTime<Utc>,
    ) -> AppResult<()> {
        Ok(())
    }
    async fn list_events(&self, query: &AdminListQuery) -> AppResult<Page<AdminEvent>> {
        Ok(page(self.events.lock().unwrap().clone(), query))
    }
    async fn find_event(&self, id: Uuid) -> AppResult<Option<AdminEvent>> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned())
    }
    async fn event_impacts(&self, _id: Uuid) -> AppResult<Vec<EventImpact>> {
        Ok(vec![])
    }
    async fn event_participants(&self, _id: Uuid) -> AppResult<Vec<AdminParticipant>> {
        Ok(vec![])
    }
    async fn event_qr_status(&self, _id: Uuid) -> AppResult<AdminQrStatus> {
        Ok(AdminQrStatus {
            has_active: false,
            activates_at: None,
            expires_at: None,
            revoked_at: None,
        })
    }
    async fn event_impact(&self, _id: Uuid) -> AppResult<ImpactStats> {
        Ok(empty_impact())
    }
    async fn set_event_status(
        &self,
        id: Uuid,
        status: EventStatus,
        previous: Option<EventStatus>,
        _by: Uuid,
        reason: &str,
    ) -> AppResult<bool> {
        let mut events = self.events.lock().unwrap();
        let Some(event) = events.iter_mut().find(|e| e.id == id) else {
            return Ok(false);
        };
        event.previous_status = previous;
        event.status = status;
        event.moderation_reason = Some(reason.to_string());
        Ok(true)
    }
    async fn revoke_event_qr_tokens(&self, _id: Uuid) -> AppResult<()> {
        Ok(())
    }
    async fn flag_participation(
        &self,
        _id: Uuid,
        _flagged: bool,
        _reason: &str,
        _by: Uuid,
    ) -> AppResult<bool> {
        Ok(true)
    }
    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()> {
        self.audits.lock().unwrap().push(entry.action);
        Ok(())
    }
    async fn list_audit(&self, _ty: &str, _id: Uuid) -> AppResult<Vec<AdminAuditEntry>> {
        Ok(vec![])
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

/// Builds a router with administration attached to an in-memory store.
pub fn test_admin_app() -> (axum::Router, Arc<MemoryStore>, Arc<MemoryAdminStore>) {
    test_admin_app_with_base("http://localhost:5173")
}

/// Same as [`test_admin_app`] but with an explicit public web origin, so tests can
/// assert on the links the API hands to users.
pub fn test_admin_app_with_base(
    web_base_url: &str,
) -> (axum::Router, Arc<MemoryStore>, Arc<MemoryAdminStore>) {
    let (state, store) = test_state(100, 900);
    let admin_store = Arc::new(MemoryAdminStore::default());
    let state = state
        .with_admin(Arc::new(AdminService::new(admin_store.clone())))
        .with_web_base_url(web_base_url);
    (
        router(state, &["http://localhost:5173".to_string()]),
        store,
        admin_store,
    )
}

pub fn sample_organization(id: Uuid) -> AdminOrganization {
    AdminOrganization {
        id,
        name: "Alex Greenworks".into(),
        organization_type: "NON_PROFIT".into(),
        location: "Quezon City".into(),
        description: "Awaiting review".into(),
        verification_status: VerificationStatus::Pending,
        owner_id: None,
        owner_username: Some("alex".into()),
        event_count: 0,
        review_reason: None,
        supporting_documents: serde_json::json!([]),
        reviewed_at: None,
        created_at: Utc::now(),
    }
}

pub fn sample_event(id: Uuid) -> AdminEvent {
    AdminEvent {
        id,
        organization_id: Uuid::new_v4(),
        organization_name: "Ben Environmental Org".into(),
        owner_username: "ben".into(),
        name: "Manila Bay Coastal Cleanup".into(),
        description: String::new(),
        activity_type: "BEACH_CLEANUP".into(),
        location: "Manila Bay".into(),
        starts_at: Utc::now() + Duration::days(1),
        ends_at: Utc::now() + Duration::days(1) + Duration::hours(3),
        capacity: 80,
        eco_points: 1000,
        status: EventStatus::Published,
        previous_status: None,
        registered_count: 1,
        flagged_count: 0,
        cancelled_by: None,
        cancelled_at: None,
        cancellation_reason: None,
        moderation_reason: None,
        created_at: Utc::now(),
    }
}

pub fn sample_admin_user(id: Uuid, username: &str, role: Role) -> AdminUser {
    AdminUser {
        id,
        username: username.into(),
        email: format!("{username}@ecoquest.test"),
        role,
        status: UserStatus::Active,
        eco_points: 0,
        wallet_address: None,
        status_reason: None,
        created_at: Utc::now(),
    }
}
