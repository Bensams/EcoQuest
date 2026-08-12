//! Environmental mission lifecycle rules.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult};

/// Activity performed by an environmental mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityType {
    TreePlanting,
    WasteCollection,
    Recycling,
    BeachCleanup,
    EnergySaving,
    Education,
    Other,
}

impl ActivityType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TreePlanting => "TREE_PLANTING",
            Self::WasteCollection => "WASTE_COLLECTION",
            Self::Recycling => "RECYCLING",
            Self::BeachCleanup => "BEACH_CLEANUP",
            Self::EnergySaving => "ENERGY_SAVING",
            Self::Education => "EDUCATION",
            Self::Other => "OTHER",
        }
    }
}

impl FromStr for ActivityType {
    type Err = DomainError;

    fn from_str(value: &str) -> DomainResult<Self> {
        match value {
            "TREE_PLANTING" => Ok(Self::TreePlanting),
            "WASTE_COLLECTION" => Ok(Self::WasteCollection),
            "RECYCLING" => Ok(Self::Recycling),
            "BEACH_CLEANUP" => Ok(Self::BeachCleanup),
            "ENERGY_SAVING" => Ok(Self::EnergySaving),
            "EDUCATION" => Ok(Self::Education),
            "OTHER" => Ok(Self::Other),
            _ => Err(DomainError::Validation(format!(
                "unknown activity type: {value}"
            ))),
        }
    }
}

/// Event lifecycle. Only published events are joinable; only active events accept QR scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventStatus {
    Draft,
    Published,
    Active,
    Completed,
    Cancelled,
}

impl EventStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Published => "PUBLISHED",
            Self::Active => "ACTIVE",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
        }
    }

    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Published | Self::Cancelled)
                | (Self::Published, Self::Active | Self::Cancelled)
                | (Self::Active, Self::Completed | Self::Cancelled)
        )
    }

    #[must_use]
    pub const fn is_joinable(self) -> bool {
        matches!(self, Self::Published)
    }

    #[must_use]
    pub const fn accepts_check_in(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl FromStr for EventStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> DomainResult<Self> {
        match value {
            "DRAFT" => Ok(Self::Draft),
            "PUBLISHED" => Ok(Self::Published),
            "ACTIVE" => Ok(Self::Active),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            _ => Err(DomainError::Validation(format!(
                "unknown event status: {value}"
            ))),
        }
    }
}

/// Player state inside an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParticipationStatus {
    Registered,
    PendingVerification,
    Verified,
    Rejected,
    Cancelled,
}

impl ParticipationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "REGISTERED",
            Self::PendingVerification => "PENDING_VERIFICATION",
            Self::Verified => "VERIFIED",
            Self::Rejected => "REJECTED",
            Self::Cancelled => "CANCELLED",
        }
    }

    #[must_use]
    pub const fn can_check_in(self) -> bool {
        matches!(self, Self::Registered)
    }
}

impl FromStr for ParticipationStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> DomainResult<Self> {
        match value {
            "REGISTERED" => Ok(Self::Registered),
            "PENDING_VERIFICATION" => Ok(Self::PendingVerification),
            "VERIFIED" => Ok(Self::Verified),
            "REJECTED" => Ok(Self::Rejected),
            "CANCELLED" => Ok(Self::Cancelled),
            _ => Err(DomainError::Validation(format!(
                "unknown participation status: {value}"
            ))),
        }
    }
}

impl fmt::Display for EventStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_lifecycle_rejects_skipped_and_terminal_transitions() {
        assert!(EventStatus::Draft.can_transition_to(EventStatus::Published));
        assert!(!EventStatus::Draft.can_transition_to(EventStatus::Active));
        assert!(EventStatus::Active.can_transition_to(EventStatus::Completed));
        assert!(!EventStatus::Completed.can_transition_to(EventStatus::Published));
        assert!(!EventStatus::Cancelled.can_transition_to(EventStatus::Active));
    }

    #[test]
    fn only_expected_states_join_or_check_in() {
        assert!(EventStatus::Published.is_joinable());
        assert!(!EventStatus::Active.is_joinable());
        assert!(EventStatus::Active.accepts_check_in());
        assert!(!EventStatus::Published.accepts_check_in());
        assert!(ParticipationStatus::Registered.can_check_in());
        assert!(!ParticipationStatus::PendingVerification.can_check_in());
    }
}
