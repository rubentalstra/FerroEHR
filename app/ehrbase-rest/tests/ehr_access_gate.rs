//! End-to-end `EHR_ACCESS` gate over the real axum app.
//!
//! `EHR_ACCESS` is the spec-grounded access-decision authority ("All access
//! decisions to data in the EHR must be made in accordance with the policies
//! and rules in this object" — RM `org.openehr.rm.ehr.ehr_access.adoc`); the
//! gate runs before dispatch on every EHR-scoped route. These tests drive the
//! assembled router with `tower`'s `oneshot`, backed by the mock platform whose
//! `EhrAccessAdapter` returns per-test scheme settings, and assert: default-open
//! admits anonymous-era flows; `restricted` denies a non-listed principal (403)
//! and admits a listed one; `role:` principals match; the Composition privacy
//! ceiling blocks a read above `max_level` and admits one below; and the
//! gate-keeper preflight blocks a non-gate-keeper `EHR_ACCESS` commit (403) while
//! admitting the gate-keeper. `403` follows the ITS-REST 401/403 discipline
//! (`ITS-REST/.../Requests_and_responses.md` §Authentication and authorization).
//!
//! The concrete scheme is our own design — no openEHR spec governs it
//! (`docs/design/ehr-access-scheme.md`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::AuthConfig;
use ehrbase_rest::access::authn::config::{BasicConfig, BasicUser, Redacted};

mod common;
use common::{Hooks, Mock};
use ehrbase_sm::EhrAccessSettings;
use http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

const BASE: &str = "/ehrbase/rest/openehr/v1";
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

/// A config with Basic auth on (or off) and no OIDC. The four users cover the
/// `user:`/`role:` matching cases.
fn rest_config(auth_enabled: bool) -> RestConfig {
    RestConfig {
        auth: AuthConfig {
            enabled: auth_enabled,
            basic: Some(BasicConfig {
                users: vec![
                    user("alice", &[]),
                    user("bob", &[]),
                    user("carol", &[]),
                    user("nadia", &["NURSE"]),
                ],
            }),
            oidc: None,
            admin_scope: None,
        },
        swagger_ui: false,
        ..RestConfig::default()
    }
}

/// Assemble the router. `authz` is `None` throughout — the enterprise RBAC/ABAC
/// layers are deliberately out, so the `EHR_ACCESS` gate is the only thing that
/// can produce a 403 (isolating what we assert).
fn app(auth_enabled: bool, settings: Option<EhrAccessSettings>) -> Router {
    let mut hooks = Hooks::default();
    if let Some(s) = settings {
        // The field type drives the unsizing coercion to `Arc<dyn Fn…>`.
        hooks.ehr_access_settings = Some(Arc::new(move |_ehr_id| Ok(Some(s.clone()))));
    }
    let backend = Mock::with(hooks);
    ehrbase_rest::build_full(
        rest_config(auth_enabled),
        Arc::new(backend),
        None,
        ehrbase_rest::Observability::default(),
    )
    .expect("build app")
}

fn settings(scheme: &Value) -> EhrAccessSettings {
    EhrAccessSettings::from_ehr_access(&json!({ "_type": "EHR_ACCESS", "settings": scheme }))
        .expect("our scheme")
}

// ── requests ──────────────────────────────────────────────────────────────────

