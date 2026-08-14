//! PostgreSQL achievement outbox and wallet-challenge adapter.
use crate::PgStore;
use chrono::{DateTime, Utc};
use ecoquest_application::{
    achievements::{Achievement, AchievementStore, MintJob, MintResult},
    AppError, AppResult,
};
use ecoquest_domain::DomainError;
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone, Debug)]
pub struct PgAchievementStore {
    pool: PgPool,
}
impl PgAchievementStore {
    #[must_use]
    pub fn new(store: &PgStore) -> Self {
        Self {
            pool: store.pool().clone(),
        }
    }
}
fn err(e: sqlx::Error) -> AppError {
    AppError::Infrastructure(e.to_string())
}
#[async_trait::async_trait]
impl AchievementStore for PgAchievementStore {
    async fn create_wallet_challenge(
        &self,
        u: Uuid,
        w: &str,
        h: &[u8],
        x: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query("INSERT INTO wallet_challenges(user_id,wallet_address,nonce_hash,expires_at) VALUES($1,$2,$3,$4)").bind(u).bind(w).bind(h).bind(x).execute(&self.pool).await.map_err(err)?;
        Ok(())
    }
    async fn consume_wallet_challenge(
        &self,
        u: Uuid,
        w: &str,
        h: &[u8],
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        Ok(sqlx::query("UPDATE wallet_challenges SET consumed_at=$4 WHERE user_id=$1 AND wallet_address=$2 AND nonce_hash=$3 AND consumed_at IS NULL AND expires_at>$4").bind(u).bind(w).bind(h).bind(now).execute(&self.pool).await.map_err(err)?.rows_affected()==1)
    }
    async fn set_wallet(&self, u: Uuid, w: &str) -> AppResult<()> {
        // users.wallet_address is uniquely indexed, so a wallet already linked to
        // another account is a caller-visible conflict, not a dependency failure.
        match sqlx::query("UPDATE users SET wallet_address=$2 WHERE id=$1")
            .bind(u)
            .bind(w)
            .execute(&self.pool)
            .await
        {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
                Err(AppError::Domain(DomainError::Conflict(
                    "wallet is already linked to another account".into(),
                )))
            }
            Err(e) => Err(err(e)),
        }
    }
    async fn list_for_user(&self, u: Uuid) -> AppResult<Vec<Achievement>> {
        sqlx::query("SELECT achievement_key,status::text status,verification_reference,wallet_address,mint_identifier,transaction_signature FROM blockchain_achievements WHERE user_id=$1 ORDER BY created_at").bind(u).fetch_all(&self.pool).await.map_err(err)?.iter().map(|r|Ok(Achievement { achievement_key:r.try_get("achievement_key").map_err(err)?,status:r.try_get("status").map_err(err)?,verification_reference:r.try_get("verification_reference").map_err(err)?,wallet_address:r.try_get("wallet_address").map_err(err)?,mint_identifier:r.try_get("mint_identifier").map_err(err)?,transaction_signature:r.try_get("transaction_signature").map_err(err)?,explorer_url:None })).collect()
    }
    async fn queue_eligible_for_wallet(&self, u: Uuid, w: &str) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let ids=sqlx::query_scalar::<_,Uuid>("UPDATE blockchain_achievements SET wallet_address=$2,status='QUEUED',next_attempt_at=now() WHERE user_id=$1 AND status='ELIGIBLE' RETURNING id").bind(u).bind(w).fetch_all(&mut *tx).await.map_err(err)?;
        for id in ids {
            sqlx::query("INSERT INTO outbox_events(aggregate_type,aggregate_id,event_type,payload) VALUES('blockchain_achievement',$1,'achievement.mint',jsonb_build_object('achievement_id',$1)) ON CONFLICT(aggregate_id,event_type) DO UPDATE SET processed_at=NULL,available_at=now()").bind(id).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
    async fn claim_mint_job(&self, now: DateTime<Utc>) -> AppResult<Option<MintJob>> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let row=sqlx::query("SELECT a.id,a.verification_reference,a.wallet_address,a.attempt_count FROM blockchain_achievements a JOIN outbox_events o ON o.aggregate_id=a.id AND o.event_type='achievement.mint' WHERE o.processed_at IS NULL AND o.available_at<=$1 AND a.status IN('QUEUED','FAILED') AND a.wallet_address IS NOT NULL ORDER BY o.created_at FOR UPDATE OF a,o SKIP LOCKED LIMIT 1").bind(now).fetch_optional(&mut *tx).await.map_err(err)?;
        let job = row
            .map(|r| {
                Ok::<MintJob, AppError>(MintJob {
                    achievement_id: r.try_get("id").map_err(err)?,
                    verification_reference: r.try_get("verification_reference").map_err(err)?,
                    wallet_address: r.try_get("wallet_address").map_err(err)?,
                    attempts: r.try_get("attempt_count").map_err(err)?,
                })
            })
            .transpose()?;
        if let Some(j) = &job {
            sqlx::query("UPDATE blockchain_achievements SET status='MINTING' WHERE id=$1")
                .bind(j.achievement_id)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(job)
    }
    async fn minted(&self, id: Uuid, r: &MintResult) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        sqlx::query("UPDATE blockchain_achievements SET status='MINTED',mint_identifier=$2,transaction_signature=$3,minted_at=now(),last_error=NULL,next_attempt_at=NULL WHERE id=$1 AND status='MINTING'").bind(id).bind(&r.mint_identifier).bind(&r.transaction_signature).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE outbox_events SET processed_at=now() WHERE aggregate_id=$1 AND event_type='achievement.mint'").bind(id).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(())
    }
    async fn failed(
        &self,
        id: Uuid,
        n: i32,
        retry: Option<DateTime<Utc>>,
        e: &str,
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        sqlx::query("UPDATE blockchain_achievements SET status=CASE WHEN $2 IS NULL THEN 'FAILED'::achievement_mint_status ELSE 'FAILED'::achievement_mint_status END,attempt_count=$3,next_attempt_at=$2,last_error=left($4,500) WHERE id=$1").bind(id).bind(retry).bind(n).bind(e).execute(&mut *tx).await.map_err(err)?;
        if let Some(at) = retry {
            sqlx::query("UPDATE outbox_events SET available_at=$2 WHERE aggregate_id=$1 AND event_type='achievement.mint'").bind(id).bind(at).execute(&mut *tx).await.map_err(err)?;
        } else {
            sqlx::query("UPDATE outbox_events SET processed_at=now() WHERE aggregate_id=$1 AND event_type='achievement.mint'").bind(id).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
}
