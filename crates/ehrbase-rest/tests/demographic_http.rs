//! End-to-end HTTP tests for the DEMOGRAPHIC group wiring: the `demographic`
//! routes are now served through the [`DemographicService`] seam (no longer a
//! blanket `501`), with `ETag`/`Location`/`Prefer` and the deleted-read→`204`
//! and precondition→`412` behaviour mirroring the EHR group — driven through the
//! assembled router with a canned backend (no database).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use ehrbase_rest::auth::config::AuthConfig;
use ehrbase_rest::backend::{DemographicService, PartyKind};
use ehrbase_rest::{EhrService, ResourceMeta, RestConfig, ServiceResponse, WebTemplateService};
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const PARTY_OVID: &str = "5f3a1c2e-1111-4222-8333-444455556666::ehrbase-rs.local::1";

/// A canned backend exercising the demographic dispatch wiring without a DB.
#[derive(Debug, Default)]
struct MockBackend;

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

#[async_trait]
impl DemographicService for MockBackend {
    async fn party_create(
        &self,
        _kind: PartyKind,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::new(
            person_body(),
            ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
        ))
    }

    async fn party_get(
        &self,
        _kind: PartyKind,
        uid_based_id: String,
        _version_at_time: Option<String>,
    ) -> Result<ServiceResponse, ApiError> {
        if uid_based_id == "deleted" {
            // A deleted current version → Null body → 204.
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(ServiceResponse::new(
            person_body(),
            ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
        ))
    }

    async fn party_update(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
        if_match: String,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        if if_match.contains("stale") {
            return Err(ApiError::PreconditionFailed("stale If-Match".to_owned()));
        }
        Ok(ServiceResponse::new(
            person_body(),
            ResourceMeta::new(String::new(), PARTY_OVID.to_owned()),
        ))
    }

    async fn party_delete(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::deleted(ResourceMeta::new(
            String::new(),
            PARTY_OVID.to_owned(),
        )))
    }

    async fn demographic_latest_meta(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(Some(ResourceMeta::new(
            String::new(),
            PARTY_OVID.to_owned(),
        )))
    }
}

impl EhrService for MockBackend {}
impl DefinitionApi for MockBackend {}
impl WebTemplateService for MockBackend {}
impl ehrbase_rest::QueryService for MockBackend {}

fn config() -> RestConfig {
    RestConfig {
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        cors_permissive: false,
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
        },
    }
}

fn app() -> Router {
    ehrbase_rest::build_with(config(), Arc::new(MockBackend)).expect("router builds")
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
