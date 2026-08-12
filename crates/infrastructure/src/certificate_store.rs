//! PostgreSQL certificate issuance worker and read model.
//!
//! Issuance consumes durable `certificate.issue` outbox events written in the same
//! transaction as verification, so a crash never loses or double-issues a certificate.
use crate::PgStore;
use ecoquest_application::certificates::{
    encode_hash, verification_hash, Certificate, CertificateData, CertificateStore,
    OrganizationStatus,
};
use ecoquest_application::{AppError, AppResult};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const SELECT_CERTIFICATE: &str = "SELECT c.certificate_number,c.participation_id,c.user_id,c.event_id,c.issued_at,c.volunteer_duration_minutes,c.verification_hash,c.status::text AS status,e.name AS event_name,o.name AS organization_name,u.username AS participant_name FROM certificates c JOIN events e ON e.id=c.event_id JOIN organizations o ON o.id=c.organization_id JOIN users u ON u.id=c.user_id";

#[derive(Clone, Debug)]
pub struct PgCertificateStore {
    pool: PgPool,
}

impl PgCertificateStore {
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

fn row_to_certificate(row: &sqlx::postgres::PgRow) -> AppResult<Certificate> {
    let hash: Vec<u8> = row.try_get("verification_hash").map_err(db_err)?;
    Ok(Certificate {
        certificate_number: row.try_get("certificate_number").map_err(db_err)?,
        participation_id: row.try_get("participation_id").map_err(db_err)?,
        user_id: row.try_get("user_id").map_err(db_err)?,
        event_id: row.try_get("event_id").map_err(db_err)?,
        event_name: row.try_get("event_name").map_err(db_err)?,
        organization_name: row.try_get("organization_name").map_err(db_err)?,
        participant_name: row.try_get("participant_name").map_err(db_err)?,
        issued_at: row.try_get("issued_at").map_err(db_err)?,
        volunteer_duration_minutes: row.try_get("volunteer_duration_minutes").map_err(db_err)?,
        verification_hash: encode_hash(&hash),
        status: row.try_get("status").map_err(db_err)?,
    })
}

/// Inserts the certificate once. The hash binds the exact stored number, time and duration.
async fn issue_certificate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    participation_id: Uuid,
    user_id: Uuid,
    event_id: Uuid,
    organization_id: Uuid,
    minutes: i32,
) -> AppResult<()> {
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM certificates WHERE participation_id=$1")
            .bind(participation_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_err)?;
    if existing.is_some() {
        return Ok(());
    }
    let reserved = sqlx::query(
        "SELECT ('ECO-' || to_char(current_date,'YYYY') || '-' || lpad(nextval('certificate_number_seq')::text,6,'0')) AS certificate_number, now() AS issued_at",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    let certificate_number: String = reserved.try_get("certificate_number").map_err(db_err)?;
    let issued_at: chrono::DateTime<chrono::Utc> = reserved.try_get("issued_at").map_err(db_err)?;
    let hash = verification_hash(&CertificateData {
        certificate_number: certificate_number.clone(),
        user_id,
        event_id,
        organization_id,
        issued_at,
        volunteer_duration_minutes: minutes,
    });
    sqlx::query(
        "INSERT INTO certificates (certificate_number,participation_id,user_id,event_id,organization_id,issued_at,volunteer_duration_minutes,verification_hash,storage_key) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(&certificate_number)
    .bind(participation_id)
    .bind(user_id)
    .bind(event_id)
    .bind(organization_id)
    .bind(issued_at)
    .bind(minutes)
    .bind(hash.as_slice())
    .bind(format!("certificates/{certificate_number}.json"))
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[async_trait::async_trait]
impl CertificateStore for PgCertificateStore {
    async fn issue_next_queued(&self) -> AppResult<bool> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        // SKIP LOCKED lets several workers run without issuing the same certificate twice.
        let Some(job) = sqlx::query(
            "SELECT id,aggregate_id FROM outbox_events WHERE event_type='certificate.issue' AND processed_at IS NULL ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        else {
            return Ok(false);
        };
        let job_id: Uuid = job.try_get("id").map_err(db_err)?;
        let participation_id: Uuid = job.try_get("aggregate_id").map_err(db_err)?;

        // Only verified participation earns a certificate; duration comes from the event window.
        let facts = sqlx::query(
            "SELECT p.user_id,e.id AS event_id,e.organization_id,GREATEST(0,(EXTRACT(EPOCH FROM (e.ends_at - e.starts_at))/60)::integer) AS minutes FROM participations p JOIN events e ON e.id=p.event_id WHERE p.id=$1 AND p.status='VERIFIED'",
        )
        .bind(participation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let Some(facts) = facts else {
            // Participation is no longer verified; retiring the job avoids an infinite retry loop.
            sqlx::query("UPDATE outbox_events SET processed_at=now() WHERE id=$1")
                .bind(job_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            tx.commit().await.map_err(db_err)?;
            return Ok(true);
        };
        let user_id: Uuid = facts.try_get("user_id").map_err(db_err)?;
        let event_id: Uuid = facts.try_get("event_id").map_err(db_err)?;
        let organization_id: Uuid = facts.try_get("organization_id").map_err(db_err)?;
        let minutes: i32 = facts.try_get("minutes").map_err(db_err)?;
        issue_certificate(
            &mut tx,
            participation_id,
            user_id,
            event_id,
            organization_id,
            minutes,
        )
        .await?;
        sqlx::query("UPDATE outbox_events SET processed_at=now() WHERE id=$1")
            .bind(job_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(true)
    }

    async fn find_for_requester(
        &self,
        participation_id: Uuid,
        requester_id: Uuid,
    ) -> AppResult<Option<Certificate>> {
        // Access is limited to the participant or a member of the hosting organization.
        let sql = format!("{SELECT_CERTIFICATE} WHERE c.participation_id=$1 AND (c.user_id=$2 OR EXISTS (SELECT 1 FROM organization_members m WHERE m.organization_id=c.organization_id AND m.user_id=$2))");
        sqlx::query(&sql)
            .bind(participation_id)
            .bind(requester_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(row_to_certificate)
            .transpose()
    }

    async fn find_by_hash(&self, verification_hash: &[u8]) -> AppResult<Option<Certificate>> {
        let sql = format!("{SELECT_CERTIFICATE} WHERE c.verification_hash=$1");
        sqlx::query(&sql)
            .bind(verification_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(row_to_certificate)
            .transpose()
    }

    async fn organization_status(&self, user_id: Uuid) -> AppResult<Vec<OrganizationStatus>> {
        sqlx::query("SELECT o.id,o.name,m.member_role,o.verification_status::text AS verification_status,o.reviewed_at FROM organization_members m JOIN organizations o ON o.id=m.organization_id WHERE m.user_id=$1 ORDER BY o.name")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(|row| {
                Ok(OrganizationStatus {
                    organization_id: row.try_get("id").map_err(db_err)?,
                    name: row.try_get("name").map_err(db_err)?,
                    member_role: row.try_get("member_role").map_err(db_err)?,
                    verification_status: row.try_get("verification_status").map_err(db_err)?,
                    reviewed_at: row.try_get("reviewed_at").map_err(db_err)?,
                })
            })
            .collect()
    }
}
