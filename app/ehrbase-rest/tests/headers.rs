#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! `ETag` expectations use the weak form (`W/"…"`): the ITS-REST overview
//! §"`ETag` and Last-Modified" makes the `ETag` weak-type ("should have a
//! weakness indicator `W/` prefix"); the bare quoted form is deprecated.
//! End-to-end HTTP tests for the response-header + `Prefer` handling:
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
            .join("../../crates/openehr-its/tests/fixtures/sdk/ips.v0.opt"),
    )
    .expect("ips.v0.opt vendored in openehr-its")
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

async fn app() -> (testkit::TestDb, Router) {
    let (pg, service) = common::test_service().await;
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

fn last_modified(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::LAST_MODIFIED).and_then(|v| v.to_str().ok())
}

/// An openEHR `DV_DATE_TIME` value rendered as an HTTP-date (IMF-fixdate,
/// RFC 9110 §5.6.7) — the form `Last-Modified` carries, and the exact value
/// ITS-REST overview §"`ETag` and Last-Modified" derives from
/// `VERSION.commit_audit.time_committed.value`.
fn imf_fixdate(iso_instant: &str) -> String {
    iso_instant
        .parse::<jiff::Timestamp>()
        .expect("commit_audit.time_committed.value is an ISO 8601 instant")
        .strftime("%a, %d %b %Y %H:%M:%S GMT")
        .to_string()
}

/// `VERSION.commit_audit.time_committed.value` of a served `ORIGINAL_VERSION`.
fn commit_instant(version: &Value) -> &str {
    version["commit_audit"]["time_committed"]["value"]
        .as_str()
        .expect("ORIGINAL_VERSION.commit_audit.time_committed.value")
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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
    let (_pg, app) = app().await;
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

// ── Last-Modified (overview §"ETag and Last-Modified") ───────────────────────
//
// "The `Last-Modified` response HTTP header, indicates the date and time when
// the resource was last modified. … For openEHR resources, this value should
// be derived from `VERSION.commit_audit.time_committed.value`." and "Both
// `ETag` and `Last-Modified` SHOULD be included in responses for VERSION,
// VERSIONED_OBJECT, or other resources that have versioning or unique state
// identifiers."

/// A VERSION read (`…/versioned_ehr_status/version/{version_uid}` and the
/// at-time sibling) carries `Last-Modified` derived from the served
/// envelope's own `commit_audit.time_committed` — not merely "present", but
/// byte-equal to the IMF-fixdate rendering of that instant.
#[tokio::test]
async fn version_read_last_modified_is_the_commit_audit_instant() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;

    // …/versioned_ehr_status/version — the ORIGINAL_VERSION at the latest time.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_ehr_status/version"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert_eq!(v["_type"], "ORIGINAL_VERSION");
    assert_eq!(
        last_modified(&h),
        Some(imf_fixdate(commit_instant(&v)).as_str()),
        "overview §\"ETag and Last-Modified\": the value \"should be derived from \
         VERSION.commit_audit.time_committed.value\""
    );

    // …/versioned_ehr_status/version/{version_uid} — the by-id VERSION read.
    let version_uid = v["uid"]["value"].as_str().expect("VERSION.uid").to_owned();
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert_eq!(
        etag(&h),
        Some(format!("W/\"{version_uid}\"").as_str()),
        "overview §\"ETag and Last-Modified\": both headers SHOULD accompany a VERSION"
    );
    assert_eq!(
        last_modified(&h),
        Some(imf_fixdate(commit_instant(&v)).as_str()),
        "overview §\"ETag and Last-Modified\": derived from \
         VERSION.commit_audit.time_committed.value"
    );
}

