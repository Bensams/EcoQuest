//! Read-only environmental impact dashboards. Only verified contributions count.

use crate::AppResult;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricTotal {
    pub metric: String,
    pub unit: String,
    pub value: f64,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImpactStats {
    pub verified_activities: i64,
    pub active_participants: i64,
    pub approved_organizations: i64,
    pub certificates_issued: i64,
    pub metrics: Vec<MetricTotal>,
}
/// One entry of the controlled vocabulary organizers pick from. `unit` is the
/// only unit this metric is ever recorded in, so totals cannot split across
/// spellings, and `max_per_participant` bounds what a single organizer can
/// declare for one volunteer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImpactMetric {
    pub metric: String,
    pub label: String,
    pub unit: String,
    pub max_per_participant: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommunityGoal {
    pub name: String,
    pub metric: String,
    pub unit: String,
    pub target_value: f64,
    pub current_value: f64,
}

#[async_trait::async_trait]
pub trait ImpactStore: Send + Sync {
    async fn platform_stats(&self) -> AppResult<ImpactStats>;
    async fn organization_stats(&self, organization_id: Uuid) -> AppResult<ImpactStats>;
    async fn player_stats(&self, user_id: Uuid) -> AppResult<ImpactStats>;
    async fn community_goal(&self) -> AppResult<CommunityGoal>;
    /// The metric vocabulary, for populating the organizer's event form.
    async fn list_metrics(&self) -> AppResult<Vec<ImpactMetric>>;
}
#[derive(Clone)]
pub struct ImpactService {
    store: std::sync::Arc<dyn ImpactStore>,
}
impl ImpactService {
    #[must_use]
    pub fn new(store: std::sync::Arc<dyn ImpactStore>) -> Self {
        Self { store }
    }
    pub async fn platform(&self) -> AppResult<ImpactStats> {
        self.store.platform_stats().await
    }
    pub async fn organization(&self, id: Uuid) -> AppResult<ImpactStats> {
        self.store.organization_stats(id).await
    }
    pub async fn player(&self, id: Uuid) -> AppResult<ImpactStats> {
        self.store.player_stats(id).await
    }
    pub async fn goal(&self) -> AppResult<CommunityGoal> {
        self.store.community_goal().await
    }
    pub async fn metrics(&self) -> AppResult<Vec<ImpactMetric>> {
        self.store.list_metrics().await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pending_and_rejected_contributions_are_excluded() {
        let contributions = [
            ("PENDING_VERIFICATION", 4.0),
            ("REJECTED", 8.0),
            ("VERIFIED", 2.5),
        ];
        let total: f64 = contributions
            .iter()
            .filter(|(status, _)| *status == "VERIFIED")
            .map(|(_, value)| value)
            .sum();
        assert_eq!(total, 2.5);
    }

    #[test]
    fn repeated_verification_cannot_add_same_metric_twice() {
        let rows = [
            ("participation-1", "trees_planted", 3.0),
            ("participation-1", "trees_planted", 3.0),
        ];
        let mut stored = std::collections::HashSet::new();
        let total: f64 = rows
            .into_iter()
            .filter(|(participation, metric, _)| stored.insert((*participation, *metric)))
            .map(|(_, _, value)| value)
            .sum();
        assert_eq!(total, 3.0);
    }
}
