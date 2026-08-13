//! Platform administration use cases (organizations and users).

use chrono::{DateTime, Utc};
use ecoquest_domain::{DomainError, EventStatus, Role, UserStatus, VerificationStatus};
use uuid::Uuid;

use crate::{AppError, AppResult};

/// Organization row as listed on the admin dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminOrganization {
    pub id: Uuid,
    pub name: String,
    pub organization_type: String,
    pub location: String,
    pub description: String,
    pub verification_status: VerificationStatus,
    pub owner_id: Option<Uuid>,
    pub owner_username: Option<String>,
    pub event_count: i64,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User row as listed on the admin dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub status: UserStatus,
    pub eco_points: i32,
    pub created_at: DateTime<Utc>,
}

/// Event row as listed on the admin moderation dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub owner_username: String,
    pub name: String,
    pub activity_type: String,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: EventStatus,
    pub registered_count: i64,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait AdminStore: Send + Sync {
    /// Lists all organizations with owner and event-count information.
    async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>>;
    /// Sets the verification status of an organization. Returns false when it does not exist.
    async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
    ) -> AppResult<bool>;
    /// Lists all users.
    async fn list_users(&self) -> AppResult<Vec<AdminUser>>;
    /// Sets the platform role of a user. Returns false when it does not exist.
    async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<bool>;
    /// Sets the lifecycle status of a user. Returns false when it does not exist.
    async fn set_user_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<bool>;
    /// Lists all events with organization and owner information.
    async fn list_events(&self) -> AppResult<Vec<AdminEvent>>;
    /// Reads a single event with organization and owner information.
    async fn find_event(&self, event_id: Uuid) -> AppResult<Option<AdminEvent>>;
    /// Cancels an event on behalf of an admin, recording who and why.
    async fn cancel_event(
        &self,
        event_id: Uuid,
        cancelled_by: Uuid,
        reason: &str,
    ) -> AppResult<bool>;
}

/// Administration use cases.
#[derive(Clone)]
pub struct AdminService {
    store: std::sync::Arc<dyn AdminStore>,
}

impl AdminService {
    #[must_use]
    pub fn new(store: std::sync::Arc<dyn AdminStore>) -> Self {
        Self { store }
    }

    pub async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>> {
        self.store.list_organizations().await
    }

    pub async fn set_organization_status(
        &self,
        organization_id: Uuid,
        status: VerificationStatus,
        reviewer_id: Uuid,
    ) -> AppResult<()> {
        if !self
            .store
            .set_organization_status(organization_id, status, reviewer_id)
            .await?
        {
            return Err(AppError::Domain(DomainError::NotFound(
                "organization".into(),
            )));
        }
        Ok(())
    }

