//! Integration tests for `GET /api/health` using a stub probe (no database).

// The shared helpers are compiled into each test binary; this one uses a subset.
#[allow(dead_code)]
mod support;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ecoquest_api::{auth::RateLimiter, router, AppState};
use ecoquest_application::auth::{AuthService, PasswordHasherService, TokenService};
use http_body_util::BodyExt;
use support::{MemoryStore, StubProbe, TEST_SECRET};
use tower::ServiceExt;

fn app_with_probe(db_up: bool) -> axum::Router {
    let auth = AuthService::new(
        Arc::new(MemoryStore::default()),
        PasswordHasherService::for_tests(),
        TokenService::new(TEST_SECRET, 900, 3600).expect("token service"),
    );
    let state = AppState::new(
        Arc::new(StubProbe(db_up)),
        Arc::new(auth),
        Arc::new(RateLimiter::new(100, 60)),
        false,
    );
    router(state, &["http://localhost:5173".to_string()])
}

async fn call_health(db_up: bool) -> (StatusCode, serde_json::Value) {
    let app = app_with_probe(db_up);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, serde_json::from_slice(&bytes).expect("json"))
}

#[tokio::test]
async fn health_returns_ok_when_database_is_reachable() {
    let (status, body) = call_health(true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["dependencies"]["database"], "up");
}

#[tokio::test]
async fn health_returns_service_unavailable_when_database_is_down() {
    let (status, body) = call_health(false).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["dependencies"]["database"], "down");
}

#[tokio::test]
async fn responses_have_request_id_and_security_headers() {
    let response = app_with_probe(true)
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(response.headers().contains_key("x-request-id"));
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert!(response.headers().contains_key("content-security-policy"));
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = app_with_probe(true);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/nope")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
