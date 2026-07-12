//! End-to-end HTTP tests for the DEMOGRAPHIC group wiring: the `demographic`
//! routes are now served through the [`DemographicService`] seam (no longer a
//! blanket `501`), with `ETag`/`Location`/`Prefer` and the deleted-read→`204`
//! and precondition→`412` behaviour mirroring the EHR group — driven through the
//! assembled router with a canned backend (no database).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_sm::SmError;
use ehrbase_sm::{ResourceMeta, ServiceResponse};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const PARTY_OVID: &str = "5f3a1c2e-1111-4222-8333-444455556666::ehrbase-rs.local::1";

fn person_body() -> Value {
    json!({
        "_type": "PERSON",
        "uid": { "_type": "OBJECT_VERSION_ID", "value": PARTY_OVID },
        "name": { "_type": "DV_TEXT", "value": "Jane Doe" },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "name": { "_type": "DV_TEXT", "value": "legal" }
        }]
    })
}

/// The demographic hooks: party create/get/update/delete + latest-meta, plus
/// the `PARTY_RELATIONSHIP` create/get, all on the SM-native `SmError`. The
/// dispatch wiring (`ETag`/`Location`/`Prefer`, deleted→`204`, stale→`412`)
/// is what is under test.
fn hooks() -> Hooks {
    Hooks {
        party_create: Some(Arc::new(|_kind, _body| {
            Ok(ServiceResponse::new(
                person_body(),
                ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
            ))
        })),
        party_get: Some(Arc::new(|_kind, uid_based_id: String, _at| {
            if uid_based_id == "deleted" {
                // A deleted current version → Null body → 204.
                return Ok(ServiceResponse::plain(Value::Null));
            }
            Ok(ServiceResponse::new(
                person_body(),
                ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
            ))
        })),
        party_update: Some(Arc::new(|_kind, _uid, if_match: String, _body| {
            if if_match.contains("stale") {
                return Err(SmError::version_mismatch("stale If-Match"));
            }
            Ok(ServiceResponse::new(
                person_body(),
                ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
            ))
        })),
        party_delete: Some(Arc::new(|_kind, _uid, _if_match| {
            Ok(ServiceResponse::deleted(ResourceMeta::new(
                String::new(),
                PARTY_OVID.to_owned(),
            )))
        })),
        demographic_latest_meta: Some(Arc::new(|_kind, _uid| {
            Ok(Some(ResourceMeta::new(
                String::new(),
                PARTY_OVID.to_owned(),
            )))
        })),
        party_relationship_create: Some(Arc::new(|_body| {
            Ok(ServiceResponse::new(
                relationship_body(),
                ResourceMeta::new(String::new(), REL_OVID.to_owned()),
            ))
        })),
        party_relationship_get: Some(Arc::new(|_uid, _at| {
            Ok(ServiceResponse::new(
                relationship_body(),
                ResourceMeta::new(String::new(), REL_OVID.to_owned()),
            ))
        })),
        ..Default::default()
    }
}

const REL_OVID: &str = "7a7a7a7a-1111-4222-8333-999999990000::ehrbase-rs.local::1";

fn relationship_body() -> Value {
    json!({
        "_type": "PARTY_RELATIONSHIP",
        "uid": { "_type": "OBJECT_VERSION_ID", "value": REL_OVID },
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PARTY_RELATIONSHIP.relationship.v1",
        "name": { "_type": "DV_TEXT", "value": "parent-of" },
        "source": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "src" } },
        "target": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "tgt" } }
    })
}

fn config() -> RestConfig {
    RestConfig {
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        cors_permissive: false,
        admin: ehrbase_rest::AdminConfig::default(),
        terminology: ehrbase_rest::TerminologyConfig::default(),
        event_subscription: ehrbase_rest::EventSubscriptionConfig::default(),
        tenancy: ehrbase_rest::TenancyConfig::default(),
        fhir: ehrbase_rest::FhirConfig::default(),
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
        },
    }
}

fn app() -> Router {
    ehrbase_rest::build_with(config(), Arc::new(Mock::with(hooks()))).expect("router builds")
}

async fn send(req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app().oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn etag(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::ETAG).and_then(|v| v.to_str().ok())
}

fn location(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::LOCATION).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn person_create_default_is_minimal_with_headers() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, body) = send(req).await;

    // 201 default (return=minimal): headers only, no body — and no longer 501.
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{PARTY_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{PARTY_OVID}").as_str())
    );
    assert!(body.is_empty(), "minimal create has no body, got {body:?}");
}

#[tokio::test]
async fn person_create_representation_returns_body() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{PARTY_OVID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PERSON");
}

#[tokio::test]
async fn person_get_sets_etag_and_location() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/person/some-uid"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{PARTY_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{PARTY_OVID}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PERSON");
}

#[tokio::test]
async fn deleted_person_read_is_204() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/person/deleted"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(req).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

#[tokio::test]
async fn person_delete_is_204_with_headers() {
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/demographic/person/{PARTY_OVID}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(etag(&h), Some(format!("\"{PARTY_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{PARTY_OVID}").as_str())
    );
    assert!(body.is_empty());
}

/// The versioned-object-uid delete shape (ECC-DEM-005 family): the path is the
/// bare `HIER_OBJECT_ID` and the preceding version is carried by `If-Match`.
/// The dispatcher must accept it and forward `If-Match` — a `204`, not a `400`.
#[tokio::test]
async fn person_delete_by_versioned_uid_with_if_match_is_204() {
    let vo = PARTY_OVID.split("::").next().unwrap();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/demographic/person/{vo}"))
        .header(header::IF_MATCH, format!("\"{PARTY_OVID}\""))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(req).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

#[tokio::test]
async fn stale_update_is_412_with_latest_headers() {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/demographic/person/{PARTY_OVID}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"stale::sys::1\"")
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, _body) = send(req).await;

    // Precondition failure → 412, decorated with the latest version headers
    // (mirrors the EHR group's ehr_status/composition update path).
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(etag(&h), Some(format!("\"{PARTY_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{PARTY_OVID}").as_str())
    );
}

#[tokio::test]
async fn role_create_uses_role_segment() {
    // The 5× kind fan-out routes each kind to its own segment.
    let body = json!({
        "_type": "ROLE",
        "name": { "_type": "DV_TEXT", "value": "clinician" },
        "identities": [{ "_type": "PARTY_IDENTITY", "name": { "_type": "DV_TEXT", "value": "r" } }],
        "performer": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                       "id": { "_type": "HIER_OBJECT_ID", "value": "x" } }
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/role"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, h, _body) = send(req).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/role/{PARTY_OVID}").as_str())
    );
}

#[tokio::test]
async fn party_relationship_create_is_mounted_with_headers() {
    // The our-own-design PARTY_RELATIONSHIP extension route is mounted and
    // reaches the seam (a create returns 201 + ETag/Location on the
    // /demographic/party_relationship segment; an unmounted route would 404).
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/party_relationship"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(relationship_body().to_string()))
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{REL_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/party_relationship/{REL_OVID}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PARTY_RELATIONSHIP");
}

#[tokio::test]
async fn party_relationship_get_is_mounted() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/party_relationship/some-uid"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{REL_OVID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PARTY_RELATIONSHIP");
}

#[tokio::test]
async fn versioned_party_relationship_is_mounted() {
    // Default seam (NotImplemented → 501) still proves the route is mounted
    // (an unmounted path would 404).
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/demographic/versioned_party_relationship/some-uid"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _body) = send(req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}
