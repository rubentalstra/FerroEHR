#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end ABAC PEP over the real axum app (§9.6 of
//! `docs/enterprise/access-control.md`): the fine-grained layer through the
//! assembled router (auth → RBAC → dispatch → ABAC pre/post checks → backend)
//! over the **real** `EhrbaseService` (W-14 B+C: the scripted `Mock` is gone).
//!
//! Drives the router with `tower`'s `oneshot`, a bearer token carrying the
//! `patient_id` claim, an in-memory subject resolver, and a permit-all PDP
//! engine — so the *patient gate* is what denies. It asserts: a composition
//! create for another patient's EHR is a **pre-check 403** (and the ATNA layer
//! records the deny); a create for the caller's own EHR clears the gate; a
//! composition read for another patient's EHR is a **post-check 403**; a read of
//! the caller's own EHR is served; a missing patient claim is a 403; and
//! disabling ABAC restores today's behaviour.
//!
//! The own-patient success paths need real data, so each test seeds its EHR(s)
//! and (for reads) a committed composition through an **auth-off seed app over
//! the same service** (bypassing the gate under test), then drives the gated
//! assertions through the ABAC app.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use ehrbase::config::auth::{AuthConfig, OidcConfig};
use ehrbase::config::authz::AuthzConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase::service::EhrbaseService;
use ehrbase::system_log::config::{AuditConfig, FailMode, StoreConfig, SyslogConfig, Transport};
use ehrbase::system_log::sender::{AuditSender, start};
use ehrbase_rest::config::AppConfig;
use ehrbase_rest::extensions::access::authz::engine::{AuthzError, PolicyEngine};
use ehrbase_rest::extensions::access::authz::request::{AuthzRequest, Decision};
use ehrbase_rest::extensions::access::authz::{AuthzHandle, AuthzResolvers, ResolveError};
use ehrbase_rest::extensions::management::Observability;
use ehrbase_rest::{build_full, build_with};
use http::{Request, StatusCode};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tower::ServiceExt;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const HMAC_SECRET: &str = "abac-test-secret";
const ISSUER: &str = "https://issuer.example";
// Two EHRs with distinct subjects (the resolver maps them below).
const EHR_OWN: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
const EHR_OTHER: &str = "11111111-2222-3333-4444-555555555555";
const PATIENT_OWN: &str = "PATIENT-1";
const PATIENT_OTHER: &str = "PATIENT-2";

// ── fixtures ───────────────────────────────────────────────────────────────────

/// A minimal *valid* templateless RM COMPOSITION (RM ehr `COMPOSITION`
/// mandatory attributes), so `create_composition` commits without an OPT upload.
fn composition() -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": "abac test" },
        "language": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" },
            "code_string": "en"
        },
        "territory": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" },
            "code_string": "NL"
        },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "433"
            }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }
    })
}

// ── a permit-all PDP engine: the patient gate is what denies here ─────────────

#[derive(Debug)]
struct PermitAll;

#[async_trait]
impl PolicyEngine for PermitAll {
    async fn decide(&self, _req: &AuthzRequest<'_>) -> Result<Decision, AuthzError> {
        Ok(Decision::Permit)
    }
}

// ── config + app assembly ─────────────────────────────────────────────────────

fn rest_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: true,
            basic: None,
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
        ..Default::default()
    }
}

/// An auth-off config used only to seed data over the shared service.
fn seed_config() -> AppConfig {
    let mut c = rest_config();
    c.auth = AuthConfig {
        enabled: false,
        ..AuthConfig::default()
    };
    c
}

/// The in-memory subject resolver: EHR → patient subject.
fn resolvers() -> AuthzResolvers {
    AuthzResolvers {
        subject: Arc::new(|ehr_id: String| {
            Box::pin(async move {
                let subject = match ehr_id.as_str() {
                    EHR_OWN => Some(PATIENT_OWN.to_owned()),
                    EHR_OTHER => Some(PATIENT_OTHER.to_owned()),
                    _ => None,
                };
                Ok::<_, ResolveError>(subject)
            })
        }),
        template_of_version: Arc::new(|_vo: String, _v: Option<String>| {
            Box::pin(async move { Ok::<_, ResolveError>(Some("org.openehr::t.v1".to_owned())) })
        }),
    }
}

