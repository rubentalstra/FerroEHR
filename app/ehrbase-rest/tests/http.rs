//! End-to-end HTTP tests: routing, authentication (401/403), and content
//! negotiation, exercised through the assembled router over a **real**
//! `EhrbaseService` on a real `PostgreSQL` (the `common` fixture).
//!
//! All API groups are mounted; these tests exercise routing, auth, and
//! negotiation across representative operations. The former `Mock` backend's
//! blanket `501 Not Implemented` default is gone — every group is now a real
//! implementation, so the old "reaches the stub → 501" probes are re-targeted
//! to the real server's behaviour for a fresh, empty database (a missing EHR is
//! a `404`, an invalid AQL a `400`, an empty template list a `200 []`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use tower::ServiceExt;

use ehrbase::config::auth::{AuthConfig, BasicConfig, BasicUser, OidcConfig};
use ehrbase::config::server::{AdminConfig, ServerConfig};
use ehrbase_rest::config::AppConfig;

mod common;

const ISSUER: &str = "https://issuer.test";
const SECRET: &str = "integration-secret";
const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A syntactically valid EHR id that does not exist: the EHR dispatcher decodes
/// `ehr_id` into a `Uuid` and consults the backend, so a "reaches the handler"
/// probe needs a real UUID (an invalid one is a 400 at the adapter). A fresh
/// database has no such EHR → the real handler answers `404`.
const EHR: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";

fn argon2_hash(pw: &str) -> String {
    let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").unwrap();
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn config(enabled: bool) -> AppConfig {
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
            enabled,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: ehrbase::config::secret::Secret::new(argon2_hash("pw")),
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: Some(OidcConfig {
                issuer: ISSUER.to_owned(),
                audiences: vec![],
                algorithms: vec!["HS256".to_owned()],
                hmac_secret: Some(ehrbase::config::secret::Secret::new(SECRET.to_owned())),
                jwks_json: None,
                ..OidcConfig::default()
            }),
            admin_scope: Some("ehrbase:admin".to_owned()),
            ..AuthConfig::default()
        },
        // The admin group must be reachable here: `admin_route_reachable_without_rbac`
        // exercises the real admin delete (not the config gate's 404).
        admin: AdminConfig { enabled: true },
        ..Default::default()
    }
}

/// Build the router over a fresh real service (unique database per test).
async fn app(enabled: bool, db: &str) -> (common::Pg, Router) {
    let (pg, service) = common::test_service(db).await;
    (pg, common::router_with(config(enabled), service))
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
    let (_pg, app) = app(true, "http_unauth_401").await;
    let (status, headers, body) = send(app, get(&format!("{BASE}/ehr/abc"))).await;
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
    let (_pg, app) = app(true, "http_status_public").await;
    let (status, _h, body) = send(app, get("/ehrbase/rest/status")).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["status"], "UP");
    // The served identity is provenance-derived (the tested development
    // edition of ITS-REST), never a hand-asserted release label.
    assert_eq!(v["openehr_rest_api_version"], "development@e8a093e");
}

