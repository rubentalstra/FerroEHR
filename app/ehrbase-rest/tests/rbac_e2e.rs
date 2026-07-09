//! End-to-end RBAC gate over the real axum app (§9.6 subset of
//! `docs/enterprise/access-control.md`).
//!
//! Drives the assembled router (auth + RBAC + dispatch) with `tower`'s
//! `oneshot`, backed by the `StubBackend`, and asserts the coarse role gate:
//! an Admin-class operation is 403 for a `USER` and passes the gate for an
//! `ADMIN`; a clinical operation needs a role; disabling RBAC restores today's
//! behaviour; a deny is attributed to the caller and audited by the ATNA layer;
//! and the deprecated `admin_scope` alias migrates (a `scope` named `ADMIN`
//! surfaces as role `ADMIN`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase_audit::{AuditConfig, AuditSender, Transport};
use ehrbase_rest::auth::AuthConfig;
use ehrbase_rest::auth::config::{BasicConfig, BasicUser, OidcConfig, Redacted};
use ehrbase_rest::authz::AuthzConfig;
use ehrbase_rest::{AdminConfig, AuthzHandle, RestConfig};

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
        password_hash: Redacted(hash_pw("pw")),
        roles: roles.iter().map(|r| (*r).to_owned()).collect(),
    }
}

fn rest_config() -> RestConfig {
    RestConfig {
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
                hmac_secret: Some(Redacted(HMAC_SECRET.to_owned())),
                jwks_json: None,
            }),
            admin_scope: None,
        },
        swagger_ui: false,
        // The admin group must be reachable so the RBAC gate is what decides
        // access (the admin tests assert 403 for USER / 501 for ADMIN at the
        // dispatcher, not the config gate's 404).
        admin: AdminConfig { enabled: true },
        ..RestConfig::default()
    }
}

fn authz(enabled: bool) -> Option<Arc<AuthzHandle>> {
    let mut cfg = AuthzConfig::default();
    cfg.rbac.enabled = enabled;
    AuthzHandle::from_config(&cfg, &rest_config().base_path).map(Arc::new)
}

fn app(rbac_enabled: bool, audit: Option<AuditSender>) -> Router {
    ehrbase_rest::build_full(
        rest_config(),
        Arc::new(common::Mock::new()),
        audit,
        authz(rbac_enabled),
        ehrbase_rest::Observability::default(),
    )
    .expect("build app")
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
    let app = app(true, None);
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "USER must not reach an admin op");
}

#[tokio::test]
async fn admin_op_passes_gate_for_admin_role() {
    let app = app(true, None);
    // ADMIN passes the RBAC gate; the admin group has no backend yet, so the op
    // itself answers 501 — the point is the gate did NOT reject it with 403.
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("root")),
    )
    .await;
    assert_ne!(s, StatusCode::FORBIDDEN, "ADMIN must clear the gate");
    assert_eq!(s, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn clinical_op_allowed_for_user_role() {
    let app = app(true, None);
    let s = status(&app, req("GET", &format!("/ehr/{EHR_ID}"), &basic("user"))).await;
    assert_ne!(s, StatusCode::FORBIDDEN, "USER may reach a clinical op");
    assert_eq!(s, StatusCode::NOT_IMPLEMENTED); // StubBackend
}

#[tokio::test]
async fn zero_role_principal_denied_clinical() {
    let app = app(true, None);
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
    let app = app(false, None);
    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_ne!(s, StatusCode::FORBIDDEN);
    assert_eq!(s, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn admin_scope_alias_migrates_via_scope_role() {
    // The deprecated `admin_scope` gate is subsumed: a token whose `scope`
    // carries `ADMIN` surfaces role `ADMIN` and clears the admin gate, while a
    // non-admin scope is rejected — the automatic migration path (§5.2).
    let app = app(true, None);
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
    assert_eq!(allowed, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn rbac_deny_is_audited() {
    // A 403 from the RBAC gate carries the Principal on the response, so the
    // outer ATNA layer records a failure audit for the denied caller (§7).
    let (sock, port) = udp_listener().await;
    let sender = audit_sender(port).await;
    let app = app(true, Some(sender));

    let s = status(
        &app,
        req("DELETE", &format!("/admin/ehr/{EHR_ID}"), &basic("user")),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let record = recv_record(&sock).await.expect("expected an audit record");
    assert_eq!(
        attr(&record, "EventOutcomeIndicator"),
        Some("4"),
        "{record}"
    );
    assert_eq!(attr(&record, "UserID"), Some("user"), "{record}");
}

// ── audit + base64 helpers ────────────────────────────────────────────────────

async fn udp_listener() -> (UdpSocket, u16) {
    let sock = UdpSocket::bind(("127.0.0.1", 0)).await.expect("bind udp");
    let port = sock.local_addr().expect("addr").port();
    (sock, port)
}

async fn audit_sender(port: u16) -> AuditSender {
    let config = AuditConfig {
        enabled: true,
        transport: Transport::Udp,
        repository_host: "127.0.0.1".to_owned(),
        repository_port: port,
        // Emit the login/auth records so the deny surfaces on the listener.
        suppress_login_events: true,
        queue_capacity: 64,
        ..AuditConfig::default()
    };
    let (sender, handle) = ehrbase_audit::start(config, None)
        .await
        .expect("start audit");
    std::mem::forget(handle);
    sender
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

fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