fn authz(abac_enabled: bool) -> Option<Arc<AuthzHandle>> {
    let mut cfg = AuthzConfig::default();
    cfg.abac.enabled = abac_enabled;
    let engine: Option<Arc<dyn PolicyEngine>> = abac_enabled.then(|| Arc::new(PermitAll) as _);
    AuthzHandle::build(&cfg, &rest_config().server.base_path, engine, resolvers()).map(Arc::new)
}

/// A real service over a fresh DB, optionally wired with an ATNA audit sender.
async fn service(name: &str, audit: Option<AuditSender>) -> (common::Pg, Arc<EhrbaseService>) {
    let (pg, pool) = common::migrated_pool(name).await;
    let mut svc = EhrbaseService::new(pool);
    if let Some(sender) = audit {
        svc = svc.with_audit(sender);
    }
    (pg, Arc::new(svc))
}

/// The ABAC app over `svc`.
fn abac_app(svc: Arc<EhrbaseService>, abac_enabled: bool) -> Router {
    build_full(
        rest_config(),
        svc,
        authz(abac_enabled),
        Observability::default(),
    )
    .expect("build app")
}

// ── seeding through an auth-off app over the same service ───────────────────────

/// Create an EHR with a chosen id (`PUT /ehr/{id}` → 201).
async fn seed_ehr(seed: &Router, ehr_id: &str) {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr_id}"))
        .body(Body::empty())
        .expect("request");
    let resp = seed.clone().oneshot(req).await.expect("seed ehr");
    assert_eq!(resp.status(), StatusCode::CREATED, "seed EHR {ehr_id}");
}

/// Commit a composition into `ehr_id`; return its version uid (from the `ETag`).
async fn seed_composition(seed: &Router, ehr_id: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
        .header("content-type", "application/json")
        .body(Body::from(composition().to_string()))
        .expect("request");
    let resp = seed.clone().oneshot(req).await.expect("seed composition");
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "seed composition in {ehr_id}"
    );
    let etag = resp
        .headers()
        .get(http::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .expect("ETag on create");
    // ETag is `W/"<version_uid>"`.
    etag.trim_start_matches("W/").trim_matches('"').to_owned()
}

// ── ATNA capture ───────────────────────────────────────────────────────────────

async fn audit_capture() -> (UdpSocket, AuditSender) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind udp");
    let port = socket.local_addr().expect("addr").port();
    let cfg = AuditConfig {
        enabled: true,
        store: StoreConfig {
            enabled: false,
            retention_days: 0,
        },
        syslog: SyslogConfig {
            enabled: true,
            transport: Transport::Udp,
            host: "127.0.0.1".to_owned(),
            port,
            ..SyslogConfig::default()
        },
        suppress_login_events: true,
        fail_mode: FailMode::Open,
        queue_capacity: 64,
        ..AuditConfig::default()
    };
    let (sender, _handle) = start(cfg, None, None).await.expect("audit start");
    (socket, sender)
}

/// Collect DICOM audit datagrams until the window elapses.
async fn drain_audit(socket: &UdpSocket) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; 65_536];
    while let Ok(Ok((n, _))) =
        tokio::time::timeout(Duration::from_millis(800), socket.recv_from(&mut buf)).await
    {
        out.push(String::from_utf8_lossy(&buf[..n]).into_owned());
    }
    out
}

// ── credentials + requests ────────────────────────────────────────────────────

/// A bearer token with a `USER` role (via `scope`) and, optionally, a
/// `patient_id` claim.
fn bearer(patient: Option<&str>) -> String {
    let exp = u64::try_from(jiff::Timestamp::now().as_second()).unwrap() + 3600;
    let mut claims = json!({ "sub": "svc", "iss": ISSUER, "exp": exp, "scope": "USER" });
    if let Some(p) = patient {
        claims["patient_id"] = json!(p);
    }
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(HMAC_SECRET.as_bytes()),
    )
    .expect("encode");
    format!("Bearer {token}")
}

fn request(method: &str, path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("authorization", token)
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9")
        .body(Body::from(composition().to_string()))
        .expect("request")
}