/// The `VERSIONED_OBJECT` container read (`…/versioned_ehr_status`) and its
/// `REVISION_HISTORY` sibling both carry the container-uid weak `ETag` AND a
/// `Last-Modified` equal to the newest held version's commit instant —
/// overview §"`ETag` and Last-Modified": "Both `ETag` and `Last-Modified`
/// SHOULD be included in responses for VERSION, `VERSIONED_OBJECT`, or other
/// resources that have versioning or unique state identifiers", the value
/// "derived from `VERSION.commit_audit.time_committed.value`".
#[tokio::test]
async fn container_reads_carry_etag_and_newest_commit_last_modified() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;

    // A second EHR_STATUS version, so "newest" is distinguishable from v1.
    let (status_body, ovid) = current_ehr_status(&app, &ehr_id).await;
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .header("content-type", "application/json")
        .header("if-match", format!("\"{ovid}\""))
        .body(Body::from(status_body.to_string()))
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The newest version's commit instant, read off the served envelope.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_ehr_status/version"))
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let latest: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert!(
        latest["uid"]["value"]
            .as_str()
            .is_some_and(|v| v.ends_with("::2")),
        "the update committed version 2"
    );
    let newest_instant = imf_fixdate(commit_instant(&latest));
    let container_uid = vo_of(latest["uid"]["value"].as_str().expect("uid")).to_owned();

    // The VERSIONED_EHR_STATUS container read.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_ehr_status"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json VERSIONED_EHR_STATUS");
    assert_eq!(v["_type"], "VERSIONED_EHR_STATUS");
    assert_eq!(
        etag(&h),
        Some(format!("W/\"{container_uid}\"").as_str()),
        "overview §\"ETag and Last-Modified\": \"VERSIONED_OBJECT.uid.value\" is an ETag source"
    );
    assert_eq!(
        last_modified(&h),
        Some(newest_instant.as_str()),
        "overview §\"ETag and Last-Modified\": a VERSIONED_OBJECT response SHOULD carry \
         Last-Modified, derived from the newest version's commit_audit.time_committed"
    );

    // The REVISION_HISTORY read — same container identity, same newest instant.
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_ehr_status/revision_history"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let rh: Value = serde_json::from_str(&body).expect("json REVISION_HISTORY");
    assert_eq!(rh["_type"], "REVISION_HISTORY");
    assert_eq!(etag(&h), Some(format!("W/\"{container_uid}\"").as_str()));
    assert_eq!(last_modified(&h), Some(newest_instant.as_str()));
}

/// The COMPOSITION VERSION read (`…/versioned_composition/{vo}/version/{uid}`)
/// and the bare COMPOSITION reads/writes all carry the same `Last-Modified`:
/// the commit instant of the version served. The bare COMPOSITION body has no
/// `commit_audit`, so this pins that the instant survives from the version row
/// (read) / the commit result (write) to the wire.
#[tokio::test]
async fn composition_read_and_write_carry_the_commit_instant() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;
    upload_opt(&app).await;

    // POST /composition — the create 201.
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(canonical_composition().to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED, "composition commit: {body}");
    let ovid = etag_uid(&h);
    let created_lm = last_modified(&h)
        .expect(
            "overview §\"ETag and Last-Modified\": Last-Modified SHOULD accompany the \
             committed VERSION",
        )
        .to_owned();

    // The VERSION envelope names the authoritative instant.
    let vo = vo_of(&ovid).to_owned();
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_composition/{vo}/version/{ovid}"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let version: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    let committed = imf_fixdate(commit_instant(&version));
    assert_eq!(
        last_modified(&h),
        Some(committed.as_str()),
        "overview §\"ETag and Last-Modified\": derived from \
         VERSION.commit_audit.time_committed.value"
    );
    assert_eq!(
        created_lm, committed,
        "the create 201 reports the same commit instant the VERSION envelope carries"
    );

    // GET the bare COMPOSITION (latest) — same instant, though the served body
    // carries no commit_audit of its own.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        last_modified(&h),
        Some(committed.as_str()),
        "overview §\"ETag and Last-Modified\": both headers SHOULD accompany a resource \
         with a unique state identifier — the bare COMPOSITION read included"
    );

    // GET the bare COMPOSITION addressed by its version uid — same instant.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{ovid}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last_modified(&h), Some(committed.as_str()));

    // PUT /composition — the update 200/204 carries the NEW version's instant.
    let mut updated = canonical_composition();
    updated.as_object_mut().unwrap().remove("uid");
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{ovid}\""))
        .body(Body::from(updated.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "composition update: {body}");
    let new_ovid = etag_uid(&h);
    assert_ne!(new_ovid, ovid, "a new version was created");
    let updated_lm = last_modified(&h)
        .expect("overview §\"ETag and Last-Modified\": Last-Modified on the update response")
        .to_owned();

    // The new VERSION envelope confirms the reported instant.
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_composition/{vo}/version/{new_ovid}"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let version: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert_eq!(updated_lm, imf_fixdate(commit_instant(&version)));
}

