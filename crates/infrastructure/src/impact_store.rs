//! PostgreSQL read model for verified environmental impact.
use crate::PgStore;
use ecoquest_application::{
    impact::{CommunityGoal, ImpactStats, ImpactStore, MetricTotal},
    AppError, AppResult,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgImpactStore {
    pool: PgPool,
}
impl PgImpactStore {
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

impl PgImpactStore {
    async fn stats(&self, filter: &str, id: Option<Uuid>) -> AppResult<ImpactStats> {
        let clause = if filter.is_empty() {
            String::new()
        } else {
            format!(" AND {filter}")
        };
        let count_sql = format!("SELECT count(DISTINCT p.event_id) AS verified_activities,count(DISTINCT p.user_id) AS active_participants FROM participations p JOIN events e ON e.id=p.event_id WHERE p.status='VERIFIED'{clause}");
        let counts = if let Some(id) = id {
            sqlx::query(&count_sql).bind(id).fetch_one(&self.pool).await
        } else {
            sqlx::query(&count_sql).fetch_one(&self.pool).await
        }
        .map_err(db_err)?;
        let metric_sql = format!("SELECT c.metric,c.unit,COALESCE(sum(c.value),0) AS value FROM impact_contributions c JOIN participations p ON p.id=c.participation_id JOIN events e ON e.id=c.event_id WHERE p.status='VERIFIED'{clause} GROUP BY c.metric,c.unit ORDER BY c.metric");
        let rows = if let Some(id) = id {
            sqlx::query(&metric_sql)
                .bind(id)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query(&metric_sql).fetch_all(&self.pool).await
        }
        .map_err(db_err)?;
        let certificate_filter = match filter {
            "p.user_id=$1" => "c.user_id=$1",
            "e.organization_id=$1" => "e.organization_id=$1",
            _ => "true",
        };
        let certificates_sql = format!("SELECT count(*) AS value FROM certificates c JOIN events e ON e.id=c.event_id WHERE {certificate_filter}");
        let certificates: i64 = if let Some(id) = id {
            sqlx::query(&certificates_sql)
                .bind(id)
                .fetch_one(&self.pool)
                .await
        } else {
            sqlx::query(&certificates_sql).fetch_one(&self.pool).await
        }
        .map_err(db_err)?
        .try_get("value")
        .map_err(db_err)?;
        Ok(ImpactStats {
            verified_activities: counts.try_get("verified_activities").map_err(db_err)?,
            active_participants: counts.try_get("active_participants").map_err(db_err)?,
            approved_organizations: sqlx::query_scalar(
                "SELECT count(*) FROM organizations WHERE verification_status='APPROVED'",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?,
            certificates_issued: certificates,
            metrics: rows
                .iter()
                .map(|r| {
                    Ok(MetricTotal {
                        metric: r.try_get("metric").map_err(db_err)?,
                        unit: r.try_get("unit").map_err(db_err)?,
                        value: r.try_get("value").map_err(db_err)?,
                    })
                })
                .collect::<AppResult<_>>()?,
        })
    }
}
#[async_trait::async_trait]
impl ImpactStore for PgImpactStore {
    async fn platform_stats(&self) -> AppResult<ImpactStats> {
        self.stats("", None).await
    }
    async fn organization_stats(&self, id: Uuid) -> AppResult<ImpactStats> {
        self.stats("e.organization_id=$1", Some(id)).await
    }
    async fn player_stats(&self, id: Uuid) -> AppResult<ImpactStats> {
        self.stats("p.user_id=$1", Some(id)).await
    }
    async fn community_goal(&self) -> AppResult<CommunityGoal> {
        let row = sqlx::query("SELECT g.name,g.metric,g.unit,g.target_value,COALESCE((SELECT sum(c.value) FROM impact_contributions c JOIN participations p ON p.id=c.participation_id WHERE p.status='VERIFIED' AND c.metric=g.metric AND c.unit=g.unit),0) AS current_value FROM community_goals g WHERE g.active").fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(CommunityGoal {
            name: row.try_get("name").map_err(db_err)?,
            metric: row.try_get("metric").map_err(db_err)?,
            unit: row.try_get("unit").map_err(db_err)?,
            target_value: row.try_get("target_value").map_err(db_err)?,
            current_value: row.try_get("current_value").map_err(db_err)?,
        })
    }
}