async fn status(app: &Router, req: Request<Body>) -> StatusCode {
    app.clone().oneshot(req).await.expect("oneshot").status()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_for_other_patient_is_pre_check_forbidden() {
    // The pre-check denies before dispatch (no data needed).
    let (_pg, svc) = service("abac_pre_other", None).await;
    let app = abac_app(svc, true);
    let s = status(
        &app,
        request(
            "POST",
            &format!("/ehr/{EHR_OTHER}/composition"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "the patient gate must deny a create for another patient's EHR"
    );
}

#[tokio::test]
async fn create_for_own_patient_clears_the_gate() {
    let (_pg, svc) = service("abac_create_own", None).await;
    let seed = build_with(seed_config(), svc.clone()).expect("seed app");
    seed_ehr(&seed, EHR_OWN).await;

    let app = abac_app(svc, true);
    let s = status(
        &app,
        request(
            "POST",
            &format!("/ehr/{EHR_OWN}/composition"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_ne!(
        s,
        StatusCode::FORBIDDEN,
        "own-patient create must clear the gate"
    );
    assert_eq!(s, StatusCode::CREATED);
}

#[tokio::test]
async fn read_of_other_patient_is_post_check_forbidden() {
    let (_pg, svc) = service("abac_read_other", None).await;
    let seed = build_with(seed_config(), svc.clone()).expect("seed app");
    // Seed EHR_OTHER + a composition so the read succeeds and the post-check can
    // evaluate it; the post-check then denies because EHR_OTHER's subject is
    // PATIENT_OTHER while the caller's claim is PATIENT_OWN.
    seed_ehr(&seed, EHR_OTHER).await;
    let uid = seed_composition(&seed, EHR_OTHER).await;

    let app = abac_app(svc, true);
    let s = status(
        &app,
        request(
            "GET",
            &format!("/ehr/{EHR_OTHER}/composition/{uid}"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "the post-check must deny a read of another patient's composition"
    );
}

#[tokio::test]
async fn read_of_own_patient_is_served() {
    let (_pg, svc) = service("abac_read_own", None).await;
    let seed = build_with(seed_config(), svc.clone()).expect("seed app");
    seed_ehr(&seed, EHR_OWN).await;
    let uid = seed_composition(&seed, EHR_OWN).await;

    let app = abac_app(svc, true);
    let s = status(
        &app,
        request(
            "GET",
            &format!("/ehr/{EHR_OWN}/composition/{uid}"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "own-patient read is served");
}

#[tokio::test]
async fn missing_patient_claim_is_forbidden() {
    let (_pg, svc) = service("abac_no_claim", None).await;
    let app = abac_app(svc, true);
    let s = status(
        &app,
        request(
            "POST",
            &format!("/ehr/{EHR_OWN}/composition"),
            &bearer(None),
        ),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "a token without the configured patient claim is denied"
    );
}

#[tokio::test]
async fn abac_disabled_restores_behaviour() {
    // ABAC off → no gate → a create for any EHR is served (auth+RBAC only).
    let (_pg, svc) = service("abac_disabled", None).await;
    let seed = build_with(seed_config(), svc.clone()).expect("seed app");
    seed_ehr(&seed, EHR_OTHER).await;

    let app = abac_app(svc, false);
    let s = status(
        &app,
        request(
            "POST",
            &format!("/ehr/{EHR_OTHER}/composition"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_ne!(s, StatusCode::FORBIDDEN);
    assert_eq!(s, StatusCode::CREATED);
}

#[tokio::test]
async fn abac_deny_is_audited() {
    // A 403 from the ABAC gate carries the Principal, so the ATNA layer records a
    // failure audit for the denied caller (§7). The emitter ships a DICOM record
    // over syslog/UDP; we assert on the datagrams: a minor-failure outcome
    // (`EventOutcomeIndicator="4"`) attributed to `svc`.
    let (socket, sender) = audit_capture().await;
    let (_pg, svc) = service("abac_deny_audit", Some(sender)).await;
    let app = abac_app(svc, true);

    let s = status(
        &app,
        request(
            "POST",
            &format!("/ehr/{EHR_OTHER}/composition"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let records = drain_audit(&socket).await;
    assert!(
        records
            .iter()
            .any(|xml| xml.contains(r#"EventOutcomeIndicator="4""#)
                && xml.contains(r#"UserID="svc""#)),
        "expected a minor-failure audit for `svc`, got {records:?}"
    );
}
