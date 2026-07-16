//! End-to-end ATNA audit over the real axum app (binding doc §8.5).
//!
//! Drives the assembled router (auth + audit + dispatch) with `tower`'s
//! `oneshot`, backed by a mock service, and asserts the [`AuditEvent`] the audit
//! middleware emits per audited request — the correct action / outcome / user /
//! object class / object id for EHR create (C), composition get (R) / update
//! (U) / delete (D), AQL execute (E), a 401 (outcome minor-failure, principal
//! UNKNOWN), and a suppressed login event; plus the fail-open drop and
//! fail-closed 503.
//!
//! Crate layout: the audit emitter now lives in the platform's SM `SystemLog` (not
//! router state), so the app carries an in-memory [`AuditSink`] on the mock
//! backend. The DICOM `AuditMessage` rendering + the syslog transport +
//! queue-fail modes are covered by `ehrbase::system_log`'s own tests; here we
//! assert the transport-agnostic `AuditEvent` the HTTP middleware produces.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

use std::sync::Arc;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase_rest::AppConfig;
use ehrbase_rest::access::authn::AuthConfig;
use ehrbase_rest::access::authn::config::{BasicConfig, BasicUser};
use ehrbase_sm::{AuditEvent, EmitOutcome, EventActionCode, EventOutcome, ObjectClass};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

mod common;
use common::{AuditSink, Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
// base64("alice:pw")
const BASIC_ALICE: &str = "Basic YWxpY2U6cHc=";
const CLIENT_IP: &str = "203.0.113.9";
// The EHR dispatcher decodes path ids before consulting the backend, so the
// audited routes use syntactically valid ids: a UUID `ehr_id`, a bare UUID
// COMPOSITION uid for the read/update, and a full OBJECT_VERSION_ID for delete.
const EHR: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
const COMP_VO: &str = "8849182c-82ad-4088-a07f-48ead4180515";
const COMP_OVID: &str = "8849182c-82ad-4088-a07f-48ead4180515::ehrbase-rs.local::1";

// ── mock platform ─────────────────────────────────────────────────────────────

/// A valid canonical COMPOSITION (the vendored Demo Vitals instance), needed
/// because the EHR dispatcher now parses the PUT body before the update call.
fn composition_body() -> serde_json::Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/demo_vitals_352.json",
    );
    let text = std::fs::read_to_string(path).expect("demo_vitals_352.json vendored");
    serde_json::from_str(&text).expect("valid canonical composition")
}

/// The audited-operation hooks. The composition read returns the canned version
/// uid the ATNA participant-object assertion checks; create returns the fixed
/// EHR id; update/delete return their new version uid.
fn hooks() -> Hooks {
    let ehr_uuid: Uuid = EHR.parse().expect("valid ehr uuid");
    Hooks {
        create_ehr: Some(Arc::new(move |_status| Ok(ehr_uuid))),
        ehr_object: Some(Arc::new(|_id| Ok(json!({ "_type": "EHR" })))),
        get_composition_latest: Some(Arc::new(|_e, _vo| {
            Ok(json!({
                "_type": "COMPOSITION",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": "comp::ehrbase::1" }
            }))
        })),
        update_composition: Some(Arc::new(|_e, _vo, _uv| Ok("comp::ehrbase::2".to_owned()))),
        delete_composition: Some(Arc::new(|_e, _ovid| Ok("comp::ehrbase::3".to_owned()))),
        ..Default::default()
    }
}

// ── harness ──────────────────────────────────────────────────────────────────

fn hash_pw(pw: &str) -> String {
    let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").expect("salt");
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

fn rest_config() -> AppConfig {
    AppConfig {
        auth: AuthConfig {
            enabled: true,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: ehrbase_sm::Secret::new(hash_pw("pw")),
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: None,
            admin_scope: None,
            ..AuthConfig::default()
        },
        ..Default::default()
    }
}

/// Build the app with an in-memory audit sink on the backend's SM `SystemLog`
///. `build_with` installs authentication from `rest_config()`.
fn app(sink: AuditSink) -> Router {
    let mut h = hooks();
    h.audit = Some(sink);
    ehrbase_rest::build_with(rest_config(), Arc::new(Mock::with(h))).expect("build app")
}

fn req(method: &str, path: &str, auth: bool) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("x-forwarded-for", CLIENT_IP)
        .header("content-type", "application/json");
    if auth {
        b = b.header("authorization", BASIC_ALICE);
    }
    // A minimal body: required by the write paths (PUT/POST parse it), ignored
    // by reads/deletes.
    b.body(Body::empty()).expect("request")
}

