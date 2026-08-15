//! Mission lifecycle and QR check-in orchestration.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, EventStatus, ParticipationStatus, VerificationStatus};
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

    /// Checks declared impacts against the metric catalogue.
    ///
    /// Impact aggregates group by `(metric, unit)` and the community goal
    /// matches on both, so an unrecognised metric or a stray unit would silently
    /// strand a mission's contribution instead of failing visibly. The ceiling
    /// stops one organizer's typo from dominating the platform-wide total.
    async fn validate_impacts(&self, impacts: &[super::models::EventImpact]) -> AppResult<()> {
        if impacts.is_empty() {
            return Ok(());
        }
        let catalog = self.store.list_impact_metrics().await?;
        let invalid = |message: String| AppError::Domain(DomainError::Validation(message));
        let mut seen = std::collections::HashSet::new();
        for impact in impacts {
            let Some(known) = catalog.iter().find(|m| m.metric == impact.metric) else {
                let names: Vec<&str> = catalog.iter().map(|m| m.metric.as_str()).collect();
                return Err(invalid(format!(
                    "unknown impact metric '{}'; choose one of: {}",
                    impact.metric,
                    names.join(", ")
                )));
            };
            if !seen.insert(impact.metric.as_str()) {
                return Err(invalid(format!(
                    "impact metric '{}' is listed more than once",
                    impact.metric
                )));
            }
            if impact.unit != known.unit {
                return Err(invalid(format!(
                    "'{}' is recorded in {}, not '{}'",
                    impact.metric, known.unit, impact.unit
                )));
            }
            // NaN fails the database CHECK as an opaque constraint violation;
            // reject it here so the caller learns what was wrong.
            if !impact.expected_value.is_finite() || impact.expected_value < 0.0 {
                return Err(invalid(format!("'{}' must be zero or more", impact.metric)));
            }
            if impact.expected_value > known.max_per_participant {
                return Err(invalid(format!(
                    "'{}' is capped at {} {} per volunteer; {} was given",
                    impact.metric, known.max_per_participant, known.unit, impact.expected_value
                )));
            }
        }
        Ok(())
    }

    pub async fn create(&self, command: CreateEventCommand, actor_id: Uuid) -> AppResult<Event> {
        validate_create(&command)?;
        self.validate_impacts(&command.impacts).await?;
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
        self.validate_impacts(&command.impacts).await?;
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
        // Check-in only succeeds inside the mission window, so a code whose
        // validity never overlaps that window can never be scanned. Refusing it
        // here beats handing the organizer a code that silently fails on site.
        if expires_at <= event.starts_at || activates_at >= event.ends_at {
            return Err(AppError::Domain(DomainError::Validation(format!(
                "a check-in code is only usable while the mission runs ({} to {}); \
                 choose a validity window that overlaps it",
                event.starts_at.to_rfc3339(),
                event.ends_at.to_rfc3339()
            ))));
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
        if let Some(participation) = self
            .store
            .check_in(token.event_id, user_id, token.id, now)
            .await?
        {
            return Ok(participation);
        }
        // The conditional UPDATE reports only that nothing matched. Work out
        // which precondition failed so the scanner sees an actionable reason
        // instead of one catch-all conflict.
        Err(self
            .explain_failed_check_in(token.event_id, user_id, now)
            .await)
    }

    /// Builds the error for a check-in the database refused. Any lookup failure
    /// here degrades to the generic conflict rather than masking the refusal.
    async fn explain_failed_check_in(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> AppError {
        let generic = || {
            AppError::Domain(DomainError::Conflict(
                "participation is not eligible for check-in".into(),
            ))
        };
        let Ok(Some(event)) = self.store.find_event(event_id).await else {
            return generic();
        };
        let conflict = |message: String| AppError::Domain(DomainError::Conflict(message));
        if event.status != EventStatus::Active {
            return conflict(format!(
                "this mission is not open for check-in yet; the organizer has not started it (status {})",
                event.status.as_str()
            ));
        }
        if now < event.starts_at {
            return conflict(format!(
                "check-in opens when the mission starts at {}",
                event.starts_at.to_rfc3339()
            ));
        }
        if now >= event.ends_at {
            return conflict(format!(
                "this mission ended at {}; check-in is closed",
                event.ends_at.to_rfc3339()
            ));
        }
        let Ok(participation) = self
            .store
            .find_participation_for_user(event_id, user_id)
            .await
        else {
            return generic();
        };
        match participation.map(|p| p.status) {
            None => conflict("join this mission before checking in".into()),
            Some(ParticipationStatus::PendingVerification) => {
                conflict("you have already checked in; the organizer will verify you".into())
            }
            Some(ParticipationStatus::Verified) => {
                conflict("your participation in this mission is already verified".into())
            }
            Some(ParticipationStatus::Rejected) => {
                conflict("your participation in this mission was rejected".into())
            }
            Some(ParticipationStatus::Cancelled) => {
                conflict("your registration for this mission was cancelled".into())
            }
            Some(ParticipationStatus::Registered) => generic(),
        }
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
    use ecoquest_domain::ActivityType;

    const ORG: Uuid = Uuid::from_u128(1);
    const EVENT: Uuid = Uuid::from_u128(2);
    const ACTOR: Uuid = Uuid::from_u128(3);
    const PARTICIPATION: Uuid = Uuid::from_u128(4);

    /// Minimal [`EventStore`] answering only the ownership, event and
    /// participation lookups the guards and the check-in diagnosis make.
    /// Anything past a guard is unreachable in these tests and stays
    /// unimplemented on purpose: if a guard ever stops firing, the test panics
    /// instead of silently passing.
    ///
    /// `check_in` always reports "nothing matched", which is what drives
    /// [`EventService::explain_failed_check_in`].
    struct GuardStore {
        status: VerificationStatus,
        event: Event,
        participation: Option<Participation>,
    }

    impl GuardStore {
        fn new(status: VerificationStatus) -> Self {
            Self {
                status,
                event: sample_event(),
                participation: Some(sample_participation(ParticipationStatus::Registered)),
            }
        }
    }

    fn sample_participation(status: ParticipationStatus) -> Participation {
        Participation {
            id: PARTICIPATION,
            event_id: EVENT,
            user_id: ACTOR,
            username: Some("player".into()),
            status,
            registered_at: Utc::now(),
            checked_in_at: None,
        }
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
            // Already under way, so check-in's window checks pass by default and
            // each test only has to set up the one condition it is about.
            starts_at: Utc::now() - chrono::Duration::hours(1),
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
        async fn list_impact_metrics(&self) -> AppResult<Vec<crate::impact::ImpactMetric>> {
            Ok(vec![crate::impact::ImpactMetric {
                metric: "waste_collected".into(),
                label: "Waste collected".into(),
                unit: "kg".into(),
                max_per_participant: 500.0,
            }])
        }
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
            Ok(Some(self.event.clone()))
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
        // The two QR writes succeed so a *passing* validation is observable as
        // an Ok. The guard tests below still assert on the error, so a guard
        // that stops firing fails them just as loudly as a panic would.
        async fn revoke_event_qr_tokens(&self, _: Uuid) -> AppResult<()> {
            Ok(())
        }
        async fn insert_event_qr_token(
            &self,
            event_id: Uuid,
            _: &[u8],
            _: Uuid,
            activates_at: DateTime<Utc>,
            expires_at: DateTime<Utc>,
        ) -> AppResult<EventQrToken> {
            Ok(EventQrToken {
                id: Uuid::from_u128(5),
                event_id,
                activates_at,
                expires_at,
                revoked_at: None,
            })
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
            Ok(None)
        }
        async fn list_participants(&self, _: Uuid) -> AppResult<Vec<Participation>> {
            unimplemented!()
        }
        async fn find_participation_for_user(
            &self,
            _: Uuid,
            _: Uuid,
        ) -> AppResult<Option<Participation>> {
            Ok(self.participation.clone())
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
        EventService::new(Arc::new(GuardStore::new(status)))
    }

    fn service_with(store: GuardStore) -> EventService {
        EventService::new(Arc::new(store))
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

    /// The organizer used to be able to mint a code valid "now" for a mission
    /// scheduled for tomorrow. Every scan then failed, because check-in is only
    /// possible inside the mission window.
    #[tokio::test]
    async fn qr_window_must_overlap_the_mission_window() {
        let mut store = GuardStore::new(VerificationStatus::Approved);
        store.event.starts_at = Utc::now() + chrono::Duration::hours(20);
        store.event.ends_at = Utc::now() + chrono::Duration::hours(24);
        let now = Utc::now();
        let err = service_with(store)
            .rotate_qr(EVENT, ACTOR, now, now + chrono::Duration::hours(2))
            .await
            .expect_err("a code that expires before the mission starts is unusable");
        match err {
            AppError::Domain(DomainError::Validation(m)) => {
                assert!(m.contains("only usable while the mission runs"), "got {m}");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn qr_window_overlapping_the_mission_passes_validation() {
        let mut store = GuardStore::new(VerificationStatus::Approved);
        store.event.starts_at = Utc::now() + chrono::Duration::hours(1);
        store.event.ends_at = Utc::now() + chrono::Duration::hours(5);
        let now = Utc::now();
        service_with(store)
            .rotate_qr(EVENT, ACTOR, now, now + chrono::Duration::hours(3))
            .await
            .expect("a window overlapping the mission is valid");
    }

    #[track_caller]
    fn assert_conflict_contains(err: &AppError, needle: &str) {
        match err {
            AppError::Domain(DomainError::Conflict(m)) => {
                assert!(m.contains(needle), "expected {needle:?} in {m:?}");
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    /// A refused check-in must say *why*. These all used to collapse into
    /// "participation is not eligible for check-in".
    #[tokio::test]
    async fn refused_check_in_explains_the_reason() {
        let now = Utc::now();

        let mut before_start = GuardStore::new(VerificationStatus::Approved);
        before_start.event.starts_at = now + chrono::Duration::hours(3);
        before_start.event.ends_at = now + chrono::Duration::hours(6);
        assert_conflict_contains(
            &service_with(before_start)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "check-in opens when the mission starts",
        );

        let mut ended = GuardStore::new(VerificationStatus::Approved);
        ended.event.starts_at = now - chrono::Duration::hours(6);
        ended.event.ends_at = now - chrono::Duration::hours(3);
        assert_conflict_contains(
            &service_with(ended)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "check-in is closed",
        );

        let mut not_started = GuardStore::new(VerificationStatus::Approved);
        not_started.event.status = EventStatus::Published;
        assert_conflict_contains(
            &service_with(not_started)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "the organizer has not started it",
        );

        let mut never_joined = GuardStore::new(VerificationStatus::Approved);
        never_joined.participation = None;
        assert_conflict_contains(
            &service_with(never_joined)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "join this mission before checking in",
        );

        let mut already = GuardStore::new(VerificationStatus::Approved);
        already.participation = Some(sample_participation(
            ParticipationStatus::PendingVerification,
        ));
        assert_conflict_contains(
            &service_with(already)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "already checked in",
        );

        let mut verified = GuardStore::new(VerificationStatus::Approved);
        verified.participation = Some(sample_participation(ParticipationStatus::Verified));
        assert_conflict_contains(
            &service_with(verified)
                .explain_failed_check_in(EVENT, ACTOR, now)
                .await,
            "already verified",
        );
    }

    #[track_caller]
    fn assert_validation_contains(err: &AppError, needle: &str) {
        match err {
            AppError::Domain(DomainError::Validation(m)) => {
                assert!(m.contains(needle), "expected {needle:?} in {m:?}");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    fn impact(metric: &str, unit: &str, value: f64) -> super::super::models::EventImpact {
        super::super::models::EventImpact {
            metric: metric.into(),
            unit: unit.into(),
            expected_value: value,
        }
    }

    /// Free-text metrics silently split impact totals and slip past the
    /// community goal, which matches on metric *and* unit.
    #[tokio::test]
    async fn declared_impacts_must_match_the_catalogue() {
        let service = service(VerificationStatus::Approved);

        assert_validation_contains(
            &service
                .validate_impacts(&[impact("waste_colected", "kg", 4.0)])
                .await
                .expect_err("a metric outside the catalogue must be refused"),
            "unknown impact metric",
        );

        assert_validation_contains(
            &service
                .validate_impacts(&[impact("waste_collected", "kgs", 4.0)])
                .await
                .expect_err("a stray unit must be refused, not silently stranded"),
            "recorded in kg",
        );

        assert_validation_contains(
            &service
                .validate_impacts(&[impact("waste_collected", "kg", 900.0)])
                .await
                .expect_err("an organizer must not declare unbounded impact"),
            "capped at",
        );

        assert_validation_contains(
            &service
                .validate_impacts(&[impact("waste_collected", "kg", f64::NAN)])
                .await
                .expect_err("NaN would fail the database CHECK opaquely"),
            "zero or more",
        );

        assert_validation_contains(
            &service
                .validate_impacts(&[
                    impact("waste_collected", "kg", 4.0),
                    impact("waste_collected", "kg", 5.0),
                ])
                .await
                .expect_err("a duplicate metric would violate the unique index"),
            "more than once",
        );

        service
            .validate_impacts(&[impact("waste_collected", "kg", 4.0)])
            .await
            .expect("a catalogued metric at a sane value is valid");
        service
            .validate_impacts(&[])
            .await
            .expect("declaring no impact stays allowed");
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
