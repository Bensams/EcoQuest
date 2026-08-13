//! Organization application routes.

use axum::{extract::State, routing::post, Json, Router};
use ecoquest_application::{CreateOrganizationCommand, Organization};
use serde::Deserialize;

use crate::{auth::AuthUser, state::AppState, ApiError};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/organizations", post(create))
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub organization_type: String,
    pub location: String,
    #[serde(default)]
    pub description: String,
}

/// Creates a PENDING organization application owned by the caller.
async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateOrganizationInput>,
) -> Result<(axum::http::StatusCode, Json<Organization>), ApiError> {
    let command = CreateOrganizationCommand {
        name: input.name,
        organization_type: input.organization_type,
        location: input.location,
        description: input.description,
    };
    let organization = s.organization_service()?.create(command, user.id).await?;
    Ok((axum::http::StatusCode::CREATED, Json(organization)))
}