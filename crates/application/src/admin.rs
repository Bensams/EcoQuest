//! Platform administration use cases.

use chrono::{DateTime, Utc};
use ecoquest_domain::{
    DomainError, EventStatus, ParticipationStatus, Role, UserStatus, VerificationStatus,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::models::AuditEntry, events::EventImpact, impact::ImpactStats, AppError, AppResult,
};

/// Shared list query for organizations, events and users.
#[derive(Debug, Clone, Default)]
pub struct AdminListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub organization_id: Option<Uuid>,
    pub activity_type: Option<String>,
    pub location: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub flagged: Option<bool>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

impl AdminListQuery {
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.page == 0 {
            self.page = 1;
        }
        if self.per_page == 0 || self.per_page > 100 {
            self.per_page = 20;
        }
        self
    }

    #[must_use]
    pub fn offset(&self) -> i64 {
        i64::from(self.page.saturating_sub(1)) * i64::from(self.per_page)
    }

    #[must_use]
    pub fn descending(&self) -> bool {
        !matches!(self.order.as_deref(), Some("asc" | "ASC"))
    }
}

/// Paginated list envelope.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// Organization row as listed on the admin dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct AdminOrganization {
    pub id: Uuid,
    pub name: String,
    pub organization_type: String,
    pub location: String,
    pub description: String,
    pub verification_status: VerificationStatus,
    pub owner_id: Option<Uuid>,
    pub owner_username: Option<String>,
    pub event_count: i64,
    pub review_reason: Option<String>,
    pub supporting_documents: serde_json::Value,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User row as listed on the admin dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub status: UserStatus,
    pub eco_points: i32,
    pub wallet_address: Option<String>,
    pub status_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Event row as listed on the admin moderation dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct AdminEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub owner_username: String,
    pub name: String,
    pub description: String,
    pub activity_type: String,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    pub status: EventStatus,
    pub previous_status: Option<EventStatus>,
    pub registered_count: i64,
    pub flagged_count: i64,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub moderation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Organization owner treated as the sole member after membership was retired.
#[derive(Debug, Clone, Serialize)]
pub struct AdminOrgMember {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub member_role: String,
}

/// Certificate summary on an organization or user profile.
#[derive(Debug, Clone, Serialize)]
pub struct AdminCertificateSummary {
    pub certificate_number: String,
    pub participation_id: Uuid,
    pub event_name: String,
    pub participant_name: String,
    pub issued_at: DateTime<Utc>,
    pub status: String,
}

