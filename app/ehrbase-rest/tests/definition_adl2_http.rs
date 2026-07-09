//! End-to-end HTTP tests for the ADL2 template wire (SM-2, `I_DEFINITION_ADL2`):
//! `POST /definition/template/adl2` (text/plain source upload, `Location` +
//! `Prefer` body), `GET /definition/template/adl2/{template_id}` (text/plain
//! source, 404 on unknown), and `GET /definition/template/adl2` (JSON list).
//! Driven through the assembled router with a canned backend — the dispatcher
//! wiring (`dispatch::definition`) is what is under test, not the DB.
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
use ehrbase_rest::{DefinitionAdl2Service, RestConfig};
use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionTemplateAdl2ListParams, DefinitionTemplateAdl2UploadParams,
};
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const HRID: &str = "openEHR-EHR-COMPOSITION.t_clinical_info.v1.0.0";
const SOURCE: &str = "operational_template (adl_version=2.0.6)\n\
    openEHR-EHR-COMPOSITION.t_clinical_info.v1.0.0\n";

/// A canned backend for the ADL2 wire: upload echoes the stored HRID, list
/// returns one metadata object, `get_artefact` serves the source (or 404).
#[derive(Debug, Default)]
struct MockBackend;

#[async_trait]
impl DefinitionApi for MockBackend {
    async fn definition_template_adl2_upload(
        &self,
        _params: DefinitionTemplateAdl2UploadParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        // The dispatcher hands the text/plain source through as a JSON string.
        assert_eq!(body.as_str(), Some(SOURCE));
        Ok(Value::String(HRID.to_owned()))
    }

    async fn definition_template_adl2_list(
        &self,
        _params: DefinitionTemplateAdl2ListParams,
    ) -> Result<Vec<Value>, ApiError> {
        Ok(vec![
            json!({ "template_id": HRID, "created_timestamp": "2017-08-14T19:24:56.639Z" }),
        ])
    }
}

#[async_trait]
impl DefinitionAdl2Service for MockBackend {
    async fn get_artefact(&self, an_id: String) -> Result<String, ApiError> {
        if an_id == HRID {
            Ok(SOURCE.to_owned())
        } else {
            Err(ApiError::NotFound(format!("ADL2 artefact {an_id}")))
        }
    }
}

impl ehrbase_rest::EhrService for MockBackend {}
impl ehrbase_rest::EhrStatusService for MockBackend {}
impl ehrbase_rest::EhrCompositionService for MockBackend {}
impl ehrbase_rest::EhrDirectoryService for MockBackend {}
impl ehrbase_rest::EhrContributionService for MockBackend {}
impl ehrbase_rest::WebTemplateService for MockBackend {}
impl ehrbase_rest::QueryService for MockBackend {}
impl ehrbase_rest::DemographicService for MockBackend {}
impl ehrbase_rest::AdminService for MockBackend {}
impl ehrbase_rest::AdminArchive for MockBackend {}
impl ehrbase_rest::TerminologyService for MockBackend {}
impl ehrbase_rest::DefinitionAdl14Service for MockBackend {}
impl ehrbase_rest::DefinitionQueryService for MockBackend {}
impl ehrbase_rest::PartyRelationshipService for MockBackend {}
impl ehrbase_rest::EhrIndexService for MockBackend {}

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
        admin: ehrbase_rest::AdminConfig::default(),
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

fn upload(prefer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/definition/template/adl2"))
        .header(header::CONTENT_TYPE, "text/plain");
    if let Some(p) = prefer {
        b = b.header("Prefer", p);
    }
    b.body(Body::from(SOURCE)).unwrap()
}

#[tokio::test]
async fn upload_minimal_returns_201_and_location_only() {
    let (status, headers, body) = send(upload(None)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        headers.get(header::LOCATION).unwrap().to_str().unwrap(),
        format!("{BASE}/definition/template/adl2/{HRID}")
    );
    assert!(
        body.is_empty(),
        "return=minimal has an empty body: {body:?}"
    );
}

#[tokio::test]
async fn upload_representation_returns_source_text() {
    let (status, headers, body) = send(upload(Some("return=representation"))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    assert!(headers.contains_key(header::LOCATION));
    assert_eq!(body, SOURCE, "representation echoes the OPT source");
}

#[tokio::test]
async fn upload_identifier_returns_template_id_json() {
    let (status, headers, body) = send(upload(Some("return=identifier"))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(headers.contains_key(header::LOCATION));
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v.get("template_id").and_then(Value::as_str), Some(HRID));
}

#[tokio::test]
async fn get_serves_source_as_text_and_404s_unknown() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2/{HRID}"))
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    assert_eq!(body, SOURCE);

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/definition/template/adl2/openEHR-EHR-COMPOSITION.absent.v1.0.0"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_returns_template_metadata() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2"))
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    let list = v.as_array().expect("array");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].get("template_id").and_then(Value::as_str),
        Some(HRID)
    );
}
