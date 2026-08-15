//! PostgreSQL implementation of mission persistence.

use chrono::{DateTime, Utc};
use ecoquest_application::{
    events::{
        Activity, CreateEventCommand, Event, EventQrToken, EventStore, Participation,
        UpdateEventCommand, VerificationResult,
    },
    AppError, AppResult,
};
use ecoquest_domain::{
    ActivityType, DomainError, EventStatus, ParticipationStatus, VerificationStatus,
};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::PgStore;

#[derive(Clone, Debug)]
pub struct PgEventStore {
    pool: PgPool,
}
impl PgEventStore {
    #[must_use]
    pub fn new(store: &PgStore) -> Self {
        Self {
            pool: store.pool().clone(),
        }
    }
}
fn db_err(e: sqlx::Error) -> AppError {
    AppError::Infrastructure(e.to_string())
}
const EVENT_COLUMNS: &str = "e.id,e.organization_id,o.name AS organization_name,e.created_by,e.name,e.description,e.activity_type::text AS activity_type,e.location,e.starts_at,e.ends_at,e.capacity,e.eco_points,e.status::text AS status,(SELECT count(*) FROM participations p WHERE p.event_id=e.id AND p.status <> 'CANCELLED') AS registered_count";

fn parse_event(row: &sqlx::postgres::PgRow) -> AppResult<Event> {
    Ok(Event {
        id: row.try_get("id").map_err(db_err)?,
        organization_id: row.try_get("organization_id").map_err(db_err)?,
        organization_name: row.try_get("organization_name").map_err(db_err)?,
        created_by: row.try_get("created_by").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        activity_type: ActivityType::from_str(
            &row.try_get::<String, _>("activity_type").map_err(db_err)?,
        )
        .map_err(AppError::Domain)?,
        location: row.try_get("location").map_err(db_err)?,
        starts_at: row.try_get("starts_at").map_err(db_err)?,
        ends_at: row.try_get("ends_at").map_err(db_err)?,
        capacity: row.try_get("capacity").map_err(db_err)?,
        eco_points: row.try_get("eco_points").map_err(db_err)?,
        status: EventStatus::from_str(&row.try_get::<String, _>("status").map_err(db_err)?)
            .map_err(AppError::Domain)?,
        impacts: vec![],
        registered_count: row.try_get("registered_count").map_err(db_err)?,
    })
}
async fn load_impacts(pool: &PgPool, event: &mut Event) -> AppResult<()> {
    event.impacts = sqlx::query("SELECT metric,unit,expected_value FROM event_impact_definitions WHERE event_id=$1 ORDER BY metric")
        .bind(event.id).fetch_all(pool).await.map_err(db_err)?.iter().map(|row| Ok(ecoquest_application::events::EventImpact { metric: row.try_get("metric").map_err(db_err)?, unit: row.try_get("unit").map_err(db_err)?, expected_value: row.try_get("expected_value").map_err(db_err)? })).collect::<AppResult<Vec<_>>>()?;
    Ok(())
}
fn parse_participation(row: &sqlx::postgres::PgRow) -> AppResult<Participation> {
    Ok(Participation {
        id: row.try_get("id").map_err(db_err)?,
        event_id: row.try_get("event_id").map_err(db_err)?,
        user_id: row.try_get("user_id").map_err(db_err)?,
        username: row.try_get("username").map_err(db_err)?,
        status: ParticipationStatus::from_str(&row.try_get::<String, _>("status").map_err(db_err)?)
            .map_err(AppError::Domain)?,
        registered_at: row.try_get("registered_at").map_err(db_err)?,
        checked_in_at: row.try_get("checked_in_at").map_err(db_err)?,
    })
}

