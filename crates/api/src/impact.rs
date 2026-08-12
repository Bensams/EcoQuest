//! Environmental impact dashboard routes.
use crate::{auth::AuthUser, state::AppState, ApiError};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/impact", get(platform))
        .route("/api/impact/community-goal", get(goal))
        .route(
            "/api/impact/organizations/:organization_id",
            get(organization),
        )
        .route("/api/impact/me", get(player))
}
async fn platform(
    State(s): State<AppState>,
) -> Result<Json<ecoquest_application::impact::ImpactStats>, ApiError> {
    Ok(Json(s.impact_service()?.platform().await?))
}
async fn goal(
    State(s): State<AppState>,
) -> Result<Json<ecoquest_application::impact::CommunityGoal>, ApiError> {
    Ok(Json(s.impact_service()?.goal().await?))
}
async fn organization(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ecoquest_application::impact::ImpactStats>, ApiError> {
    Ok(Json(s.impact_service()?.organization(id).await?))
}
async fn player(
    State(s): State<AppState>,
    user: AuthUser,
) -> Result<Json<ecoquest_application::impact::ImpactStats>, ApiError> {
    Ok(Json(s.impact_service()?.player(user.id).await?))
}
