//! Certificate download and public verification routes.
use crate::{auth::AuthUser, state::AppState, ApiError};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use ecoquest_application::certificates::Certificate;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/organizations/me/status", get(organization_status))
        .route(
            "/api/certificates/participations/:participation_id",
            get(mine),
        )
        .route(
            "/api/certificates/public/:verification_hash",
            get(public_verify),
        )
}

/// Returns the certificate only to the participant or a member of the hosting organization.
async fn mine(
    State(s): State<AppState>,
    user: AuthUser,
    Path(participation_id): Path<Uuid>,
) -> Result<Json<Certificate>, ApiError> {
    Ok(Json(
        s.certificate_service()?
            .for_requester(participation_id, user.id)
            .await?,
    ))
}

/// Public verification. Exposes only facts already printed on the certificate.
async fn public_verify(
    State(s): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<Certificate>, ApiError> {
    Ok(Json(s.certificate_service()?.verify_public(&hash).await?))
}

async fn organization_status(
    State(s): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ecoquest_application::certificates::OrganizationStatus>>, ApiError> {
    Ok(Json(
        s.certificate_service()?
            .organization_status(user.id)
            .await?,
    ))
}