/// One append-only audit row.
#[derive(Debug, Clone, Serialize)]
pub struct AdminAuditEntry {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Organization profile for administrators.
#[derive(Debug, Clone, Serialize)]
pub struct AdminOrganizationDetail {
    pub organization: AdminOrganization,
    pub events: Vec<AdminEvent>,
    pub members: Vec<AdminOrgMember>,
    pub certificates: Vec<AdminCertificateSummary>,
    pub impact: ImpactStats,
    pub audit: Vec<AdminAuditEntry>,
}

/// Live QR state; the raw code is never stored, so only metadata is returned.
#[derive(Debug, Clone, Serialize)]
pub struct AdminQrStatus {
    pub has_active: bool,
    pub activates_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Participant row on an event, including flags and attendance.
#[derive(Debug, Clone, Serialize)]
pub struct AdminParticipant {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub status: ParticipationStatus,
    pub registered_at: DateTime<Utc>,
    pub checked_in_at: Option<DateTime<Utc>>,
    pub flagged: bool,
    pub flag_reason: Option<String>,
    pub points_awarded: i32,
}

/// Event profile for administrators.
#[derive(Debug, Clone, Serialize)]
pub struct AdminEventDetail {
    pub event: AdminEvent,
    pub impacts: Vec<EventImpact>,
    pub participants: Vec<AdminParticipant>,
    pub qr: AdminQrStatus,
    pub impact: ImpactStats,
    pub audit: Vec<AdminAuditEntry>,
}

/// Point ledger row.
#[derive(Debug, Clone, Serialize)]
pub struct AdminPointTransaction {
    pub id: Uuid,
    pub amount: i32,
    pub reason: String,
    pub participation_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// User activity row.
#[derive(Debug, Clone, Serialize)]
pub struct AdminUserActivity {
    pub participation_id: Uuid,
    pub event_id: Uuid,
    pub event_name: String,
    pub status: ParticipationStatus,
    pub registered_at: DateTime<Utc>,
    pub checked_in_at: Option<DateTime<Utc>>,
    pub points_awarded: i32,
}

/// Achievement row on a user profile.
#[derive(Debug, Clone, Serialize)]
pub struct AdminUserAchievement {
    pub achievement_key: String,
    pub status: String,
    pub verification_reference: String,
}

/// User profile for administrators.
#[derive(Debug, Clone, Serialize)]
pub struct AdminUserDetail {
    pub user: AdminUser,
    pub activities: Vec<AdminUserActivity>,
    pub certificates: Vec<AdminCertificateSummary>,
    pub achievements: Vec<AdminUserAchievement>,
    pub ledger: Vec<AdminPointTransaction>,
    pub audit: Vec<AdminAuditEntry>,
}

/// One-time password reset link. The token is shown once and never stored raw.
#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetLink {
    pub user_id: Uuid,
    pub reset_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Result of a ledger correction.
#[derive(Debug, Clone, Serialize)]
pub struct PointCorrection {
    pub user_id: Uuid,
    pub amount: i32,
    pub eco_points: i32,
}

#[async_trait::async_trait]
pub trait AdminStore: Send + Sync {
    async fn list_organizations(
        &self,
        query: &AdminListQuery,
    ) -> AppResult<Page<AdminOrganization>>;
    async fn find_organization(
        &self,
        organization_id: Uuid,
    ) -> AppResult<Option<AdminOrganization>>;
    async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
        reason: &str,
    ) -> AppResult<bool>;
    async fn organization_events(&self, organization_id: Uuid) -> AppResult<Vec<AdminEvent>>;
    async fn organization_members(&self, organization_id: Uuid) -> AppResult<Vec<AdminOrgMember>>;
    async fn organization_certificates(
        &self,
        organization_id: Uuid,
    ) -> AppResult<Vec<AdminCertificateSummary>>;
    async fn organization_impact(&self, organization_id: Uuid) -> AppResult<ImpactStats>;

    async fn list_users(&self, query: &AdminListQuery) -> AppResult<Page<AdminUser>>;
    async fn find_user(&self, user_id: Uuid) -> AppResult<Option<AdminUser>>;
    async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<bool>;
    async fn set_user_status(
        &self,
        user_id: Uuid,
        status: UserStatus,
        reason: &str,
    ) -> AppResult<bool>;
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<u64>;
    async fn user_activities(&self, user_id: Uuid) -> AppResult<Vec<AdminUserActivity>>;
    async fn user_certificates(&self, user_id: Uuid) -> AppResult<Vec<AdminCertificateSummary>>;
    async fn user_achievements(&self, user_id: Uuid) -> AppResult<Vec<AdminUserAchievement>>;
    async fn user_ledger(&self, user_id: Uuid) -> AppResult<Vec<AdminPointTransaction>>;
    async fn apply_point_correction(
        &self,
        user_id: Uuid,
        amount: i32,
        reason: &str,
        created_by: Uuid,
    ) -> AppResult<Option<i32>>;
    async fn insert_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        created_by: Uuid,
        expires_at: DateTime<Utc>,
    ) -> AppResult<()>;

