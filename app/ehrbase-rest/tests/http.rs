//! End-to-end HTTP tests: routing, authentication (401/403), and content
//! negotiation, exercised through the assembled router via `tower`'s `oneshot`.
//!
//! All five API groups are mounted; these tests exercise routing, auth, and
//! negotiation across representative operations.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use tower::ServiceExt;

use std::sync::Arc;

use ehrbase_rest::access::authn::config::{
    AuthConfig, BasicConfig, BasicUser, OidcConfig, Redacted,
};
use ehrbase_rest::{AdminConfig, RestConfig};

mod common;

const ISSUER: &str = "https://issuer.test";
const SECRET: &str = "integration-secret";
const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A syntactically valid EHR id: the EHR dispatcher decodes `ehr_id` into a
/// `Uuid` before consulting the backend, so a "reaches the handler → 501"
/// probe needs a real UUID (an invalid one is a 400 at the adapter).
const EHR: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";

fn argon2_hash(pw: &str) -> String {
    let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").unwrap();
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn config(enabled: bool) -> RestConfig {
    RestConfig {
        smart: Default::default(),
        system: Default::default(),
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        cors_permissive: false,
        auth: AuthConfig {
            enabled,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: Redacted(argon2_hash("pw")),
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: Some(OidcConfig {
                issuer: ISSUER.to_owned(),
                audiences: vec![],
                algorithms: vec!["HS256".to_owned()],
                hmac_secret: Some(Redacted(SECRET.to_owned())),
                jwks_json: None,
            }),
            admin_scope: Some("ehrbase:admin".to_owned()),
        },
        // The admin group must be reachable here: `admin_route_reachable_without_rbac`
        // asserts the dispatcher's 501 (StubBackend), not the config gate's 404.
        admin: AdminConfig { enabled: true },
        terminology: ehrbase_rest::TerminologyConfig::default(),
        event_subscription: ehrbase_rest::EventSubscriptionConfig::default(),
        tenancy: ehrbase_rest::TenancyConfig::default(),
        fhir: ehrbase_rest::FhirConfig::default(),
    }
}

fn app(enabled: bool) -> Router {
    ehrbase_rest::build_with(config(enabled), Arc::new(common::Mock::new())).expect("router builds")
}

fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn bearer(scope: &str) -> String {
    let claims = serde_json::json!({
        "sub": "alice",
        "iss": ISSUER,
        "exp": now() + 3600,
        "scope": scope,
    });
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap();
    format!("Bearer {token}")
}

fn basic(user: &str, pw: &str) -> String {
    format!("Basic {}", b64(format!("{user}:{pw}").as_bytes()))
}

/// Minimal standard base64 encoder (avoids adding a base64 dev-dep).
fn b64(bytes: &[u8]) -> String {
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

async fn send(app: Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn unauthenticated_request_is_401_with_challenge() {
    let (status, headers, body) = send(app(true), get(&format!("{BASE}/ehr/abc"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(headers.contains_key(header::WWW_AUTHENTICATE));
    // JSON error body.
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"], "Unauthorized");
}

#[tokio::test]
async fn status_endpoint_is_public() {
    let (status, _h, body) = send(app(true), get("/ehrbase/rest/status")).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["status"], "UP");
    // The served identity is provenance-derived (the tested development
    // edition of ITS-REST), never a hand-asserted release label.
    assert_eq!(v["openehr_rest_api_version"], "development@e8a093e");
}

#[tokio::test]
async fn health_endpoint_is_public() {
    let (status, _h, body) = send(app(true), get("/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "OK");
}

#[tokio::test]
async fn valid_basic_reaches_handler_and_gets_not_implemented() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn wrong_basic_password_is_401() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/abc"))
        .header(header::AUTHORIZATION, basic("alice", "wrong"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_bearer_reaches_handler() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}"))
        .header(header::AUTHORIZATION, bearer("openid"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn admin_route_reachable_without_rbac() {
    // The legacy path-string `admin_scope` gate was removed (§5.2); this harness
    // builds without an RBAC handle, so an authenticated caller reaches the admin
    // dispatcher regardless of scope (→ 501, not gated). The role-based admin
    // gate is exercised end-to-end in `rbac_e2e`.
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/admin/ehr/abc"))
        .header(header::AUTHORIZATION, bearer("openid"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn json_composition_body_is_accepted_and_reaches_handler() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{EHR}/composition"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"_type":"COMPOSITION"}"#))
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    // Body decoded as JSON, handler reached → NotImplemented (not 400/415).
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn malformed_xml_composition_body_is_400() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{EHR}/composition"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from("<not-a-composition>"))
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    // The XML branch runs and fails to parse a COMPOSITION → BadRequest.
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unknown_route_is_404() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/nonexistent"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn auth_disabled_lets_requests_through() {
    let (status, _h, _b) = send(app(false), get(&format!("{BASE}/ehr/{EHR}"))).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

// The other four groups mount identically; smoke-test one representative route
// of each (unauthenticated → 401; authenticated → reaches the stub → 501).

#[tokio::test]
async fn query_group_is_mounted_and_authenticated() {
    let (status, _h, _b) = send(app(true), get(&format!("{BASE}/query/aql?q=SELECT%20c"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/query/aql?q=SELECT%20c"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn demographic_group_is_mounted() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/agent/abc"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn unimplemented_groups_answer_501_with_the_standard_error_body() {
    // F-13-03: the demographic / query / admin groups route through the generic
    // not-implemented dispatcher; the wire behaviour must be identical to the
    // old per-operation arms forwarding to a `NotImplemented` backend — 501 +
    // the standard `{ error, message }` JSON body.
    // The POST case carries a well-formed JSON body: body deserialization runs
    // before dispatch, so a malformed/empty body is (correctly) a 400 and would
    // never reach the not-implemented arm this test exercises.
    let cases = [
        (
            "GET",
            format!("{BASE}/demographic/agent/abc"),
            "openid",
            None,
        ),
        (
            "POST",
            format!("{BASE}/demographic/agent"),
            "openid",
            Some(r#"{"_type": "AGENT"}"#),
        ),
        (
            "GET",
            format!("{BASE}/query/aql?q=SELECT%20c"),
            "openid",
            None,
        ),
        (
            "DELETE",
            format!("{BASE}/admin/ehr/abc"),
            "ehrbase:admin",
            None,
        ),
    ];
    for (method, uri, scope, body) in cases {
        let mut builder = Request::builder()
            .method(method)
            .uri(&uri)
            .header(header::AUTHORIZATION, bearer(scope));
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        let req = builder
            .body(body.map_or_else(Body::empty, Body::from))
            .unwrap();
        let (status, headers, body) = send(app(true), req).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{method} {uri}");
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json",
            "{method} {uri}"
        );
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["error"], "Not Implemented", "{method} {uri}");
        assert_eq!(v["message"], "not implemented", "{method} {uri}");
    }
}

#[tokio::test]
async fn definition_group_is_mounted_with_dotted_route() {
    // Exercises the dotted path segment (`adl1.4`) and dotted operation id.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl1.4"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app(true), req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}