/// The `EHR_STATUS` resource reads and its `PUT` carry `Last-Modified` too —
/// the bare `EHR_STATUS` body has no `commit_audit`, so the instant comes from
/// the version row (read) and the commit result (write).
#[tokio::test]
async fn ehr_status_read_and_write_carry_the_commit_instant() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;

    // GET /ehr_status.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let current: Value = serde_json::from_str(&body).expect("json EHR_STATUS");
    let ovid = current["uid"]["value"]
        .as_str()
        .expect("EHR_STATUS.uid")
        .to_owned();
    let read_lm = last_modified(&h)
        .expect(
            "overview §\"ETag and Last-Modified\": both headers SHOULD accompany a resource \
             with a unique state identifier",
        )
        .to_owned();

    // GET /ehr_status/{version_uid} — the same version, the same instant.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status/{ovid}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last_modified(&h), Some(read_lm.as_str()));

    // PUT /ehr_status — the new version's commit instant, cross-checked
    // against the VERSION envelope's commit_audit.
    let mut next = current;
    next.as_object_mut().unwrap().remove("uid");
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{ovid}\""))
        .body(Body::from(next.to_string()))
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let new_ovid = etag_uid(&h);
    let write_lm = last_modified(&h)
        .expect("overview §\"ETag and Last-Modified\": Last-Modified on the EHR_STATUS write")
        .to_owned();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_ehr_status/version/{new_ovid}"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let version: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert_eq!(write_lm, imf_fixdate(commit_instant(&version)));
}

/// The EHR create already carried the weak `ETag`; the EHR **reads** now do
/// too. Overview §"`ETag` and Last-Modified": the value "is usually taken from
/// e.g. `VERSIONED_OBJECT.uid.value`, `VERSION.uid.value`, `EHR.ehr_id.value`",
/// and both headers SHOULD accompany "resources that have versioning or unique
/// state identifiers". No `Location` on a GET (§Location).
#[tokio::test]
async fn ehr_get_carries_the_weak_ehr_id_etag() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json EHR");
    assert_eq!(v["_type"], "EHR");
    assert_eq!(
        v["ehr_id"]["value"].as_str(),
        Some(ehr_id.as_str()),
        "the served EHR names the addressed ehr_id"
    );
    assert_eq!(
        etag(&h),
        Some(format!("W/\"{ehr_id}\"").as_str()),
        "overview §\"ETag and Last-Modified\": EHR.ehr_id.value is the named ETag source"
    );
    assert_eq!(
        location(&h),
        None,
        "overview §Location: Location MUST NOT indicate an alternate representation \
         of an existing resource (e.g. via GET)"
    );
}

/// The EHR create `201` carries `Last-Modified` (the creating CONTRIBUTION's
/// commit instant) alongside the `ETag`/`Location` it already emitted.
#[tokio::test]
async fn ehr_create_carries_last_modified() {
    let (_pg, app) = app().await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    let lm = last_modified(&h).expect(
        "overview §\"ETag and Last-Modified\": both headers SHOULD accompany a resource \
         with a unique state identifier",
    );
    assert!(
        lm.ends_with("GMT"),
        "Last-Modified is an HTTP-date (RFC 9110 §5.6.7): {lm:?}"
    );

    // The EHR's own EHR_STATUS VERSION was committed in the same CONTRIBUTION,
    // so its commit_audit instant is the one reported.
    let ehr_id = etag_uid(&h);
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_ehr_status/version"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let version: Value = serde_json::from_str(&body).expect("json ORIGINAL_VERSION");
    assert_eq!(lm, imf_fixdate(commit_instant(&version)));
}

// ── openehr-item-tag / openehr-version-item-tag (overview §"openehr-item-tag ──
//    and openehr-version-item-tag") ────────────────────────────────────────────

