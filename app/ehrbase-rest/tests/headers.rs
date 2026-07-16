//! `ETag` expectations use the weak form (`W/"…"`): the ITS-REST overview
//! §"`ETag` and Last-Modified" makes the `ETag` weak-type ("should have a
//! weakness indicator `W/` prefix"); the bare quoted form is deprecated.
//! End-to-end HTTP tests for the W2-A response-header + `Prefer` handling:
//! `ETag`/`Location` on the EHR / `EHR_STATUS` / COMPOSITION writes and reads, and
//! the `return=minimal` (default, header-only) vs `return=representation`
//! (full body) `Prefer` policy — driven through the assembled router over a
//! **real** `EhrbaseService` on a real `PostgreSQL`.
//!
//! The former `Mock` backend returned fixed resource ids; the real service
//! assigns each version its own `OBJECT_VERSION_ID`, so the tests create the
//! resources through the wire and read the server-assigned ids back from the
//! `ETag`/body. The wire assertions (weak `ETag`, `Location` presence/absence,
//! `Prefer` body policy, status codes) are unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase_rest::config::AppConfig;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";

// The IPS OPT + its canonical composition are the pair driven end-to-end
// through the real `EhrbaseService` (upload → create-EHR → commit) in
// `app/ehrbase/tests/service_validation.rs`, so they commit cleanly here too.
fn opt_xml() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/openehr-flat/tests/fixtures/sdk/ips.v0.opt"),
    )
    .expect("ips.v0.opt vendored in openehr-flat")
}

fn canonical_composition() -> Value {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/ips_canonical.json",
        ),
    )
    .expect("ips_canonical.json vendored in openehr-its");
    serde_json::from_str(&text).expect("valid canonical composition")
}

fn config() -> AppConfig {
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
            admin_scope: None,
            ..AuthConfig::default()
        },
        ..Default::default()
    }
}

async fn app(db: &str) -> (common::Pg, Router) {
    let (pg, service) = common::test_service(db).await;
    (pg, common::router_with(config(), service))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
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

fn etag(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::ETAG).and_then(|v| v.to_str().ok())
}

fn location(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::LOCATION).and_then(|v| v.to_str().ok())
}

/// The bare uid inside a weak `ETag` (`W/"{uid}"`).
fn etag_uid(h: &header::HeaderMap) -> String {
    etag(h)
        .expect("ETag present")
        .trim_start_matches("W/")
        .trim_matches('"')
        .to_owned()
}

fn vo_of(ovid: &str) -> &str {
    ovid.split("::").next().expect("vo uuid")
}

// ── wire setup helpers ───────────────────────────────────────────────────────

/// Create an EHR through the wire; return its id (from the create `ETag`).
async fn create_ehr(app: &Router) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    etag_uid(&h)
}

/// Upload the Demo Vitals OPT (canonical XML) through the wire.
async fn upload_opt(app: &Router) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/definition/template/adl1.4"))
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(opt_xml()))
        .unwrap();
    let (status, _h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED, "OPT upload: {body}");
}

/// Commit the IPS composition into `ehr_id`; return its `version_uid`.
async fn commit_composition(app: &Router, ehr_id: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(canonical_composition().to_string()))
        .unwrap();
    let (status, h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED, "composition commit: {body}");
    etag_uid(&h)
}

/// The current `EHR_STATUS` body + its `version_uid`.
async fn current_ehr_status(app: &Router, ehr_id: &str) -> (Value, String) {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK, "ehr_status read: {body}");
    let v: Value = serde_json::from_str(&body).expect("json ehr_status");
    let ovid = v["uid"]["value"].as_str().expect("status uid").to_owned();
    (v, ovid)
}

// ── EHR create ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn ehr_create_default_is_minimal_with_headers() {
    let (_pg, app) = app("hdr_ehr_create_min").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 201_EHR default (return=minimal): headers only, no body.
    assert_eq!(status, StatusCode::CREATED);
    let ehr_id = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{ehr_id}\"").as_str()));
    assert_eq!(location(&h), Some(format!("{BASE}/ehr/{ehr_id}").as_str()));
    assert!(body.is_empty(), "minimal create has no body, got {body:?}");
}

