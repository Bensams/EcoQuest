//! Central API error type and wire format.
//!
//! Every endpoint returns errors as `{"error":{"code":"...","message":"..."}}`
//! so clients can branch on a stable code instead of parsing prose.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ecoquest_application::AppError;
use ecoquest_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Error payload envelope.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorBody {
    /// The error details.
    pub error: ApiErrorDetail,
}

/// Stable error code plus a human readable message.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    /// Machine readable code, e.g. `not_found`.
    pub code: String,
    /// Human readable message, safe to show to end users.
    pub message: String,
}

/// Error type returned by handlers.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Request failed validation.
    #[error("{0}")]
    BadRequest(String),
    /// Caller is not authenticated.
    #[error("{0}")]
    Unauthorized(String),
    /// Caller may not perform this operation.
    #[error("{0}")]
    Forbidden(String),
    /// Entity does not exist.
    #[error("{0}")]
    NotFound(String),
    /// Operation conflicts with current state.
    #[error("{0}")]
    Conflict(String),
    /// Caller exceeded a rate limit.
    #[error("{0}")]
    TooManyRequests(String),
    /// A dependency is unavailable (e.g. database down).
    #[error("{0}")]
    ServiceUnavailable(String),
    /// Unexpected failure; details stay in the logs.
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    /// HTTP status paired with this error.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine readable code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized(_) => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::TooManyRequests(_) => "too_many_requests",
            Self::ServiceUnavailable(_) => "service_unavailable",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::Validation(m) => Self::BadRequest(m),
            DomainError::NotFound(m) => Self::NotFound(m),
            DomainError::Conflict(m) => Self::Conflict(m),
            DomainError::Forbidden(m) => Self::Forbidden(m),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Domain(d) => d.into(),
            // Never leak connection strings or driver detail to clients.
            AppError::Infrastructure(detail) => {
                tracing::error!(error = %detail, "infrastructure failure");
                Self::ServiceUnavailable("a dependency is currently unavailable".into())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, code = self.code(), "request failed");
        }
        let body = ApiErrorBody {
            error: ApiErrorDetail {
                code: self.code().to_string(),
                message: self.to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_domain_errors_to_status_codes() {
        let cases = [
            (DomainError::Validation("x".into()), StatusCode::BAD_REQUEST),
            (DomainError::NotFound("x".into()), StatusCode::NOT_FOUND),
            (DomainError::Conflict("x".into()), StatusCode::CONFLICT),
            (DomainError::Forbidden("x".into()), StatusCode::FORBIDDEN),
        ];
        for (domain, expected) in cases {
            assert_eq!(ApiError::from(domain).status(), expected);
        }
    }

    #[test]
    fn infrastructure_detail_is_not_exposed() {
        let err = ApiError::from(AppError::Infrastructure("postgres://secret".into()));
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(!err.to_string().contains("secret"));
    }
}