/// The spec sentence these assertions encode: "`openehr-item-tag` applies to
/// *`VERSIONED_OBJECT`* targets" while "`openehr-version-item-tag` applies to a
/// specific target *VERSION* within a `VERSIONED_OBJECT`" — two DISTINCT targets,
/// each confirmed by "the actual list of `ITEM_TAGs` stored" for that target
/// (§Usage in Responses).
const DISTINCT_TARGETS: &str = "overview §\"openehr-item-tag and openehr-version-item-tag\": \
     openehr-item-tag applies to VERSIONED_OBJECT targets, openehr-version-item-tag to a \
     specific VERSION — each response header confirms only its own target's stored list";

fn raw(h: &header::HeaderMap, name: &str) -> Option<String> {
    h.get(name).and_then(|v| v.to_str().ok()).map(str::to_owned)
}

/// The `key="…"` values of an item-tag wrapper header value, in order.
fn tag_keys(value: &str) -> Vec<String> {
    value
        .split(';')
        .filter_map(|entry| {
            entry
                .split(',')
                .map(str::trim)
                .find_map(|pair| pair.strip_prefix("key=").map(|k| k.trim_matches('"')))
        })
        .map(str::to_owned)
        .collect()
}

/// The stored `ITEM_TAG` keys of a COMPOSITION tag target, read back through
/// the dedicated `composition_tags_get` operation.
async fn stored_tag_keys(app: &Router, ehr_id: &str, uid_based_id: &str) -> Vec<String> {
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/composition/{uid_based_id}/tags"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK, "tags read: {body}");
    let tags: Value = serde_json::from_str(&body).expect("json ITEM_TAG list");
    let mut keys: Vec<String> = tags
        .as_array()
        .expect("ITEM_TAG list")
        .iter()
        .map(|t| t["key"].as_str().expect("ITEM_TAG.key").to_owned())
        .collect();
    keys.sort();
    keys
}

/// Both wrapper headers on one write address different targets, so each
/// response header echoes ONLY its own target's stored collection — never the
/// union of the two.
#[tokio::test]
async fn composition_update_echoes_each_item_tag_target_under_its_own_header() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;
    upload_opt(&app).await;
    let ovid = commit_composition(&app, &ehr_id).await;
    let vo = vo_of(&ovid).to_owned();

    let mut updated = canonical_composition();
    updated.as_object_mut().unwrap().remove("uid");
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{ovid}\""))
        .header("openehr-item-tag", "key=\"category\",value=\"final\"")
        .header(
            "openehr-version-item-tag",
            "key=\"reviewed\",value=\"true\"",
        )
        .body(Body::from(updated.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "composition update: {body}");
    let new_ovid = etag_uid(&h);

    let object_echo = raw(&h, "openehr-item-tag").expect("openehr-item-tag echoed");
    let version_echo =
        raw(&h, "openehr-version-item-tag").expect("openehr-version-item-tag echoed");
    assert_eq!(
        tag_keys(&object_echo),
        vec!["category".to_owned()],
        "{DISTINCT_TARGETS}; got openehr-item-tag: {object_echo:?}"
    );
    assert_eq!(
        tag_keys(&version_echo),
        vec!["reviewed".to_owned()],
        "{DISTINCT_TARGETS}; got openehr-version-item-tag: {version_echo:?}"
    );

    // …and the echo is the truth: the dedicated tag reads show the same split
    // between the VERSIONED_OBJECT and the committed VERSION.
    assert_eq!(
        stored_tag_keys(&app, &ehr_id, &vo).await,
        vec!["category".to_owned()],
        "{DISTINCT_TARGETS}"
    );
    assert_eq!(
        stored_tag_keys(&app, &ehr_id, &new_ovid).await,
        vec!["reviewed".to_owned()],
        "{DISTINCT_TARGETS}"
    );
}

