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
