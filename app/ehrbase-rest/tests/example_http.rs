//! End-to-end HTTP tests for the template-example endpoint
//! (`GET /definition/template/adl1.4/{template_id}/example`, a post-1.0.3
//! dev-OAS operation).
//!
//! A mock [`Backend`] generates the example COMPOSITION from the Demo Vitals
//! `WebTemplate` (the same generator the service uses) and the assembled router
//! is driven for each supported `Accept`:
//!
//! * default / `application/json` → canonical JSON COMPOSITION;
//! * `application/xml` → canonical XML;
//! * `application/openehr.wt.flat+json` / `…wt.structured+json` → the FLAT /
//!   STRUCTURED converters (reached through the shared `WebTemplateService`);
//! * an unsupported `Accept` → `406`;
//! * an unknown `template_id` → `404`;
//! * an invalid `detail_level` → `400`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::auth::config::AuthConfig;
use ehrbase_rest::{EhrService, RestConfig, WebTemplateService};
use openehr_flat::{DetailLevel, ExampleType};
use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionTemplateAdl14ExampleGetParams,
};
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const TEMPLATE_ID: &str = "Demo Vitals";
const JSON_MIME: &str = "application/json";
const XML_MIME: &str = "application/xml";
const FLAT_MIME: &str = "application/openehr.wt.flat+json";
const STRUCTURED_MIME: &str = "application/openehr.wt.structured+json";

fn flat_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/openehr-flat")
}

fn opt_xml() -> String {
    std::fs::read_to_string(flat_crate_dir().join("tests/fixtures/better/Demo Vitals.opt"))
        .expect("Demo Vitals.opt vendored in openehr-flat")
}

/// The Demo Vitals `WebTemplate` (built from the vendored OPT).
fn web_template() -> openehr_flat::WebTemplate {
    let opt = openehr_its::opt14::from_xml(&opt_xml()).expect("parse OPT");
    openehr_flat::build_web_template(&opt).expect("build web template")
}

/// A minimal backend that generates the example for the known template and 404s
/// for anything else — mirroring the service's `template_example` seam.
#[derive(Debug, Default)]
struct MockBackend;

#[async_trait]
impl DefinitionApi for MockBackend {
    async fn definition_template_adl1_4_example_get(
        &self,
        params: DefinitionTemplateAdl14ExampleGetParams,
    ) -> Result<Value, ApiError> {
        let level = DetailLevel::from_query(params.detail_level.as_deref())
            .map_err(ApiError::BadRequest)?;
        let kind =
            ExampleType::from_query(params.r#type.as_deref()).map_err(ApiError::BadRequest)?;
        if params.template_id != TEMPLATE_ID {
            return Err(ApiError::NotFound(format!(
                "template {} not found",
                params.template_id
            )));
        }
        let mut comp = openehr_flat::example_composition(&web_template(), level);
        if kind == ExampleType::Output {
            openehr_flat::apply_output_uid(&mut comp, &params.template_id);
        }
        Ok(comp)
    }
}

#[async_trait]
impl WebTemplateService for MockBackend {
    async fn web_template(
        &self,
        _template_id: &str,
    ) -> Result<Arc<openehr_flat::WebTemplate>, ApiError> {
        Ok(Arc::new(web_template()))
    }
}

impl EhrService for MockBackend {}
impl ehrbase_rest::EhrStatusService for MockBackend {}
impl ehrbase_rest::EhrCompositionService for MockBackend {}
impl ehrbase_rest::EhrDirectoryService for MockBackend {}
impl ehrbase_rest::EhrContributionService for MockBackend {}
impl ehrbase_rest::QueryService for MockBackend {}
impl ehrbase_rest::DemographicService for MockBackend {}
impl ehrbase_rest::AdminService for MockBackend {}
impl ehrbase_rest::DefinitionAdl14Service for MockBackend {}
impl ehrbase_rest::DefinitionQueryService for MockBackend {}

fn config() -> RestConfig {
    RestConfig {
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        cors_permissive: false,
        admin: ehrbase_rest::AdminConfig::default(),
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

async fn get(uri: &str, accept: Option<&str>) -> (StatusCode, Option<String>, String) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(a) = accept {
        builder = builder.header(header::ACCEPT, a);
    }
    let resp = app()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .expect("response");
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        content_type,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn example_uri() -> String {
    // The template id carries a space; percent-encode it in the path segment.
    let id = TEMPLATE_ID.replace(' ', "%20");
    format!("{BASE}/definition/template/adl1.4/{id}/example")
}

#[tokio::test]
async fn example_default_is_canonical_json() {
    let (status, content_type, body) = get(&example_uri(), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some(JSON_MIME));
    let comp: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        comp.get("_type").and_then(Value::as_str),
        Some("COMPOSITION")
    );
}

#[tokio::test]
async fn example_as_canonical_xml() {
    let (status, content_type, body) = get(&example_uri(), Some(XML_MIME)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some(XML_MIME));
    assert!(body.contains("<composition"), "canonical XML root: {body}");
}

#[tokio::test]
async fn example_as_flat() {
    let (status, content_type, body) = get(&example_uri(), Some(FLAT_MIME)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some(FLAT_MIME));
    let flat: serde_json::Map<String, Value> = serde_json::from_str(&body).unwrap();
    assert!(
        flat.contains_key("ctx/language"),
        "flat has ctx keys: {body}"
    );
}

#[tokio::test]
async fn example_as_structured() {
    let (status, content_type, _body) = get(&example_uri(), Some(STRUCTURED_MIME)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some(STRUCTURED_MIME));
}

#[tokio::test]
async fn example_unsupported_accept_is_406() {
    let (status, _content_type, _body) = get(&example_uri(), Some("application/pdf")).await;
    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn example_unknown_template_is_404() {
    let uri = format!("{BASE}/definition/template/adl1.4/nope.v0/example");
    let (status, _content_type, _body) = get(&uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn example_invalid_detail_level_is_400() {
    let uri = format!("{}?detail_level=exhaustive", example_uri());
    let (status, _content_type, _body) = get(&uri, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
