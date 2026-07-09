//! End-to-end ATNA audit over the real axum app (binding doc §8.5).
//!
//! Drives the assembled router (auth + audit + dispatch) with `tower`'s
//! `oneshot`, backed by a mock service, and asserts that exactly one framed
//! DICOM AuditMessage lands on an in-process UDP syslog listener per audited
//! request — with the correct action / outcome / user / object for EHR create
//! (C), composition get (R) / update (U) / delete (D), AQL execute (E), a 401
//! (outcome 4, principal UNKNOWN), and a suppressed login event; plus the
//! channel-full fail-open drop and fail-closed 503.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

use std::sync::Arc;
use std::time::Duration;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase_audit::{AuditConfig, AuditSender, FailMode, Transport};
use ehrbase_rest::access::authn::AuthConfig;
use ehrbase_rest::access::authn::config::{BasicConfig, BasicUser, Redacted};
use ehrbase_rest::{RestConfig, build_with_audit};
use http::{Request, StatusCode};
use serde_json::json;
use tokio::net::UdpSocket;
use tower::ServiceExt;
use uuid::Uuid;

mod common;
use common::{Hooks, Mock};

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

fn rest_config() -> RestConfig {
    RestConfig {
        auth: AuthConfig {
            enabled: true,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: Redacted(hash_pw("pw")),
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: None,
            admin_scope: None,
        },
        ..RestConfig::default()
    }
}

async fn listener() -> (UdpSocket, u16) {
    let sock = UdpSocket::bind(("127.0.0.1", 0)).await.expect("bind udp");
    let port = sock.local_addr().expect("addr").port();
    (sock, port)
}

async fn sender(port: u16, suppress_login: bool, fail_mode: FailMode) -> AuditSender {
    let config = AuditConfig {
        enabled: true,
        transport: Transport::Udp,
        repository_host: "127.0.0.1".to_owned(),
        repository_port: port,
        source_id: "ehrbase".to_owned(),
        enterprise_site_id: Some("site-1".to_owned()),
        server_host: Some("10.0.0.1".to_owned()),
        suppress_login_events: suppress_login,
        fail_mode,
        queue_capacity: 64,
        ..AuditConfig::default()
    };
    let (sender, handle) = ehrbase_audit::start(config, None)
        .await
        .expect("start audit");
    // Detach the drain task for the test's lifetime.
    std::mem::forget(handle);
    sender
}

async fn app(port: u16, suppress_login: bool) -> Router {
    let audit = sender(port, suppress_login, FailMode::Open).await;
    build_with_audit(rest_config(), Arc::new(Mock::with(hooks())), Some(audit)).expect("build app")
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
    // A minimal JSON body: required by the write paths (PUT/POST parse it),
    // ignored by reads/deletes.
    b.body(Body::empty()).expect("request")
}

/// Send a request through the app and return the one framed audit record.
async fn drive_expect_record(app: &Router, sock: &UdpSocket, request: Request<Body>) -> String {
    let _resp = app.clone().oneshot(request).await.expect("oneshot");
    recv_record(sock).await.expect("expected one audit record")
}

async fn recv_record(sock: &UdpSocket) -> Option<String> {
    let mut buf = vec![0u8; 65536];
    match tokio::time::timeout(Duration::from_secs(2), sock.recv(&mut buf)).await {
        Ok(Ok(n)) => Some(String::from_utf8_lossy(&buf[..n]).into_owned()),
        _ => None,
    }
}

