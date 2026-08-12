//! Wallet ownership and server-authoritative achievement endpoints.
use crate::{auth::extract::AuthUser, state::AppState, ApiError};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use ecoquest_application::achievements::Achievement;
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub wallet_address: String,
}
#[derive(Serialize)]
pub struct ChallengeResponse {
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}
#[derive(Deserialize)]
pub struct VerifyRequest {
    pub wallet_address: String,
    pub nonce: String,
    pub signature: String,
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/achievements/me", get(mine))
        .route("/api/wallet/challenge", post(challenge))
        .route("/api/wallet/verify", post(verify))
}
async fn mine(State(s): State<AppState>, u: AuthUser) -> Result<Json<Vec<Achievement>>, ApiError> {
    Ok(Json(s.achievement_service()?.mine(u.id).await?))
}
async fn challenge(
    State(s): State<AppState>,
    u: AuthUser,
    Json(b): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, ApiError> {
    let c = s
        .achievement_service()?
        .challenge(u.id, &b.wallet_address)
        .await?;
    Ok(Json(ChallengeResponse {
        nonce: c.nonce,
        message: c.message,
        expires_at: c.expires_at,
    }))
}
async fn verify(
    State(s): State<AppState>,
    u: AuthUser,
    Json(b): Json<VerifyRequest>,
) -> Result<(), ApiError> {
    s.achievement_service()?
        .verify_wallet(u.id, &b.wallet_address, &b.nonce, &b.signature)
        .await?;
    Ok(())
}
