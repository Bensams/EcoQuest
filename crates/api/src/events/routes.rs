//! Mission, participant and QR routes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use ecoquest_application::events::{CreateEventCommand, EventImpact, UpdateEventCommand};
use ecoquest_domain::ActivityType;
use qrcode::{render::svg, QrCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    state::AppState,
    ApiError,
};

#[derive(Debug, Deserialize)]
pub struct EventInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub activity_type: ActivityType,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    #[serde(default)]
    pub impacts: Vec<EventImpact>,
}
#[derive(Debug, Deserialize)]
pub struct UpdateInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub activity_type: ActivityType,
    pub location: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub capacity: i32,
    pub eco_points: i32,
    #[serde(default)]
    pub impacts: Vec<EventImpact>,
}
#[derive(Debug, Deserialize)]
pub struct QrInput {
    pub activates_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Deserialize)]
pub struct CheckInInput {
    pub code: String,
}
#[derive(Debug, Deserialize)]
pub struct VerificationInput {
    pub participation_ids: Vec<Uuid>,
    #[serde(default)]
    pub reason: String,
}
#[derive(Serialize)]
struct VerificationBatchResponse {
    results: Vec<ecoquest_application::events::VerificationResult>,
}
#[derive(Serialize)]
struct QrResponse {
    code: String,
    activates_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    svg: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/events", get(browse))
        .route("/api/me/activities", get(my_activities))
        .route(
            "/api/organizations/:organization_id/events",
            get(list_organization_events).post(create),
        )
        .route("/api/events/:event_id", get(detail).put(update))
        .route("/api/events/:event_id/publish", post(publish))
        .route("/api/events/:event_id/activate", post(activate))
        .route("/api/events/:event_id/cancel", post(cancel))
        .route("/api/events/:event_id/join", post(join))
        .route("/api/events/:event_id/participants", get(participants))
        .route(
            "/api/events/:event_id/participants/verify",
            post(verify_participants),
        )
        .route(
            "/api/events/:event_id/participants/reject",
            post(reject_participants),
        )
        .route("/api/events/:event_id/qr", post(rotate_qr))
        .route("/api/check-in", post(check_in))
}

async fn browse(
    State(s): State<AppState>,
) -> Result<Json<Vec<ecoquest_application::events::Event>>, ApiError> {
    Ok(Json(s.event_service()?.browse().await?))
}
async fn my_activities(
    State(s): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ecoquest_application::events::Activity>>, ApiError> {
    Ok(Json(s.event_service()?.my_activities(user.id).await?))
}
async fn list_organization_events(
    State(s): State<AppState>,
    user: AuthUser,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<ecoquest_application::events::Event>>, ApiError> {
    Ok(Json(
        s.event_service()?
            .list_for_organization(organization_id, user.id)
            .await?,
    ))
}
async fn detail(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ecoquest_application::events::Event>, ApiError> {
    Ok(Json(s.event_service()?.detail(id).await?))
}
async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Path(organization_id): Path<Uuid>,
    Json(v): Json<EventInput>,
) -> Result<(StatusCode, Json<ecoquest_application::events::Event>), ApiError> {
    let c = CreateEventCommand {
        organization_id,
        name: v.name,
        description: v.description,
        activity_type: v.activity_type,
        location: v.location,
        starts_at: v.starts_at,
        ends_at: v.ends_at,
        capacity: v.capacity,
        eco_points: v.eco_points,
        impacts: v.impacts,
    };
    Ok((
        StatusCode::CREATED,
        Json(s.event_service()?.create(c, user.id).await?),
    ))
}
async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(v): Json<UpdateInput>,
) -> Result<Json<ecoquest_application::events::Event>, ApiError> {
    let c = UpdateEventCommand {
        name: v.name,
        description: v.description,
        activity_type: v.activity_type,
        location: v.location,
        starts_at: v.starts_at,
        ends_at: v.ends_at,
        capacity: v.capacity,
        eco_points: v.eco_points,
        impacts: v.impacts,
    };
    Ok(Json(s.event_service()?.update(id, c, user.id).await?))
}
async fn publish(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    s.event_service()?.publish(id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn activate(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    s.event_service()?.activate(id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn cancel(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    s.event_service()?.cancel(id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn join(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<
    (
        StatusCode,
        Json<ecoquest_application::events::Participation>,
    ),
    ApiError,
> {
    Ok((
        StatusCode::CREATED,
        Json(s.event_service()?.join(id, user.id).await?),
    ))
}
async fn participants(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ecoquest_application::events::Participation>>, ApiError> {
    Ok(Json(s.event_service()?.participants(id, user.id).await?))
}
async fn verify_participants(
    State(s): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(input): Json<VerificationInput>,
) -> Result<Json<VerificationBatchResponse>, ApiError> {
    if input.participation_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "select at least one participant".into(),
        ));
    }
    let mut results = Vec::with_capacity(input.participation_ids.len());
    for id in input.participation_ids {
        results.push(
            s.event_service()?
                .verify_participation(event_id, id, user.id, input.reason.trim())
                .await?,
        );
    }
    Ok(Json(VerificationBatchResponse { results }))
}
async fn reject_participants(
    State(s): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(input): Json<VerificationInput>,
) -> Result<Json<VerificationBatchResponse>, ApiError> {
    if input.participation_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "select at least one participant".into(),
        ));
    }
    let mut results = Vec::with_capacity(input.participation_ids.len());
    for id in input.participation_ids {
        results.push(
            s.event_service()?
                .reject_participation(event_id, id, user.id, input.reason.trim())
                .await?,
        );
    }
    Ok(Json(VerificationBatchResponse { results }))
}
async fn rotate_qr(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(v): Json<QrInput>,
) -> Result<Json<QrResponse>, ApiError> {
    let now = Utc::now();
    let (code, t) = s
        .event_service()?
        .rotate_qr(
            id,
            user.id,
            v.activates_at.unwrap_or(now),
            v.expires_at.unwrap_or(now + Duration::hours(2)),
        )
        .await?;
    let svg = QrCode::new(code.as_bytes())
        .map_err(|_| ApiError::BadRequest("could not render QR".into()))?
        .render::<svg::Color>()
        .min_dimensions(320, 320)
        .build();
    Ok(Json(QrResponse {
        code,
        activates_at: t.activates_at,
        expires_at: t.expires_at,
        svg,
    }))
}
async fn check_in(
    State(s): State<AppState>,
    user: AuthUser,
    Json(v): Json<CheckInInput>,
) -> Result<Json<ecoquest_application::events::Participation>, ApiError> {
    Ok(Json(
        s.event_service()?.check_in(v.code.trim(), user.id).await?,
    ))
}
