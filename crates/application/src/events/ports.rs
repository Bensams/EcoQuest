//! Persistence port for mission use cases.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::models::{CreateEventCommand, Event, EventQrToken, Participation, UpdateEventCommand};
use crate::AppResult;

#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    async fn is_organization_member(&self, organization_id: Uuid, user_id: Uuid)
        -> AppResult<bool>;
    async fn create_event(&self, command: CreateEventCommand, actor_id: Uuid) -> AppResult<Event>;
    async fn update_event(
        &self,
        event_id: Uuid,
        command: UpdateEventCommand,
    ) -> AppResult<Option<Event>>;
    async fn find_event(&self, event_id: Uuid) -> AppResult<Option<Event>>;
    async fn list_published_events(&self, now: DateTime<Utc>) -> AppResult<Vec<Event>>;
    /// Lists every event for an organization regardless of status.
    async fn list_organization_events(&self, organization_id: Uuid) -> AppResult<Vec<Event>>;
    async fn transition_event(
        &self,
        event_id: Uuid,
        expected_status: &str,
        next_status: &str,
    ) -> AppResult<bool>;
    async fn revoke_event_qr_tokens(&self, event_id: Uuid) -> AppResult<()>;
    async fn insert_event_qr_token(
        &self,
        event_id: Uuid,
        token_hash: &[u8],
        actor_id: Uuid,
        activates_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> AppResult<EventQrToken>;
    async fn find_event_qr_token(&self, token_hash: &[u8]) -> AppResult<Option<EventQrToken>>;
    /// Atomically inserts registration after state/capacity checks.
    async fn join_event(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> AppResult<Participation>;
    /// Changes REGISTERED only; repeated scans cannot change pending verification.
    async fn check_in(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        qr_token_id: Uuid,
        now: DateTime<Utc>,
    ) -> AppResult<Option<Participation>>;
    async fn list_participants(&self, event_id: Uuid) -> AppResult<Vec<Participation>>;
    async fn find_participation(&self, participation_id: Uuid) -> AppResult<Option<Participation>>;
    async fn verify_participation(
        &self,
        participation_id: Uuid,
        verifier_id: Uuid,
        reason: &str,
    ) -> AppResult<super::models::VerificationResult>;
    async fn reject_participation(
        &self,
        participation_id: Uuid,
        verifier_id: Uuid,
        reason: &str,
    ) -> AppResult<super::models::VerificationResult>;
}
