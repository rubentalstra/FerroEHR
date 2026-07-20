#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end HTTP tests for the ADL2 template wire (SM-2, `I_DEFINITION_ADL2`):
//! `POST /definition/template/adl2` (text/plain source upload, `Location` +
//! `Prefer` body, `422`-with-rule-codes on an invalid source),
//! `GET /definition/template/adl2/{template_id}` (text/plain source /
//! `application/json` `OperationalTemplateV2` / `406` on xml-only),
//! `GET …/{template_id}/{version}` (the deprecated versioned get),
//! `GET …/{template_id}/example` (`501`), and `GET /definition/template/adl2`
//! (`TemplateMetadata` list). Driven through the assembled router over a
//! **real** `EhrbaseService` on a real `PostgreSQL` — the source is a spec-valid
//! ADL2 operational template validated by the `openehr-adl` engine, uploaded
//! through the wire and stored verbatim, so the text/plain GET echoes it
//! exactly.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase_rest::config::AppConfig;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const HRID: &str = "openEHR-EHR-COMPOSITION.t_clinical_info.v1.0.0";

/// A spec-valid ADL2 operational-template source (`adl_version=2.0.6`), the same
/// shape `app/ehrbase/tests/service_definition.rs` builds: header + HRID,
/// `language`, `description` (mandatory — AOM2 master03 §Validity Rules VARD),
/// `definition` (root `id1`), `terminology` blocks. The `openehr-adl` engine
/// validates it, and the store keeps it verbatim.
fn adl2_source(keyword: &str, hrid: &str) -> String {
    let rm_type = hrid
        .split('.')
        .next()
        .and_then(|q| q.rsplit_once('-').map(|(_, e)| e))
        .expect("HRID carries an RM entity");
    format!(
        "{keyword} (adl_version=2.0.6; rm_release=1.1.0)\n    {hrid}\n\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         description\n    lifecycle_state = <\"published\">\n    details = <\n        \
         [\"en\"] = <\n            language = <[ISO_639-1::en]>\n        >\n    >\n\n\
         definition\n    {rm_type}[id1] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            \
         [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

fn source() -> String {
    adl2_source("operational_template", HRID)
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

async fn app() -> (testkit::TestDb, Router) {
    let (pg, service) = common::test_service().await;
    (pg, common::router_with(config(), service))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn upload_req(prefer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/definition/template/adl2"))
        .header(header::CONTENT_TYPE, "text/plain");
    if let Some(p) = prefer {
        b = b.header("Prefer", p);
    }
    b.body(Body::from(source())).unwrap()
}

#[tokio::test]
async fn upload_minimal_returns_201_and_location_only() {
    let (_pg, app) = app().await;
    let (status, headers, body) = send(&app, upload_req(None)).await;
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
    let (_pg, app) = app().await;
    let (status, headers, body) = send(&app, upload_req(Some("return=representation"))).await;
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
    assert_eq!(body, source(), "representation echoes the OPT source");
}

#[tokio::test]
async fn upload_identifier_returns_template_id_json() {
    let (_pg, app) = app().await;
    let (status, headers, body) = send(&app, upload_req(Some("return=identifier"))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(headers.contains_key(header::LOCATION));
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v.get("template_id").and_then(Value::as_str), Some(HRID));
}

#[tokio::test]
async fn get_serves_source_as_text_and_404s_unknown() {
    let (_pg, app) = app().await;
    // Upload first so the artefact exists.
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2/{HRID}"))
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    assert_eq!(body, source());

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/definition/template/adl2/openEHR-EHR-COMPOSITION.absent.v1.0.0"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_returns_template_metadata() {
    let (_pg, app) = app().await;
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2"))
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    let list = v.as_array().expect("array");
    assert_eq!(list.len(), 1);
    let row = &list[0];
    // TemplateMetadata: template_id + concept + archetype_id + created_timestamp
    // (schemas/definition/TemplateMetadata.yaml).
    assert_eq!(row.get("template_id").and_then(Value::as_str), Some(HRID));
    assert_eq!(row.get("archetype_id").and_then(Value::as_str), Some(HRID));
    assert_eq!(
        row.get("concept").and_then(Value::as_str),
        Some("t_clinical_info"),
        "concept is the HRID concept segment"
    );
    assert!(row.get("created_timestamp").is_some());
}

#[tokio::test]
async fn get_serves_operational_template_v2_json() {
    let (_pg, app) = app().await;
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Accept: application/json → the OperationalTemplateV2 canonical-JSON
    // projection (200_Template_adl2_retrieved.yaml, application/json branch).
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2/{HRID}"))
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    let v: Value = serde_json::from_str(&body).expect("OperationalTemplateV2 is a JSON object");
    // AOM2 canonical JSON self-tags every object with `_type`
    // (OperationalTemplateV2 is an opaque object; any JSON object satisfies it).
    assert!(v.is_object(), "OperationalTemplateV2 body is an object");
    assert_eq!(
        v.get("_type").and_then(Value::as_str),
        Some("OPERATIONAL_TEMPLATE")
    );
}

#[tokio::test]
async fn get_406_when_only_xml_acceptable() {
    let (_pg, app) = app().await;
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    // application/xml has no declared response body → 406.
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2/{HRID}"))
        .header(header::ACCEPT, "application/xml")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn version_get_resolves_and_serves_both_representations() {
    let (_pg, app) = app().await;
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    // The deprecated versioned get: template family + a `1` major prefix →
    // the stored v1.0.0 source (text/plain).
    let concept_family = "openEHR-EHR-COMPOSITION.t_clinical_info";
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/definition/template/adl2/{concept_family}/1"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    assert_eq!(body, source());

    // application/json → the OperationalTemplateV2 projection at that version.
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/definition/template/adl2/{concept_family}/1.0.0"
        ))
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        v.get("_type").and_then(Value::as_str),
        Some("OPERATIONAL_TEMPLATE")
    );

    // A version that does not exist → 404.
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/definition/template/adl2/{concept_family}/9"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn example_get_is_501() {
    let (_pg, app) = app().await;
    let (status, _h, _b) = send(&app, upload_req(None)).await;
    assert_eq!(status, StatusCode::CREATED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/definition/template/adl2/{HRID}/example"))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn upload_invalid_source_is_422_with_rule_codes() {
    let (_pg, app) = app().await;
    // A source missing the mandatory `description` section → VARD (AOM2
    // master03 §Validity Rules). The engine rejects it; the wire renders a 422
    // `Error` object whose `validationErrors` carry the rule code.
    let invalid = "operational_template (adl_version=2.0.6; rm_release=1.1.0)\n    \
                   openEHR-EHR-COMPOSITION.t_no_desc.v1.0.0\n\n\
                   language\n    original_language = <[ISO_639-1::en]>\n\n\
                   definition\n    COMPOSITION[id1] matches { *}\n\n\
                   terminology\n    term_definitions = <\n        [\"en\"] = <\n            \
                   [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n";
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/definition/template/adl2"))
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(invalid))
        .unwrap();
    let (status, _headers, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let v: Value = serde_json::from_str(&body).unwrap();
    let errors = v
        .get("validationErrors")
        .and_then(Value::as_array)
        .expect("the Error object carries validationErrors");
    assert!(
        errors
            .iter()
            .any(|e| e.as_str().is_some_and(|s| s.contains("VARD"))),
        "the rule code VARD is reported in the 422 body: {errors:?}"
    );
}
