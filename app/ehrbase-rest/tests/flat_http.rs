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

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_sm::{CallStatusType, SmError};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const FLAT_MIME: &str = "application/openehr.wt.flat+json";
/// Valid EHR + COMPOSITION ids: the EHR dispatcher decodes `ehr_id` and
/// `uid_based_id` before the read, so the routes need real UUIDs.
const EHR: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
const COMP_VO: &str = "8849182c-82ad-4088-a07f-48ead4180515";

fn flat_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/openehr-flat")
}

fn opt_xml() -> String {
    std::fs::read_to_string(flat_crate_dir().join("tests/fixtures/better/Demo Vitals.opt"))
        .expect("Demo Vitals.opt vendored in openehr-flat")
}

fn canonical_composition() -> Value {
    let text = std::fs::read_to_string(flat_crate_dir().join(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/demo_vitals_352.json",
    ))
    .expect("demo_vitals_352.json vendored in openehr-its");
    serde_json::from_str(&text).expect("valid canonical composition")
}

/// The stored/created composition, shared between the hooks and the test for
/// assertions. `create_composition` stores the canonical body it was handed
/// (the SM catalog is fed `uv.data`, the composition the dispatcher rebuilt
/// from FLAT); `get_composition_latest` echoes it; the OPT + `WebTemplate` seams
/// serve the Demo Vitals template.
#[derive(Clone, Default)]
struct Store {
    stored: Arc<Mutex<Option<Value>>>,
    created_body: Arc<Mutex<Option<Value>>>,
}

fn config() -> RestConfig {
    RestConfig {
        smart: ehrbase_rest::SmartConfig::default(),
        system: ehrbase_rest::SystemOptionsConfig::default(),
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        max_in_flight: 1024,
        swagger_ui: false,
        cors_permissive: false,
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
        },
        admin: ehrbase_rest::AdminConfig::default(),
        terminology: ehrbase_rest::TerminologyConfig::default(),
        event_subscription: ehrbase_rest::EventSubscriptionConfig::default(),
        tenancy: ehrbase_rest::TenancyConfig::default(),
        fhir: ehrbase_rest::FhirConfig::default(),
    }
}

fn app(store: &Store) -> Router {
    let s_create = store.clone();
    let s_get = store.stored.clone();
    let hooks = Hooks {
        // The FLAT/STRUCTURED create passes the dispatcher-rebuilt canonical
        // COMPOSITION as `uv.data`; capture it (create + stored), return a uid.
        create_composition: Some(Arc::new(move |_ehr, uv| {
            *s_create.created_body.lock().unwrap() = Some(uv.data.clone());
            *s_create.stored.lock().unwrap() = Some(uv.data);
            Ok(format!("{COMP_VO}::ehrbase-rs.local::1"))
        })),
        get_composition_latest: Some(Arc::new(move |_e, _vo| {
            s_get.lock().unwrap().clone().ok_or_else(|| {
                SmError::new(
                    CallStatusType::CompositionDoesNotExist,
                    "composition not found",
                )
            })
        })),
        // The adl1.4 GET routes through the wire-shaped
        // `DefinitionAdapter::template_adl14_get` (template_id-keyed —
        // the SM `get_opt` is UUID-keyed per SM `i_definition_adl14.adoc`
        // `get_opt(an_opt_id: UUID)`, so the wire's string template id cannot
        // be served through it).
        template_adl14_get: Some(Arc::new(|_id| Ok(opt_xml()))),
        web_template: Some(Arc::new(|_id| Ok(Arc::new(web_template())))),
        ..Default::default()
    };
    ehrbase_rest::build_with(config(), Arc::new(Mock::with(hooks))).expect("router builds")
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
    let store = Store::default();
    *store.stored.lock().unwrap() = Some(canonical_composition());

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}/composition/{COMP_VO}"))
        .header(header::ACCEPT, FLAT_MIME)
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(app(&store), req).await;

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
    let store = Store::default();

    // Derive a real flat body from the canonical composition + its template.
    let wt = web_template();
    let flat = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_map: serde_json::Map<String, Value> = flat.into_iter().collect();
    let flat_body = serde_json::to_string(&flat_map).unwrap();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "{BASE}/ehr/{EHR}/composition?template_id=Demo%20Vitals"
        ))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from(flat_body))
        .unwrap();
    let (status, _h, _b) = send(app(&store), req).await;

    assert_eq!(status, StatusCode::CREATED);
    // The service received a canonical COMPOSITION, not the flat map.
    let created = store.created_body.lock().unwrap().clone().unwrap();
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
    let store = Store::default();
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{EHR}/composition"))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from("{\"ctx/language\":\"en\"}"))
        .unwrap();
    let (status, _h, _b) = send(app(&store), req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn flat_round_trips_through_http() {
    let store = Store::default();
    let wt = web_template();
    let flat_in = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_in_map: serde_json::Map<String, Value> = flat_in.clone().into_iter().collect();

    // POST the flat body → the mock stores the rebuilt canonical composition.
    let post = Request::builder()
        .method("POST")
        .uri(format!(
            "{BASE}/ehr/{EHR}/composition?templateId=Demo%20Vitals"
        ))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from(serde_json::to_string(&flat_in_map).unwrap()))
        .unwrap();
    let (status, _h, _b) = send(app(&store), post).await;
    assert_eq!(status, StatusCode::CREATED);

    // GET it back as flat.
    let get = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}/composition/{COMP_VO}"))
        .header(header::ACCEPT, FLAT_MIME)
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app(&store), get).await;
    assert_eq!(status, StatusCode::OK);
    let flat_out: std::collections::BTreeMap<String, Value> = serde_json::from_str(&body).unwrap();

    let flat_in_sorted: std::collections::BTreeMap<String, Value> = flat_in.into_iter().collect();
    assert_eq!(
        flat_in_sorted, flat_out,
        "flat → RM → flat stable through the HTTP endpoints"
    );
}
