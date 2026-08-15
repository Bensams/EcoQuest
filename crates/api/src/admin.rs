//! Platform administration routes. Every handler requires [`AdminUser`].

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use ecoquest_application::admin::{
    AdminEvent, AdminEventDetail, AdminListQuery, AdminOrganization, AdminOrganizationDetail,
    AdminUser as AdminUserRow, AdminUserDetail, EventModeration, Page, PasswordResetLink,
    PointCorrection,
};
use ecoquest_domain::{Role, UserStatus, VerificationStatus};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::AdminUser, state::AppState, ApiError};

/// Namespace router for administration.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/organizations", get(list_organizations))
        .route(
            "/api/admin/organizations/:organization_id",
            get(organization_detail),
        )
        .route(
            "/api/admin/organizations/:organization_id/status",
            patch(set_organization_status),
        )
        .route("/api/admin/users", get(list_users))
        .route("/api/admin/users/:user_id", get(user_detail))
        .route("/api/admin/users/:user_id/role", patch(set_user_role))
        .route("/api/admin/users/:user_id/status", patch(set_user_status))
        .route(
            "/api/admin/users/:user_id/points",
            post(correct_user_points),
        )
        .route(
            "/api/admin/users/:user_id/password-reset",
            post(issue_password_reset),
        )
        .route("/api/admin/events", get(list_events))
        .route("/api/admin/events/:event_id", get(event_detail))
        .route("/api/admin/events/:event_id/moderate", post(moderate_event))
        .route("/api/admin/events/:event_id/cancel", post(cancel_event))
        .route(
            "/api/admin/participations/:participation_id/flag",
            post(flag_participation),
        )
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub organization_id: Option<Uuid>,
    pub activity_type: Option<String>,
    pub location: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub flagged: Option<bool>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl From<ListQuery> for AdminListQuery {
    fn from(value: ListQuery) -> Self {
        Self {
            q: value.q,
            status: value.status,
            organization_id: value.organization_id,
            activity_type: value.activity_type,
            location: value.location,
            from: value.from,
            to: value.to,
            flagged: value.flagged,
            sort: value.sort,
            order: value.order,
            page: value.page.unwrap_or(1),
            per_page: value.per_page.unwrap_or(20),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StatusInput {
    pub status: VerificationStatus,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct UserStatusInput {
    pub status: UserStatus,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RoleInput {
    pub role: Role,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelEventInput {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ModerateEventInput {
    pub action: EventModeration,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct PointCorrectionInput {
    pub amount: i32,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct FlagInput {
    pub flagged: bool,
    pub reason: String,
}

async fn list_organizations(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Page<AdminOrganization>>, ApiError> {
    Ok(Json(
        s.admin_service()?.list_organizations(query.into()).await?,
    ))
}

async fn organization_detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<AdminOrganizationDetail>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .organization_detail(organization_id)
            .await?,
    ))
}

async fn set_organization_status(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(organization_id): Path<Uuid>,
    Json(input): Json<StatusInput>,
) -> Result<Json<AdminOrganization>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .set_organization_status(organization_id, input.status, actor.id, &input.reason)
            .await?,
    ))
}

async fn list_users(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Page<AdminUserRow>>, ApiError> {
    Ok(Json(s.admin_service()?.list_users(query.into()).await?))
}

async fn user_detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<AdminUserDetail>, ApiError> {
    Ok(Json(s.admin_service()?.user_detail(user_id).await?))
}

async fn set_user_role(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<RoleInput>,
) -> Result<Json<AdminUserRow>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .set_user_role(user_id, input.role, actor.id, &input.reason)
            .await?,
    ))
}

async fn set_user_status(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<UserStatusInput>,
) -> Result<Json<AdminUserRow>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .set_user_status(user_id, input.status, actor.id, &input.reason)
            .await?,
    ))
}

async fn correct_user_points(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<PointCorrectionInput>,
) -> Result<Json<PointCorrection>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .correct_points(user_id, input.amount, actor.id, &input.reason)
            .await?,
    ))
}

async fn issue_password_reset(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PasswordResetLink>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .issue_password_reset(user_id, actor.id, &s.web_base_url)
            .await?,
    ))
}

async fn list_events(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Page<AdminEvent>>, ApiError> {
    Ok(Json(s.admin_service()?.list_events(query.into()).await?))
}

async fn event_detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(event_id): Path<Uuid>,
) -> Result<Json<AdminEventDetail>, ApiError> {
    Ok(Json(s.admin_service()?.event_detail(event_id).await?))
}

async fn moderate_event(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(event_id): Path<Uuid>,
    Json(input): Json<ModerateEventInput>,
) -> Result<Json<AdminEvent>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .moderate_event(event_id, input.action, actor.id, &input.reason)
            .await?,
    ))
}

async fn cancel_event(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(event_id): Path<Uuid>,
    Json(input): Json<CancelEventInput>,
) -> Result<Json<AdminEvent>, ApiError> {
    Ok(Json(
        s.admin_service()?
            .moderate_event(event_id, EventModeration::Cancel, actor.id, &input.reason)
            .await?,
    ))
}

async fn flag_participation(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(participation_id): Path<Uuid>,
    Json(input): Json<FlagInput>,
) -> Result<(), ApiError> {
    s.admin_service()?
        .flag_participation(participation_id, input.flagged, actor.id, &input.reason)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_input_deserializes_enums() {
        let json = r#"{"role":"ADMIN","reason":"grant platform access"}"#;
        let input: RoleInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.role, Role::Admin);
        assert_eq!(input.reason, "grant platform access");
    }

    #[test]
    fn status_input_deserializes_enums() {
        let json = r#"{"status":"APPROVED","reason":"documents verified"}"#;
        let input: StatusInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.status, VerificationStatus::Approved);
    }

    #[test]
    fn cancel_event_input_deserializes() {
        let json = r#"{"reason":"safety concern"}"#;
        let input: CancelEventInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.reason, "safety concern");
    }

    #[test]
    fn moderate_event_input_deserializes_actions() {
        let json = r#"{"action":"SUSPEND","reason":"weather"}"#;
        let input: ModerateEventInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.action, EventModeration::Suspend);
    }
}
