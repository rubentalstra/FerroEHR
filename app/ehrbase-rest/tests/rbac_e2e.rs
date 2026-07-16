//! End-to-end RBAC gate over the real axum app (§9.6 subset of
//! `docs/enterprise/access-control.md`).
//!
//! Drives the assembled router (auth + RBAC + dispatch) with `tower`'s
//! `oneshot`, over the **real** `EhrbaseService` (W-14 B+C: the scripted `Mock`
//! is gone), and asserts the coarse role gate: an Admin-class operation is 403
//! for a `USER` and clears the gate for an `ADMIN`; a clinical operation needs a
//! role; disabling RBAC restores today's behaviour; a deny is attributed to the
//! caller and audited by the ATNA layer; and the deprecated `admin_scope` alias
//! migrates (a `scope` named `ADMIN` surfaces as role `ADMIN`).
//!
//! Where the old test asserted the exact post-gate status (`501`, the removed
//! stub backend), it now asserts only that the gate did not reject (`!= 403`) —
//! the concrete post-gate status is real-backend behaviour, not the gate's.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase::config::auth::{AuthConfig, BasicConfig, BasicUser, OidcConfig};
use ehrbase::config::authz::AuthzConfig;
use ehrbase::config::server::AdminConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase::service::EhrbaseService;
use ehrbase::system_log::config::{AuditConfig, FailMode, Transport};
use ehrbase::system_log::{AuditSender, start};
use ehrbase_rest::{AppConfig, AuthzHandle};

mod common;
use http::{Request, StatusCode};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tower::ServiceExt;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const HMAC_SECRET: &str = "rbac-test-secret";
const ISSUER: &str = "https://issuer.example";
const EHR_ID: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";

// ── config + app assembly ─────────────────────────────────────────────────────

fn hash_pw(pw: &str) -> String {
    let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").expect("salt");
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

fn user(name: &str, roles: &[&str]) -> BasicUser {
    BasicUser {
        username: name.to_owned(),
        password_hash: ehrbase::config::secret::Secret::new(hash_pw("pw")),
        roles: roles.iter().map(|r| (*r).to_owned()).collect(),
    }
}

fn rest_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: true,
            basic: Some(BasicConfig {
                users: vec![
                    user("user", &["USER"]),
                    user("root", &["ADMIN"]),
                    user("noroles", &[]),
                ],
            }),
            oidc: Some(OidcConfig {
                issuer: ISSUER.to_owned(),
                audiences: vec![],
                algorithms: vec!["HS256".to_owned()],
                hmac_secret: Some(ehrbase::config::secret::Secret::new(HMAC_SECRET.to_owned())),
                jwks_json: None,
                ..OidcConfig::default()
            }),
            admin_scope: None,
            ..AuthConfig::default()
        },
        // The admin group must be reachable so the RBAC gate is what decides
        // access (the admin tests assert 403 for USER vs a cleared gate for
        // ADMIN at the dispatcher, not the config gate's 404).
        admin: AdminConfig { enabled: true },
        ..Default::default()
    }
}

fn authz(enabled: bool) -> Option<Arc<AuthzHandle>> {
    let mut cfg = AuthzConfig::default();
    cfg.rbac.enabled = enabled;
    AuthzHandle::from_config(&cfg, &rest_config().server.base_path).map(Arc::new)
}

/// A real service over a fresh DB, optionally wired with an ATNA audit sender.
async fn service(name: &str, audit: Option<AuditSender>) -> Arc<EhrbaseService> {
    let pool = common::migrated_pool(name).await;
    let mut svc = EhrbaseService::new(pool);
    if let Some(sender) = audit {
        svc = svc.with_audit(sender);
    }
    Arc::new(svc)
}

async fn app(name: &str, rbac_enabled: bool, audit: Option<AuditSender>) -> Router {
    ehrbase_rest::build_full(
        rest_config(),
        service(name, audit).await,
        authz(rbac_enabled),
        ehrbase_rest::Observability::default(),
    )
    .expect("build app")
}

// ── ATNA capture: a UDP listener + a sender pointed at it ──────────────────────

/// Bind a UDP listener and build an [`AuditSender`] shipping DICOM records to it
/// (fail-open, login events suppressed). The returned socket receives the RFC
/// 5424 datagrams; each carries the DICOM `AuditMessage` XML in its `MSG` field.
async fn audit_capture() -> (UdpSocket, AuditSender) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind udp");
    let port = socket.local_addr().expect("addr").port();
    let cfg = AuditConfig {
        enabled: true,
        transport: Transport::Udp,
        repository_host: "127.0.0.1".to_owned(),
        repository_port: port,
        suppress_login_events: true,
        fail_mode: FailMode::Open,
        queue_capacity: 64,
        ..AuditConfig::default()
    };
    // The drain task is detached (we drop the handle); it runs while the sender
    // — held by the service — is alive, which spans the whole test.
    let (sender, _handle) = start(cfg, None).await.expect("audit start");
    (socket, sender)
}

