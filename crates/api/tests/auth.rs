//! Integration tests for the authentication endpoints.

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use ecoquest_domain::UserStatus;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use support::test_app;
use tower::ServiceExt;

/// Response of a single call: status, JSON body and any `Set-Cookie` values.
struct Call {
    status: StatusCode,
    body: Value,
    cookies: Vec<String>,
}

impl Call {
    /// Value of a cookie set by this response, if any.
    fn cookie(&self, name: &str) -> Option<String> {
        self.cookies
            .iter()
            .find_map(|c| c.strip_prefix(&format!("{name}=")))
            .map(|rest| {
                rest.split(';')
                    .next()
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string()
            })
    }

    fn cookie_attrs(&self, name: &str) -> String {
        self.cookies
            .iter()
            .find(|c| c.starts_with(&format!("{name}=")))
            .cloned()
            .unwrap_or_default()
    }
}

async fn call(app: &Router, req: Request<Body>) -> Call {
    let response = app.clone().oneshot(req).await.expect("response");
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(ToString::to_string))
        .collect();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json")
    };
    Call {
        status,
        body,
        cookies,
    }
}

fn post(path: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn post_with_cookie(path: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("request")
}

/// Attaches a client address so the rate limiter and audit log see an IP,
/// mimicking what `into_make_service_with_connect_info` does in production.
fn with_client(mut req: Request<Body>, addr: std::net::SocketAddr) -> Request<Body> {
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(addr));
    req
}

fn get_with_cookie(path: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("request")
}

fn registration(email: &str, username: &str) -> Value {
    json!({
        "username": username,
        "email": email,
        "password": "correct-horse-battery-1",
    })
}

async fn register(app: &Router, email: &str, username: &str) -> Call {
    call(
        app,
        post("/api/auth/register", &registration(email, username)),
    )
    .await
}

#[tokio::test]
async fn registration_creates_a_session_in_httponly_cookies() {
    let (app, store) = test_app(100, 900);
    let res = register(&app, "player@example.com", "eco_player").await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["user"]["email"], "player@example.com");
    assert_eq!(res.body["user"]["role"], "PLAYER");
    assert!(res.body["user"]["password_hash"].is_null());
    // Tokens must never reach JavaScript.
    assert!(res.body.get("access_token").is_none());
    assert!(res.body.get("refresh_token").is_none());

    let attrs = res.cookie_attrs("eq_access");
    assert!(attrs.contains("HttpOnly"), "access cookie must be HttpOnly");
    assert!(attrs.contains("SameSite=Strict"), "got {attrs}");
    let refresh_attrs = res.cookie_attrs("eq_refresh");
    assert!(refresh_attrs.contains("HttpOnly"));
    assert!(
        refresh_attrs.contains("Path=/api/auth"),
        "got {refresh_attrs}"
    );

    assert!(store
        .audit_actions()
        .contains(&"user.registered".to_string()));
}

#[tokio::test]
async fn duplicate_email_is_rejected_with_conflict() {
    let (app, _) = test_app(100, 900);
    register(&app, "dupe@example.com", "first_user").await;
    let res = register(&app, "DUPE@example.com", "second_user").await;

    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.body["error"]["code"], "conflict");
}

#[tokio::test]
async fn invalid_registration_input_is_rejected() {
    let (app, _) = test_app(100, 900);
    for (field, value) in [
        ("email", json!("not-an-email")),
        ("password", json!("short")),
        ("username", json!("x")),
    ] {
        let mut body = registration("ok@example.com", "eco_player");
        body[field] = value;
        let res = call(&app, post("/api/auth/register", &body)).await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "field {field} should be rejected"
        );
        assert_eq!(res.body["error"]["code"], "bad_request");
    }
}

