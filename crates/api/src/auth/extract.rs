//! Authentication extractors and role guards.

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use axum_extra::extract::cookie::CookieJar;
use ecoquest_domain::Role;
use uuid::Uuid;

use crate::{state::AppState, ApiError};

/// Name of the HttpOnly cookie holding the access token.
pub const ACCESS_COOKIE: &str = "eq_access";
/// Name of the HttpOnly cookie holding the refresh token.
pub const REFRESH_COOKIE: &str = "eq_refresh";

/// An authenticated caller, resolved from the access token.
///
/// The token is read from the `eq_access` cookie first and from an
/// `Authorization: Bearer` header as a fallback for non-browser clients
/// (the CLI). Browsers never need to touch the token, so it stays HttpOnly.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    /// Authenticated user id.
    pub id: Uuid,
    /// Role carried by the token.
    pub role: Role,
}

impl AuthUser {
    /// Fails unless the caller satisfies `required` (admins satisfy everything).
    ///
    /// # Errors
    /// Returns [`ApiError::Forbidden`] when the role is insufficient.
    pub fn require_role(&self, required: Role) -> Result<(), ApiError> {
        if self.role.satisfies(required) {
            Ok(())
        } else {
            Err(ApiError::Forbidden(
                "your role may not perform this action".into(),
            ))
        }
    }
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(ACCESS_COOKIE)
            .map(|c| c.value().to_string())
            .or_else(|| bearer_token(parts))
            .ok_or_else(|| ApiError::Unauthorized("authentication required".into()))?;

        let claims = state
            .auth
            .verify_access_token(&token)
            .map_err(|_| ApiError::Unauthorized("session is invalid or expired".into()))?;

        Ok(Self {
            id: claims.sub,
            role: claims.role,
        })
    }
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(ToString::to_string)
}

/// Extractor that additionally requires the `ADMIN` role.
#[derive(Debug, Clone, Copy)]
pub struct AdminUser(pub AuthUser);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        user.require_role(Role::Admin)?;
        Ok(Self(user))
    }
}

/// Extractor that requires organization membership (or admin).
#[derive(Debug, Clone, Copy)]
pub struct OrganizationUser(pub AuthUser);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for OrganizationUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        user.require_role(Role::OrganizationMember)?;
        Ok(Self(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_role_allows_exact_match_and_admin() {
        let player = AuthUser {
            id: Uuid::new_v4(),
            role: Role::Player,
        };
        let admin = AuthUser {
            id: Uuid::new_v4(),
            role: Role::Admin,
        };
        assert!(player.require_role(Role::Player).is_ok());
        assert!(player.require_role(Role::Admin).is_err());
        assert!(admin.require_role(Role::OrganizationMember).is_ok());
    }
}