fn basic(name: &str) -> String {
    format!("Basic {}", base64_encode(format!("{name}:pw").as_bytes()))
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn req(method: &str, path: &str, authorization: Option<&str>, body: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9");
    if let Some(auth) = authorization {
        b = b.header("authorization", auth);
    }
    b.body(Body::from(body.to_owned())).expect("request")
}

async fn status(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone()
        .oneshot(request)
        .await
        .expect("oneshot")
        .status()
}

// ── tests ───────────────────────────────────────────────────────────────────

/// Default-open (no scheme settings) admits an anonymous request — the
/// gate never turns a working, unconfigured EHR away (`master07` "sensible
/// defaults"). The backend then answers (`501`, unhooked); the point is it is
/// NOT a `403` from the gate.
#[tokio::test]
async fn default_open_admits_anonymous() {
    let app = app(false, None);
    let code = status(&app, req("GET", &format!("/ehr/{EHR_ID}"), None, "")).await;
    assert_ne!(
        code,
        StatusCode::FORBIDDEN,
        "default-open must not gate anonymous"
    );
}

/// `restricted` denies a principal absent from the access list (403) and admits
/// a listed one.
#[tokio::test]
async fn restricted_gates_by_user_principal() {
    let s = settings(&json!({
        "_type": "EHRBASE_ACCESS_CONTROL_V1",
        "default_access": "restricted",
        "access_list": [ { "principal": "user:bob", "access": "full" } ]
    }));
    let app = app(true, Some(s));

    let denied = status(
        &app,
        req("GET", &format!("/ehr/{EHR_ID}"), Some(&basic("carol")), ""),
    )
    .await;
    assert_eq!(
        denied,
        StatusCode::FORBIDDEN,
        "carol is not on the access list"
    );

    let allowed = status(
        &app,
        req("GET", &format!("/ehr/{EHR_ID}"), Some(&basic("bob")), ""),
    )
    .await;
    assert_ne!(
        allowed,
        StatusCode::FORBIDDEN,
        "bob is listed → gate passes"
    );
}

/// A `role:` access-list entry matches the caller's roles.
#[tokio::test]
async fn restricted_gates_by_role_principal() {
    let s = settings(&json!({
        "_type": "EHRBASE_ACCESS_CONTROL_V1",
        "default_access": "restricted",
        "access_list": [ { "principal": "role:nurse", "access": "full" } ]
    }));
    let app = app(true, Some(s));

    let allowed = status(
        &app,
        req("GET", &format!("/ehr/{EHR_ID}"), Some(&basic("nadia")), ""),
    )
    .await;
    assert_ne!(
        allowed,
        StatusCode::FORBIDDEN,
        "nadia has role NURSE → gate passes"
    );

    let denied = status(
        &app,
        req("GET", &format!("/ehr/{EHR_ID}"), Some(&basic("alice")), ""),
    )
    .await;
    assert_eq!(denied, StatusCode::FORBIDDEN, "alice has no matching role");
}

/// The Composition privacy ceiling blocks a read of a Composition raised above
/// the caller's `max_level` and admits one at/below it.
#[tokio::test]
async fn privacy_ceiling_gates_composition_reads() {
    // vo head of the "high" composition uid the override pins to level 3.
    let high = "8849182c-82ad-4088-a07f-48ead4180515";
    let low = "11111111-1111-4111-8111-111111111111";
    let s = settings(&json!({
        "_type": "EHRBASE_ACCESS_CONTROL_V1",
        "default_access": "open",
        "access_list": [ { "principal": "role:nurse", "access": "restricted_below", "max_level": 2 } ],
        "privacy": {
            "default_level": 0,
            "composition_overrides": [ { "uid": high, "level": 3 } ]
        }
    }));
    let app = app(true, Some(s));

    // nadia (NURSE, ceiling 2): the level-3 composition is blocked.
    let blocked = status(
        &app,
        req(
            "GET",
            &format!("/ehr/{EHR_ID}/composition/{high}::sys::1"),
            Some(&basic("nadia")),
            "",
        ),
    )
    .await;
    assert_eq!(blocked, StatusCode::FORBIDDEN, "level 3 >= ceiling 2");

    // A default-level (0) composition is readable (0 < 2).
    let readable = status(
        &app,
        req(
            "GET",
            &format!("/ehr/{EHR_ID}/composition/{low}::sys::1"),
            Some(&basic("nadia")),
            "",
        ),
    )
    .await;
    assert_ne!(
        readable,
        StatusCode::FORBIDDEN,
        "level 0 < ceiling 2 → gate passes"
    );
}

/// The gate-keeper preflight: a CONTRIBUTION carrying an `EHR_ACCESS` version is
/// refused (403) for a non-gate-keeper and admitted for the gate-keeper.
#[tokio::test]
async fn gate_keeper_guards_ehr_access_commits() {
    let s = settings(&json!({
        "_type": "EHRBASE_ACCESS_CONTROL_V1",
        "gate_keeper": "user:alice",
        "default_access": "open"
    }));
    let app = app(true, Some(s));

    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [ {
            "_type": "ORIGINAL_VERSION",
            "data": { "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1" }
        } ],
        "audit": { "committer": { "_type": "PARTY_IDENTIFIED", "name": "x" } }
    })
    .to_string();

    let denied = status(
        &app,
        req(
            "POST",
            &format!("/ehr/{EHR_ID}/contribution"),
            Some(&basic("bob")),
            &contribution,
        ),
    )
    .await;
    assert_eq!(denied, StatusCode::FORBIDDEN, "bob is not the gate-keeper");

    let allowed = status(
        &app,
        req(
            "POST",
            &format!("/ehr/{EHR_ID}/contribution"),
            Some(&basic("alice")),
            &contribution,
        ),
    )
    .await;
    assert_ne!(
        allowed,
        StatusCode::FORBIDDEN,
        "alice is the gate-keeper → gate passes"
    );
}

/// A CONTRIBUTION that does NOT carry an `EHR_ACCESS` version is not gate-kept,
/// even from a non-gate-keeper (the preflight is scoped to `EHR_ACCESS` writes).
#[tokio::test]
async fn gate_keeper_ignores_non_ehr_access_contributions() {
    let s = settings(&json!({
        "_type": "EHRBASE_ACCESS_CONTROL_V1",
        "gate_keeper": "user:alice",
        "default_access": "open"
    }));
    let app = app(true, Some(s));

    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [ { "_type": "ORIGINAL_VERSION", "data": { "_type": "COMPOSITION" } } ],
        "audit": { "committer": { "_type": "PARTY_IDENTIFIED", "name": "x" } }
    })
    .to_string();

    let code = status(
        &app,
        req(
            "POST",
            &format!("/ehr/{EHR_ID}/contribution"),
            Some(&basic("bob")),
            &contribution,
        ),
    )
    .await;
    assert_ne!(
        code,
        StatusCode::FORBIDDEN,
        "a COMPOSITION contribution is not gate-kept"
    );
}