    async fn list_events(&self, query: &AdminListQuery) -> AppResult<Page<AdminEvent>>;
    async fn find_event(&self, event_id: Uuid) -> AppResult<Option<AdminEvent>>;
    async fn event_impacts(&self, event_id: Uuid) -> AppResult<Vec<EventImpact>>;
    async fn event_participants(&self, event_id: Uuid) -> AppResult<Vec<AdminParticipant>>;
    async fn event_qr_status(&self, event_id: Uuid) -> AppResult<AdminQrStatus>;
    async fn event_impact(&self, event_id: Uuid) -> AppResult<ImpactStats>;
    async fn set_event_status(
        &self,
        event_id: Uuid,
        status: EventStatus,
        previous: Option<EventStatus>,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<bool>;
    async fn revoke_event_qr_tokens(&self, event_id: Uuid) -> AppResult<()>;
    async fn flag_participation(
        &self,
        participation_id: Uuid,
        flagged: bool,
        reason: &str,
        actor_id: Uuid,
    ) -> AppResult<bool>;

    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()>;
    async fn list_audit(
        &self,
        entity_type: &str,
        entity_id: Uuid,
    ) -> AppResult<Vec<AdminAuditEntry>>;
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

    pub async fn list_organizations(
        &self,
        query: AdminListQuery,
    ) -> AppResult<Page<AdminOrganization>> {
        self.store.list_organizations(&query.normalized()).await
    }

    pub async fn organization_detail(
        &self,
        organization_id: Uuid,
    ) -> AppResult<AdminOrganizationDetail> {
        let organization = self
            .store
            .find_organization(organization_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("organization".into())))?;
        Ok(AdminOrganizationDetail {
            events: self.store.organization_events(organization_id).await?,
            members: self.store.organization_members(organization_id).await?,
            certificates: self
                .store
                .organization_certificates(organization_id)
                .await?,
            impact: self.store.organization_impact(organization_id).await?,
            audit: self
                .store
                .list_audit("organization", organization_id)
                .await?,
            organization,
        })
    }

