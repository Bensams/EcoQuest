//! Health endpoint: reports API liveness plus PostgreSQL readiness.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Health response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// `ok` when every dependency is reachable, otherwise `degraded`.
    pub status: &'static str,
    /// Crate version of the running API.
    pub version: &'static str,
    /// Per dependency readiness.
    pub dependencies: Dependencies,
}

/// Dependency readiness details.
#[derive(Debug, Serialize, Deserialize)]
pub struct Dependencies {
    /// `up` or `down`.
    pub database: &'static str,
}

/// `GET /api/health`.
///
/// Returns `200` when PostgreSQL answers, `503` when it does not, so container
/// orchestrators can use it directly as a readiness probe.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_up = match state.health_probe.check().await {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %err, "database readiness check failed");
            false
        }
    };

    let status = if db_up {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(HealthResponse {
            status: if db_up { "ok" } else { "degraded" },
            version: env!("CARGO_PKG_VERSION"),
            dependencies: Dependencies {
                database: if db_up { "up" } else { "down" },
            },
        }),
    )
}
