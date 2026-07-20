#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end HTTP tests for the MUST-level ITS-REST protocol tail (B6 cluster
//! 4): the `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal request
//! headers (parse + merge), `If-Match` hardening (malformed → 400), the
//! `OPTIONS /` System-Options-and-Conformance endpoint, and canonical-XML
//! responses for the VERSION family. Driven through the assembled
//! router over a **real** `EhrbaseService` on a real `PostgreSQL`.
//!
//! The committal-header merge is now verified end-to-end: the update is
//! committed and the persisted `ORIGINAL_VERSION` is read back to confirm the
//! header-supplied lifecycle/audit values were merged (replacing the former
//! `Mock` hook that captured the `UpdateVersion` in-process). The signed-version
//! XML asserts the real server-side digest signature (the default `EhrbaseService`
//! signer is enabled), not the Mock's injected fixture.
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
/// A syntactically valid EHR/VO id for the malformed-If-Match probes (the
/// precondition is rejected before the backend, so the ids need not exist).
const EHR_ID: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const VO_ID: &str = "8849182c-82ad-4088-a07f-48ead4180515";

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

fn etag_uid(h: &header::HeaderMap) -> String {
    h.get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .expect("ETag present")
        .trim_start_matches("W/")
        .trim_matches('"')
        .to_owned()
}

fn vo_of(ovid: &str) -> &str {
    ovid.split("::").next().expect("vo uuid")
}

/// Create an EHR, upload the IPS OPT, and commit the IPS composition; return the
/// `(ehr_id, version_uid)` of the committed COMPOSITION.
async fn commit_ips_composition(app: &Router) -> (String, String) {
    let (status, h, _b) = send(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/ehr"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ehr_id = etag_uid(&h);

    let (status, _h, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/definition/template/adl1.4"))
            .header(header::CONTENT_TYPE, "application/xml")
            .body(Body::from(opt_xml()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "OPT upload: {body}");

    let (status, h, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(canonical_composition().to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "composition commit: {body}");
    (ehr_id, etag_uid(&h))
}

// ── OPTIONS / (R32) ────────────────────────────────────────────────────────

#[tokio::test]
async fn options_root_is_system_options_and_conformance() {
    let (_pg, app) = app().await;
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK);
    // The `Allow` header lists the supported methods.
    assert_eq!(
        h.get(header::ALLOW).and_then(|v| v.to_str().ok()),
        Some("GET, POST, PUT, DELETE, OPTIONS")
    );
    // The `Options` conformance manifest body.
    let v: Value = serde_json::from_str(&body).expect("options body");
    // The served identity is the released ITS-REST contract version, shared
    // from the single provenance constant.
    assert_eq!(v["restapi_specs_version"], "Release-1.1.0");
    assert_eq!(v["conformance_profile"], "STANDARD");
    assert!(v["endpoints"].as_array().is_some_and(|e| !e.is_empty()));
}

// ── committal headers (openEHR-VERSION.* / openEHR-AUDIT_DETAILS.*) ──────────

#[tokio::test]
async fn committal_headers_merge_into_the_commit() {
    let (_pg, app) = app().await;
    let (ehr_id, v1) = commit_ips_composition(&app).await;
    let vo = vo_of(&v1).to_owned();

    // Update the composition, supplying the committal metadata via the MUST-level
    // request headers; re-post the canonical body (strip the server-owned uid).
    let mut body = canonical_composition();
    body.as_object_mut().unwrap().remove("uid");
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition/{vo}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{v1}\""))
        .header("openEHR-VERSION.lifecycle_state", "code_string=\"523\"")
        .header("openEHR-AUDIT_DETAILS.change_type", "code_string=\"251\"")
        .header(
            "openEHR-AUDIT_DETAILS.description",
            "value=\"An updated composition\"",
        )
        .header(
            "openEHR-AUDIT_DETAILS.committer",
            "name=\"John Doe\", external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\", \
             external_ref.namespace=\"demographic\", external_ref.type=\"PERSON\"",
        )
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, h, resp_body) = send(&app, req).await;
    assert!(
        status.is_success(),
        "update succeeded, got {status}: {resp_body}"
    );
    let v2 = etag_uid(&h);

    // Read the persisted ORIGINAL_VERSION back and confirm the header-supplied
    // committal metadata was merged into the commit.
    let (status, _h, body) = send(
        &app,
        Request::builder()
            .method("GET")
            .uri(format!(
                "{BASE}/ehr/{ehr_id}/versioned_composition/{vo}/version/{v2}"
            ))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "version read: {body}");
    let ver: Value = serde_json::from_str(&body).expect("original_version json");
    assert_eq!(ver["_type"], "ORIGINAL_VERSION");
    // Spec MUST (ITS-REST overview §"openehr-version and openehr-audit-details"):
    // "whatever is provided [in the committal headers] MUST be merged with the
    // default VERSION and VERSION.audit_details attributes on commit runtime."
    // The former `Mock` hook only captured the dispatcher-built `UpdateVersion`
    // (which is correct — see committal.rs unit tests); with the real service the
    // *persisted* ORIGINAL_VERSION must reflect the merged values. These
    // assertions verify the end-to-end MUST.
    assert_eq!(
        ver["lifecycle_state"]["defining_code"]["code_string"], "523",
        "openEHR-VERSION.lifecycle_state merged: {ver}"
    );
    let audit = &ver["commit_audit"];
    assert_eq!(
        audit["change_type"]["defining_code"]["code_string"], "251",
        "openEHR-AUDIT_DETAILS.change_type merged"
    );
    assert_eq!(
        audit["description"]["value"], "An updated composition",
        "openEHR-AUDIT_DETAILS.description merged"
    );
    assert_eq!(audit["committer"]["name"], "John Doe");
    assert_eq!(audit["committer"]["external_ref"]["type"], "PERSON");
}