#[tokio::test]
async fn ehr_create_representation_returns_body() {
    let (_pg, app) = app("hdr_ehr_create_repr").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header("Prefer", "return=representation")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::CREATED);
    let ehr_id = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{ehr_id}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR");
}

// ── EHR_STATUS update ────────────────────────────────────────────────────────

#[tokio::test]
async fn ehr_status_update_default_is_204_with_headers() {
    let (_pg, app) = app("hdr_status_update_min").await;
    let ehr_id = create_ehr(&app).await;
    let (mut status_body, current) = current_ehr_status(&app, &ehr_id).await;
    // Re-commit the current status (a new version) — strip the uid the server owns.
    status_body.as_object_mut().unwrap().remove("uid");

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{current}\""))
        .body(Body::from(status_body.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 204_EHR_STATUS (default minimal): no body, ETag + Location of the new version.
    assert_eq!(status, StatusCode::NO_CONTENT);
    let new_ovid = etag_uid(&h);
    assert_ne!(new_ovid, current, "a new version was created");
    assert_eq!(etag(&h), Some(format!("W/\"{new_ovid}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{ehr_id}/ehr_status/{new_ovid}").as_str())
    );
    assert!(body.is_empty());
}

#[tokio::test]
async fn ehr_status_update_representation_is_200_with_body() {
    let (_pg, app) = app("hdr_status_update_repr").await;
    let ehr_id = create_ehr(&app).await;
    let (mut status_body, current) = current_ehr_status(&app, &ehr_id).await;
    status_body.as_object_mut().unwrap().remove("uid");

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{current}\""))
        .header("Prefer", "return=representation")
        .body(Body::from(status_body.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 200_EHR_STATUS_updated (representation): body present.
    assert_eq!(status, StatusCode::OK);
    let new_ovid = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{new_ovid}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR_STATUS");
}

// ── COMPOSITION read/delete ──────────────────────────────────────────────────

#[tokio::test]
async fn composition_get_sets_etag_and_location() {
    let (_pg, app) = app("hdr_comp_get").await;
    let ehr_id = create_ehr(&app).await;
    upload_opt(&app).await;
    let ovid = commit_composition(&app, &ehr_id).await;
    let vo = vo_of(&ovid);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 200_COMPOSITION_retrieved: ETag(version_uid) + Location deprecated (absent).
    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("W/\"{ovid}\"").as_str()));
    assert_eq!(
        location(&h),
        None,
        "200_COMPOSITION_retrieved marks Location Location_deprecated — not emitted"
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "COMPOSITION");
}

#[tokio::test]
async fn versioned_ehr_status_version_at_time_sets_version_headers() {
    // 200_VERSION_of_EHR_STATUS_at_time declares ETag_VERSION (the version_uid)
    // plus a `Location_deprecated` header — the deprecated Location is no longer
    // emitted (overview §"Deprecated headers").
    let (_pg, app) = app("hdr_versioned_status").await;
    let ehr_id = create_ehr(&app).await;
    let (_body, current) = current_ehr_status(&app, &ehr_id).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_ehr_status/version"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("W/\"{current}\"").as_str()));
    assert_eq!(
        location(&h),
        None,
        "the OAS marks Location on this response Location_deprecated — not emitted"
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "ORIGINAL_VERSION");
}

#[tokio::test]
async fn composition_delete_is_204_with_headers() {
    let (_pg, app) = app("hdr_comp_delete").await;
    let ehr_id = create_ehr(&app).await;
    upload_opt(&app).await;
    let ovid = commit_composition(&app, &ehr_id).await;

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{ovid}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 204_version_deleted: ETag of the deleted version; its Location is
    // `Location_deprecated` in the OAS and is no longer emitted.
    assert_eq!(status, StatusCode::NO_CONTENT);
    let deleted = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{deleted}\"").as_str()));
    assert_eq!(
        location(&h),
        None,
        "the OAS marks Location on this response Location_deprecated — not emitted"
    );
    assert!(body.is_empty());
}
