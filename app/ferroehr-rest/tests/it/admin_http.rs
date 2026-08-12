// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! End-to-end HTTP tests for the ADMIN API group (physical EHR delete): the
//! config gate (`AppConfig::admin.enabled`) and the `204`/`404` wire outcomes —
//! driven through the assembled router over a **real** `AdminService` on a real
//! `PostgreSQL`. Per the vendored Admin OAS (`operations/admin_ehr_delete_all.yaml`)
//! the bulk delete declares only bodyless successes (`202`/`204`), and an absent
//! `ehr_id` list means "delete ALL EHRs" (`parameters/query/ehr_id_Admin.yaml`).
//!
//! Spec grounding: SM `I_ADMIN_SERVICE.physical_ehr_delete` and the CNF Robot
//! prior art (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`:
//! `DELETE /admin/ehr/{ehr_id}` → `204`).
//!
//! The former `Mock` backend's call-counter/recorder assertions (proving the
//! backend was/was-not consulted) are re-targeted to real observation: the EHRs
//! are created through the wire and their subsequent existence (`GET /ehr/{id}`
//! → `200` vs `404`) proves whether the delete reached the backend and took
//! effect.
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_assert_message,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use ferroehr::config::auth::AuthConfig;
use ferroehr::config::server::{AdminConfig, ServerConfig};
use ferroehr_rest::config::AppConfig;

use crate::common;

const BASE: &str = "/ferroehr/rest/openehr/v1";
/// A syntactically valid EHR id that is never created — the "unknown" probe.
const OTHER: &str = "11111111-2222-3333-4444-555555555555";

fn config(admin_enabled: bool) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            base_path: BASE.to_owned(),
            max_in_flight: 1024,
            swagger_ui: false,
            cors_permissive: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            ..AuthConfig::default()
        },
        admin: AdminConfig {
            enabled: admin_enabled,
        },
        ..Default::default()
    }
}

async fn app(admin_enabled: bool) -> (testkit::TestDb, Router) {
    let (pg, service) = common::test_service().await;
    (pg, common::router_with(config(admin_enabled), service))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, String) {
    let (status, _headers, body) = send_full(app, req).await;
    (status, body)
}

async fn send_full(app: &Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

/// The `Allow` field value of a `405`, which RFC 9110 §15.5.6 makes mandatory:
/// "The origin server MUST generate an Allow header field in a 405 response
/// containing a list of the target resource's currently supported methods."
/// Fails loudly when the header is absent — an empty VALUE is legal, an absent
/// header is not.
fn allow_of(headers: &header::HeaderMap) -> &str {
    headers
        .get(header::ALLOW)
        .expect("a 405 MUST carry an Allow header (RFC 9110 §15.5.6)")
        .to_str()
        .expect("Allow is ASCII")
}

fn delete(uri: String) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn get(uri: String) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Create a real EHR through the wire; return its server-assigned id.
async fn create_ehr(app: &Router) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.expect("create ehr");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let raw = resp
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .expect("ETag on create");
    raw.trim_start_matches("W/").trim_matches('"').to_owned()
}

/// Whether an EHR still exists (`GET /ehr/{id}` → 200 vs 404).
async fn ehr_exists(app: &Router, id: &str) -> bool {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.expect("get ehr");
    match resp.status() {
        StatusCode::OK => true,
        StatusCode::NOT_FOUND => false,
        other => panic!("unexpected GET /ehr status {other}"),
    }
}

#[tokio::test]
async fn disabled_admin_is_405_and_never_deletes() {
    let (_pg, app) = app(false).await;
    // Create a real EHR, then attempt the (disabled) admin delete.
    let id = create_ehr(&app).await;
    let (status, headers, _) = send_full(&app, delete(format!("{BASE}/admin/ehr/{id}"))).await;
    // The gate answers 405 Method Not Allowed — the status the OAS itself
    // declares for a disabled admin operation
    // (`admin_ehr_delete_all.yaml` + `responses/405.yaml`).
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    // This 405 comes from a MATCHED handler, so axum's allow-header machinery
    // never runs and the handler must supply `Allow` itself. The value is the
    // EMPTY field value RFC 9110 §10.2.1 defines for exactly this situation:
    // "An empty Allow field value indicates that the resource allows no
    // methods, which might occur in a 405 response if the resource has been
    // temporarily disabled by configuration."
    assert_eq!(allow_of(&headers), "");
    // RE-TARGET (was a call-counter assertion): the backend was never reached,
    // proven by the EHR still existing.
    assert!(ehr_exists(&app, &id).await, "the EHR must not be deleted");
}