    pub async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
        reason: &str,
    ) -> AppResult<AdminOrganization> {
        let reason = require_reason(reason, "a review reason is required")?;
        let current = self
            .store
            .find_organization(organization_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("organization".into())))?;
        validate_org_transition(current.verification_status, status)?;
        if !self
            .store
            .set_organization_status(organization_id, status, reviewer_id, reason)
            .await?
        {
            return Err(AppError::Domain(DomainError::NotFound(
                "organization".into(),
            )));
        }
        self.audit(
            reviewer_id,
            match status {
                VerificationStatus::Approved => "organization.approved",
                VerificationStatus::Rejected => "organization.rejected",
                VerificationStatus::Suspended => "organization.suspended",
                VerificationStatus::Inactive => "organization.inactivated",
                VerificationStatus::Pending => "organization.reopened",
            },
            "organization",
            organization_id,
            serde_json::json!({
                "reason": reason,
                "previous": current.verification_status.as_str(),
                "new": status.as_str(),
            }),
        )
        .await;
        self.store
            .find_organization(organization_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("organization".into())))
    }

    pub async fn list_users(&self, query: AdminListQuery) -> AppResult<Page<AdminUser>> {
        self.store.list_users(&query.normalized()).await
    }

    pub async fn user_detail(&self, user_id: Uuid) -> AppResult<AdminUserDetail> {
        let user = self
            .store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        Ok(AdminUserDetail {
            activities: self.store.user_activities(user_id).await?,
            certificates: self.store.user_certificates(user_id).await?,
            achievements: self.store.user_achievements(user_id).await?,
            ledger: self.store.user_ledger(user_id).await?,
            audit: self.store.list_audit("user", user_id).await?,
            user,
        })
    }

    pub async fn set_user_role(
        &self,
        user_id: Uuid,
        role: Role,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<AdminUser> {
        let reason = require_reason(reason, "a reason is required to change a role")?;
        if user_id == actor_id {
            return Err(AppError::Domain(DomainError::Forbidden(
                "you cannot change your own role".into(),
            )));
        }
        let current = self
            .store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        if current.role == role {
            return Err(AppError::Domain(DomainError::Conflict(
                "user already has that role".into(),
            )));
        }
        if !self.store.set_user_role(user_id, role).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        let _ = self.store.revoke_all_user_tokens(user_id).await?;
        self.audit(
            actor_id,
            "user.role_changed",
            "user",
            user_id,
            serde_json::json!({
                "reason": reason,
                "previous": current.role.as_str(),
                "new": role.as_str(),
            }),
        )
        .await;
        self.store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))
    }

    pub async fn set_user_status(
        &self,
        user_id: Uuid,
        status: UserStatus,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<AdminUser> {
        let reason = require_reason(reason, "a reason is required to change account status")?;
        if user_id == actor_id {
            return Err(AppError::Domain(DomainError::Forbidden(
                "you cannot change your own account status".into(),
            )));
        }
        let current = self
            .store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        validate_user_transition(current.status, status)?;
        if !self.store.set_user_status(user_id, status, reason).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        if !status.can_authenticate() {
            let _ = self.store.revoke_all_user_tokens(user_id).await?;
        }
        self.audit(
            actor_id,
            match status {
                UserStatus::Active => "user.reactivated",
                UserStatus::Suspended => "user.suspended",
                UserStatus::Deleted => "user.deactivated",
            },
            "user",
            user_id,
            serde_json::json!({
                "reason": reason,
                "previous": current.status.as_str(),
                "new": status.as_str(),
            }),
        )
        .await;
        self.store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))
    }

    pub async fn correct_points(
        &self,
        user_id: Uuid,
        amount: i32,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<PointCorrection> {
        let reason = require_reason(reason, "a reason is required for a point correction")?;
        if amount == 0 {
            return Err(AppError::Domain(DomainError::Validation(
                "correction amount must not be zero".into(),
            )));
        }
        let current = self
            .store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        let Some(eco_points) = self
            .store
            .apply_point_correction(user_id, amount, reason, actor_id)
            .await?
        else {
            return Err(AppError::Domain(DomainError::Conflict(
                "correction would make the Eco Point balance negative".into(),
            )));
        };
        self.audit(
            actor_id,
            "user.points_corrected",
            "user",
            user_id,
            serde_json::json!({
                "reason": reason,
                "previous": current.eco_points,
                "new": eco_points,
                "amount": amount,
            }),
        )
        .await;
        Ok(PointCorrection {
            user_id,
            amount,
            eco_points,
        })
    }

    pub async fn issue_password_reset(
        &self,
        user_id: Uuid,
        actor_id: Uuid,
        public_base: &str,
    ) -> AppResult<PasswordResetLink> {
        let user = self
            .store
            .find_user(user_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("user".into())))?;
        if !user.status.can_authenticate() {
            return Err(AppError::Domain(DomainError::Conflict(
                "cannot reset a password for a non-active account".into(),
            )));
        }
        let token = Uuid::new_v4().simple().to_string() + &Uuid::new_v4().simple().to_string();
        let hash = sha2_bytes(&token);
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        self.store
            .insert_password_reset(user_id, &hash, actor_id, expires_at)
            .await?;
        self.audit(
            actor_id,
            "user.password_reset_issued",
            "user",
            user_id,
            serde_json::json!({ "expires_at": expires_at }),
        )
        .await;
        let base = public_base.trim_end_matches('/');
        Ok(PasswordResetLink {
            user_id,
            reset_url: format!("{base}/reset-password?token={token}"),
            expires_at,
        })
    }

    pub async fn list_events(&self, query: AdminListQuery) -> AppResult<Page<AdminEvent>> {
        self.store.list_events(&query.normalized()).await
    }

    pub async fn event_detail(&self, event_id: Uuid) -> AppResult<AdminEventDetail> {
        let event = self
            .store
            .find_event(event_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))?;
        Ok(AdminEventDetail {
            impacts: self.store.event_impacts(event_id).await?,
            participants: self.store.event_participants(event_id).await?,
            qr: self.store.event_qr_status(event_id).await?,
            impact: self.store.event_impact(event_id).await?,
            audit: self.store.list_audit("event", event_id).await?,
            event,
        })
    }

    pub async fn moderate_event(
        &self,
        event_id: Uuid,
        action: EventModeration,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<AdminEvent> {
        let reason = require_reason(reason, "a reason is required to change an event")?;
        let event = self
            .store
            .find_event(event_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))?;
        let (next, previous, audit_action) = match action {
            EventModeration::Cancel => {
                if matches!(
                    event.status,
                    EventStatus::Cancelled | EventStatus::Completed | EventStatus::Archived
                ) {
                    return Err(AppError::Domain(DomainError::Conflict(
                        "event cannot be cancelled".into(),
                    )));
                }
                (
                    EventStatus::Cancelled,
                    Some(event.status),
                    "event.cancelled",
                )
            }
            EventModeration::Suspend => {
                if !matches!(event.status, EventStatus::Published | EventStatus::Active) {
                    return Err(AppError::Domain(DomainError::Conflict(
                        "only published or active events can be suspended".into(),
                    )));
                }
                (
                    EventStatus::Suspended,
                    Some(event.status),
                    "event.suspended",
                )
            }
            EventModeration::Restore => {
                if event.status != EventStatus::Suspended {
                    return Err(AppError::Domain(DomainError::Conflict(
                        "only a suspended event can be restored".into(),
                    )));
                }
                let restored = event.previous_status.unwrap_or(EventStatus::Published);
                (restored, Some(EventStatus::Suspended), "event.restored")
            }
            EventModeration::Archive => {
                if !matches!(
                    event.status,
                    EventStatus::Cancelled | EventStatus::Completed | EventStatus::Suspended
                ) {
                    return Err(AppError::Domain(DomainError::Conflict(
                        "only cancelled, completed or suspended events can be archived".into(),
                    )));
                }
                (EventStatus::Archived, Some(event.status), "event.archived")
            }
        };
        if !self
            .store
            .set_event_status(event_id, next, previous, actor_id, reason)
            .await?
        {
            return Err(AppError::Domain(DomainError::Conflict(
                "event state changed; retry".into(),
            )));
        }
        if matches!(
            next,
            EventStatus::Cancelled | EventStatus::Suspended | EventStatus::Archived
        ) {
            self.store.revoke_event_qr_tokens(event_id).await?;
        }
        self.audit(
            actor_id,
            audit_action,
            "event",
            event_id,
            serde_json::json!({
                "reason": reason,
                "previous": event.status.as_str(),
                "new": next.as_str(),
            }),
        )
        .await;
        self.store
            .find_event(event_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))
    }

    pub async fn flag_participation(
        &self,
        participation_id: Uuid,
        flagged: bool,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<()> {
        let reason = require_reason(reason, "a reason is required to flag participation")?;
        if !self
            .store
            .flag_participation(participation_id, flagged, reason, actor_id)
            .await?
        {
            return Err(AppError::Domain(DomainError::NotFound(
                "participation".into(),
            )));
        }
        self.audit(
            actor_id,
            if flagged {
                "participation.flagged"
            } else {
                "participation.unflagged"
            },
            "participation",
            participation_id,
            serde_json::json!({ "reason": reason, "flagged": flagged }),
        )
        .await;
        Ok(())
    }

    async fn audit(
        &self,
        actor_id: Uuid,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        metadata: serde_json::Value,
    ) {
        let entry = AuditEntry {
            actor_id: Some(actor_id),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: Some(entity_id),
            metadata,
            ip_address: None,
        };
        if let Err(err) = self.store.record_audit(entry).await {
            tracing::error!(error = %err, action, "failed to write admin audit entry");
        }
    }
}

