//! Authentication endpoints.
//!
//! Tokens travel in HttpOnly cookies, never in the response body, so browser
//! JavaScript (and therefore XSS) cannot read them.

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use ecoquest_application::auth::{
    models::{AuthOutcome, LoginCommand, RegisterCommand, RequestContext, UserProfile},
    SessionTokens,
};
use ecoquest_domain::{EmailAddress, Password, Role, Username};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use super::extract::{AuthUser, ACCESS_COOKIE, REFRESH_COOKIE};
use crate::{state::AppState, ApiError};

/// Registration payload.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Desired username.
    pub username: String,
    /// Desired email address.
    pub email: String,
    /// Plaintext password.
    pub password: String,
    /// Requested role; defaults to `PLAYER`.
    #[serde(default)]
    pub role: Option<Role>,
}

/// Login payload.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Registered email address.
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Successful authentication response. Contains no tokens by design.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    /// The authenticated user.
    pub user: UserProfile,
    /// Seconds until the access cookie expires, so clients know when to refresh.
    pub expires_in: i64,
}

/// Builds the `/api/auth` routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let context = request_context(&headers, connect_info.as_ref());
    state.enforce_auth_rate_limit(context.ip_address)?;

    let command = RegisterCommand {
        username: Username::parse(&body.username)?,
        email: EmailAddress::parse(&body.email)?,
        password: Password::parse(&body.password)?,
        role: body.role.unwrap_or(Role::Player),
    };
    let outcome = state.auth.register(command, &context).await?;
    Ok(session_response(&state, jar, outcome, StatusCode::CREATED))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let context = request_context(&headers, connect_info.as_ref());
    state.enforce_auth_rate_limit(context.ip_address)?;

    let command = LoginCommand {
        email: EmailAddress::parse(&body.email)
            // Never reveal that the address was merely malformed.
            .map_err(|_| ApiError::Unauthorized("invalid credentials".into()))?,
        password: body.password,
    };
    let outcome = state
        .auth
        .login(command, &context)
        .await
        .map_err(login_failure)?;
    Ok(session_response(&state, jar, outcome, StatusCode::OK))
}

async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> Result<impl IntoResponse, ApiError> {
    let context = request_context(&headers, connect_info.as_ref());
    state.enforce_auth_rate_limit(context.ip_address)?;

    let token = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| ApiError::Unauthorized("no session cookie".into()))?;

    let outcome = state
        .auth
        .refresh(&token, &context)
        .await
        .map_err(|_| ApiError::Unauthorized("session is invalid or expired".into()))?;
    Ok(session_response(&state, jar, outcome, StatusCode::OK))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> Result<impl IntoResponse, ApiError> {
    let context = request_context(&headers, connect_info.as_ref());
    if let Some(cookie) = jar.get(REFRESH_COOKIE) {
        state.auth.logout(cookie.value(), &context).await?;
    }
    // Always clear cookies so a client with a stale token ends up logged out.
    let jar = jar
        .add(clear_cookie(&state, ACCESS_COOKIE))
        .add(clear_cookie(&state, REFRESH_COOKIE));
    Ok((StatusCode::NO_CONTENT, jar))
}

async fn me(State(state): State<AppState>, user: AuthUser) -> Result<Json<UserProfile>, ApiError> {
    Ok(Json(state.auth.current_user(user.id).await?))
}

/// Login failures are reported as 401 regardless of the underlying cause, so the
/// endpoint cannot be used to enumerate accounts.
fn login_failure(err: ecoquest_application::AppError) -> ApiError {
    match ApiError::from(err) {
        ApiError::Forbidden(_) => ApiError::Unauthorized("invalid credentials".into()),
        other => other,
    }
}

fn session_response(
    state: &AppState,
    jar: CookieJar,
    outcome: AuthOutcome,
    status: StatusCode,
) -> impl IntoResponse {
    let body = SessionResponse {
        user: outcome.user,
        expires_in: outcome.tokens.access_expires_in,
    };
    let jar = set_session_cookies(state, jar, &outcome.tokens);
    (status, jar, Json(body))
}

fn set_session_cookies(state: &AppState, jar: CookieJar, tokens: &SessionTokens) -> CookieJar {
    let access = build_cookie(
        state,
        ACCESS_COOKIE,
        tokens.access_token.clone(),
        tokens.access_expires_in,
        "/",
    );
    // The refresh cookie is scoped to the refresh and logout paths only, so it is
    // not sent with every ordinary API request.
    let refresh = build_cookie(
        state,
        REFRESH_COOKIE,
        tokens.refresh_token.clone(),
        state.auth.refresh_ttl_secs(),
        "/api/auth",
    );
    jar.add(access).add(refresh)
}

fn build_cookie(
    state: &AppState,
    name: &'static str,
    value: String,
    max_age_secs: i64,
    path: &'static str,
) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    cookie.set_http_only(true);
    // Secure is disabled only for plain-HTTP local development.
    cookie.set_secure(state.cookies_secure);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path(path);
    cookie.set_max_age(Some(time::Duration::seconds(max_age_secs)));
    cookie
}

fn clear_cookie(state: &AppState, name: &'static str) -> Cookie<'static> {
    let path = if name == REFRESH_COOKIE {
        "/api/auth"
    } else {
        "/"
    };
    let mut cookie = build_cookie(state, name, String::new(), 0, path);
    cookie.make_removal();
    cookie
}

fn request_context(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<SocketAddr>>,
) -> RequestContext {
    RequestContext {
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.chars().take(255).collect()),
        ip_address: connect_info.map(|ConnectInfo(addr)| addr.ip()),
    }
}