#[tokio::test]
async fn enabled_delete_is_204() {
    let (_pg, app) = app(true).await;
    let id = create_ehr(&app).await;
    let (status, body) = send(&app, delete(format!("{BASE}/admin/ehr/{id}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    // RE-TARGET (was `calls == 1`): the delete took effect — the EHR is gone.
    assert!(!ehr_exists(&app, &id).await, "the EHR must be deleted");
}

#[tokio::test]
async fn enabled_delete_unknown_maps_to_404() {
    let (_pg, app) = app(true).await;
    // The backend's NotFound (ehr_id_does_not_exist) surfaces as HTTP 404.
    let (status, _) = send(&app, delete(format!("{BASE}/admin/ehr/{OTHER}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn enabled_delete_all_is_204_bodyless() {
    // `admin_ehr_delete_all.yaml:18-26`: the only declared success responses are
    // `202` (async) and `204 No Content` (sync) — both bodyless.
    let (_pg, app) = app(true).await;
    let a = create_ehr(&app).await;
    let b = create_ehr(&app).await;
    let (status, body) = send(&app, delete(format!("{BASE}/admin/ehr/all?ehr_id={a},{b}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    // RE-TARGET (was `calls == 1`): both listed EHRs are gone.
    assert!(!ehr_exists(&app, &a).await, "a deleted");
    assert!(!ehr_exists(&app, &b).await, "b deleted");
}

#[tokio::test]
async fn enabled_delete_all_repeated_param_reaches_backend_with_both_ids() {
    // The repeated `?ehr_id=` form is surfaced from the raw query (the generated
    // Option<String> param would otherwise keep only the first). RE-TARGET (was
    // a recorder of the ids handed to the mock): both EHRs being deleted proves
    // the RFC 6570 `{?ehr_id*}` list handling passed both ids to the backend.
    let (_pg, app) = app(true).await;
    let a = create_ehr(&app).await;
    let b = create_ehr(&app).await;
    let (status, body) = send(
        &app,
        delete(format!("{BASE}/admin/ehr/all?ehr_id={a}&ehr_id={b}")),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    assert!(!ehr_exists(&app, &a).await, "first repeated ehr_id deleted");
    assert!(
        !ehr_exists(&app, &b).await,
        "second repeated ehr_id deleted"
    );
}

#[tokio::test]
async fn disabled_admin_config_is_405() {
    // The config view shares the group gate: with the admin API disabled it
    // answers 405 Method Not Allowed (never a 403) — the status the OAS
    // declares for a disabled admin operation
    // (`admin_ehr_delete_all.yaml` + `responses/405.yaml`), applied
    // uniformly across the group.
    let (_pg, app) = app(false).await;
    let (status, headers, body) = send_full(&app, get(format!("{BASE}/admin/config"))).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    // Mandatory on every 405 (RFC 9110 §15.5.6), empty because the resource
    // currently allows no methods at all (RFC 9110 §10.2.1) — see
    // `disabled_admin_is_405_and_never_deletes`.
    assert_eq!(allow_of(&headers), "");
    // …and the openEHR `{ error, message }` body, as on every other error path.
    let v: serde_json::Value = serde_json::from_str(&body).expect("openEHR error body");
    assert_eq!(v["error"], "Method Not Allowed");
}

#[tokio::test]
async fn enabled_admin_config_is_200_json() {
    // With the admin API enabled (and auth off in this fixture) the config view
    // returns 200 with a JSON body. The redaction of every secret field is
    // proven by the unit test on `FerroEhrConfig::to_redacted_json`; here the
    // point is the route is mounted and served under the enabled gate.
    let (_pg, app) = app(true).await;
    let (status, _body) = send(&app, get(format!("{BASE}/admin/config"))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn enabled_delete_all_missing_list_deletes_all() {
    // `parameters/query/ehr_id_Admin.yaml`: `ehr_id` is "an optional parameter
    // to perform the operation on a subset of EHRs" — an absent list means
    // "delete ALL EHRs" (`admin_ehr_delete_all.yaml:5`), expressed to the
    // backend as the empty list.
    let (_pg, app) = app(true).await;
    let a = create_ehr(&app).await;
    let b = create_ehr(&app).await;
    let (status, body) = send(&app, delete(format!("{BASE}/admin/ehr/all"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    // RE-TARGET (was `calls == 1`): the all-EHRs request removed every EHR.
    assert!(!ehr_exists(&app, &a).await, "all EHRs deleted (a)");
    assert!(!ehr_exists(&app, &b).await, "all EHRs deleted (b)");
}