/// Administrator event lifecycle action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventModeration {
    Cancel,
    Suspend,
    Restore,
    Archive,
}

fn require_reason<'a>(reason: &'a str, message: &str) -> AppResult<&'a str> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(AppError::Domain(DomainError::Validation(message.into())));
    }
    Ok(trimmed)
}

fn validate_org_transition(from: VerificationStatus, to: VerificationStatus) -> AppResult<()> {
    let ok = matches!(
        (from, to),
        (
            VerificationStatus::Pending,
            VerificationStatus::Approved | VerificationStatus::Rejected
        ) | (
            VerificationStatus::Approved,
            VerificationStatus::Suspended | VerificationStatus::Inactive
        ) | (
            VerificationStatus::Suspended,
            VerificationStatus::Approved | VerificationStatus::Inactive
        ) | (VerificationStatus::Rejected, VerificationStatus::Pending)
            | (VerificationStatus::Inactive, VerificationStatus::Approved)
    );
    if ok {
        Ok(())
    } else {
        Err(AppError::Domain(DomainError::Conflict(format!(
            "cannot change organization from {} to {}",
            from.as_str(),
            to.as_str()
        ))))
    }
}

fn validate_user_transition(from: UserStatus, to: UserStatus) -> AppResult<()> {
    let ok = matches!(
        (from, to),
        (
            UserStatus::Active,
            UserStatus::Suspended | UserStatus::Deleted
        ) | (
            UserStatus::Suspended,
            UserStatus::Active | UserStatus::Deleted
        )
    );
    if ok {
        Ok(())
    } else {
        Err(AppError::Domain(DomainError::Conflict(format!(
            "cannot change account from {} to {}",
            from.as_str(),
            to.as_str()
        ))))
    }
}

