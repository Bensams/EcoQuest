//! Data crossing event use-case boundaries.

use chrono::{DateTime, Utc};
use ecoquest_domain::{ActivityType, EventStatus, ParticipationStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventImpact {
    pub metric: String,
    pub unit: String,
    pub expected_value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub created_by: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub activity_type: ActivityType,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    pub status: EventStatus,
    pub impacts: Vec<EventImpact>,
    pub registered_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Participation {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub status: ParticipationStatus,
    pub registered_at: DateTime<Utc>,
    pub checked_in_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationResult {
    pub participation_id: Uuid,
    pub status: ParticipationStatus,
    pub points_awarded: i32,
    pub player_eco_points: i32,
    pub player_level: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventQrToken {
    pub id: Uuid,
    pub event_id: Uuid,
    pub activates_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl EventQrToken {
    #[must_use]
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.activates_at <= now && self.expires_at > now
    }
}

#[derive(Debug)]
pub struct CreateEventCommand {
    pub organization_id: Uuid,
    pub name: String,
    pub description: String,
    pub activity_type: ActivityType,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    pub impacts: Vec<EventImpact>,
}

#[derive(Debug)]
pub struct UpdateEventCommand {
    pub name: String,
    pub description: String,
    pub activity_type: ActivityType,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    pub impacts: Vec<EventImpact>,
}
