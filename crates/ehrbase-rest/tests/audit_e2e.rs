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
use ehrbase_rest::auth::AuthConfig;
use ehrbase_rest::auth::config::{BasicConfig, BasicUser, Redacted};
use ehrbase_rest::{EhrService, ResourceMeta, RestConfig, ServiceResponse, build_with_audit};
use http::{Request, StatusCode};
use openehr_its::rest::generated::ehr::{
    CompositionDeleteParams, CompositionGetParams, CompositionUpdateParams, EhrCreateParams,
};
use openehr_its::rest::runtime::ApiError;
use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tower::ServiceExt;

const BASE: &str = "/ehrbase/rest/openehr/v1";
// base64("alice:pw")
const BASIC_ALICE: &str = "Basic YWxpY2U6cHc=";
const CLIENT_IP: &str = "203.0.113.9";

// ── mock backend ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct MockBackend;

#[async_trait::async_trait]
impl EhrService for MockBackend {
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::new(
            json!({"_type": "EHR"}),
            ResourceMeta::new("ehr-1", "ehr-1"),
        ))
    }

    async fn composition_get(
        &self,
        _params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::new(
            json!({"_type": "COMPOSITION"}),
            ResourceMeta::new("ehr-1", "comp::ehrbase::1"),
        ))
    }

    async fn composition_update(
        &self,
        _params: CompositionUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::new(
            json!({"_type": "COMPOSITION"}),
            ResourceMeta::new("ehr-1", "comp::ehrbase::2"),
        ))
    }

    async fn composition_delete(
        &self,
        _params: CompositionDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::deleted(ResourceMeta::new(
            "ehr-1",
            "comp::ehrbase::3",
        )))
    }
}

impl openehr_its::rest::generated::definition::DefinitionApi for MockBackend {}
impl ehrbase_rest::WebTemplateService for MockBackend {}

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
    build_with_audit(rest_config(), Arc::new(MockBackend), Some(audit)).expect("build app")
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
    b.body(Body::from("{}")).expect("request")
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
        req("GET", "/ehr/ehr-1/composition/comp::x", true),
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
        .uri(format!("{BASE}/ehr/ehr-1/composition/comp::x"))
        .header("x-forwarded-for", CLIENT_IP)
        .header("content-type", "application/json")
        .header("authorization", BASIC_ALICE)
        .header("if-match", "comp::ehrbase::1")
        .body(Body::from("{}"))
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
        req("DELETE", "/ehr/ehr-1/composition/comp::x", true),
    )
    .await;
    assert_eq!(attr(&rec, "EventActionCode"), Some("D"));
    assert_eq!(attr(&rec, "EventOutcomeIndicator"), Some("0"));
}

#[tokio::test]
async fn aql_execute_emits_execute_record() {
    let (sock, port) = listener().await;
    let app = app(port, true).await;
    // The query group is unimplemented (501) → outcome 8; action still E.
    let rec = drive_expect_record(&app, &sock, req("GET", "/query/aql", true)).await;
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
async fn login_event_is_suppressed_by_default() {
    let (sock, port) = listener().await;
    // suppress_login=true: an authenticated request to an unaudited endpoint
    // (template list) must NOT emit a login/application-activity record.
    let app = app(port, true).await;
    let _resp = app
        .clone()
        .oneshot(req("GET", "/definition/template/adl1.4", true))
        .await
        .expect("oneshot");
    assert!(
        recv_record(&sock).await.is_none(),
        "suppressed login must emit no record"
    );
}

#[tokio::test]
async fn login_event_emitted_when_not_suppressed() {
    let (sock, port) = listener().await;
    let app = app(port, false).await;
    let rec =
        drive_expect_record(&app, &sock, req("GET", "/definition/template/adl1.4", true)).await;
    // A successful-authentication application-activity (login) event.
    assert!(rec.contains(r#"originalText="Application Activity""#));
    assert_eq!(attr(&rec, "UserID"), Some("alice"));
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
    let app = build_with_audit(rest_config(), Arc::new(MockBackend), Some(sender)).expect("app");
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
    let app = build_with_audit(rest_config(), Arc::new(MockBackend), Some(sender)).expect("app");
    let resp = app
        .oneshot(req("POST", "/ehr", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