/// Receive one DICOM audit datagram (as UTF-8) within a short window.
async fn recv_audit(socket: &UdpSocket) -> Option<String> {
    let mut buf = vec![0u8; 65_536];
    match tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf)).await {
        Ok(Ok((n, _))) => Some(String::from_utf8_lossy(&buf[..n]).into_owned()),
        _ => None,
    }
}

// ── credentials + requests ────────────────────────────────────────────────────

/// A `Basic user:pw` header for a configured user.
fn basic(name: &str) -> String {
    format!("Basic {}", base64_encode(format!("{name}:pw").as_bytes()))
}

/// A `Bearer` HMAC token carrying the given `scope` claim.
fn bearer(scope: &str) -> String {
    let exp = u64::try_from(jiff::Timestamp::now().as_second()).unwrap() + 3600;
    let claims: Value = json!({ "sub": "svc", "iss": ISSUER, "exp": exp, "scope": scope });
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(HMAC_SECRET.as_bytes()),
    )
    .expect("encode");
    format!("Bearer {token}")
}

fn req(method: &str, path: &str, authorization: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("authorization", authorization)
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9")
        .body(Body::from("{}"))
        .expect("request")
}

async fn status(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone()
        .oneshot(request)
        .await
        .expect("oneshot")
        .status()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn admin_op_forbidden_for_user_role() {
    let app = app("rbac_admin_user", true, None).await;
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "USER must not reach an admin op");
}

#[tokio::test]
async fn admin_op_passes_gate_for_admin_role() {
    let app = app("rbac_admin_admin", true, None).await;
    // ADMIN clears the RBAC gate; the concrete post-gate status is real-backend
    // behaviour — the point is the gate did NOT reject it with 403.
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("root")),
    )
    .await;
    assert_ne!(s, StatusCode::FORBIDDEN, "ADMIN must clear the gate");
}

#[tokio::test]
async fn clinical_op_allowed_for_user_role() {
    let app = app("rbac_clinical_user", true, None).await;
    let s = status(&app, req("GET", &format!("/ehr/{EHR_ID}"), &basic("user"))).await;
    assert_ne!(s, StatusCode::FORBIDDEN, "USER may reach a clinical op");
}

#[tokio::test]
async fn zero_role_principal_denied_clinical() {
    let app = app("rbac_zero_role", true, None).await;
    let s = status(
        &app,
        req("GET", &format!("/ehr/{EHR_ID}"), &basic("noroles")),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "a roleless principal is denied when RBAC is enabled"
    );
}

#[tokio::test]
async fn rbac_disabled_restores_admin_access() {
    // With RBAC disabled the handle is None → the gate is skipped; a USER reaches
    // the admin op exactly as before this feature (auth-only behaviour).
    let app = app("rbac_disabled", false, None).await;
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_ne!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_scope_alias_migrates_via_scope_role() {
    // The deprecated `admin_scope` gate is subsumed: a token whose `scope`
    // carries `ADMIN` surfaces role `ADMIN` and clears the admin gate, while a
    // non-admin scope is rejected — the automatic migration path (§5.2).
    let app = app("rbac_scope_alias", true, None).await;
    let denied = status(
        &app,
        req(
            "DELETE",
            &format!("/admin/ehr/{EHR_ID}"),
            &bearer("openid profile"),
        ),
    )
    .await;
    assert_eq!(denied, StatusCode::FORBIDDEN);
    let allowed = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &bearer("ADMIN")),
    )
    .await;
    assert_ne!(
        allowed,
        StatusCode::FORBIDDEN,
        "scope ADMIN → role ADMIN clears the gate"
    );
}

#[tokio::test]
async fn rbac_deny_is_audited() {
    // A 403 from the RBAC gate carries the Principal, so the ATNA audit layer
    // records a failure audit for the denied caller (§7). The emitter ships a
    // DICOM record over syslog/UDP; we assert on the datagram: an
    // Application-Activity event (`csd-code="110100"`), a minor-failure outcome
    // (`EventOutcomeIndicator="4"`), attributed to `user`.
    let (socket, sender) = audit_capture().await;
    let app = app("rbac_deny_audit", true, Some(sender)).await;

    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let xml = recv_audit(&socket)
        .await
        .expect("an audit datagram is emitted for the denied caller");
    assert!(
        xml.contains(r#"csd-code="110100""#),
        "Application-Activity event: {xml}"
    );
    assert!(
        xml.contains(r#"EventOutcomeIndicator="4""#),
        "minor-failure outcome: {xml}"
    );
    assert!(
        xml.contains(r#"UserID="user""#),
        "attributed to `user`: {xml}"
    );
}

// ── base64 helper ─────────────────────────────────────────────────────────────

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
