//! End-to-end HTTP tests for the FLAT (simSDT) COMPOSITION endpoints.
//!
//! A mock [`Backend`] serves the Demo Vitals OPT (for the `WebTemplate`) and
//! stores/echoes compositions, so the FLAT create + get glue in `dispatch::flat`
//! is exercised through the assembled router:
//!
//! * GET with `Accept: application/openehr.wt.flat+json` → the stored canonical
//!   composition is returned as a flat map;
//! * POST with `Content-Type: application/openehr.wt.flat+json` + `?template_id`
//!   → the flat body is rebuilt into a canonical composition before the service
//!   sees it (asserted via the captured create body);
//! * POST flat without a template id → 400;
//! * a full flat → RM → flat round-trip through the two endpoints is stable.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::auth::config::AuthConfig;
use ehrbase_rest::{EhrService, RestConfig, ServiceResponse, WebTemplateService};
use openehr_its::rest::generated::definition::{DefinitionApi, DefinitionTemplateAdl14GetParams};
use openehr_its::rest::generated::ehr::{CompositionCreateParams, CompositionGetParams};
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const FLAT_MIME: &str = "application/openehr.wt.flat+json";

fn flat_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openehr-flat")
}

fn opt_xml() -> String {
    std::fs::read_to_string(flat_crate_dir().join("tests/fixtures/better/Demo Vitals.opt"))
        .expect("Demo Vitals.opt vendored in openehr-flat")
}

fn canonical_composition() -> Value {
    let text = std::fs::read_to_string(flat_crate_dir().join(
        "../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/demo_vitals_352.json",
    ))
    .expect("demo_vitals_352.json vendored in openehr-its");
    serde_json::from_str(&text).expect("valid canonical composition")
}

/// A minimal backend: serves the Demo Vitals OPT, and stores/echoes the
/// composition it is asked to create.
#[derive(Debug, Default)]
struct MockBackend {
    stored: Mutex<Option<Value>>,
    created_body: Mutex<Option<Value>>,
}

#[async_trait]
impl EhrService for MockBackend {
    async fn composition_create(
        &self,
        _params: CompositionCreateParams,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        *self.created_body.lock().unwrap() = Some(body.clone());
        *self.stored.lock().unwrap() = Some(body.clone());
        Ok(ServiceResponse::plain(body))
    }

    async fn composition_get(
        &self,
        _params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        self.stored
            .lock()
            .unwrap()
            .clone()
            .map(ServiceResponse::plain)
            .ok_or_else(|| ApiError::NotFound("composition not found".to_owned()))
    }
}

#[async_trait]
impl DefinitionApi for MockBackend {
    async fn definition_template_adl1_4_get(
        &self,
        _params: DefinitionTemplateAdl14GetParams,
    ) -> Result<Value, ApiError> {
        Ok(Value::String(opt_xml()))
    }
}

// The single WebTemplate resolution seam (W2-K): the mock serves the Demo
// Vitals WebTemplate the way the service would (built once, shared).
#[async_trait]
impl WebTemplateService for MockBackend {
    async fn web_template(
        &self,
        _template_id: &str,
    ) -> Result<Arc<openehr_flat::WebTemplate>, ApiError> {
        Ok(Arc::new(web_template()))
    }
}

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

fn app(backend: Arc<MockBackend>) -> Router {
    ehrbase_rest::build_with(config(), backend).expect("router builds")
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

/// The Demo Vitals `WebTemplate` (built from the vendored OPT).
fn web_template() -> openehr_flat::WebTemplate {
    let opt = openehr_its::opt14::from_xml(&opt_xml()).expect("parse OPT");
    openehr_flat::build_web_template(&opt).expect("build web template")
}

#[tokio::test]
async fn get_composition_as_flat() {
    let backend = Arc::new(MockBackend::default());
    *backend.stored.lock().unwrap() = Some(canonical_composition());

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/e1/composition/c1"))
        .header(header::ACCEPT, FLAT_MIME)
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(app(backend), req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), FLAT_MIME);
    let flat: serde_json::Map<String, Value> = serde_json::from_str(&body).unwrap();
    assert!(flat.contains_key("ctx/language"), "flat has ctx keys");
    assert!(
        flat.keys().any(|k| k.ends_with("|magnitude")),
        "flat has a |magnitude leaf: {:?}",
        flat.keys().collect::<Vec<_>>()
    );
    assert!(
        !flat.keys().any(|k| k.ends_with("|units")),
        "|unit is singular"
    );
}

#[tokio::test]
async fn post_flat_composition_is_rebuilt_to_canonical() {
    let backend = Arc::new(MockBackend::default());

    // Derive a real flat body from the canonical composition + its template.
    let wt = web_template();
    let flat = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_map: serde_json::Map<String, Value> = flat.into_iter().collect();
    let flat_body = serde_json::to_string(&flat_map).unwrap();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "{BASE}/ehr/e1/composition?template_id=Demo%20Vitals"
        ))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from(flat_body))
        .unwrap();
    let (status, _h, _b) = send(app(backend.clone()), req).await;

    assert_eq!(status, StatusCode::CREATED);
    // The service received a canonical COMPOSITION, not the flat map.
    let created = backend.created_body.lock().unwrap().clone().unwrap();
    assert_eq!(
        created.get("_type").and_then(Value::as_str),
        Some("COMPOSITION")
    );
    assert!(created.get("content").is_some(), "rebuilt content present");
    assert!(
        created.pointer("/context/start_time/value").is_some(),
        "rebuilt context from ctx/"
    );
}

#[tokio::test]
async fn post_flat_without_template_id_is_400() {
    let backend = Arc::new(MockBackend::default());
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/e1/composition"))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from("{\"ctx/language\":\"en\"}"))
        .unwrap();
    let (status, _h, _b) = send(app(backend), req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn flat_round_trips_through_http() {
    let backend = Arc::new(MockBackend::default());
    let wt = web_template();
    let flat_in = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_in_map: serde_json::Map<String, Value> = flat_in.clone().into_iter().collect();

    // POST the flat body → the mock stores the rebuilt canonical composition.
    let post = Request::builder()
        .method("POST")
        .uri(format!(
            "{BASE}/ehr/e1/composition?templateId=Demo%20Vitals"
        ))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from(serde_json::to_string(&flat_in_map).unwrap()))
        .unwrap();
    let (status, _h, _b) = send(app(backend.clone()), post).await;
    assert_eq!(status, StatusCode::CREATED);

    // GET it back as flat.
    let get = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/e1/composition/c1"))
        .header(header::ACCEPT, FLAT_MIME)
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app(backend), get).await;
    assert_eq!(status, StatusCode::OK);
    let flat_out: std::collections::BTreeMap<String, Value> = serde_json::from_str(&body).unwrap();

    let flat_in_sorted: std::collections::BTreeMap<String, Value> = flat_in.into_iter().collect();
    assert_eq!(
        flat_in_sorted, flat_out,
        "flat → RM → flat stable through the HTTP endpoints"
    );
}