#[tokio::test]
async fn admin_role_cannot_be_self_assigned() {
    let (app, _) = test_app(100, 900);
    let mut body = registration("sneaky@example.com", "sneaky_user");
    body["role"] = json!("ADMIN");
    let res = call(&app, post("/api/auth/register", &body)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_succeeds_with_correct_credentials() {
    let (app, store) = test_app(100, 900);
    register(&app, "player@example.com", "eco_player").await;

    let res = call(
        &app,
        post(
            "/api/auth/login",
            &json!({ "email": "player@example.com", "password": "correct-horse-battery-1" }),
        ),
    )
    .await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["user"]["email"], "player@example.com");
    assert!(res.cookie("eq_access").is_some());
    assert!(store.audit_actions().contains(&"user.login".to_string()));
}

#[tokio::test]
async fn login_with_wrong_password_or_unknown_email_is_unauthorized() {
    let (app, store) = test_app(100, 900);
    register(&app, "player@example.com", "eco_player").await;

    for body in [
        json!({ "email": "player@example.com", "password": "wrong-password-99" }),
        json!({ "email": "nobody@example.com", "password": "correct-horse-battery-1" }),
    ] {
        let res = call(&app, post("/api/auth/login", &body)).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        // Identical message either way: no account enumeration.
        assert_eq!(res.body["error"]["message"], "invalid credentials");
        assert!(res.cookie("eq_access").is_none());
    }
    assert_eq!(
        store
            .audit_actions()
            .iter()
            .filter(|a| *a == "user.login_failed")
            .count(),
        2
    );
}

#[tokio::test]
async fn suspended_accounts_cannot_log_in() {
    let (app, store) = test_app(100, 900);
    register(&app, "player@example.com", "eco_player").await;
    store.set_status(store.only_user_id(), UserStatus::Suspended);

    let res = call(
        &app,
        post(
            "/api/auth/login",
            &json!({ "email": "player@example.com", "password": "correct-horse-battery-1" }),
        ),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_returns_the_current_user_and_requires_authentication() {
    let (app, _) = test_app(100, 900);
    let session = register(&app, "player@example.com", "eco_player").await;
    let access = session.cookie("eq_access").expect("access cookie");

    let res = call(
        &app,
        get_with_cookie("/api/auth/me", &format!("eq_access={access}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["email"], "player@example.com");

    let anonymous = call(
        &app,
        Request::builder()
            .uri("/api/auth/me")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_access_tokens_are_rejected() {
    // TTL of -1s makes every issued token already expired.
    let (app, _) = test_app(100, -1);
    let session = register(&app, "player@example.com", "eco_player").await;
    let access = session.cookie("eq_access").expect("access cookie");

    let res = call(
        &app,
        get_with_cookie("/api/auth/me", &format!("eq_access={access}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tampered_access_tokens_are_rejected() {
    let (app, _) = test_app(100, 900);
    let session = register(&app, "player@example.com", "eco_player").await;
    let mut access = session.cookie("eq_access").expect("access cookie");
    access.push('x');

    let res = call(
        &app,
        get_with_cookie("/api/auth/me", &format!("eq_access={access}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_rotates_the_token_and_rejects_reuse() {
    let (app, store) = test_app(100, 900);
    let session = register(&app, "player@example.com", "eco_player").await;
    let refresh = session.cookie("eq_refresh").expect("refresh cookie");

    let rotated = call(
        &app,
        post_with_cookie("/api/auth/refresh", &format!("eq_refresh={refresh}")),
    )
    .await;
    assert_eq!(rotated.status, StatusCode::OK);
    let new_refresh = rotated.cookie("eq_refresh").expect("new refresh cookie");
    assert_ne!(new_refresh, refresh, "refresh token must rotate");

    // Replaying the old token is treated as theft: it fails and kills the family.
    let replay = call(
        &app,
        post_with_cookie("/api/auth/refresh", &format!("eq_refresh={refresh}")),
    )
    .await;
    assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
    assert!(store
        .audit_actions()
        .contains(&"session.reuse_detected".to_string()));

    // The rotated token was revoked along with the rest of the family.
    let after_reuse = call(
        &app,
        post_with_cookie("/api/auth/refresh", &format!("eq_refresh={new_refresh}")),
    )
    .await;
    assert_eq!(after_reuse.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_sessions_cannot_be_refreshed() {
    let (app, store) = test_app(100, 900);
    let session = register(&app, "player@example.com", "eco_player").await;
    let refresh = session.cookie("eq_refresh").expect("refresh cookie");
    store.expire_all_tokens();

    let res = call(
        &app,
        post_with_cookie("/api/auth/refresh", &format!("eq_refresh={refresh}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_without_a_cookie_is_unauthorized() {
    let (app, _) = test_app(100, 900);
    let res = call(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/refresh")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_revokes_the_session_and_clears_cookies() {
    let (app, store) = test_app(100, 900);
    let session = register(&app, "player@example.com", "eco_player").await;
    let refresh = session.cookie("eq_refresh").expect("refresh cookie");

    let res = call(
        &app,
        post_with_cookie("/api/auth/logout", &format!("eq_refresh={refresh}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(res.cookie("eq_access").as_deref(), Some(""));
    assert!(store
        .audit_actions()
        .contains(&"session.logout".to_string()));

    let reuse = call(
        &app,
        post_with_cookie("/api/auth/refresh", &format!("eq_refresh={refresh}")),
    )
    .await;
    assert_eq!(reuse.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authentication_is_rate_limited_per_client() {
    let (app, _) = test_app(2, 900);
    let body = json!({ "email": "nobody@example.com", "password": "correct-horse-battery-1" });
    let client = "203.0.113.7:5000".parse().expect("addr");

    for _ in 0..2 {
        let res = call(&app, with_client(post("/api/auth/login", &body), client)).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    }
    let blocked = call(&app, with_client(post("/api/auth/login", &body), client)).await;
    assert_eq!(blocked.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(blocked.body["error"]["code"], "too_many_requests");

    // A different client is unaffected.
    let other = call(
        &app,
        with_client(
            post("/api/auth/login", &body),
            "198.51.100.4:5000".parse().expect("addr"),
        ),
    )
    .await;
    assert_eq!(other.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn role_guard_blocks_players_from_admin_routes() {
    let (state, _) = support::test_state(100, 900);
    let admin_only = Router::new()
        .route(
            "/admin",
            axum::routing::get(|_: ecoquest_api::auth::AdminUser| async { "ok" }),
        )
        .merge(ecoquest_api::auth::routes())
        .with_state(state);

    let session = register(&admin_only, "player@example.com", "eco_player").await;
    let access = session.cookie("eq_access").expect("access cookie");

    let res = call(
        &admin_only,
        get_with_cookie("/admin", &format!("eq_access={access}")),
    )
    .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.body["error"]["code"], "forbidden");

    let anonymous = call(
        &admin_only,
        Request::builder()
            .uri("/admin")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}