// ── If-Match hardening ─────────────────────────────────────

#[tokio::test]
async fn malformed_if_match_is_rejected_not_bypassed() {
    // A required If-Match that is not a well-formed OBJECT_VERSION_ID must be a
    // client error (400), never a silent skip of the precondition — rejected
    // before the backend, so the (non-existent) target ids are irrelevant.
    let (_pg, app) = app().await;
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{VO_ID}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"not-an-object-version-id\"")
        .body(Body::from(r#"{"_type":"COMPOSITION"}"#))
        .unwrap();
    let (status, _h, _body) = send(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_if_match_on_ehr_status_update_is_rejected() {
    // The required-If-Match ehr_status update rejects a malformed precondition
    // (400) before the backend, never treating it as no-precondition.
    let (_pg, app) = app().await;
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"garbage\"")
        .body(Body::from(r#"{"_type":"EHR_STATUS"}"#))
        .unwrap();
    let (status, _h, _body) = send(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_if_match_on_directory_update_is_rejected() {
    // The required-If-Match directory update rejects a malformed precondition
    // (400) at the wire, never a silent bypass.
    let (_pg, app) = app().await;
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/directory"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"a::b::c::3\"")
        .body(Body::from(r#"{"_type":"FOLDER","name":{"value":"root"}}"#))
        .unwrap();
    let (status, _h, _body) = send(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ── VERSION-family canonical XML (ECC-COM-022, ECC-SIG-001) ──────────────────

#[tokio::test]
async fn versioned_composition_serves_xml() {
    let (_pg, app) = app().await;
    let (ehr_id, v1) = commit_ips_composition(&app).await;
    let vo = vo_of(&v1);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr_id}/versioned_composition/{vo}"))
        .header(header::ACCEPT, "application/xml")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        h.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("application/xml")
    );
    assert!(body.contains("<versioned_composition"), "root: {body}");
    assert!(body.contains(vo), "uid present: {body}");
}

#[tokio::test]
async fn composition_version_serves_xml_with_signature() {
    // ECC-SIG-001: the ORIGINAL_VERSION XML carries the `<signature>` element.
    // RE-TARGET: the old Mock injected a fake PGP signature; the real default
    // `EhrbaseService` signer is enabled (SHA-256 digest), so the committed
    // version carries a genuine `sha256:` signature which the canonical XML
    // serializes into `<signature>`.
    let (_pg, app) = app().await;
    let (ehr_id, v1) = commit_ips_composition(&app).await;
    let vo = vo_of(&v1);

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_composition/{vo}/version/{v1}"
        ))
        .header(header::ACCEPT, "application/xml")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        h.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("application/xml")
    );
    assert!(body.contains("<original_version"), "root: {body}");
    assert!(body.contains("<signature"), "signature element: {body}");
    assert!(body.contains("sha256:"), "digest signature value: {body}");
}
