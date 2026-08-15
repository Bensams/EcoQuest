//! PostgreSQL implementation of [`AdminStore`].

use chrono::{DateTime, Utc};
use ecoquest_application::{
    admin::{
        AdminAuditEntry, AdminCertificateSummary, AdminEvent, AdminListQuery, AdminOrgMember,
        AdminOrganization, AdminParticipant, AdminPointTransaction, AdminQrStatus, AdminStore,
        AdminUser, AdminUserAchievement, AdminUserActivity, Page,
    },
    auth::models::AuditEntry,
    events::EventImpact,
    impact::ImpactStats,
    AppError, AppResult,
};
use ecoquest_domain::{EventStatus, ParticipationStatus, Role, UserStatus, VerificationStatus};
use sqlx::{postgres::PgRow, QueryBuilder, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::PgStore;

#[derive(Clone, Debug)]
pub struct PgAdminStore {
    pool: sqlx::PgPool,
}

impl PgAdminStore {
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

const ORG_SELECT: &str = "SELECT o.id, o.name, o.organization_type, o.location, o.description, \
     o.verification_status::text AS verification_status, o.reviewed_at, o.created_at, \
     o.owner_id, o.review_reason, o.supporting_documents, \
     (SELECT username FROM users u WHERE u.id = o.owner_id) AS owner_username, \
     (SELECT count(*) FROM events e WHERE e.organization_id = o.id)::bigint AS event_count \
     FROM organizations o";

const EVENT_SELECT: &str = "SELECT e.id, e.organization_id, o.name AS organization_name, \
     COALESCE(u.username, '(no owner)') AS owner_username, \
     e.name, e.description, e.activity_type::text AS activity_type, e.location, \
     e.starts_at, e.ends_at, e.capacity, e.eco_points, e.status::text AS status, \
     e.previous_status::text AS previous_status, \
     (SELECT count(*) FROM participations p WHERE p.event_id=e.id AND p.status <> 'CANCELLED')::bigint AS registered_count, \
     (SELECT count(*) FROM participations p WHERE p.event_id=e.id AND p.flagged)::bigint AS flagged_count, \
     e.cancelled_by, e.cancelled_at, e.cancellation_reason, e.moderation_reason, e.created_at \
     FROM events e JOIN organizations o ON o.id=e.organization_id \
     LEFT JOIN users u ON u.id=o.owner_id";

const USER_SELECT: &str =
    "SELECT id, username, email, role::text AS role, status::text AS status, \
     eco_points, wallet_address, status_reason, created_at FROM users";

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
        review_reason: row.try_get("review_reason").map_err(db_err)?,
        supporting_documents: row.try_get("supporting_documents").map_err(db_err)?,
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
        wallet_address: row.try_get("wallet_address").map_err(db_err)?,
        status_reason: row.try_get("status_reason").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

fn event_from_row(row: &PgRow) -> AppResult<AdminEvent> {
    let status: String = row.try_get("status").map_err(db_err)?;
    let previous: Option<String> = row.try_get("previous_status").map_err(db_err)?;
    Ok(AdminEvent {
        id: row.try_get("id").map_err(db_err)?,
        organization_id: row.try_get("organization_id").map_err(db_err)?,
        organization_name: row.try_get("organization_name").map_err(db_err)?,
        owner_username: row.try_get("owner_username").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        activity_type: row.try_get("activity_type").map_err(db_err)?,
        location: row.try_get("location").map_err(db_err)?,
        starts_at: row.try_get("starts_at").map_err(db_err)?,
        ends_at: row.try_get("ends_at").map_err(db_err)?,
        capacity: row.try_get("capacity").map_err(db_err)?,
        eco_points: row.try_get("eco_points").map_err(db_err)?,
        status: EventStatus::from_str(&status).map_err(AppError::Domain)?,
        previous_status: previous
            .map(|value| EventStatus::from_str(&value).map_err(AppError::Domain))
            .transpose()?,
        registered_count: row.try_get("registered_count").map_err(db_err)?,
        flagged_count: row.try_get("flagged_count").map_err(db_err)?,
        cancelled_by: row.try_get("cancelled_by").map_err(db_err)?,
        cancelled_at: row.try_get("cancelled_at").map_err(db_err)?,
        cancellation_reason: row.try_get("cancellation_reason").map_err(db_err)?,
        moderation_reason: row.try_get("moderation_reason").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

fn like(value: &str) -> String {
    format!("%{}%", value.replace('%', "\\%").replace('_', "\\_"))
}

fn org_order(sort: Option<&str>, desc: bool) -> &'static str {
    match (sort, desc) {
        (Some("name"), false) => "o.name ASC",
        (Some("name"), true) => "o.name DESC",
        (Some("status"), false) => "o.verification_status ASC, o.created_at DESC",
        (Some("status"), true) => "o.verification_status DESC, o.created_at DESC",
        (_, false) => "o.created_at ASC",
        (_, true) => "o.created_at DESC",
    }
}

fn user_order(sort: Option<&str>, desc: bool) -> &'static str {
    match (sort, desc) {
        (Some("username"), false) => "username ASC",
        (Some("username"), true) => "username DESC",
        (Some("points"), false) => "eco_points ASC",
        (Some("points"), true) => "eco_points DESC",
        (Some("status"), false) => "status ASC, created_at DESC",
        (Some("status"), true) => "status DESC, created_at DESC",
        (_, false) => "created_at ASC",
        (_, true) => "created_at DESC",
    }
}

fn event_order(sort: Option<&str>, desc: bool) -> &'static str {
    match (sort, desc) {
        (Some("name"), false) => "e.name ASC",
        (Some("name"), true) => "e.name DESC",
        (Some("starts_at"), false) => "e.starts_at ASC",
        (Some("starts_at"), true) => "e.starts_at DESC",
        (Some("status"), false) => "e.status ASC, e.created_at DESC",
        (Some("status"), true) => "e.status DESC, e.created_at DESC",
        (_, false) => "e.created_at ASC",
        (_, true) => "e.created_at DESC",
    }
}

async fn impact_for(pool: &sqlx::PgPool, clause: &str, id: Uuid) -> AppResult<ImpactStats> {
    let row = sqlx::query(&format!(
        "SELECT \
         (SELECT count(*) FROM participations p JOIN events e ON e.id=p.event_id WHERE p.status='VERIFIED' {clause})::bigint AS verified_activities, \
         (SELECT count(DISTINCT p.user_id) FROM participations p JOIN events e ON e.id=p.event_id WHERE p.status='VERIFIED' {clause})::bigint AS active_participants, \
         (SELECT count(*) FROM organizations o WHERE o.verification_status='APPROVED')::bigint AS approved_organizations, \
         (SELECT count(*) FROM certificates c JOIN participations p ON p.id=c.participation_id JOIN events e ON e.id=p.event_id WHERE c.status='VALID' {clause})::bigint AS certificates_issued"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    let metrics = sqlx::query(&format!(
        "SELECT metric, unit, COALESCE(sum(value),0)::float8 AS value \
         FROM impact_contributions ic JOIN events e ON e.id=ic.event_id \
         WHERE 1=1 {clause} GROUP BY metric, unit ORDER BY metric"
    ))
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    Ok(ImpactStats {
        verified_activities: row.try_get("verified_activities").map_err(db_err)?,
        active_participants: row.try_get("active_participants").map_err(db_err)?,
        approved_organizations: row.try_get("approved_organizations").map_err(db_err)?,
        certificates_issued: row.try_get("certificates_issued").map_err(db_err)?,
        metrics: metrics
            .iter()
            .map(|m| {
                Ok(ecoquest_application::impact::MetricTotal {
                    metric: m.try_get("metric").map_err(db_err)?,
                    unit: m.try_get("unit").map_err(db_err)?,
                    value: m.try_get("value").map_err(db_err)?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?,
    })
}

fn certificate_from_row(row: &PgRow) -> AppResult<AdminCertificateSummary> {
    Ok(AdminCertificateSummary {
        certificate_number: row.try_get("certificate_number").map_err(db_err)?,
        participation_id: row.try_get("participation_id").map_err(db_err)?,
        event_name: row.try_get("event_name").map_err(db_err)?,
        participant_name: row.try_get("participant_name").map_err(db_err)?,
        issued_at: row.try_get("issued_at").map_err(db_err)?,
        status: row.try_get("status").map_err(db_err)?,
    })
}

#[async_trait::async_trait]
impl AdminStore for PgAdminStore {
    async fn list_organizations(
        &self,
        query: &AdminListQuery,
    ) -> AppResult<Page<AdminOrganization>> {
        let mut count = QueryBuilder::new("SELECT count(*)::bigint FROM organizations o WHERE 1=1");
        let mut select = QueryBuilder::new(format!("{ORG_SELECT} WHERE 1=1"));
        if let Some(q) = query.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = like(q);
            count.push(" AND (o.name ILIKE ");
            count.push_bind(pattern.clone());
            count.push(" OR o.location ILIKE ");
            count.push_bind(pattern.clone());
            count.push(')');
            select.push(" AND (o.name ILIKE ");
            select.push_bind(pattern.clone());
            select.push(" OR o.location ILIKE ");
            select.push_bind(pattern);
            select.push(')');
        }
        if let Some(status) = query.status.as_deref().filter(|s| !s.is_empty()) {
            count.push(" AND o.verification_status = ");
            count.push_bind(status.to_string());
            count.push("::verification_status");
            select.push(" AND o.verification_status = ");
            select.push_bind(status.to_string());
            select.push("::verification_status");
        }
        let total: i64 = count
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        select.push(" ORDER BY ");
        select.push(org_order(query.sort.as_deref(), query.descending()));
        select.push(" LIMIT ");
        select.push_bind(i64::from(query.per_page));
        select.push(" OFFSET ");
        select.push_bind(query.offset());
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(organization_from_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Page {
            items,
            total,
            page: query.page,
            per_page: query.per_page,
        })
    }

    async fn find_organization(
        &self,
        organization_id: Uuid,
    ) -> AppResult<Option<AdminOrganization>> {
        sqlx::query(&format!("{ORG_SELECT} WHERE o.id=$1"))
            .bind(organization_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(organization_from_row)
            .transpose()
    }

    async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
        reason: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE organizations SET verification_status=$2::verification_status, \
             reviewed_by=$3, reviewed_at=now(), review_reason=$4, updated_at=now() WHERE id=$1",
        )
        .bind(organization_id)
        .bind(status.as_str())
        .bind(reviewer_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn organization_events(&self, organization_id: Uuid) -> AppResult<Vec<AdminEvent>> {
        sqlx::query(&format!(
            "{EVENT_SELECT} WHERE e.organization_id=$1 ORDER BY e.starts_at DESC"
        ))
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(event_from_row)
        .collect()
    }

    async fn organization_members(&self, organization_id: Uuid) -> AppResult<Vec<AdminOrgMember>> {
        sqlx::query(
            "SELECT u.id AS user_id, u.username, u.email FROM organizations o \
             JOIN users u ON u.id=o.owner_id WHERE o.id=$1",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(AdminOrgMember {
                user_id: row.try_get("user_id").map_err(db_err)?,
                username: row.try_get("username").map_err(db_err)?,
                email: row.try_get("email").map_err(db_err)?,
                member_role: "OWNER".into(),
            })
        })
        .collect()
    }

    async fn organization_certificates(
        &self,
        organization_id: Uuid,
    ) -> AppResult<Vec<AdminCertificateSummary>> {
        sqlx::query(
            "SELECT c.certificate_number, c.participation_id, e.name AS event_name, \
             u.username AS participant_name, c.issued_at, c.status::text AS status \
             FROM certificates c JOIN participations p ON p.id=c.participation_id \
             JOIN events e ON e.id=p.event_id JOIN users u ON u.id=p.user_id \
             WHERE e.organization_id=$1 ORDER BY c.issued_at DESC",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(certificate_from_row)
        .collect()
    }

    async fn organization_impact(&self, organization_id: Uuid) -> AppResult<ImpactStats> {
        impact_for(&self.pool, "AND e.organization_id=$1", organization_id).await
    }

    async fn list_users(&self, query: &AdminListQuery) -> AppResult<Page<AdminUser>> {
        let mut count = QueryBuilder::new("SELECT count(*)::bigint FROM users WHERE 1=1");
        let mut select = QueryBuilder::new(format!("{USER_SELECT} WHERE 1=1"));
        if let Some(q) = query.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = like(q);
            count.push(" AND (username ILIKE ");
            count.push_bind(pattern.clone());
            count.push(" OR email ILIKE ");
            count.push_bind(pattern.clone());
            count.push(')');
            select.push(" AND (username ILIKE ");
            select.push_bind(pattern.clone());
            select.push(" OR email ILIKE ");
            select.push_bind(pattern);
            select.push(')');
        }
        if let Some(status) = query.status.as_deref().filter(|s| !s.is_empty()) {
            count.push(" AND status = ");
            count.push_bind(status.to_string());
            count.push("::user_status");
            select.push(" AND status = ");
            select.push_bind(status.to_string());
            select.push("::user_status");
        }
        let total: i64 = count
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        select.push(" ORDER BY ");
        select.push(user_order(query.sort.as_deref(), query.descending()));
        select.push(" LIMIT ");
        select.push_bind(i64::from(query.per_page));
        select.push(" OFFSET ");
        select.push_bind(query.offset());
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(user_from_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Page {
            items,
            total,
            page: query.page,
            per_page: query.per_page,
        })
    }

    async fn find_user(&self, user_id: Uuid) -> AppResult<Option<AdminUser>> {
        sqlx::query(&format!("{USER_SELECT} WHERE id=$1"))
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(user_from_row)
            .transpose()
    }

    async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<bool> {
        let result =
            sqlx::query("UPDATE users SET role=$2::user_role, updated_at=now() WHERE id=$1")
                .bind(user_id)
                .bind(role.as_str())
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_user_status(
        &self,
        user_id: Uuid,
        status: UserStatus,
        reason: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE users SET status=$2::user_status, status_reason=$3, updated_at=now() WHERE id=$1",
        )
        .bind(user_id)
        .bind(status.as_str())
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected())
    }

    async fn user_activities(&self, user_id: Uuid) -> AppResult<Vec<AdminUserActivity>> {
        sqlx::query(
            "SELECT p.id AS participation_id, p.event_id, e.name AS event_name, \
             p.status::text AS status, p.registered_at, p.checked_in_at, \
             COALESCE(p.points_awarded,0) AS points_awarded \
             FROM participations p JOIN events e ON e.id=p.event_id \
             WHERE p.user_id=$1 ORDER BY p.registered_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            let status: String = row.try_get("status").map_err(db_err)?;
            Ok(AdminUserActivity {
                participation_id: row.try_get("participation_id").map_err(db_err)?,
                event_id: row.try_get("event_id").map_err(db_err)?,
                event_name: row.try_get("event_name").map_err(db_err)?,
                status: ParticipationStatus::from_str(&status).map_err(AppError::Domain)?,
                registered_at: row.try_get("registered_at").map_err(db_err)?,
                checked_in_at: row.try_get("checked_in_at").map_err(db_err)?,
                points_awarded: row.try_get("points_awarded").map_err(db_err)?,
            })
        })
        .collect()
    }

    async fn user_certificates(&self, user_id: Uuid) -> AppResult<Vec<AdminCertificateSummary>> {
        sqlx::query(
            "SELECT c.certificate_number, c.participation_id, e.name AS event_name, \
             u.username AS participant_name, c.issued_at, c.status::text AS status \
             FROM certificates c JOIN participations p ON p.id=c.participation_id \
             JOIN events e ON e.id=p.event_id JOIN users u ON u.id=p.user_id \
             WHERE p.user_id=$1 ORDER BY c.issued_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(certificate_from_row)
        .collect()
    }

    async fn user_achievements(&self, user_id: Uuid) -> AppResult<Vec<AdminUserAchievement>> {
        sqlx::query(
            "SELECT achievement_key, status::text AS status, verification_reference \
             FROM blockchain_achievements WHERE user_id=$1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(AdminUserAchievement {
                achievement_key: row.try_get("achievement_key").map_err(db_err)?,
                status: row.try_get("status").map_err(db_err)?,
                verification_reference: row.try_get("verification_reference").map_err(db_err)?,
            })
        })
        .collect()
    }

    async fn user_ledger(&self, user_id: Uuid) -> AppResult<Vec<AdminPointTransaction>> {
        sqlx::query(
            "SELECT id, amount, reason, participation_id, created_by, created_at \
             FROM point_transactions WHERE user_id=$1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(AdminPointTransaction {
                id: row.try_get("id").map_err(db_err)?,
                amount: row.try_get("amount").map_err(db_err)?,
                reason: row.try_get("reason").map_err(db_err)?,
                participation_id: row.try_get("participation_id").map_err(db_err)?,
                created_by: row.try_get("created_by").map_err(db_err)?,
                created_at: row.try_get("created_at").map_err(db_err)?,
            })
        })
        .collect()
    }

    async fn apply_point_correction(
        &self,
        user_id: Uuid,
        amount: i32,
        reason: &str,
        created_by: Uuid,
    ) -> AppResult<Option<i32>> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query(
            "INSERT INTO point_transactions (user_id, amount, reason, created_by) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(user_id)
        .bind(amount)
        .bind(reason)
        .bind(created_by)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let updated = sqlx::query(
            "UPDATE users SET \
             eco_points=(SELECT COALESCE(sum(amount),0)::integer FROM point_transactions WHERE user_id=$1), \
             level=LEAST((SELECT COALESCE(sum(amount),0) FROM point_transactions WHERE user_id=$1)/100 + 1, 30), \
             updated_at=now() \
             WHERE id=$1 RETURNING eco_points",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await;
        match updated {
            Ok(Some(row)) => {
                tx.commit().await.map_err(db_err)?;
                Ok(Some(row.try_get("eco_points").map_err(db_err)?))
            }
            Ok(None) => {
                tx.rollback().await.map_err(db_err)?;
                Ok(None)
            }
            Err(sqlx::Error::Database(err))
                if err.constraint() == Some("users_eco_points_check")
                    || err.code().as_deref() == Some("23514") =>
            {
                tx.rollback().await.map_err(db_err)?;
                Ok(None)
            }
            Err(err) => Err(db_err(err)),
        }
    }

    async fn insert_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        created_by: Uuid,
        expires_at: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (user_id, token_hash, created_by, expires_at) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(created_by)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn list_events(&self, query: &AdminListQuery) -> AppResult<Page<AdminEvent>> {
        let mut count = QueryBuilder::new(
            "SELECT count(*)::bigint FROM events e JOIN organizations o ON o.id=e.organization_id WHERE 1=1",
        );
        let mut select = QueryBuilder::new(format!("{EVENT_SELECT} WHERE 1=1"));
        if let Some(q) = query.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = like(q);
            count.push(" AND (e.name ILIKE ");
            count.push_bind(pattern.clone());
            count.push(" OR e.location ILIKE ");
            count.push_bind(pattern.clone());
            count.push(" OR o.name ILIKE ");
            count.push_bind(pattern.clone());
            count.push(')');
            select.push(" AND (e.name ILIKE ");
            select.push_bind(pattern.clone());
            select.push(" OR e.location ILIKE ");
            select.push_bind(pattern.clone());
            select.push(" OR o.name ILIKE ");
            select.push_bind(pattern);
            select.push(')');
        }
        if let Some(status) = query.status.as_deref().filter(|s| !s.is_empty()) {
            count.push(" AND e.status = ");
            count.push_bind(status.to_string());
            count.push("::event_status");
            select.push(" AND e.status = ");
            select.push_bind(status.to_string());
            select.push("::event_status");
        }
        if let Some(org) = query.organization_id {
            count.push(" AND e.organization_id = ");
            count.push_bind(org);
            select.push(" AND e.organization_id = ");
            select.push_bind(org);
        }
        if let Some(activity) = query.activity_type.as_deref().filter(|s| !s.is_empty()) {
            count.push(" AND e.activity_type = ");
            count.push_bind(activity.to_string());
            count.push("::activity_type");
            select.push(" AND e.activity_type = ");
            select.push_bind(activity.to_string());
            select.push("::activity_type");
        }
        if let Some(location) = query.location.as_deref().filter(|s| !s.is_empty()) {
            let pattern = like(location);
            count.push(" AND e.location ILIKE ");
            count.push_bind(pattern.clone());
            select.push(" AND e.location ILIKE ");
            select.push_bind(pattern);
        }
        if let Some(from) = query.from {
            count.push(" AND e.starts_at >= ");
            count.push_bind(from);
            select.push(" AND e.starts_at >= ");
            select.push_bind(from);
        }
        if let Some(to) = query.to {
            count.push(" AND e.starts_at <= ");
            count.push_bind(to);
            select.push(" AND e.starts_at <= ");
            select.push_bind(to);
        }
        if query.flagged == Some(true) {
            count.push(
                " AND EXISTS (SELECT 1 FROM participations p WHERE p.event_id=e.id AND p.flagged)",
            );
            select.push(
                " AND EXISTS (SELECT 1 FROM participations p WHERE p.event_id=e.id AND p.flagged)",
            );
        }
        let total: i64 = count
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        select.push(" ORDER BY ");
        select.push(event_order(query.sort.as_deref(), query.descending()));
        select.push(" LIMIT ");
        select.push_bind(i64::from(query.per_page));
        select.push(" OFFSET ");
        select.push_bind(query.offset());
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(event_from_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Page {
            items,
            total,
            page: query.page,
            per_page: query.per_page,
        })
    }

    async fn find_event(&self, event_id: Uuid) -> AppResult<Option<AdminEvent>> {
        sqlx::query(&format!("{EVENT_SELECT} WHERE e.id=$1"))
            .bind(event_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(event_from_row)
            .transpose()
    }

    async fn event_impacts(&self, event_id: Uuid) -> AppResult<Vec<EventImpact>> {
        sqlx::query(
            "SELECT metric, unit, expected_value FROM event_impact_definitions WHERE event_id=$1 ORDER BY metric",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(EventImpact {
                metric: row.try_get("metric").map_err(db_err)?,
                unit: row.try_get("unit").map_err(db_err)?,
                expected_value: row.try_get("expected_value").map_err(db_err)?,
            })
        })
        .collect()
    }

    async fn event_participants(&self, event_id: Uuid) -> AppResult<Vec<AdminParticipant>> {
        sqlx::query(
            "SELECT p.id, p.event_id, p.user_id, u.username, p.status::text AS status, \
             p.registered_at, p.checked_in_at, p.flagged, p.flag_reason, \
             COALESCE(p.points_awarded,0) AS points_awarded \
             FROM participations p JOIN users u ON u.id=p.user_id \
             WHERE p.event_id=$1 ORDER BY p.flagged DESC, p.registered_at",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            let status: String = row.try_get("status").map_err(db_err)?;
            Ok(AdminParticipant {
                id: row.try_get("id").map_err(db_err)?,
                event_id: row.try_get("event_id").map_err(db_err)?,
                user_id: row.try_get("user_id").map_err(db_err)?,
                username: row.try_get("username").map_err(db_err)?,
                status: ParticipationStatus::from_str(&status).map_err(AppError::Domain)?,
                registered_at: row.try_get("registered_at").map_err(db_err)?,
                checked_in_at: row.try_get("checked_in_at").map_err(db_err)?,
                flagged: row.try_get("flagged").map_err(db_err)?,
                flag_reason: row.try_get("flag_reason").map_err(db_err)?,
                points_awarded: row.try_get("points_awarded").map_err(db_err)?,
            })
        })
        .collect()
    }

    async fn event_qr_status(&self, event_id: Uuid) -> AppResult<AdminQrStatus> {
        let row = sqlx::query(
            "SELECT activates_at, expires_at, revoked_at FROM event_qr_tokens \
             WHERE event_id=$1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(match row {
            Some(row) => {
                let revoked_at: Option<DateTime<Utc>> =
                    row.try_get("revoked_at").map_err(db_err)?;
                let expires_at: DateTime<Utc> = row.try_get("expires_at").map_err(db_err)?;
                AdminQrStatus {
                    has_active: revoked_at.is_none() && expires_at > Utc::now(),
                    activates_at: row.try_get("activates_at").map_err(db_err)?,
                    expires_at: Some(expires_at),
                    revoked_at,
                }
            }
            None => AdminQrStatus {
                has_active: false,
                activates_at: None,
                expires_at: None,
                revoked_at: None,
            },
        })
    }

    async fn event_impact(&self, event_id: Uuid) -> AppResult<ImpactStats> {
        impact_for(&self.pool, "AND e.id=$1", event_id).await
    }

    async fn set_event_status(
        &self,
        event_id: Uuid,
        status: EventStatus,
        previous: Option<EventStatus>,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE events SET status=$2::event_status, previous_status=$3::event_status, \
             moderation_reason=$4, moderated_by=$5, moderated_at=now(), \
             cancelled_by=CASE WHEN $2='CANCELLED' THEN $5 ELSE cancelled_by END, \
             cancelled_at=CASE WHEN $2='CANCELLED' THEN now() ELSE cancelled_at END, \
             cancellation_reason=CASE WHEN $2='CANCELLED' THEN $4 ELSE cancellation_reason END, \
             updated_at=now() WHERE id=$1",
        )
        .bind(event_id)
        .bind(status.as_str())
        .bind(previous.map(EventStatus::as_str))
        .bind(reason)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() == 1)
    }

    async fn revoke_event_qr_tokens(&self, event_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE event_qr_tokens SET revoked_at=now() WHERE event_id=$1 AND revoked_at IS NULL",
        )
        .bind(event_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn flag_participation(
        &self,
        participation_id: Uuid,
        flagged: bool,
        reason: &str,
        actor_id: Uuid,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE participations SET flagged=$2, flag_reason=$3, flagged_at=CASE WHEN $2 THEN now() ELSE NULL END, \
             flagged_by=CASE WHEN $2 THEN $4 ELSE NULL END WHERE id=$1",
        )
        .bind(participation_id)
        .bind(flagged)
        .bind(reason)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(result.rows_affected() == 1)
    }

    async fn record_audit(&self, entry: AuditEntry) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO audit_logs (actor_id, action, entity_type, entity_id, metadata, ip_address) \
             VALUES ($1,$2,$3,$4,$5,$6::inet)",
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

    async fn list_audit(
        &self,
        entity_type: &str,
        entity_id: Uuid,
    ) -> AppResult<Vec<AdminAuditEntry>> {
        sqlx::query(
            "SELECT a.id, a.actor_id, u.username AS actor_username, a.action, a.entity_type, \
             a.entity_id, a.metadata, a.created_at \
             FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id \
             WHERE a.entity_type=$1 AND a.entity_id=$2 ORDER BY a.created_at DESC LIMIT 100",
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(AdminAuditEntry {
                id: row.try_get("id").map_err(db_err)?,
                actor_id: row.try_get("actor_id").map_err(db_err)?,
                actor_username: row.try_get("actor_username").map_err(db_err)?,
                action: row.try_get("action").map_err(db_err)?,
                entity_type: row.try_get("entity_type").map_err(db_err)?,
                entity_id: row.try_get("entity_id").map_err(db_err)?,
                metadata: row.try_get("metadata").map_err(db_err)?,
                created_at: row.try_get("created_at").map_err(db_err)?,
            })
        })
        .collect()
    }
}
