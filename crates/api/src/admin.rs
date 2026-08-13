//! Platform administration routes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
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
            "/api/admin/organizations/:organization_id/status",
            patch(set_organization_status),
        )
        .route("/api/admin/users", get(list_users))
        .route("/api/admin/users/:user_id/role", patch(set_user_role))
        .route("/api/admin/users/:user_id/status", patch(set_user_status))
}

async fn list_organizations(
    State(s): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<ecoquest_application::AdminOrganization>>, ApiError> {
    Ok(Json(s.admin_service()?.list_organizations().await?))
}

#[derive(Debug, Deserialize)]
pub struct StatusInput {
    pub status: VerificationStatus,
}

#[derive(Debug, Deserialize)]
pub struct UserStatusInput {
    pub status: UserStatus,
}

#[derive(Debug, Deserialize)]
pub struct RoleInput {
    pub role: Role,
}

async fn set_organization_status(
    State(s): State<AppState>,
    AdminUser(actor): AdminUser,
    Path(organization_id): Path<Uuid>,
    Json(input): Json<StatusInput>,
) -> Result<StatusCode, ApiError> {
    s.admin_service()?
        .set_organization_status(organization_id, input.status, actor.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(
    State(s): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<ecoquest_application::AdminUser>>, ApiError> {
    Ok(Json(s.admin_service()?.list_users().await?))
}

async fn set_user_role(
    State(s): State<AppState>,
    AdminUser(_actor): AdminUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<RoleInput>,
) -> Result<StatusCode, ApiError> {
    s.admin_service()?
        .set_user_role(user_id, input.role)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_user_status(
    State(s): State<AppState>,
    AdminUser(_actor): AdminUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<UserStatusInput>,
) -> Result<StatusCode, ApiError> {
    s.admin_service()?
        .set_user_status(user_id, input.status)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_input_deserializes_enums() {
        let json = r#"{"role":"ADMIN"}"#;
        let input: RoleInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.role, Role::Admin);
    }

    #[test]
    fn status_input_deserializes_enums() {
        let json = r#"{"status":"APPROVED"}"#;
        let input: StatusInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.status, VerificationStatus::Approved);
    }
}