    pub async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
        self.store.list_users().await
    }

    pub async fn set_user_role(&self, user_id: Uuid, role: Role) -> AppResult<()> {
        if !self.store.set_user_role(user_id, role).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        Ok(())
    }

    pub async fn set_user_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<()> {
        if !self.store.set_user_status(user_id, status).await? {
            return Err(AppError::Domain(DomainError::NotFound("user".into())));
        }
        Ok(())
    }

    pub async fn list_events(&self) -> AppResult<Vec<AdminEvent>> {
        self.store.list_events().await
    }

    pub async fn find_event(&self, event_id: Uuid) -> AppResult<AdminEvent> {
        self.store
            .find_event(event_id)
            .await?
            .ok_or_else(|| AppError::Domain(DomainError::NotFound("event".into())))
    }

    pub async fn cancel_event(&self, event_id: Uuid, actor_id: Uuid, reason: &str) -> AppResult<()> {
        if reason.trim().is_empty() {
            return Err(AppError::Domain(DomainError::Validation(
                "a cancellation reason is required".into(),
            )));
        }
        let event = self.find_event(event_id).await?;
        if event.status == EventStatus::Cancelled || event.status == EventStatus::Completed {
            return Err(AppError::Domain(DomainError::Conflict(
                "event cannot be cancelled".into(),
            )));
        }
        if !self
            .store
            .cancel_event(event_id, actor_id, reason.trim())
            .await?
        {
            return Err(AppError::Domain(DomainError::Conflict(
                "event state changed; retry".into(),
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub that reports the event as cancellable (DRAFT) and records cancel calls.
    struct StubAdminStore {
        cancelled: std::sync::Mutex<Vec<(Uuid, Uuid, String)>>,
    }

    impl StubAdminStore {
        fn event() -> AdminEvent {
            AdminEvent {
                id: Uuid::new_v4(),
                organization_id: Uuid::new_v4(),
                organization_name: "Green Earth Butuan".into(),
                owner_username: "eco_organizer".into(),
                name: "Coastal Cleanup".into(),
                activity_type: "CLEANUP".into(),
                location: "Butuan Bay".into(),
                starts_at: Utc::now(),
                ends_at: Utc::now() + chrono::Duration::hours(2),
                status: EventStatus::Draft,
                registered_count: 0,
                cancelled_by: None,
                cancelled_at: None,
                cancellation_reason: None,
                created_at: Utc::now(),
            }
        }
    }

    #[async_trait::async_trait]
    impl AdminStore for StubAdminStore {
        async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>> {
            Ok(vec![])
        }
        async fn set_organization_status(
            &self,
            _organization_id: Uuid,
            _status: VerificationStatus,
            _reviewer_id: Uuid,
        ) -> AppResult<bool> {
            Ok(true)
        }
        async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
            Ok(vec![])
        }
        async fn set_user_role(&self, _user_id: Uuid, _role: Role) -> AppResult<bool> {
            Ok(true)
        }
        async fn set_user_status(&self, _user_id: Uuid, _status: UserStatus) -> AppResult<bool> {
            Ok(true)
        }
        async fn list_events(&self) -> AppResult<Vec<AdminEvent>> {
            Ok(vec![Self::event()])
        }
        async fn find_event(&self, _event_id: Uuid) -> AppResult<Option<AdminEvent>> {
            Ok(Some(Self::event()))
        }
        async fn cancel_event(
            &self,
            event_id: Uuid,
            cancelled_by: Uuid,
            reason: &str,
        ) -> AppResult<bool> {
            self.cancelled
                .lock()
                .unwrap()
                .push((event_id, cancelled_by, reason.to_string()));
            Ok(true)
        }
    }

    #[tokio::test]
    async fn cancel_requires_a_reason() {
        let service = AdminService::new(std::sync::Arc::new(StubAdminStore {
            cancelled: std::sync::Mutex::new(vec![]),
        }));
        let err = service
            .cancel_event(Uuid::new_v4(), Uuid::new_v4(), "   ")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn cancel_records_who_and_why() {
        let store = std::sync::Arc::new(StubAdminStore {
            cancelled: std::sync::Mutex::new(vec![]),
        });
        let service = AdminService::new(store.clone());
        let event_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        service
            .cancel_event(event_id, admin_id, "unsafe location")
            .await
            .expect("cancel should succeed");
        let calls = store.cancelled.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, event_id);
        assert_eq!(calls[0].1, admin_id);
        assert_eq!(calls[0].2, "unsafe location");
    }

    #[tokio::test]
    async fn cancelled_or_completed_events_cannot_be_cancelled() {
        struct DoneStore;
        #[async_trait::async_trait]
        impl AdminStore for DoneStore {
            async fn list_organizations(&self) -> AppResult<Vec<AdminOrganization>> {
                Ok(vec![])
            }
            async fn set_organization_status(
                &self,
                _id: Uuid,
                _s: VerificationStatus,
                _r: Uuid,
            ) -> AppResult<bool> {
                Ok(true)
            }
            async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
                Ok(vec![])
            }
            async fn set_user_role(&self, _id: Uuid, _r: Role) -> AppResult<bool> {
                Ok(true)
            }
            async fn set_user_status(&self, _id: Uuid, _s: UserStatus) -> AppResult<bool> {
                Ok(true)
            }
            async fn list_events(&self) -> AppResult<Vec<AdminEvent>> {
                Ok(vec![])
            }
            async fn find_event(&self, _id: Uuid) -> AppResult<Option<AdminEvent>> {
                let mut event = StubAdminStore::event();
                event.status = EventStatus::Cancelled;
                Ok(Some(event))
            }
            async fn cancel_event(
                &self,
                _id: Uuid,
                _by: Uuid,
                _reason: &str,
            ) -> AppResult<bool> {
                Ok(true)
            }
        }
        let service = AdminService::new(std::sync::Arc::new(DoneStore));
        let err = service
            .cancel_event(Uuid::new_v4(), Uuid::new_v4(), "nope")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::Conflict(_))));
    }
}