/// Drive one request; return the response status and the audit events emitted.
async fn drive(
    sink: &AuditSink,
    app: &Router,
    request: Request<Body>,
) -> (StatusCode, Vec<AuditEvent>) {
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    (resp.status(), sink.events())
}

/// The first recorded event of the given object class (the operation record).
fn record_of(events: &[AuditEvent], class: ObjectClass) -> &AuditEvent {
    events
        .iter()
        .find(|e| e.object == class)
        .unwrap_or_else(|| panic!("no {class:?} audit record in {events:?}"))
}

// ── tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ehr_create_emits_create_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    let (_s, events) = drive(&sink, &app, req("POST", "/ehr", true)).await;
    let rec = record_of(&events, ObjectClass::Ehr);
    assert_eq!(rec.action, EventActionCode::Create);
    assert_eq!(rec.outcome, EventOutcome::Success);
    assert_eq!(rec.user_id, "alice");
    assert_eq!(rec.client_ip.as_deref(), Some(CLIENT_IP));
}

#[tokio::test]
async fn composition_get_emits_read_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    let (_s, events) = drive(
        &sink,
        &app,
        req("GET", &format!("/ehr/{EHR}/composition/{COMP_VO}"), true),
    )
    .await;
    let rec = record_of(&events, ObjectClass::Composition);
    assert_eq!(rec.action, EventActionCode::Read);
    assert_eq!(rec.outcome, EventOutcome::Success);
    // The object id carries the version uid from the ResourceMeta.
    assert_eq!(rec.object_id.as_deref(), Some("comp::ehrbase::1"));
}

#[tokio::test]
async fn composition_update_emits_update_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    // composition_update requires an If-Match header (optimistic concurrency).
    let request = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR}/composition/{COMP_VO}"))
        .header("x-forwarded-for", CLIENT_IP)
        .header("content-type", "application/json")
        .header("authorization", BASIC_ALICE)
        .header("if-match", COMP_OVID)
        .body(Body::from(composition_body().to_string()))
        .expect("request");
    let (_s, events) = drive(&sink, &app, request).await;
    let rec = record_of(&events, ObjectClass::Composition);
    assert_eq!(rec.action, EventActionCode::Update);
    assert_eq!(rec.outcome, EventOutcome::Success);
}

#[tokio::test]
async fn composition_delete_emits_delete_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    let (_s, events) = drive(
        &sink,
        &app,
        req(
            "DELETE",
            &format!("/ehr/{EHR}/composition/{COMP_OVID}"),
            true,
        ),
    )
    .await;
    let rec = record_of(&events, ObjectClass::Composition);
    assert_eq!(rec.action, EventActionCode::Delete);
    assert_eq!(rec.outcome, EventOutcome::Success);
}

#[tokio::test]
async fn aql_execute_emits_execute_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    // A well-formed ad-hoc query (`q` supplied) reaches the QueryService seam,
    // which the mock leaves unimplemented → 501 → outcome serious-failure; action
    // E, object class Query (an ad-hoc query has no object id).
    let (_s, events) = drive(
        &sink,
        &app,
        req(
            "GET",
            "/query/aql?q=SELECT%20c%20FROM%20COMPOSITION%20c",
            true,
        ),
    )
    .await;
    let rec = record_of(&events, ObjectClass::Query);
    assert_eq!(rec.action, EventActionCode::Execute);
    assert_eq!(rec.outcome, EventOutcome::SeriousFailure);
}