fn attr<'a>(record: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let start = record.find(&needle)? + needle.len();
    let rest = &record[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

// ── tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ehr_create_emits_create_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    let rec = drive_expect_record(&app, &sock, req("POST", "/ehr", true)).await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("C"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("0"));
    assert_eq!(attr(&rec, "UserID"), Some("alice"));
    assert_eq!(attr(&rec, "NetworkAccessPointID"), Some(CLIENT_IP));
    assert!(rec.contains("<85>1 "), "RFC 5424 PRI/version header: {rec}");
    assert!(rec.contains("IHE+DICOM"), "MSGID present");
    assert!(rec.contains(r#"originalText="Patient Record""#));
}

#[tokio::test]
async fn composition_get_emits_read_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    let rec = drive_expect_record(
        &app,
        &sock,
        req("GET", &format!("/ehr/{EHR}/composition/{COMP_VO}"), true),
    )
    .await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("R"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("0"));
    assert!(rec.contains(r#"originalText="composition""#));
    // The object URI participant carries the version uid from the ResourceMeta.
    assert!(
        rec.contains(r#"ParticipantObjectID="comp::ehrbase::1""#),
        "object uid participant: {rec}"
    );
}

#[tokio::test]
async fn composition_update_emits_update_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
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
    let rec = drive_expect_record(&app, &sock, request).await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("U"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("0"));
}

#[tokio::test]
async fn composition_delete_emits_delete_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    let rec = drive_expect_record(
        &app,
        &sock,
        req(
            "DELETE",
            &format!("/ehr/{EHR}/composition/{COMP_OVID}"),
            true,
        ),
    )
    .await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("D"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("0"));
}

#[tokio::test]
async fn aql_execute_emits_execute_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    // A well-formed ad-hoc query (`q` supplied) reaches the QueryService seam,
    // which the MockBackend leaves unimplemented → 501 → outcome 8; action E,
    // participant object "Search Criteria" (an ad-hoc query has no object id).
    let rec = drive_expect_record(
        &app,
        &sock,
        req(
            "GET",
            "/query/aql?q=SELECT%20c%20FROM%20COMPOSITION%20c",
            true,
        ),
    )
    .await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("E"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("8"));
    assert!(rec.contains(r#"originalText="Search Criteria""#));
}

#[tokio::test]
async fn unauthenticated_request_emits_401_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    let resp = app
        .clone()
        .oneshot(req("POST", "/ehr", false))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let rec = recv_record(&sock).await.expect("401 audit record");
    // An authentication event: minor failure, principal UNKNOWN.
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("4"));
    assert_eq!(attr(&rec, "UserID"), Some("UNKNOWN"));
    assert!(rec.contains(r#"originalText="Application Activity""#));
}

#[tokio::test]
async fn template_get_emits_template_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    // The template GET is audited as the Template class (R). The stub backend
    // answers 501 → outcome 8; the template id is derived from the path.
    let rec = drive_expect_record(
        &app,
        &sock,
        req("GET", "/definition/template/adl1.4/vital_signs.v1", true),
    )
    .await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("R"));
    assert!(rec.contains(r#"originalText="template""#));
    assert!(
        rec.contains(r#"ParticipantObjectID="vital_signs.v1""#),
        "template id from the path: {rec}"
    );
}

#[tokio::test]
async fn demographic_get_emits_demographic_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    // Demographic is unimplemented (501) but fully audited: R + outcome 8, the
    // party uid derived from the path.
    let rec = drive_expect_record(&app, &sock, req("GET", "/demographic/person/p-42", true)).await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("R"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("8"));
    assert!(rec.contains(r#"originalText="demographic""#));
    assert!(rec.contains(r#"ParticipantObjectID="p-42""#));
}

#[tokio::test]
async fn login_event_is_suppressed_by_default() {
    let (sock, port) = listener().await;
    // suppress_login=true: an audited operation emits exactly ONE record (the
    // operation itself) and no login/application-activity record.
    let app = app(port, true).await;
    let rec =
        drive_expect_record(&app, &sock, req("GET", "/definition/template/adl1.4", true)).await;
    assert!(rec.contains(r#"originalText="template""#));
    assert!(
        !rec.contains("Application Activity"),
        "no login record inside the op record: {rec}"
    );
    assert!(
        recv_record(&sock).await.is_none(),
        "suppressed login must emit no second record"
    );
}

#[tokio::test]
async fn login_event_emitted_when_not_suppressed() {
    let (sock, port) = listener().await;
    let app = app(port, false).await;
    // Not suppressed: the operation record AND a login (Application Activity)
    // record are both emitted.
    let first =
        drive_expect_record(&app, &sock, req("GET", "/definition/template/adl1.4", true)).await;
    let second = recv_record(&sock).await.expect("login record");
    let (op_rec, login_rec) = if first.contains("Application Activity") {
        (second, first)
    } else {
        (first, second)
    };
    assert!(op_rec.contains(r#"originalText="template""#));
    assert!(login_rec.contains(r#"originalText="Application Activity""#));
    assert_eq!(attr(&login_rec, "UserID"), Some("alice"));
    assert_eq!(attr(&login_rec, "EventOutcomeIndicator"), Some("0"));
}

/// A sender whose single-slot queue is deterministically full: the drain is
/// parked forever on a blocking subject resolver (event #1), and event #2 fills
/// the one remaining slot — so the next `emit` sees a full channel.
async fn full_queue_sender(fail_mode: FailMode) -> AuditSender {
    use ehrbase_audit::{AuditEvent, EventActionCode, EventOutcome, ObjectClass, SubjectResolver};

    let config = AuditConfig {
        enabled: true,
        transport: Transport::Udp,
        repository_host: "127.0.0.1".to_owned(),
        repository_port: 1,
        queue_capacity: 1,
        resolve_subject: true,
        fail_mode,
        ..AuditConfig::default()
    };
    // A resolver that never returns parks the drain on event #1.
    let resolver: SubjectResolver =
        Arc::new(|_id| Box::pin(async { std::future::pending::<Option<String>>().await }));
    let (sender, handle) = ehrbase_audit::start(config, Some(resolver))
        .await
        .expect("start");
    std::mem::forget(handle);

    let mut ev = AuditEvent::new(
        EventActionCode::Read,
        ObjectClass::Ehr,
        EventOutcome::Success,
    );
    ev.ehr_id = Some("ehr-1".to_owned());
    let _ = sender.emit(ev.clone()); // drain takes this and blocks on the resolver
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = sender.emit(ev); // fills the single remaining slot
    sender
}

#[tokio::test]
async fn fail_open_serves_request_when_channel_full() {
    // Fail-open: the app's audit record is dropped (queue full) but the request
    // still succeeds (201), and the drop is metered by the sender.
    let sender = full_queue_sender(FailMode::Open).await;
    let app =
        build_with_audit(rest_config(), Arc::new(Mock::with(hooks())), Some(sender)).expect("app");
    let resp = app
        .oneshot(req("POST", "/ehr", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn fail_closed_returns_503_when_channel_full() {
    // Fail-closed: with the queue full, the next auditable request is rejected.
    let sender = full_queue_sender(FailMode::Closed).await;
    let app =
        build_with_audit(rest_config(), Arc::new(Mock::with(hooks())), Some(sender)).expect("app");
    let resp = app
        .oneshot(req("POST", "/ehr", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