#[tokio::test]
async fn health_endpoint_is_public() {
    let (_pg, app) = app(true, "http_health_public").await;
    let (status, _h, body) = send(app, get("/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "OK");
}

#[tokio::test]
async fn valid_basic_reaches_handler() {
    // RE-TARGET: the old Mock backend answered a blanket `501`; the real EHR
    // service answers `404` for a syntactically valid but non-existent EHR —
    // which still proves the authenticated request reached the real handler.
    let (_pg, app) = app(true, "http_basic_reaches").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn wrong_basic_password_is_401() {
    let (_pg, app) = app(true, "http_wrong_basic_401").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/abc"))
        .header(header::AUTHORIZATION, basic("alice", "wrong"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_bearer_reaches_handler() {
    // RE-TARGET: was a Mock `501`; the real handler answers `404` for the
    // non-existent EHR (the bearer token reached the handler).
    let (_pg, app) = app(true, "http_bearer_reaches").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}"))
        .header(header::AUTHORIZATION, bearer("openid"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_route_reachable_without_rbac() {
    // The legacy path-string `admin_scope` gate was removed; this harness builds
    // without an RBAC handle, so an authenticated caller reaches the admin
    // dispatcher regardless of scope. RE-TARGET: was a Mock `501`; the real
    // admin delete physically removes a real EHR → `204`, proving the route is
    // reachable and not RBAC-gated. The role-based admin gate is exercised
    // end-to-end in `rbac_e2e`.
    let (_pg, app) = app(true, "http_admin_reachable").await;

    // Create a real EHR (authenticated), then physically delete it via admin.
    let create = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, headers, _b) = send(app.clone(), create).await;
    assert_eq!(status, StatusCode::CREATED);
    let ehr_id = etag_uid(&headers).expect("ETag carries the new ehr_id");

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/admin/ehr/{ehr_id}"))
        .header(header::AUTHORIZATION, bearer("openid"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// Extract the bare uid carried in a weak `ETag` (`W/"{uid}"`).
fn etag_uid(h: &header::HeaderMap) -> Option<String> {
    let raw = h.get(header::ETAG)?.to_str().ok()?;
    Some(raw.trim_start_matches("W/").trim_matches('"').to_owned())
}

#[tokio::test]
async fn json_composition_body_is_accepted_and_reaches_handler() {
    // RE-TARGET: was a Mock `501`. The JSON body is decoded and the handler
    // reached; the EHR does not exist so the real service answers `404` (not a
    // 400/415 negotiation error — which is what "reaches the handler" asserts).
    let (_pg, app) = app(true, "http_json_comp_reaches").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{EHR}/composition"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"_type":"COMPOSITION"}"#))
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_xml_composition_body_is_400() {
    let (_pg, app) = app(true, "http_bad_xml_400").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{EHR}/composition"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from("<not-a-composition>"))
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    // The XML branch runs and fails to parse a COMPOSITION → BadRequest (before
    // the backend, so it is independent of whether the EHR exists).
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unknown_route_is_404() {
    let (_pg, app) = app(true, "http_unknown_route_404").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/nonexistent"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn auth_disabled_lets_requests_through() {
    // RE-TARGET: was a Mock `501`; with auth disabled the request reaches the
    // real handler, which answers `404` for the non-existent EHR.
    let (_pg, app) = app(false, "http_auth_disabled").await;
    let (status, _h, _b) = send(app, get(&format!("{BASE}/ehr/{EHR}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// The other groups mount identically; smoke-test one representative route of
// each (unauthenticated → 401; authenticated → reaches the real handler).

#[tokio::test]
async fn query_group_is_mounted_and_authenticated() {
    let (_pg, app) = app(true, "http_query_mounted").await;
    let (status, _h, _b) = send(app.clone(), get(&format!("{BASE}/query/aql?q=SELECT%20c"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/query/aql?q=SELECT%20c"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    // RE-TARGET: was a Mock `501`. The real query engine parses the AQL; the
    // incomplete `SELECT c` fails to parse → `400` (PreconditionViolation),
    // which still proves the authenticated request reached the real handler.
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn demographic_group_is_mounted() {
    // RE-TARGET: was a Mock `501`; the real demographic service answers `404`
    // for a non-existent agent (the request reached the real handler).
    let (_pg, app) = app(true, "http_demographic_mounted").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/agent/{EHR}"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn implemented_groups_are_reachable() {
    // RE-TARGET: this test previously asserted a uniform `501 Not Implemented`
    // from the Mock's not-implemented default for the demographic / query /
    // admin groups. Those groups are all real implementations now, so the probe
    // is re-targeted to the real per-endpoint behaviour on a fresh database:
    // a missing agent → 404, an invalid AQL → 400, a missing admin EHR → 404.
    // The POST case carries a well-formed JSON body so body deserialization
    // (which runs before dispatch) does not short-circuit to a 400.
    let cases: [(&str, String, &str, Option<&str>, StatusCode); 3] = [
        (
            "GET",
            format!("{BASE}/demographic/agent/{EHR}"),
            "openid",
            None,
            StatusCode::NOT_FOUND,
        ),
        (
            "GET",
            format!("{BASE}/query/aql?q=SELECT%20c"),
            "openid",
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            "DELETE",
            format!("{BASE}/admin/ehr/{EHR}"),
            "ehrbase:admin",
            None,
            StatusCode::NOT_FOUND,
        ),
    ];
    let (_pg, app) = app(true, "http_groups_reachable").await;
    for (method, uri, scope, body, expected) in cases {
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
        let (status, _headers, _body) = send(app.clone(), req).await;
        assert_eq!(status, expected, "{method} {uri}");
        // None of these reach the retired not-implemented default.
        assert_ne!(status, StatusCode::NOT_IMPLEMENTED, "{method} {uri}");
    }
}

#[tokio::test]
async fn definition_group_is_mounted_with_dotted_route() {
    // Exercises the dotted path segment (`adl1.4`) and dotted operation id.
    // RE-TARGET: was a Mock `501`; the real definition service lists the stored
    // OPT 1.4 templates → `200` with an empty JSON array on a fresh database.
    let (_pg, app) = app(true, "http_definition_dotted").await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl1.4"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v.as_array().map(Vec::len), Some(0), "no templates uploaded");
}

/// A default-committer audit (a write whose request carries NO committal
/// headers) is attributed to the AUTHENTICATED principal, not the system
/// identity (`AUDIT_DETAILS.committer` 1..1 — RM common master04 §Audit
/// Details; the committal-header merge only overrides what the caller
/// supplies).
#[tokio::test]
async fn authenticated_write_attributes_the_default_committer() {
    let (_pg, app) = app(true, "http_committer_attribution").await;

    // Create an EHR as `alice` (Basic), no committal headers.
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let ehr_id = resp
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .trim_matches(['W', '/', '"'])
        .to_owned();

    // The EHR_STATUS's stored commit audit names the principal.
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{ehr_id}/versioned_ehr_status/revision_history"
        ))
        .header(header::AUTHORIZATION, basic("alice", "pw"))
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let history: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let audit = &history["items"][0]["audits"][0];
    assert_eq!(
        audit["committer"]["name"], "alice",
        "default committer is the authenticated principal: {history}"
    );
    assert_eq!(
        audit["committer"]["identifiers"][0]["type"], "basic",
        "identifier records the mechanism"
    );
}