#[tokio::test]
async fn unauthenticated_request_emits_401_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    let (status, events) = drive(&sink, &app, req("POST", "/ehr", false)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // An authentication event: minor failure, no principal — the `AuditEvent`
    // carries an empty `user_id` (the DICOM renderer substitutes the configured
    // `value_if_missing`, i.e. "UNKNOWN", which `ehrbase::system_log` tests).
    let rec = record_of(&events, ObjectClass::ApplicationActivity);
    assert_eq!(rec.outcome, EventOutcome::MinorFailure);
    assert!(rec.user_id.is_empty(), "no principal → empty subject");
}

#[tokio::test]
async fn template_get_emits_template_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    // The template GET is audited as the Template class (R); the template id is
    // derived from the path.
    let (_s, events) = drive(
        &sink,
        &app,
        req("GET", "/definition/template/adl1.4/vital_signs.v1", true),
    )
    .await;
    let rec = record_of(&events, ObjectClass::Template);
    assert_eq!(rec.action, EventActionCode::Read);
    assert_eq!(rec.object_id.as_deref(), Some("vital_signs.v1"));
}

#[tokio::test]
async fn demographic_get_emits_demographic_record() {
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    // Demographic is unimplemented (501) but fully audited: R + serious-failure,
    // the party uid derived from the path.
    let (_s, events) = drive(&sink, &app, req("GET", "/demographic/person/p-42", true)).await;
    let rec = record_of(&events, ObjectClass::Demographic);
    assert_eq!(rec.action, EventActionCode::Read);
    assert_eq!(rec.outcome, EventOutcome::SeriousFailure);
    assert_eq!(rec.object_id.as_deref(), Some("p-42"));
}

#[tokio::test]
async fn login_event_is_suppressed_by_default() {
    // suppress_login=true: an audited operation emits exactly ONE record (the
    // operation itself) and no login/application-activity record.
    let sink = AuditSink::recording().with_suppress_login(true);
    let app = app(sink.clone());
    let (_s, events) = drive(&sink, &app, req("GET", "/definition/template/adl1.4", true)).await;
    assert!(
        events.iter().any(|e| e.object == ObjectClass::Template),
        "the op record is emitted: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.object == ObjectClass::ApplicationActivity),
        "suppressed login must emit no application-activity record: {events:?}"
    );
}

#[tokio::test]
async fn login_event_emitted_when_not_suppressed() {
    // Not suppressed: the operation record AND a login (Application Activity)
    // record are both emitted.
    let sink = AuditSink::recording();
    let app = app(sink.clone());
    let (_s, events) = drive(&sink, &app, req("GET", "/definition/template/adl1.4", true)).await;
    assert!(
        events.iter().any(|e| e.object == ObjectClass::Template),
        "op record present: {events:?}"
    );
    let login = record_of(&events, ObjectClass::ApplicationActivity);
    assert_eq!(login.user_id, "alice");
    assert_eq!(login.outcome, EventOutcome::Success);
}

#[tokio::test]
async fn fail_open_serves_request_when_channel_full() {
    // Fail-open: the audit emit is dropped (queue full → `Dropped`) but the
    // request still succeeds (201).
    let sink = AuditSink::recording().with_emit_outcome(EmitOutcome::Dropped);
    let app = app(sink.clone());
    let (status, _events) = drive(&sink, &app, req("POST", "/ehr", true)).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn fail_closed_returns_503_when_channel_full() {
    // Fail-closed: an auditable operation whose record is rejected (queue full →
    // `Rejected`) makes the middleware return 503.
    let sink = AuditSink::recording().with_emit_outcome(EmitOutcome::Rejected);
    let app = app(sink.clone());
    let (status, _events) = drive(&sink, &app, req("POST", "/ehr", true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn fail_closed_503_carries_openehr_error_body_and_retry_after() {
    // F-34: the fail-closed 503 must emit the standard openEHR `{ error, message }`
    // JSON error body + a `Retry-After` header (the overload-shed contract), not a
    // plain-text body.
    let sink = AuditSink::recording().with_emit_outcome(EmitOutcome::Rejected);
    let app = app(sink);
    let resp = app
        .oneshot(req("POST", "/ehr", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp.headers().get(http::header::RETRY_AFTER).unwrap(), "1");
    assert_eq!(
        resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json error body");
    assert_eq!(body["error"], "Service Unavailable");
    assert!(
        body.get("message")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
}
