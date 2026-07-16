//! End-to-end HTTP tests for the FLAT (simSDT) COMPOSITION endpoints, driven
//! through the assembled router over a **real** `EhrbaseService` on a real
//! `PostgreSQL`.
//!
//! The IPS OPT + its canonical composition are the pair driven end-to-end
//! through the real service in `app/ehrbase/tests/service_validation.rs`, so
//! they upload + commit cleanly here (the Demo Vitals corpus composition the
//! former Mock served fails the real template value-set validation; the
//! generated `Medium` example fails a proportion constraint — neither is
//! actually committable). The FLAT glue is exercised through the router:
//!
//! * GET with `Accept: application/openehr.wt.flat+json` → the stored canonical
//!   composition is returned as a flat map;
//! * POST with `Content-Type: application/openehr.wt.flat+json` + `?template_id`
//!   → the flat body is rebuilt into a canonical composition before the service
//!   commits it (verified by reading the stored composition back);
//! * POST flat without a template id → 400;
//! * a full flat → RM → flat round-trip through the two endpoints is stable
//!   (modulo the server-assigned version `_uid`s the Mock never produced).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase_rest::AppConfig;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";
const FLAT_MIME: &str = "application/openehr.wt.flat+json";
/// The IPS template id, percent-encoded for the `template_id` query parameter.
const TEMPLATE_ID_ENC: &str = "International%20Patient%20Summary";

fn opt_xml() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/openehr-flat/tests/fixtures/sdk/ips.v0.opt"),
    )
    .expect("ips.v0.opt vendored in openehr-flat")
}

/// The IPS canonical composition (with its stored `uid` removed — a create
/// assigns a fresh one).
fn canonical_composition() -> Value {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/ips_canonical.json",
        ),
    )
    .expect("ips_canonical.json vendored in openehr-its");
    let mut v: Value = serde_json::from_str(&text).expect("valid canonical composition");
    v.as_object_mut().unwrap().remove("uid");
    v
}

/// The IPS `WebTemplate` (built from the vendored OPT).
fn web_template() -> openehr_flat::WebTemplate {
    let opt = openehr_its::opt14::from_xml(&opt_xml()).expect("parse OPT");
    openehr_flat::build_web_template(&opt).expect("build web template")
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

fn etag_uid(h: &header::HeaderMap) -> String {
    h.get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .expect("ETag present")
        .trim_start_matches("W/")
        .trim_matches('"')
        .to_owned()
}

fn vo_of(ovid: &str) -> String {
    ovid.split("::").next().expect("vo uuid").to_owned()
}

/// Drop the `.../_uid` leaves (server-assigned version ids) so a flat→RM→flat
/// comparison isolates the data — the real service assigns version `_uid`s the
/// former in-memory Mock never produced.
fn without_uids(map: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    map.into_iter()
        .filter(|(k, _)| !k.ends_with("/_uid"))
        .collect()
}

/// A router over a fresh real service with the IPS OPT uploaded; returns the
/// router and a created EHR id.
async fn app_with_ehr(db: &str) -> (Router, String) {
    let app = common::router_with(config(), common::test_service(db).await);
    let (status, _h, body) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/definition/template/adl1.4"))
            .header(header::CONTENT_TYPE, "application/xml")
            .body(Body::from(opt_xml()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "OPT upload: {body}");

    let (status, h, _b) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/ehr"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    (app, etag_uid(&h))
}

/// Commit the canonical `comp` into `ehr_id`; return the new versioned-object uuid.
async fn commit_canonical(app: &Router, ehr_id: &str, comp: &Value) -> String {
    let (status, h, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("{BASE}/ehr/{ehr_id}/composition"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(comp.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "canonical commit: {body}");
    vo_of(&etag_uid(&h))
}

#[tokio::test]
async fn get_composition_as_flat() {
    let (app, ehr) = app_with_ehr("flat_get").await;
    let vo = commit_canonical(&app, &ehr, &canonical_composition()).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr}/composition/{vo}"))
        .header(header::ACCEPT, FLAT_MIME)
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK, "flat get: {body}");
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
    let (app, ehr) = app_with_ehr("flat_post_rebuild").await;

    // Derive a real flat body from the canonical composition + its template.
    let wt = web_template();
    let flat = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_map: serde_json::Map<String, Value> = flat.into_iter().collect();
    let flat_body = serde_json::to_string(&flat_map).unwrap();

    let (status, h, body) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!(
                "{BASE}/ehr/{ehr}/composition?template_id={TEMPLATE_ID_ENC}"
            ))
            .header(header::CONTENT_TYPE, FLAT_MIME)
            .body(Body::from(flat_body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "flat commit: {body}");
    let vo = vo_of(&etag_uid(&h));

    // The service received (and stored) a canonical COMPOSITION, not the flat map.
    let (status, _h, body) = send(
        &app,
        Request::builder()
            .method("GET")
            .uri(format!("{BASE}/ehr/{ehr}/composition/{vo}"))
            .header(header::ACCEPT, "application/json")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "canonical read: {body}");
    let stored: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        stored.get("_type").and_then(Value::as_str),
        Some("COMPOSITION")
    );
    assert!(stored.get("content").is_some(), "rebuilt content present");
    assert!(
        stored.pointer("/context/start_time/value").is_some(),
        "rebuilt context from ctx/"
    );
}

#[tokio::test]
async fn post_flat_without_template_id_is_400() {
    let (app, ehr) = app_with_ehr("flat_post_no_tid").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr}/composition"))
        .header(header::CONTENT_TYPE, FLAT_MIME)
        .body(Body::from("{\"ctx/language\":\"en\"}"))
        .unwrap();
    let (status, _h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn flat_round_trips_through_http() {
    let (app, ehr) = app_with_ehr("flat_roundtrip").await;
    let wt = web_template();
    let flat_in = openehr_flat::to_flat(&canonical_composition(), &wt).unwrap();
    let flat_in_map: serde_json::Map<String, Value> = flat_in.clone().into_iter().collect();

    // POST the flat body → the service stores the rebuilt canonical composition.
    let (status, h, body) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!(
                "{BASE}/ehr/{ehr}/composition?templateId={TEMPLATE_ID_ENC}"
            ))
            .header(header::CONTENT_TYPE, FLAT_MIME)
            .body(Body::from(serde_json::to_string(&flat_in_map).unwrap()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "flat commit: {body}");
    let vo = vo_of(&etag_uid(&h));

    // GET it back as flat.
    let (status, _h, body) = send(
        &app,
        Request::builder()
            .method("GET")
            .uri(format!("{BASE}/ehr/{ehr}/composition/{vo}"))
            .header(header::ACCEPT, FLAT_MIME)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "flat get: {body}");
    let flat_out: BTreeMap<String, Value> = serde_json::from_str(&body).unwrap();

    let flat_in_sorted: BTreeMap<String, Value> = flat_in.into_iter().collect();
    assert_eq!(
        without_uids(flat_in_sorted),
        without_uids(flat_out),
        "flat → RM → flat stable through the HTTP endpoints (modulo server uids)"
    );
}