fn parse_activity_participation(row: &sqlx::postgres::PgRow) -> AppResult<Participation> {
    Ok(Participation {
        id: row.try_get("participation_id").map_err(db_err)?,
        event_id: row.try_get("event_id").map_err(db_err)?,
        user_id: row.try_get("user_id").map_err(db_err)?,
        username: row.try_get("username").map_err(db_err)?,
        status: ParticipationStatus::from_str(
            &row.try_get::<String, _>("participation_status")
                .map_err(db_err)?,
        )
        .map_err(AppError::Domain)?,
        registered_at: row.try_get("registered_at").map_err(db_err)?,
        checked_in_at: row.try_get("checked_in_at").map_err(db_err)?,
    })
}

#[async_trait::async_trait]
impl EventStore for PgEventStore {
    async fn list_impact_metrics(
        &self,
    ) -> AppResult<Vec<ecoquest_application::impact::ImpactMetric>> {
        crate::impact_store::load_impact_metrics(&self.pool).await
    }
    async fn is_organization_owner(&self, org: Uuid, user: Uuid) -> AppResult<bool> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM organizations WHERE id=$1 AND owner_id=$2)")
            .bind(org)
            .bind(user)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }
    async fn organization_verification_status(
        &self,
        organization_id: Uuid,
    ) -> AppResult<Option<VerificationStatus>> {
        let row = sqlx::query("SELECT verification_status::text AS verification_status FROM organizations WHERE id=$1")
            .bind(organization_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let status: String = row.try_get("verification_status").map_err(db_err)?;
        Ok(Some(
            VerificationStatus::from_str(&status).map_err(AppError::Domain)?,
        ))
    }
    async fn create_event(&self, c: CreateEventCommand, actor: Uuid) -> AppResult<Event> {
        let impacts = c.impacts.clone();
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        // parse_event needs organization_name, which a bare INSERT ... RETURNING
        // cannot produce; the CTE joins organizations for it.
        let row = sqlx::query("WITH inserted AS (INSERT INTO events (organization_id,created_by,name,description,activity_type,location,starts_at,ends_at,capacity,eco_points) VALUES ($1,$2,$3,$4,$5::activity_type,$6,$7,$8,$9,$10) RETURNING id,organization_id,created_by,name,description,activity_type::text AS activity_type,location,starts_at,ends_at,capacity,eco_points,status::text AS status,0::bigint AS registered_count) SELECT i.id,i.organization_id,o.name AS organization_name,i.created_by,i.name,i.description,i.activity_type,i.location,i.starts_at,i.ends_at,i.capacity,i.eco_points,i.status,i.registered_count FROM inserted i JOIN organizations o ON o.id=i.organization_id")
            .bind(c.organization_id).bind(actor).bind(c.name).bind(c.description).bind(c.activity_type.as_str()).bind(c.location).bind(c.starts_at).bind(c.ends_at).bind(c.capacity).bind(c.eco_points).fetch_one(&mut *tx).await.map_err(db_err)?;
        let event = parse_event(&row)?;
        for impact in impacts {
            sqlx::query("INSERT INTO event_impact_definitions (event_id,metric,unit,expected_value) VALUES ($1,$2,$3,$4)").bind(event.id).bind(impact.metric).bind(impact.unit).bind(impact.expected_value).execute(&mut *tx).await.map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        let mut event = event;
        load_impacts(&self.pool, &mut event).await?;
        Ok(event)
    }
    async fn update_event(&self, id: Uuid, c: UpdateEventCommand) -> AppResult<Option<Event>> {
        let impacts = c.impacts;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = sqlx::query("WITH updated AS (UPDATE events SET name=$2,description=$3,activity_type=$4::activity_type,location=$5,starts_at=$6,ends_at=$7,capacity=$8,eco_points=$9 WHERE id=$1 AND status='DRAFT' RETURNING id,organization_id,created_by,name,description,activity_type::text AS activity_type,location,starts_at,ends_at,capacity,eco_points,status::text AS status,0::bigint AS registered_count) SELECT up.id,up.organization_id,o.name AS organization_name,up.created_by,up.name,up.description,up.activity_type,up.location,up.starts_at,up.ends_at,up.capacity,up.eco_points,up.status,up.registered_count FROM updated up JOIN organizations o ON o.id=up.organization_id")
            .bind(id).bind(c.name).bind(c.description).bind(c.activity_type.as_str()).bind(c.location).bind(c.starts_at).bind(c.ends_at).bind(c.capacity).bind(c.eco_points).fetch_optional(&mut *tx).await.map_err(db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        sqlx::query("DELETE FROM event_impact_definitions WHERE event_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for impact in &impacts {
            sqlx::query("INSERT INTO event_impact_definitions (event_id,metric,unit,expected_value) VALUES ($1,$2,$3,$4)").bind(id).bind(&impact.metric).bind(&impact.unit).bind(impact.expected_value).execute(&mut *tx).await.map_err(db_err)?;
        }
        let mut event = parse_event(&row)?;
        event.impacts = impacts;
        tx.commit().await.map_err(db_err)?;
        Ok(Some(event))
    }
    async fn find_event(&self, id: Uuid) -> AppResult<Option<Event>> {
        let row = sqlx::query(&format!(
            "SELECT {EVENT_COLUMNS} FROM events e JOIN organizations o ON o.id=e.organization_id WHERE e.id=$1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut event = parse_event(&row)?;
        load_impacts(&self.pool, &mut event).await?;
        Ok(Some(event))
    }
    async fn list_published_events(&self, now: DateTime<Utc>) -> AppResult<Vec<Event>> {
        // ACTIVE is included so a mission that is already under way stays
        // discoverable; it is the only window in which check-in works, and a
        // player who finds it there can still join and scan.
        let rows = sqlx::query(&format!("SELECT {EVENT_COLUMNS} FROM events e JOIN organizations o ON o.id=e.organization_id WHERE e.status IN ('PUBLISHED','ACTIVE') AND e.ends_at>$1 ORDER BY e.starts_at")).bind(now).fetch_all(&self.pool).await.map_err(db_err)?;
        let mut events = rows
            .iter()
            .map(parse_event)
            .collect::<AppResult<Vec<_>>>()?;
        for event in &mut events {
            load_impacts(&self.pool, event).await?;
        }
        Ok(events)
    }
    async fn list_organization_events(&self, organization_id: Uuid) -> AppResult<Vec<Event>> {
        let rows = sqlx::query(&format!(
            "SELECT {EVENT_COLUMNS} FROM events e JOIN organizations o ON o.id=e.organization_id WHERE e.organization_id=$1 ORDER BY e.starts_at"
        ))
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        let mut events = rows
            .iter()
            .map(parse_event)
            .collect::<AppResult<Vec<_>>>()?;
        for event in &mut events {
            load_impacts(&self.pool, event).await?;
        }
        Ok(events)
    }
    async fn transition_event(&self, id: Uuid, old: &str, new: &str) -> AppResult<bool> {
        Ok(sqlx::query(
            "UPDATE events SET status=$3::event_status WHERE id=$1 AND status=$2::event_status",
        )
        .bind(id)
        .bind(old)
        .bind(new)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected()
            == 1)
    }
    async fn revoke_event_qr_tokens(&self, id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE event_qr_tokens SET revoked_at=now() WHERE event_id=$1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
    async fn insert_event_qr_token(
        &self,
        event_id: Uuid,
        hash: &[u8],
        actor: Uuid,
        activates: DateTime<Utc>,
        expires: DateTime<Utc>,
    ) -> AppResult<EventQrToken> {
        let r=sqlx::query("INSERT INTO event_qr_tokens (event_id,token_hash,created_by,activates_at,expires_at) VALUES ($1,$2,$3,$4,$5) RETURNING id,event_id,activates_at,expires_at,revoked_at").bind(event_id).bind(hash).bind(actor).bind(activates).bind(expires).fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(EventQrToken {
            id: r.try_get("id").map_err(db_err)?,
            event_id: r.try_get("event_id").map_err(db_err)?,
            activates_at: r.try_get("activates_at").map_err(db_err)?,
            expires_at: r.try_get("expires_at").map_err(db_err)?,
            revoked_at: r.try_get("revoked_at").map_err(db_err)?,
        })
    }
    async fn find_event_qr_token(&self, hash: &[u8]) -> AppResult<Option<EventQrToken>> {
        let r=sqlx::query("SELECT id,event_id,activates_at,expires_at,revoked_at FROM event_qr_tokens WHERE token_hash=$1").bind(hash).fetch_optional(&self.pool).await.map_err(db_err)?;
        r.map(|r| {
            Ok(EventQrToken {
                id: r.try_get("id").map_err(db_err)?,
                event_id: r.try_get("event_id").map_err(db_err)?,
                activates_at: r.try_get("activates_at").map_err(db_err)?,
                expires_at: r.try_get("expires_at").map_err(db_err)?,
                revoked_at: r.try_get("revoked_at").map_err(db_err)?,
            })
        })
        .transpose()
    }
    async fn join_event(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> AppResult<Participation> {
        // Lock event row: capacity check plus insert is serial per event.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row=sqlx::query("SELECT status::text AS status,starts_at,ends_at,capacity FROM events WHERE id=$1 FOR UPDATE").bind(event_id).fetch_optional(&mut *tx).await.map_err(db_err)?.ok_or_else(||AppError::Domain(DomainError::NotFound("event".into())))?;
        let status: String = row.try_get("status").map_err(db_err)?;
        let ends: DateTime<Utc> = row.try_get("ends_at").map_err(db_err)?;
        // ACTIVE counts as open: registration closes when the mission ends, not
        // when it starts, so a walk-up volunteer can join on site and check in.
        if !matches!(status.as_str(), "PUBLISHED" | "ACTIVE") || ends <= now {
            return Err(AppError::Domain(DomainError::Conflict(
                "event is not open for registration".into(),
            )));
        }
        // Checked before capacity: a player who already holds a seat on a full
        // event would otherwise be told the event is full, which reads as though
        // they had lost their place.
        let already_joined: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM participations WHERE event_id=$1 AND user_id=$2)",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        if already_joined {
            return Err(AppError::Domain(DomainError::Conflict(
                "already joined event".into(),
            )));
        }
        let capacity: i32 = row.try_get("capacity").map_err(db_err)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM participations WHERE event_id=$1 AND status <> 'CANCELLED'",
        )
        .bind(event_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        if count >= i64::from(capacity) {
            return Err(AppError::Domain(DomainError::Conflict(
                "event is full".into(),
            )));
        }
        // The row is mapped by parse_participation, which needs username; a bare
        // INSERT ... RETURNING cannot join users, so wrap it in a CTE.
        let inserted=sqlx::query("WITH inserted AS (INSERT INTO participations (event_id,user_id) VALUES ($1,$2) RETURNING id,event_id,user_id,status::text AS status,registered_at,checked_in_at) SELECT i.id,i.event_id,i.user_id,u.username,i.status,i.registered_at,i.checked_in_at FROM inserted i JOIN users u ON u.id=i.user_id").bind(event_id).bind(user_id).fetch_one(&mut *tx).await;
        let inserted = match inserted {
            Ok(r) => r,
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
                return Err(AppError::Domain(DomainError::Conflict(
                    "already joined event".into(),
                )))
            }
            Err(e) => return Err(db_err(e)),
        };
        tx.commit().await.map_err(db_err)?;
        parse_participation(&inserted)
    }
    async fn check_in(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        qr_id: Uuid,
        now: DateTime<Utc>,
    ) -> AppResult<Option<Participation>> {
        let r=sqlx::query("WITH updated AS (UPDATE participations p SET status='PENDING_VERIFICATION',checked_in_at=$4,qr_token_id=$3 FROM events e WHERE p.event_id=$1 AND p.user_id=$2 AND p.status='REGISTERED' AND e.id=p.event_id AND e.status='ACTIVE' AND e.starts_at <= $4 AND e.ends_at > $4 RETURNING p.id,p.event_id,p.user_id,p.status::text AS status,p.registered_at,p.checked_in_at) SELECT up.id,up.event_id,up.user_id,u.username,up.status,up.registered_at,up.checked_in_at FROM updated up JOIN users u ON u.id=up.user_id").bind(event_id).bind(user_id).bind(qr_id).bind(now).fetch_optional(&self.pool).await.map_err(db_err)?;
        r.map(|r| parse_participation(&r)).transpose()
    }
    async fn list_participants(&self, event_id: Uuid) -> AppResult<Vec<Participation>> {
        sqlx::query(
            "SELECT p.id,p.event_id,p.user_id,u.username,p.status::text AS status,p.registered_at,p.checked_in_at \
             FROM participations p JOIN users u ON u.id=p.user_id WHERE p.event_id=$1 ORDER BY p.registered_at",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(parse_participation)
        .collect()
    }
    async fn find_participation_for_user(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<Participation>> {
        sqlx::query("SELECT p.id,p.event_id,p.user_id,u.username,p.status::text AS status,p.registered_at,p.checked_in_at FROM participations p JOIN users u ON u.id=p.user_id WHERE p.event_id=$1 AND p.user_id=$2").bind(event_id).bind(user_id).fetch_optional(&self.pool).await.map_err(db_err)?.map(|row| parse_participation(&row)).transpose()
    }
    async fn find_participation(&self, id: Uuid) -> AppResult<Option<Participation>> {
        sqlx::query("SELECT p.id,p.event_id,p.user_id,u.username,p.status::text AS status,p.registered_at,p.checked_in_at FROM participations p JOIN users u ON u.id=p.user_id WHERE p.id=$1").bind(id).fetch_optional(&self.pool).await.map_err(db_err)?.map(|row| parse_participation(&row)).transpose()
    }
    async fn list_my_activities(&self, user_id: Uuid) -> AppResult<Vec<Activity>> {
        let rows = sqlx::query(&format!(
            "SELECT p.id AS participation_id,p.event_id,p.user_id,u.username,p.status::text AS participation_status,p.registered_at,p.checked_in_at,{EVENT_COLUMNS} \
             FROM participations p JOIN events e ON e.id=p.event_id JOIN organizations o ON o.id=e.organization_id JOIN users u ON u.id=p.user_id \
             WHERE p.user_id=$1 ORDER BY e.starts_at DESC"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        let rows2 = rows;
        let mut activities = Vec::with_capacity(rows2.len());
        for row in &rows2 {
            let participation = parse_activity_participation(row)?;
            let mut event = parse_event(row)?;
            load_impacts(&self.pool, &mut event).await?;
            activities.push(Activity {
                participation,
                event,
            });
        }
        Ok(activities)
    }
    async fn verify_participation(
        &self,
        id: Uuid,
        verifier: Uuid,
        reason: &str,
    ) -> AppResult<VerificationResult> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = sqlx::query("SELECT p.id,p.user_id,p.status::text AS status,e.id AS event_id,e.eco_points FROM participations p JOIN events e ON e.id=p.event_id WHERE p.id=$1 FOR UPDATE").bind(id).fetch_optional(&mut *tx).await.map_err(db_err)?.ok_or_else(|| AppError::Domain(DomainError::NotFound("participation".into())))?;
        let status: String = row.try_get("status").map_err(db_err)?;
        if status != "PENDING_VERIFICATION" {
            return Err(AppError::Domain(DomainError::Conflict(
                "participation is not pending verification".into(),
            )));
        }
        let user_id: Uuid = row.try_get("user_id").map_err(db_err)?;
        let event_id: Uuid = row.try_get("event_id").map_err(db_err)?;
        let points: i32 = row.try_get("eco_points").map_err(db_err)?;
        sqlx::query("INSERT INTO participation_verifications (participation_id,verifier_id,decision,reason) VALUES ($1,$2,'VERIFIED',$3)").bind(id).bind(verifier).bind(reason).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("UPDATE participations SET status='VERIFIED',verified_by=$2,verified_at=now(),points_awarded=$3 WHERE id=$1").bind(id).bind(verifier).bind(points).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("INSERT INTO point_transactions (user_id,participation_id,amount,reason,created_by) VALUES ($1,$2,$3,$4,$5)").bind(user_id).bind(id).bind(points).bind("verified event participation").bind(verifier).execute(&mut *tx).await.map_err(db_err)?;
        // The ledger stays the source of truth for the total; `level` is a
        // generated column derived from it, so it is never written here.
        let player = sqlx::query("UPDATE users SET eco_points=(SELECT COALESCE(sum(amount),0)::integer FROM point_transactions WHERE user_id=$1) WHERE id=$1 RETURNING eco_points,level").bind(user_id).fetch_one(&mut *tx).await.map_err(db_err)?;
        // Each definition is a per-person contribution. One verified participation
        // inserts one row; no whole-event total is multiplied by attendance.
        sqlx::query("INSERT INTO impact_contributions (participation_id,event_id,metric,unit,value,source,verifier_id,attribution_method) SELECT $1,$2,metric,unit,expected_value,'EVENT_IMPACT_DEFINITION',$3,'INDIVIDUAL_CONTRIBUTION' FROM event_impact_definitions WHERE event_id=$2").bind(id).bind(event_id).bind(verifier).execute(&mut *tx).await.map_err(db_err)?;
        for (key, amount) in [("verified_participations", 1), ("eco_points", points)] {
            sqlx::query("INSERT INTO achievement_progress (user_id,achievement_key,progress) VALUES ($1,$2,$3) ON CONFLICT (user_id,achievement_key) DO UPDATE SET progress=achievement_progress.progress+EXCLUDED.progress").bind(user_id).bind(key).bind(amount).execute(&mut *tx).await.map_err(db_err)?;
        }
        sqlx::query("INSERT INTO outbox_events (aggregate_type,aggregate_id,event_type,payload) VALUES ('participation',$1,'certificate.issue',jsonb_build_object('participation_id',$1,'user_id',$2,'event_id',$3))").bind(id).bind(user_id).bind(event_id).execute(&mut *tx).await.map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(VerificationResult {
            participation_id: id,
            status: ParticipationStatus::Verified,
            points_awarded: points,
            player_eco_points: player.try_get("eco_points").map_err(db_err)?,
            player_level: player.try_get("level").map_err(db_err)?,
        })
    }
    async fn reject_participation(
        &self,
        id: Uuid,
        verifier: Uuid,
        reason: &str,
    ) -> AppResult<VerificationResult> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = sqlx::query(
            "SELECT user_id,status::text AS status FROM participations WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AppError::Domain(DomainError::NotFound("participation".into())))?;
        if row.try_get::<String, _>("status").map_err(db_err)? != "PENDING_VERIFICATION" {
            return Err(AppError::Domain(DomainError::Conflict(
                "participation is not pending verification".into(),
            )));
        }
        let user_id: Uuid = row.try_get("user_id").map_err(db_err)?;
        sqlx::query("INSERT INTO participation_verifications (participation_id,verifier_id,decision,reason) VALUES ($1,$2,'REJECTED',$3)").bind(id).bind(verifier).bind(reason).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("UPDATE participations SET status='REJECTED',verified_by=$2,verified_at=now() WHERE id=$1").bind(id).bind(verifier).execute(&mut *tx).await.map_err(db_err)?;
        let player = sqlx::query("SELECT eco_points,level FROM users WHERE id=$1")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(VerificationResult {
            participation_id: id,
            status: ParticipationStatus::Rejected,
            points_awarded: 0,
            player_eco_points: player.try_get("eco_points").map_err(db_err)?,
            player_level: player.try_get("level").map_err(db_err)?,
        })
    }
}
