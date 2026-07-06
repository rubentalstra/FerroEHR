//! End-to-end HTTP tests for the W2-A response-header + `Prefer` handling:
//! `ETag`/`Location` on the EHR / `EHR_STATUS` / COMPOSITION writes and reads, and
//! the `return=minimal` (default, header-only) vs `return=representation`
//! (full body) `Prefer` policy — driven through the assembled router with a
//! canned [`EhrService`] backend.
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
use ehrbase_rest::{EhrService, ResourceMeta, RestConfig, ServiceResponse};
use openehr_its::rest::generated::admin::AdminApi;
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::generated::demographic::DemographicApi;
use openehr_its::rest::generated::ehr::{
    CompositionDeleteParams, CompositionGetParams, EhrCreateParams, EhrStatusUpdateParams,
};
use openehr_its::rest::generated::query::QueryApi;
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const EHR_ID: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const STATUS_OVID: &str = "6cb19121-4307-4648-9da0-d62e4d51f19b::openEHRSys::2";
const COMP_OVID: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys::3";

/// A canned backend that echoes fixed resources + metadata so the header/`Prefer`
/// wiring in `dispatch::ehr` is exercised without a database.
#[derive(Debug, Default)]
struct MockBackend;

#[async_trait]
impl EhrService for MockBackend {
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let body =
            json!({ "_type": "EHR", "ehr_id": { "_type": "HIER_OBJECT_ID", "value": EHR_ID } });
        // 201_EHR: ETag/Location keyed by the ehr_id.
        Ok(ServiceResponse::new(
            body,
            ResourceMeta::new(EHR_ID.to_owned(), EHR_ID.to_owned()),
        ))
    }

    async fn ehr_status_update(
        &self,
        _params: EhrStatusUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let body = json!({
            "_type": "EHR_STATUS",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": STATUS_OVID },
            "subject": { "_type": "PARTY_SELF" }
        });
        Ok(ServiceResponse::new(
            body,
            ResourceMeta::new(EHR_ID.to_owned(), STATUS_OVID.to_owned()),
        ))
    }

    async fn composition_get(
        &self,
        _params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let body = json!({
            "_type": "COMPOSITION",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": COMP_OVID },
            "name": { "_type": "DV_TEXT", "value": "Encounter" }
        });
        Ok(ServiceResponse::new(
            body,
            ResourceMeta::new(EHR_ID.to_owned(), COMP_OVID.to_owned()),
        ))
    }

    async fn composition_delete(
        &self,
        _params: CompositionDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::deleted(ResourceMeta::new(
            EHR_ID.to_owned(),
            COMP_OVID.to_owned(),
        )))
    }
}

impl DemographicApi for MockBackend {}
impl DefinitionApi for MockBackend {}
impl QueryApi for MockBackend {}
impl AdminApi for MockBackend {}

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
async fn ehr_create_default_is_minimal_with_headers() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 201_EHR default (return=minimal): headers only, no body.
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{EHR_ID}\"").as_str()));
    assert_eq!(location(&h), Some(format!("{BASE}/ehr/{EHR_ID}").as_str()));
    assert!(body.is_empty(), "minimal create has no body, got {body:?}");
}

#[tokio::test]
async fn ehr_create_representation_returns_body() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header("Prefer", "return=representation")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{EHR_ID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR");
}

#[tokio::test]
async fn ehr_status_update_default_is_204_with_headers() {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"prev::v::1\"")
        .body(Body::from(
            r#"{"_type":"EHR_STATUS","subject":{"_type":"PARTY_SELF"}}"#,
        ))
        .unwrap();
    let (status, h, body) = send(req).await;

    // 204_EHR_STATUS (default minimal): no body, ETag + Location.
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(etag(&h), Some(format!("\"{STATUS_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/ehr_status/{STATUS_OVID}").as_str())
    );
    assert!(body.is_empty());
}

#[tokio::test]
async fn ehr_status_update_representation_is_200_with_body() {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"prev::v::1\"")
        .header("Prefer", "return=representation")
        .body(Body::from(
            r#"{"_type":"EHR_STATUS","subject":{"_type":"PARTY_SELF"}}"#,
        ))
        .unwrap();
    let (status, h, body) = send(req).await;

    // 200_EHR_STATUS_updated (representation): body present.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{STATUS_OVID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR_STATUS");
}

#[tokio::test]
async fn composition_get_sets_etag_and_location() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/some-uid"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 200_COMPOSITION_retrieved: ETag(version_uid) + Location.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{COMP_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "COMPOSITION");
}

#[tokio::test]
async fn composition_delete_is_204_with_headers() {
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 204_COMPOSITION_deleted: ETag + Location of the deleted version.
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(etag(&h), Some(format!("\"{COMP_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}").as_str())
    );
    assert!(body.is_empty());
}