fn sha2_bytes(value: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(value.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn empty_impact() -> ImpactStats {
        ImpactStats {
            verified_activities: 0,
            active_participants: 0,
            approved_organizations: 0,
            certificates_issued: 0,
            metrics: vec![],
        }
    }

    fn sample_event() -> AdminEvent {
        AdminEvent {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            organization_name: "Green Earth Butuan".into(),
            owner_username: "eco_organizer".into(),
            name: "Coastal Cleanup".into(),
            description: String::new(),
            activity_type: "BEACH_CLEANUP".into(),
            location: "Butuan Bay".into(),
            starts_at: Utc::now(),
            ends_at: Utc::now() + chrono::Duration::hours(2),
            capacity: 50,
            eco_points: 100,
            status: EventStatus::Draft,
            previous_status: None,
            registered_count: 0,
            flagged_count: 0,
            cancelled_by: None,
            cancelled_at: None,
            cancellation_reason: None,
            moderation_reason: None,
            created_at: Utc::now(),
        }
    }

    struct RecordingStore {
        cancelled: Mutex<Vec<(Uuid, Uuid, String)>>,
        audits: Mutex<Vec<String>>,
        event: Mutex<AdminEvent>,
        org: Mutex<Option<AdminOrganization>>,
        user: Mutex<Option<AdminUser>>,
    }

    impl Default for RecordingStore {
        fn default() -> Self {
            Self {
                cancelled: Mutex::new(vec![]),
                audits: Mutex::new(vec![]),
                event: Mutex::new(sample_event()),
                org: Mutex::new(None),
                user: Mutex::new(None),
            }
        }
    }

    impl RecordingStore {
        fn with_event(event: AdminEvent) -> Self {
            Self {
                event: Mutex::new(event),
                ..Self::default()
            }
        }
    }

    #[async_trait::async_trait]
    impl AdminStore for RecordingStore {
        async fn list_organizations(
            &self,
            _query: &AdminListQuery,
        ) -> AppResult<Page<AdminOrganization>> {
            Ok(Page {
                items: vec![],
                total: 0,
                page: 1,
                per_page: 20,
            })
        }
        async fn find_organization(&self, _id: Uuid) -> AppResult<Option<AdminOrganization>> {
            Ok(self.org.lock().unwrap().clone())
        }
        async fn set_organization_status(
            &self,
            _id: Uuid,
            status: VerificationStatus,
            _reviewer: Uuid,
            _reason: &str,
        ) -> AppResult<bool> {
            if let Some(org) = self.org.lock().unwrap().as_mut() {
                org.verification_status = status;
                return Ok(true);
            }
            Ok(false)
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
        async fn list_users(&self, _query: &AdminListQuery) -> AppResult<Page<AdminUser>> {
            Ok(Page {
                items: vec![],
                total: 0,
                page: 1,
                per_page: 20,
            })
        }
        async fn find_user(&self, _id: Uuid) -> AppResult<Option<AdminUser>> {
            Ok(self.user.lock().unwrap().clone())
        }
        async fn set_user_role(&self, _id: Uuid, role: Role) -> AppResult<bool> {
            if let Some(user) = self.user.lock().unwrap().as_mut() {
                user.role = role;
                return Ok(true);
            }
            Ok(false)
        }
        async fn set_user_status(
            &self,
            _id: Uuid,
            status: UserStatus,
            _reason: &str,
        ) -> AppResult<bool> {
            if let Some(user) = self.user.lock().unwrap().as_mut() {
                user.status = status;
                return Ok(true);
            }
            Ok(false)
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
            _id: Uuid,
            amount: i32,
            _reason: &str,
            _by: Uuid,
        ) -> AppResult<Option<i32>> {
            let mut guard = self.user.lock().unwrap();
            let Some(user) = guard.as_mut() else {
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
        async fn list_events(&self, _query: &AdminListQuery) -> AppResult<Page<AdminEvent>> {
            Ok(Page {
                items: vec![self.event.lock().unwrap().clone()],
                total: 1,
                page: 1,
                per_page: 20,
            })
        }
        async fn find_event(&self, _id: Uuid) -> AppResult<Option<AdminEvent>> {
            Ok(Some(self.event.lock().unwrap().clone()))
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
            event_id: Uuid,
            status: EventStatus,
            _previous: Option<EventStatus>,
            actor_id: Uuid,
            reason: &str,
        ) -> AppResult<bool> {
            let mut event = self.event.lock().unwrap();
            event.status = status;
            event.moderation_reason = Some(reason.to_string());
            self.cancelled
                .lock()
                .unwrap()
                .push((event_id, actor_id, reason.to_string()));
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

    #[tokio::test]
    async fn cancel_requires_a_reason() {
        let service = AdminService::new(std::sync::Arc::new(RecordingStore::with_event(
            sample_event(),
        )));
        let err = service
            .moderate_event(
                Uuid::new_v4(),
                EventModeration::Cancel,
                Uuid::new_v4(),
                "   ",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn cancel_records_who_and_why() {
        let store = std::sync::Arc::new(RecordingStore::with_event(sample_event()));
        let service = AdminService::new(store.clone());
        let event_id = store.event.lock().unwrap().id;
        let admin_id = Uuid::new_v4();
        service
            .moderate_event(
                event_id,
                EventModeration::Cancel,
                admin_id,
                "unsafe location",
            )
            .await
            .expect("cancel should succeed");
        let calls = store.cancelled.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, admin_id);
        assert_eq!(calls[0].2, "unsafe location");
        assert!(store
            .audits
            .lock()
            .unwrap()
            .iter()
            .any(|a| a == "event.cancelled"));
    }

    #[tokio::test]
    async fn cancelled_or_completed_events_cannot_be_cancelled() {
        let mut event = sample_event();
        event.status = EventStatus::Cancelled;
        let service = AdminService::new(std::sync::Arc::new(RecordingStore::with_event(event)));
        let err = service
            .moderate_event(
                Uuid::new_v4(),
                EventModeration::Cancel,
                Uuid::new_v4(),
                "nope",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Conflict(_))));
    }

    #[tokio::test]
    async fn org_reject_requires_reason_and_valid_transition() {
        let store = RecordingStore::default();
        *store.org.lock().unwrap() = Some(AdminOrganization {
            id: Uuid::new_v4(),
            name: "Alex Greenworks".into(),
            organization_type: "NON_PROFIT".into(),
            location: "QC".into(),
            description: String::new(),
            verification_status: VerificationStatus::Pending,
            owner_id: None,
            owner_username: None,
            event_count: 0,
            review_reason: None,
            supporting_documents: serde_json::json!([]),
            reviewed_at: None,
            created_at: Utc::now(),
        });
        let service = AdminService::new(std::sync::Arc::new(store));
        let err = service
            .set_organization_status(
                Uuid::new_v4(),
                VerificationStatus::Rejected,
                Uuid::new_v4(),
                "",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn point_correction_rejects_zero_and_negative_balance() {
        let store = RecordingStore::default();
        *store.user.lock().unwrap() = Some(AdminUser {
            id: Uuid::new_v4(),
            username: "erwin".into(),
            email: "erwin@ecoquest.test".into(),
            role: Role::User,
            status: UserStatus::Active,
            eco_points: 10,
            wallet_address: None,
            status_reason: None,
            created_at: Utc::now(),
        });
        let service = AdminService::new(std::sync::Arc::new(store));
        let zero = service
            .correct_points(Uuid::new_v4(), 0, Uuid::new_v4(), "adjust")
            .await
            .unwrap_err();
        assert!(matches!(zero, AppError::Domain(DomainError::Validation(_))));
        let neg = service
            .correct_points(Uuid::new_v4(), -50, Uuid::new_v4(), "clawback")
            .await
            .unwrap_err();
        assert!(matches!(neg, AppError::Domain(DomainError::Conflict(_))));
    }

    #[tokio::test]
    async fn admin_cannot_change_own_role() {
        let id = Uuid::new_v4();
        let store = RecordingStore::default();
        *store.user.lock().unwrap() = Some(AdminUser {
            id,
            username: "eco_admin".into(),
            email: "admin@ecoquest.test".into(),
            role: Role::Admin,
            status: UserStatus::Active,
            eco_points: 0,
            wallet_address: None,
            status_reason: None,
            created_at: Utc::now(),
        });
        let service = AdminService::new(std::sync::Arc::new(store));
        let err = service
            .set_user_role(id, Role::User, id, "stepping down")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Forbidden(_))));
    }
}
