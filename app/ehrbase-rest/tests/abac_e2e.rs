//! End-to-end ABAC PEP over the real axum app (§9.6 of
//! `docs/enterprise/access-control.md`): the fine-grained layer through the
//! assembled router (auth → RBAC → dispatch → ABAC pre/post checks → backend).
//!
//! Drives the router with `tower`'s `oneshot`, a bearer token carrying the
//! `patient_id` claim, an in-memory subject resolver, and a permit-all PDP
//! engine — so the *patient gate* is what denies. It asserts: a composition
//! create for another patient's EHR is a **pre-check 403** (and the ATNA layer
//! records the deny); a create for the caller's own EHR clears the gate; a
//! composition read for another patient's EHR is a **post-check 403**; a read of
//! the caller's own EHR is served; a missing patient claim is a 403; and
//! disabling ABAC restores today's behaviour.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use ehrbase_rest::access::authn::AuthConfig;
use ehrbase_rest::access::authn::config::{OidcConfig, Redacted};
use ehrbase_rest::access::authz::AuthzConfig;
use ehrbase_rest::access::authz::engine::{AuthzError, PolicyEngine};
use ehrbase_rest::access::authz::request::{AuthzRequest, Decision};
use ehrbase_rest::{
    AuthzHandle, AuthzResolvers, Observability, ResolveError, RestConfig, build_full,
};
use ehrbase_sm::EventOutcome;
use http::{Request, StatusCode};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::{AuditSink, Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const HMAC_SECRET: &str = "abac-test-secret";
const ISSUER: &str = "https://issuer.example";
// Two EHRs with distinct subjects (the resolver maps them below).
const EHR_OWN: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
const EHR_OTHER: &str = "11111111-2222-3333-4444-555555555555";
const PATIENT_OWN: &str = "PATIENT-1";
const PATIENT_OTHER: &str = "PATIENT-2";

// ── mock platform: composition create/read succeed so the ABAC gate is what
// decides (the pre-check runs before dispatch; the post-check after the read).

/// A valid canonical COMPOSITION (the vendored Demo Vitals instance) — the EHR
/// dispatcher parses the create body before the call, so the allowed-create
/// tests need a real one.
fn composition_body() -> serde_json::Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/demo_vitals_352.json",
    );
    let text = std::fs::read_to_string(path).expect("demo_vitals_352.json vendored");
    serde_json::from_str(&text).expect("valid canonical composition")
}

fn hooks() -> Hooks {
    Hooks {
        create_composition: Some(Arc::new(|_e, _uv| Ok(format!("{EHR_OWN}::sys::1")))),
        // The read uses a versioned uid ({ehr}::sys::1) → get_composition_at_version.
        get_composition_at_version: Some(Arc::new(|_e, _ovid| {
            Ok(json!({
                "_type": "COMPOSITION",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": format!("{EHR_OWN}::sys::1") }
            }))
        })),
        ..Default::default()
    }
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

fn rest_config() -> RestConfig {
    RestConfig {
        smart: Default::default(),
        system: Default::default(),
        auth: AuthConfig {
            enabled: true,
            basic: None,
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
        ..RestConfig::default()
    }
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
    AuthzHandle::build(&cfg, &rest_config().base_path, engine, resolvers()).map(Arc::new)
}

fn app(abac_enabled: bool, audit: Option<AuditSink>) -> Router {
    // ATNA audit now emits through the backend's SM `SystemLog`; the
    // in-memory sink on the mock records events for assertions.
    let mut h = hooks();
    h.audit = audit;
    build_full(
        rest_config(),
        Arc::new(Mock::with(h)),
        authz(abac_enabled),
        Observability::default(),
    )
    .expect("build app")
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
        .body(Body::from(composition_body().to_string()))
        .expect("request")
}

async fn status(app: &Router, req: Request<Body>) -> StatusCode {
    app.clone().oneshot(req).await.expect("oneshot").status()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_for_other_patient_is_pre_check_forbidden() {
    let app = app(true, None);
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
    let app = app(true, None);
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
    let app = app(true, None);
    let s = status(
        &app,
        request(
            "GET",
            &format!("/ehr/{EHR_OTHER}/composition/{EHR_OWN}::sys::1"),
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
    let app = app(true, None);
    let s = status(
        &app,
        request(
            "GET",
            &format!("/ehr/{EHR_OWN}/composition/{EHR_OWN}::sys::1"),
            &bearer(Some(PATIENT_OWN)),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "own-patient read is served");
}

#[tokio::test]
async fn missing_patient_claim_is_forbidden() {
    let app = app(true, None);
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
    let app = app(false, None);
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
    // A 403 from the ABAC gate carries the Principal, so the ATNA layer records
    // a failure audit for the denied caller (§7). The emitter now lives in the
    // backend's SM `SystemLog`, so we assert the recorded `AuditEvent`
    // (a minor-failure attributed to `svc`); the DICOM rendering is covered by
    // the `ehrbase::system_log` message tests.
    let sink = AuditSink::recording();
    let app = app(true, Some(sink.clone()));

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

    // The ABAC deny fires in the dispatcher, so both the operation record and
    // the auth-failure Application-Activity record are minor-failures for `svc`;
    // assert (faithful to the original) that a failure is audited for the caller.
    let events = sink.events();
    assert!(
        events
            .iter()
            .any(|e| e.outcome == EventOutcome::MinorFailure && e.user_id == "svc"),
        "expected a minor-failure audit for `svc`, got {events:?}"
    );
}