/// A request that carries only one wrapper header writes — and echoes — only
/// that target: "an absent header leaves the other target's tags untouched"
/// follows from the headers addressing distinct targets, so the absent
/// header's collection is not confirmed at all.
#[tokio::test]
async fn composition_update_echoes_nothing_for_an_absent_item_tag_header() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;
    upload_opt(&app).await;
    let ovid = commit_composition(&app, &ehr_id).await;
    let vo = vo_of(&ovid).to_owned();

    let mut updated = canonical_composition();
    updated.as_object_mut().unwrap().remove("uid");
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{ovid}\""))
        .header(
            "openehr-version-item-tag",
            "key=\"reviewed\",value=\"true\"",
        )
        .body(Body::from(updated.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "composition update: {body}");
    let new_ovid = etag_uid(&h);

    assert_eq!(
        raw(&h, "openehr-version-item-tag").as_deref().map(tag_keys),
        Some(vec!["reviewed".to_owned()]),
        "{DISTINCT_TARGETS}"
    );
    assert!(
        raw(&h, "openehr-item-tag").is_none(),
        "{DISTINCT_TARGETS}: the VERSIONED_OBJECT collection was never addressed, so \
         nothing is confirmed for it — got {:?}",
        raw(&h, "openehr-item-tag")
    );
    assert!(
        stored_tag_keys(&app, &ehr_id, &vo).await.is_empty(),
        "{DISTINCT_TARGETS}: the VERSIONED_OBJECT target stays untagged"
    );
    assert_eq!(
        stored_tag_keys(&app, &ehr_id, &new_ovid).await,
        vec!["reviewed".to_owned()],
        "{DISTINCT_TARGETS}"
    );
}

// ── Preference-Applied (overview §Representation details negotiation) ────────

fn preference_applied(h: &header::HeaderMap) -> Option<String> {
    raw(h, "preference-applied")
}

/// "The service MAY include a `Preference-Applied` header in the response …
/// to indicate that the client's preference has been honored"; "if no `Prefer`
/// header is provided, the default behavior is assumed to be `return=minimal`"
/// — every write path declares what it applied, including the applied default.
#[tokio::test]
async fn writes_declare_the_applied_preference() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;

    // Template upload (the definition group's own identifier body).
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/definition/template/adl1.4"))
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(opt_xml()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED, "OPT upload: {body}");
    assert_eq!(
        preference_applied(&h).as_deref(),
        Some("return=minimal"),
        "the template upload declares the applied default"
    );

    // COMPOSITION create with an explicit representation preference.
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(canonical_composition().to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED, "composition commit: {body}");
    assert_eq!(
        preference_applied(&h).as_deref(),
        Some("return=representation")
    );
    let ovid = etag_uid(&h);
    let vo = vo_of(&ovid).to_owned();

    // The ITEM_TAG collection write is not a uid-versioned resource, so it
    // declares minimal / representation only.
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}/tags"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!([{ "key": "category", "value": "final" }]).to_string(),
        ))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "tags update: {body}");
    assert_eq!(preference_applied(&h).as_deref(), Some("return=minimal"));

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}/tags"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(
            serde_json::json!([{ "key": "category", "value": "final" }]).to_string(),
        ))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "tags update: {body}");
    assert_eq!(
        preference_applied(&h).as_deref(),
        Some("return=representation")
    );
}

/// "This is a variant of preference that implies minimal response semantics,
/// but with a non-empty response body (i.e. the status will be `201 Created`
/// or `200 OK`, never `204 No Content`)" — on the `EHR_STATUS` PUT, whose
/// minimal outcome IS `204`.
#[tokio::test]
async fn ehr_status_update_identifier_is_never_204() {
    let (_pg, app) = app().await;
    let ehr_id = create_ehr(&app).await;
    let (mut status_body, current) = current_ehr_status(&app, &ehr_id).await;
    status_body.as_object_mut().unwrap().remove("uid");

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{current}\""))
        .header("Prefer", "return=identifier")
        .body(Body::from(status_body.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "overview §\"Prefer only identifier\": the status will be 201 Created or 200 OK, \
         never 204 No Content — got {status} with body {body:?}"
    );
    assert_eq!(preference_applied(&h).as_deref(), Some("return=identifier"));
    let uid = etag_uid(&h);
    let v: Value = serde_json::from_str(&body).expect("json identifier body");
    assert_eq!(
        v,
        serde_json::json!({ "uid": uid }),
        "overview §\"Prefer only identifier\": a single JSON object with a single uid attribute"
    );
}
