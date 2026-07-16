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

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{AppConfig, ServerConfig};
use ehrbase::service::{CallStatusType, SmError};
use openehr_flat::{DetailLevel, ExampleType};

mod common;
use common::{Hooks, Mock};

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
fn hooks() -> Hooks {
    Hooks {
        // The example generator (wire-shaped DefinitionAdapter → `SmError`; the
        // dispatcher passes the raw `detail_level`/`type` query values).
        template_adl14_example: Some(Arc::new(
            |template_id: String, detail_level: Option<String>, kind: Option<String>| {
                let level = DetailLevel::from_query(detail_level.as_deref())
                    .map_err(SmError::precondition)?;
                let kind =
                    ExampleType::from_query(kind.as_deref()).map_err(SmError::precondition)?;
                if template_id != TEMPLATE_ID {
                    return Err(SmError::new(
                        CallStatusType::VersionedObjectDoesNotExist,
                        format!("template {template_id} not found"),
                    ));
                }
                let mut comp = openehr_flat::example_composition(&web_template(), level);
                if kind == ExampleType::Output {
                    openehr_flat::apply_output_uid(&mut comp, &template_id);
                }
                Ok(comp)
            },
        )),
        // The shared WebTemplate resolution seam (SM-native → `SmError`).
        web_template: Some(Arc::new(|_id| Ok(Arc::new(web_template())))),
        ..Default::default()
    }
}

fn config() -> AppConfig {
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
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
            ..AuthConfig::default()
        },
        ..Default::default()
    }
}

fn app() -> Router {
    ehrbase_rest::build_with(config(), Arc::new(Mock::with(hooks()))).expect("router builds")
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
