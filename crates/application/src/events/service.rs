//! Mission lifecycle and QR check-in orchestration.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, EventStatus, VerificationStatus};
use uuid::Uuid;

use super::{
    models::{
        Activity, CreateEventCommand, Event, EventQrToken, Participation, UpdateEventCommand,
    },
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

    /// Fails unless `actor_id` owns `organization_id`.
    ///
    /// Ownership is the only authorization for organization event management:
    /// the platform no longer tracks organization membership.
    async fn require_owner(&self, organization_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        if self
            .store
            .is_organization_owner(organization_id, actor_id)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::Domain(DomainError::Forbidden(
                "organization ownership required".into(),
            )))
        }
    }

    /// Fails unless `actor_id` owns `organization_id` and the organization is
    /// approved. Every owner action that advances an event or mints value —
    /// creating, editing, publishing, activating, issuing check-in codes, and
    /// adjudicating participation — requires a verified organization, so a
    /// suspended or rejected organization cannot keep running events. Reads and
    /// cancellation stay owner-only so a suspended organization can still see
    /// its data and wind events down.
    async fn require_approved_owner(&self, organization_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        self.require_owner(organization_id, actor_id).await?;
        match self
            .store
            .organization_verification_status(organization_id)
            .await?
        {
            Some(VerificationStatus::Approved) => Ok(()),
            _ => Err(AppError::Domain(DomainError::Forbidden(
                "only approved organizations may run events".into(),
            ))),
        }
    }

    pub async fn create(&self, command: CreateEventCommand, actor_id: Uuid) -> AppResult<Event> {
        validate_create(&command)?;
        self.require_approved_owner(command.organization_id, actor_id)
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
        let event = self
            .event_for_approved_organizer(event_id, actor_id)
            .await?;
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
        let event = self.event_for_organizer(event_id, actor_id).await?;
        self.require_approved_owner(event.organization_id, actor_id)
            .await?;
        self.transition(
            event_id,
            actor_id,
            EventStatus::Draft,
            EventStatus::Published,
        )
        .await
    }

    pub async fn activate(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<()> {
        let event = self.event_for_organizer(event_id, actor_id).await?;
        self.require_approved_owner(event.organization_id, actor_id)
            .await?;
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
        let event = self
            .event_for_approved_organizer(event_id, actor_id)
            .await?;
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
    /// Lists the user's registrations joined with their events.
    pub async fn my_activities(&self, user_id: Uuid) -> AppResult<Vec<Activity>> {
        self.store.list_my_activities(user_id).await
    }
    /// Lists every event belonging to an organization the actor owns.
    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<Vec<Event>> {
        self.require_owner(organization_id, actor_id).await?;
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
        self.require_approved_owner(event.organization_id, actor_id)
            .await
    }

    async fn event_for_organizer(&self, event_id: Uuid, actor_id: Uuid) -> AppResult<Event> {
        let event = self.detail(event_id).await?;
        self.require_owner(event.organization_id, actor_id).await?;
        Ok(event)
    }

    /// [`Self::event_for_organizer`] that additionally requires the owning
    /// organization to be approved.
    async fn event_for_approved_organizer(
        &self,
        event_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<Event> {
        let event = self.detail(event_id).await?;
        self.require_approved_owner(event.organization_id, actor_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::models::VerificationResult;
    use ecoquest_domain::{ActivityType, ParticipationStatus};

    const ORG: Uuid = Uuid::from_u128(1);
    const EVENT: Uuid = Uuid::from_u128(2);
    const ACTOR: Uuid = Uuid::from_u128(3);
    const PARTICIPATION: Uuid = Uuid::from_u128(4);

    /// Minimal [`EventStore`] answering only the ownership and verification
    /// lookups the guards make. Anything past a guard is unreachable in these
    /// tests and stays unimplemented on purpose: if a guard ever stops firing,
    /// the test panics instead of silently passing.
    struct GuardStore {
        status: VerificationStatus,
    }

    fn sample_event() -> Event {
        Event {
            id: EVENT,
            organization_id: ORG,
            organization_name: "Org".into(),
            created_by: Some(ACTOR),
            name: "Event".into(),
            description: "d".into(),
            activity_type: ActivityType::BeachCleanup,
            location: "x".into(),
            starts_at: Utc::now(),
            ends_at: Utc::now() + chrono::Duration::hours(2),
            capacity: 10,
            eco_points: 100,
            status: EventStatus::Active,
            impacts: vec![],
            registered_count: 0,
        }
    }

    #[async_trait::async_trait]
    impl EventStore for GuardStore {
        async fn is_organization_owner(&self, _: Uuid, _: Uuid) -> AppResult<bool> {
            Ok(true)
        }
        async fn organization_verification_status(
            &self,
            _: Uuid,
        ) -> AppResult<Option<VerificationStatus>> {
            Ok(Some(self.status))
        }
        async fn find_event(&self, _: Uuid) -> AppResult<Option<Event>> {
            Ok(Some(sample_event()))
        }
        async fn find_participation(&self, id: Uuid) -> AppResult<Option<Participation>> {
            Ok(Some(Participation {
                id,
                event_id: EVENT,
                user_id: ACTOR,
                username: Some("player".into()),
                status: ParticipationStatus::PendingVerification,
                registered_at: Utc::now(),
                checked_in_at: Some(Utc::now()),
            }))
        }
        async fn create_event(&self, _: CreateEventCommand, _: Uuid) -> AppResult<Event> {
            unimplemented!("guard should have rejected")
        }
        async fn update_event(&self, _: Uuid, _: UpdateEventCommand) -> AppResult<Option<Event>> {
            unimplemented!("guard should have rejected")
        }
        async fn list_published_events(&self, _: DateTime<Utc>) -> AppResult<Vec<Event>> {
            unimplemented!()
        }
        async fn list_organization_events(&self, _: Uuid) -> AppResult<Vec<Event>> {
            unimplemented!()
        }
        async fn transition_event(&self, _: Uuid, _: &str, _: &str) -> AppResult<bool> {
            unimplemented!("guard should have rejected")
        }
        async fn revoke_event_qr_tokens(&self, _: Uuid) -> AppResult<()> {
            unimplemented!("guard should have rejected")
        }
        async fn insert_event_qr_token(
            &self,
            _: Uuid,
            _: &[u8],
            _: Uuid,
            _: DateTime<Utc>,
            _: DateTime<Utc>,
        ) -> AppResult<EventQrToken> {
            unimplemented!("guard should have rejected")
        }
        async fn find_event_qr_token(&self, _: &[u8]) -> AppResult<Option<EventQrToken>> {
            unimplemented!()
        }
        async fn join_event(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> AppResult<Participation> {
            unimplemented!()
        }
        async fn check_in(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
            _: DateTime<Utc>,
        ) -> AppResult<Option<Participation>> {
            unimplemented!()
        }
        async fn list_participants(&self, _: Uuid) -> AppResult<Vec<Participation>> {
            unimplemented!()
        }
        async fn list_my_activities(&self, _: Uuid) -> AppResult<Vec<Activity>> {
            unimplemented!()
        }
        async fn verify_participation(
            &self,
            _: Uuid,
            _: Uuid,
            _: &str,
        ) -> AppResult<VerificationResult> {
            unimplemented!("guard should have rejected")
        }
        async fn reject_participation(
            &self,
            _: Uuid,
            _: Uuid,
            _: &str,
        ) -> AppResult<VerificationResult> {
            unimplemented!("guard should have rejected")
        }
    }

    fn service(status: VerificationStatus) -> EventService {
        EventService::new(Arc::new(GuardStore { status }))
    }

    #[track_caller]
    fn assert_not_approved(err: &AppError, label: &str) {
        match err {
            AppError::Domain(DomainError::Forbidden(m)) => assert!(
                m.contains("only approved organizations"),
                "{label}: unexpected message {m}"
            ),
            other => panic!("{label}: expected Forbidden, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn suspended_organization_cannot_issue_check_in_codes() {
        let now = Utc::now();
        let err = service(VerificationStatus::Suspended)
            .rotate_qr(EVENT, ACTOR, now, now + chrono::Duration::hours(1))
            .await
            .expect_err("suspended organization must not mint a check-in code");
        assert_not_approved(&err, "rotate_qr");
    }

    #[tokio::test]
    async fn suspended_organization_cannot_adjudicate_participation() {
        let err = service(VerificationStatus::Suspended)
            .verify_participation(EVENT, PARTICIPATION, ACTOR, "reason")
            .await
            .expect_err("suspended organization must not award points");
        assert_not_approved(&err, "verify_participation");

        let err = service(VerificationStatus::Suspended)
            .reject_participation(EVENT, PARTICIPATION, ACTOR, "reason")
            .await
            .expect_err("suspended organization must not adjudicate");
        assert_not_approved(&err, "reject_participation");
    }

    #[tokio::test]
    async fn rejected_and_pending_organizations_are_blocked_too() {
        for status in [VerificationStatus::Rejected, VerificationStatus::Pending] {
            let now = Utc::now();
            let err = service(status)
                .rotate_qr(EVENT, ACTOR, now, now + chrono::Duration::hours(1))
                .await
                .expect_err("only approved organizations may run events");
            assert_not_approved(&err, "rotate_qr");
        }
    }
}
