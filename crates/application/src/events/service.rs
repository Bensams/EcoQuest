//! Mission lifecycle and QR check-in orchestration.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, EventStatus};
use uuid::Uuid;

use super::{
    models::{CreateEventCommand, Event, EventQrToken, Participation, UpdateEventCommand},
    ports::EventStore,
    qr::{generate_check_in_code, hash_check_in_code},
};
use crate::{AppError, AppResult};

#[derive(Clone)]
pub struct EventService {
    store: Arc<dyn EventStore>,
}

impl EventService {
    #[must_use]
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        Self { store }
    }

    async fn require_organizer(&self, organization_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        if self
            .store
            .is_organization_member(organization_id, actor_id)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::Domain(DomainError::Forbidden(
                "organization membership required".into(),
            )))
        }
    }

    pub async fn create(&self, command: CreateEventCommand, actor_id: Uuid) -> AppResult<Event> {
        validate_create(&command)?;
        self.require_organizer(command.organization_id, actor_id)
            .await?;
        self.store.create_event(command, actor_id).await
    }

    pub async fn update(
        &self,
        event_id: Uuid,
        command: UpdateEventCommand,
        actor_id: Uuid,
    ) -> AppResult<Event> {
        validate_update(&command)?;
        let event = self.event_for_organizer(event_id, actor_id).await?;
        if event.status != EventStatus::Draft {
            return Err(AppError::Domain(DomainError::Conflict(
                "only draft events may be edited".into(),
            )));
        }
        self.store
            .update_event(event_id, command)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))
    }

    pub async fn publish(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        self.transition(
            event_id,
            actor_id,
            EventStatus::Draft,
            EventStatus::Published,
        )
        .await
    }

    pub async fn activate(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        self.transition(
            event_id,
            actor_id,
            EventStatus::Published,
            EventStatus::Active,
        )
        .await
    }

    async fn transition(
        &self,
        event_id: Uuid,
        actor_id: Uuid,
        expected: EventStatus,
        next: EventStatus,
    ) -> AppResult<()> {
        let event = self.event_for_organizer(event_id, actor_id).await?;
        if event.status != expected || !event.status.can_transition_to(next) {
            return Err(AppError::Domain(DomainError::Conflict(format!(
                "event is not {}",
                expected.as_str()
            ))));
        }
        if self
            .store
            .transition_event(event_id, expected.as_str(), next.as_str())
            .await?
        {
            Ok(())
        } else {
            Err(AppError::Domain(DomainError::Conflict(
                "event state changed; retry".into(),
            )))
        }
    }

    pub async fn cancel(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        let event = self.event_for_organizer(event_id, actor_id).await?;
        if !matches!(
            event.status,
            EventStatus::Draft | EventStatus::Published | EventStatus::Active
        ) {
            return Err(AppError::Domain(DomainError::Conflict(
                "event cannot be cancelled".into(),
            )));
        }
        if !self
            .store
            .transition_event(
                event_id,
                event.status.as_str(),
                EventStatus::Cancelled.as_str(),
            )
            .await?
        {
            return Err(AppError::Domain(DomainError::Conflict(
                "event state changed; retry".into(),
            )));
        }
        self.store.revoke_event_qr_tokens(event_id).await
    }

    pub async fn rotate_qr(
        &self,
        event_id: Uuid,
        actor_id: Uuid,
        activates_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> AppResult<(String, EventQrToken)> {
        let event = self.event_for_organizer(event_id, actor_id).await?;
        if event.status != EventStatus::Active {
            return Err(AppError::Domain(DomainError::Conflict(
                "only active events may have a check-in code".into(),
            )));
        }
        if expires_at <= activates_at {
            return Err(AppError::Domain(DomainError::Validation(
                "QR expiry must be after activation".into(),
            )));
        }
        self.store.revoke_event_qr_tokens(event_id).await?;
        let code = generate_check_in_code();
        let token = self
            .store
            .insert_event_qr_token(
                event_id,
                &hash_check_in_code(&code),
                actor_id,
                activates_at,
                expires_at,
            )
            .await?;
        Ok((code, token))
    }

    pub async fn browse(&self) -> AppResult<Vec<Event>> {
        self.store.list_published_events(Utc::now()).await
    }
    /// Lists every event belonging to an organization the actor belongs to.
    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<Vec<Event>> {
        self.require_organizer(organization_id, actor_id).await?;
        self.store.list_organization_events(organization_id).await
    }
    pub async fn detail(&self, event_id: Uuid) -> AppResult<Event> {
        self.store
            .find_event(event_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))
    }
    pub async fn join(&self, event_id: Uuid, user_id: Uuid) -> AppResult<Participation> {
        self.store.join_event(event_id, user_id, Utc::now()).await
    }

    pub async fn check_in(&self, code: &str, user_id: Uuid) -> AppResult<Participation> {
        let now = Utc::now();
        let token = self
            .store
            .find_event_qr_token(&hash_check_in_code(code))
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("check-in code".into())))?;
        if !token.is_usable(now) {
            return Err(AppError::Domain(DomainError::Conflict(
                "check-in code is expired or inactive".into(),
            )));
        }
        self.store
            .check_in(token.event_id, user_id, token.id, now)
            .await?
            .ok_or_else(|| {
                AppError::Domain(DomainError::Conflict(
                    "participation is not eligible for check-in".into(),
                ))
            })
    }

    pub async fn participants(
        &self,
        event_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<Vec<Participation>> {
        self.event_for_organizer(event_id, actor_id).await?;
        self.store.list_participants(event_id).await
    }

    pub async fn verify_participation(
        &self,
        event_id: Uuid,
        participation_id: Uuid,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<super::models::VerificationResult> {
        self.participation_for_organizer(event_id, participation_id, actor_id)
            .await?;
        self.store
            .verify_participation(participation_id, actor_id, reason)
            .await
    }

    pub async fn reject_participation(
        &self,
        event_id: Uuid,
        participation_id: Uuid,
        actor_id: Uuid,
        reason: &str,
    ) -> AppResult<super::models::VerificationResult> {
        self.participation_for_organizer(event_id, participation_id, actor_id)
            .await?;
        self.store
            .reject_participation(participation_id, actor_id, reason)
            .await
    }

    async fn participation_for_organizer(
        &self,
        event_id: Uuid,
        participation_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<()> {
        let participation = self
            .store
            .find_participation(participation_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("participation".into())))?;
        if participation.event_id != event_id {
            return Err(AppError::Domain(DomainError::NotFound(
                "participation".into(),
            )));
        }
        let event = self.detail(event_id).await?;
        self.require_organizer(event.organization_id, actor_id)
            .await
    }

    async fn event_for_organizer(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<Event> {
        let event = self.detail(event_id).await?;
        self.require_organizer(event.organization_id, actor_id)
            .await?;
        Ok(event)
    }
}

fn validate_common(
    name: &str,
    location: &str,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    capacity: i32,
    eco_points: i32,
) -> AppResult<()> {
    if name.trim().is_empty()
        || name.len() > 160
        || location.trim().is_empty()
        || location.len() > 300
    {
        return Err(AppError::Domain(DomainError::Validation(
            "event name or location is invalid".into(),
        )));
    }
    if ends_at <= starts_at {
        return Err(AppError::Domain(DomainError::Validation(
            "event end must be after start".into(),
        )));
    }
    if !(1..=100_000).contains(&capacity) || !(0..=100_000).contains(&eco_points) {
        return Err(AppError::Domain(DomainError::Validation(
            "event capacity or reward is invalid".into(),
        )));
    }
    Ok(())
}

fn validate_create(command: &CreateEventCommand) -> AppResult<()> {
    validate_common(
        &command.name,
        &command.location,
        command.starts_at,
        command.ends_at,
        command.capacity,
        command.eco_points,
    )
}

fn validate_update(command: &UpdateEventCommand) -> AppResult<()> {
    validate_common(
        &command.name,
        &command.location,
        command.starts_at,
        command.ends_at,
        command.capacity,
        command.eco_points,
    )
}
