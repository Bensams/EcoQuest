//! Integration tests for administrator routes.

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use ecoquest_application::auth::PasswordHasherService;
use ecoquest_domain::Role;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use support::{
    sample_admin_user, sample_event, sample_organization, test_admin_app, test_admin_app_with_base,
};
use tower::ServiceExt;
use uuid::Uuid;

struct Call {
    status: StatusCode,
    body: Value,
    cookies: Vec<String>,
}

impl Call {
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
}

async fn call(app: &axum::Router, req: Request<Body>) -> Call {
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
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    Call {
        status,
        body,
        cookies,
    }
}

fn get_with_cookie(path: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("request")
}

fn json_with_cookie(method: &str, path: &str, cookie: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

async fn login(app: &axum::Router, email: &str, password: &str) -> String {
    let res = call(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "email": email, "password": password }).to_string(),
            ))
            .expect("request"),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    format!(
        "eq_access={}",
        res.cookie("eq_access").expect("access cookie")
    )
}

async fn register_player(app: &axum::Router) -> String {
    let res = call(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "username": "eco_player",
                    "email": "player@example.com",
                    "password": "correct-horse-battery-1"
                })
                .to_string(),
            ))
            .expect("request"),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    format!(
        "eq_access={}",
        res.cookie("eq_access").expect("access cookie")
    )
}

fn seed_admin(store: &support::MemoryStore) {
    let hash = PasswordHasherService::for_tests()
        .hash("correct-horse-battery-1")
        .expect("hash");
    store.insert_user("eco_admin", "admin@ecoquest.test", &hash, Role::Admin);
}

#[tokio::test]
async fn player_receives_403_on_every_admin_route() {
    let (app, _, _) = test_admin_app();
    let cookie = register_player(&app).await;
    let org = Uuid::new_v4();
    let user = Uuid::new_v4();
    let event = Uuid::new_v4();
    let paths = [
        ("GET", "/api/admin/organizations", None),
        ("GET", "/api/admin/users", None),
        ("GET", "/api/admin/events", None),
        (
            "PATCH",
            "/api/admin/organizations/{org}/status",
            Some(json!({"status":"APPROVED","reason":"ok"})),
        ),
        (
            "PATCH",
            "/api/admin/users/{user}/status",
            Some(json!({"status":"SUSPENDED","reason":"abuse"})),
        ),
        (
            "POST",
            "/api/admin/events/{event}/cancel",
            Some(json!({"reason":"unsafe"})),
        ),
    ];
    for (method, template, body) in paths {
        let path = template
            .replace("{org}", &org.to_string())
            .replace("{user}", &user.to_string())
            .replace("{event}", &event.to_string());
        let req = if let Some(payload) = body {
            json_with_cookie(method, &path, &cookie, &payload)
        } else {
            get_with_cookie(&path, &cookie)
        };
        let res = call(&app, req).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(res.body["error"]["code"], "forbidden");
    }
}

#[tokio::test]
async fn anonymous_caller_receives_401_on_admin_routes() {
    let (app, _, _) = test_admin_app();
    let res = call(
        &app,
        Request::builder()
            .uri("/api/admin/organizations")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn administrator_can_list_and_review_an_organization() {
    let (app, store, admin) = test_admin_app();
    seed_admin(&store);
    let org_id = Uuid::new_v4();
    admin.seed_org(sample_organization(org_id));
    let cookie = login(&app, "admin@ecoquest.test", "correct-horse-battery-1").await;

    let listed = call(&app, get_with_cookie("/api/admin/organizations", &cookie)).await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.body["total"], 1);
    assert_eq!(listed.body["items"][0]["name"], "Alex Greenworks");

    let reviewed = call(
        &app,
        json_with_cookie(
            "PATCH",
            &format!("/api/admin/organizations/{org_id}/status"),
            &cookie,
            &json!({"status":"APPROVED","reason":"documents verified"}),
        ),
    )
    .await;
    assert_eq!(reviewed.status, StatusCode::OK, "{}", reviewed.body);
    assert_eq!(reviewed.body["verification_status"], "APPROVED");
    assert!(admin
        .audit_actions()
        .iter()
        .any(|action| action == "organization.approved"));
}

#[tokio::test]
async fn administrator_can_moderate_events_and_users() {
    let (app, store, admin) = test_admin_app();
    seed_admin(&store);
    let event_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    admin.seed_event(sample_event(event_id));
    admin.seed_user(sample_admin_user(user_id, "erwin", Role::User));
    let cookie = login(&app, "admin@ecoquest.test", "correct-horse-battery-1").await;

    let cancelled = call(
        &app,
        json_with_cookie(
            "POST",
            &format!("/api/admin/events/{event_id}/cancel"),
            &cookie,
            &json!({"reason":"unsafe shoreline"}),
        ),
    )
    .await;
    assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
    assert_eq!(cancelled.body["status"], "CANCELLED");

    let missing_reason = call(
        &app,
        json_with_cookie(
            "PATCH",
            &format!("/api/admin/users/{user_id}/status"),
            &cookie,
            &json!({"status":"SUSPENDED","reason":"   "}),
        ),
    )
    .await;
    assert_eq!(missing_reason.status, StatusCode::BAD_REQUEST);

    let suspended = call(
        &app,
        json_with_cookie(
            "PATCH",
            &format!("/api/admin/users/{user_id}/status"),
            &cookie,
            &json!({"status":"SUSPENDED","reason":"repeated no-shows"}),
        ),
    )
    .await;
    assert_eq!(suspended.status, StatusCode::OK, "{}", suspended.body);
    assert_eq!(suspended.body["status"], "SUSPENDED");

    let reset = call(
        &app,
        json_with_cookie(
            "POST",
            &format!("/api/admin/users/{user_id}/password-reset"),
            &cookie,
            &json!({}),
        ),
    )
    .await;
    assert_eq!(reset.status, StatusCode::CONFLICT);

    let points = call(
        &app,
        json_with_cookie(
            "POST",
            &format!("/api/admin/users/{user_id}/points"),
            &cookie,
            &json!({"amount":25,"reason":"manual adjustment after appeal"}),
        ),
    )
    .await;
    assert_eq!(points.status, StatusCode::OK, "{}", points.body);
    assert_eq!(points.body["eco_points"], 25);
}

#[tokio::test]
async fn password_reset_link_uses_the_configured_web_origin() {
    let (app, store, admin) = test_admin_app_with_base("https://app.ecoquest.example/");
    seed_admin(&store);
    let user_id = Uuid::new_v4();
    admin.seed_user(sample_admin_user(user_id, "erwin", Role::User));
    let cookie = login(&app, "admin@ecoquest.test", "correct-horse-battery-1").await;

    let reset = call(
        &app,
        json_with_cookie(
            "POST",
            &format!("/api/admin/users/{user_id}/password-reset"),
            &cookie,
            &json!({}),
        ),
    )
    .await;

    assert_eq!(reset.status, StatusCode::OK, "{}", reset.body);
    let url = reset.body["reset_url"].as_str().expect("reset_url");
    assert!(
        url.starts_with("https://app.ecoquest.example/reset-password?token="),
        "link must point at the deployed client, got {url}"
    );
    // The trailing slash on the configured origin must not produce a double slash.
    assert!(!url.contains("example//"), "got {url}");
    assert!(admin
        .audit_actions()
        .iter()
        .any(|action| action == "user.password_reset_issued"));
}
