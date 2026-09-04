// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! End-to-end HTTP tests for the tenant admin extension API group:
//! the config gate (`AppConfig::tenancy.enabled`), the `200`/`201`/`204`/`404`/
//! `400`/`409` wire outcomes for the CRUD verbs, and the JSON body shapes —
//! driven through the assembled router over the **real** `FerroEhrService` on a
//! real Postgres database (the scripted `Mock` is gone; the tenant
//! CRUD persists to the real `tenant` table).
//!
//! Design: the tenancy model is spec-silent — our own extension; the surface is
//! our own, config-gated like the event-subscription group, mounted under
//! `/admin/` and dispatching to the `TenantAdapter` extension.
//!
//! Note: `migrations/ehr/0004_multitenancy.sql` seeds one reserved `default`
//! tenant (the nil UUID), so a freshly-migrated DB already lists one tenant.
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use ferroehr::config::auth::AuthConfig;
use ferroehr::config::server::{ServerConfig, TenancyConfig};
use ferroehr_rest::config::AppConfig;

use crate::common;

const BASE: &str = "/ferroehr/rest/openehr/v1";
const GROUP: &str = "/ferroehr/rest/openehr/v1/admin/tenant";

fn config(enabled: bool) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            base_path: BASE.to_owned(),
            swagger_ui: ferroehr::config::management::AccessLevel::Off,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            ..AuthConfig::default()
        },
        tenancy: TenancyConfig {
            enabled,
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn app(enabled: bool) -> (testkit::TestDb, Router) {
    let (pg, service) = common::test_service().await;
    (
        pg,
        ferroehr_rest::build_with(config(enabled), service).expect("router builds"),
    )
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder().method(method).uri(uri);
    match body {
        Some(v) => builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn disabled_group_is_404() {
    let (_pg, a) = app(false).await;
    let (status, _) = send(a, req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The shared config with the dev-only header override armed, so a test can
/// scope a request to a tenant without a JWT (`TenancyConfig::header` — the
/// header wins over the claim by design).
fn config_with_header(enabled: bool) -> AppConfig {
    let mut c = config(enabled);
    c.tenancy.header = Some("X-Tenant".to_owned());
    c
}

#[tokio::test]
async fn current_reports_the_default_when_unscoped() {
    // GET /admin/tenant/current with no tenant key: the request runs unscoped
    // on the reserved default tenant, and the read says so rather than
    // fabricating a record.
    let (_pg, app) = app(true).await;
    let (status, body) = send(app, req("GET", &format!("{GROUP}/current"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["default"], json!(true));
    assert!(
        body["tenant"].is_null(),
        "unscoped carries no record: {body}"
    );
}

#[tokio::test]
async fn current_reports_the_resolved_tenant_when_scoped() {
    let (_pg, service) = common::test_service().await;
    let app = ferroehr_rest::build_with(config_with_header(true), service).expect("router builds");

    let create_body = json!({ "name": "acme-current", "system_id": "acme.example.org" });
    let (status, created) = send(app.clone(), req("POST", GROUP, Some(create_body))).await;
    assert_eq!(status, StatusCode::CREATED, "create: {created}");

    // Scoped by the dev header: the read reports the resolved registry record.
    let scoped = Request::builder()
        .method("GET")
        .uri(format!("{GROUP}/current"))
        .header("X-Tenant", "acme-current")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(app.clone(), scoped).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["default"], json!(false));
    assert_eq!(body["tenant"]["name"], "acme-current");
    assert_eq!(body["tenant"]["id"], created["id"]);

    // An UNKNOWN key runs unscoped -> the default answer (the documented
    // unknown-key policy: engine-level default scope, never a 403).
    let unknown = Request::builder()
        .method("GET")
        .uri(format!("{GROUP}/current"))
        .header("X-Tenant", "no-such-tenant")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(app, unknown).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["default"], json!(true));
}

#[tokio::test]
async fn crud_round_trip() {
    let (_pg, app) = app(true).await;

    // A freshly-migrated DB seeds the reserved `default` tenant.
    let (status, body) = send(app.clone(), req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::OK);
    let initial = body.as_array().unwrap().len();
    assert!(initial >= 1, "the seeded default tenant is present");

    // Create — 201 with the stored record + a generated id.
    let create_body = json!({ "name": "acme", "system_id": "acme.example.org" });
    let (status, created) = send(app.clone(), req("POST", GROUP, Some(create_body))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "acme");
    assert_eq!(created["system_id"], "acme.example.org");
    let id = created["id"].as_str().unwrap().to_owned();

    // Get by id — 200.
    let (status, got) = send(app.clone(), req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["id"], id);

    // Update — 200, system_id changed.
    let update_body = json!({ "name": "acme", "system_id": "acme-2.example.org" });
    let (status, updated) = send(
        app.clone(),
        req("PUT", &format!("{GROUP}/{id}"), Some(update_body)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["system_id"], "acme-2.example.org");

    // List now has one more than the initial (seeded) set.
    let (_status, list) = send(app.clone(), req("GET", GROUP, None)).await;
    assert_eq!(list.as_array().unwrap().len(), initial + 1);

    // Delete — 204.
    let (status, _) = send(app.clone(), req("DELETE", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Gone — 404.
    let (status, _) = send(app, req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_without_required_fields_is_400() {
    // Missing system_id.
    let (_pg, a) = app(true).await;
    let (status, _) = send(a, req("POST", GROUP, Some(json!({ "name": "x" })))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Missing name.
    let (_pg, a) = app(true).await;
    let (status, _) = send(a, req("POST", GROUP, Some(json!({ "system_id": "s" })))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_name_is_409() {
    let (_pg, app) = app(true).await;
    let body = json!({ "name": "dup", "system_id": "s" });
    let (status, _) = send(app.clone(), req("POST", GROUP, Some(body.clone()))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = send(app, req("POST", GROUP, Some(body))).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn unknown_id_is_404() {
    let (_pg, app) = app(true).await;
    let id = Uuid::now_v7();
    let (status, _) = send(app.clone(), req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(app, req("DELETE", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_id_is_400() {
    let (_pg, a) = app(true).await;
    let (status, _) = send(a, req("GET", &format!("{GROUP}/not-a-uuid"), None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn enabled_group_lists_real_store() {
    // Re-targeted from the old `unhooked → 501` Mock-scaffolding case: with the
    // concrete service the tenant CRUD persists to the real store, so an enabled
    // group answers 200 (never the trait-default 501).
    let (_pg, a) = app(true).await;
    let (status, body) = send(a, req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}
