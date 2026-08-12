// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! End-to-end inbound FHIR connector tests against a real `PostgreSQL` 18
//! (the shared `testkit` harness), driven through the assembled `ferroehr-rest`
//! router over the real DB-backed `FerroEhrService` (our own extension — no openEHR spec governs this; E3).
//!
//! Covers: a valid FHIR Observation → `201`, committed through the NORMAL
//! validated path and **readable via the openEHR surface with `FEEDER_AUDIT`
//! present**; a resource that maps to an invalid COMPOSITION → `422`
//! `OperationOutcome` carrying the openEHR validator's message; the config gate
//! (`404`); an out-of-scope resource type (`501`); a resource type with no
//! mapping (`404`); and the mapping-store CRUD over HTTP.
//!
//! Each test takes a fresh, fully-migrated database from the shared `testkit`
//! harness (`tools/testkit`); the returned guard releases the clone on drop.
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use ferroehr::config::auth::AuthConfig;
use ferroehr::config::server::ServerConfig;
use ferroehr::service::FerroEhrService;
use ferroehr_rest::config::AppConfig;

const BASE: &str = "/ferroehr/rest/openehr/v1";
const MAPPINGS: &str = "/ferroehr/rest/openehr/v1/admin/fhir_mapping";
// A minimal EVALUATION template (ITEM_TREE data, no HISTORY/EVENT), chosen so a
// small FHIR→FLAT mapping builds a COMPOSITION that passes the openEHR validator
// on the real commit path — an OBSERVATION template would need event `offset`
// and an ITEM_LIST-constrained data slot that the reverse-FLAT builder does not
// materialise (an ArchetypeValidation/from_flat gap, out of E3 scope).
const OPT_REL: &str = "tests/resources/service/knowledge/opt/minimal_evaluation.opt";
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";
const PROFILE_OK: &str = "http://example.org/StructureDefinition/bp";
const PROFILE_BAD: &str = "http://example.org/StructureDefinition/bp-bad";

fn fixture(rel: &str) -> String {
    let path = format!("{}/../ferroehr/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn config(fhir_enabled: bool) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            base_path: BASE.to_owned(),
            swagger_ui: false,
            ..ServerConfig::default()
        },
        auth: AuthConfig {
            enabled: false,
            ..AuthConfig::default()
        },
        fhir_api_enabled: fhir_enabled,
        ..Default::default()
    }
}

/// Build the router over the real service with an OPT already ingested.
async fn app_with_template(pool: PgPool, fhir_enabled: bool) -> (Arc<FerroEhrService>, Router) {
    let svc = Arc::new(FerroEhrService::new(pool));
    svc.template_adl14_upload(fixture(OPT_REL))
        .await
        .expect("ingest OPT");
    let router =
        ferroehr_rest::build_with(config(fhir_enabled), Arc::clone(&svc)).expect("router builds");
    (svc, router)
}

/// A mapping definition (as the `{name, definition}` create body) for the BP
/// template with the given profile + territory (an invalid territory makes the
/// built COMPOSITION fail terminology validation).
fn mapping_body(name: &str, profile: &str, territory: &str) -> Value {
    json!({
        "name": name,
        "definition": {
            "resource_type": "Observation",
            "profile_url": profile,
            "template_id": TEMPLATE_ID,
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "context": {
                "ctx/language": "en",
                "ctx/territory": territory,
                "ctx/composer_name": "fhir-connector",
                "ctx/time": "2026-02-03T04:05:06Z"
            },
            "entries": [
                { "openehr_path": "minimal/minimal:0/quantity",
                  "fhir_path": "valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "valueQuantity.unit" } }
            ]
        }
    })
}

fn bp_observation(profile: &str) -> Value {
    json!({
        "resourceType": "Observation",
        "id": "bp-obs-1",
        "meta": { "versionId": "1", "profile": [profile] },
        "status": "final",
        "subject": { "reference": "Patient/p-42" },
        "valueQuantity": { "value": 118, "unit": "kg" }
    })
}

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let b = Request::builder().method(method).uri(uri);
    match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn send(router: &Router, req: Request<Body>) -> (StatusCode, Option<String>, Value) {
    let resp = router.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let location = resp
        .headers()
        .get(http::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, location, body)
}

#[tokio::test]
async fn valid_resource_commits_and_is_readable_with_feeder_audit() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;

    // Create the mapping (over HTTP → exercises the CRUD wire).
    let (status, _, _) = send(
        &router,
        req("POST", MAPPINGS, Some(mapping_body("bp", PROFILE_OK, "US"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "mapping created");

    // POST the FHIR Observation → committed COMPOSITION.
    let (status, location, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/Observation"),
            Some(bp_observation(PROFILE_OK)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "ingest committed: {oo}");
    let location = location.expect("Location header");
    assert!(
        location.contains("/composition/"),
        "location targets composition: {location}"
    );

    // Read it back through the openEHR surface: FEEDER_AUDIT must be present with
    // the fhir-connector provenance, and the mapped value.
    let (status, _, comp) = send(&router, req("GET", &location, None)).await;
    assert_eq!(status, StatusCode::OK, "composition readable: {comp}");
    let feeder = &comp["feeder_audit"];
    assert_eq!(
        feeder["_type"], "FEEDER_AUDIT",
        "feeder_audit present: {comp}"
    );
    assert_eq!(
        feeder["originating_system_audit"]["system_id"], "fhir-connector",
        "originating system recorded"
    );
    assert_eq!(
        feeder["originating_system_audit"]["version_id"], "1",
        "resource version recorded"
    );
    assert_eq!(
        feeder["originating_system_item_ids"][0]["id"], "bp-obs-1",
        "resource id recorded"
    );
    // The mapped systolic magnitude survived the full FHIR→FLAT→RM→commit→read.
    assert!(
        comp.to_string().contains("118"),
        "systolic value stored + read back"
    );
}

#[tokio::test]
async fn facade_returns_committed_resource_as_searchset_bundle() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;

    // Map + commit a FHIR Observation (subject Patient/p-42 → EHR subject p-42).
    let (status, _, _) = send(
        &router,
        req("POST", MAPPINGS, Some(mapping_body("bp", PROFILE_OK, "US"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/Observation"),
            Some(bp_observation(PROFILE_OK)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "ingest committed: {oo}");

    // Read it back through the FHIR read façade, scoped by the patient subject.
    let (status, _, bundle) = send(
        &router,
        req(
            "GET",
            &format!("{BASE}/fhir/r4/Observation?patient=p-42&_count=10"),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "façade query: {bundle}");
    assert_eq!(bundle["resourceType"], "Bundle");
    assert_eq!(bundle["type"], "searchset");
    assert_eq!(bundle["total"], 1, "one committed Observation: {bundle}");
    let resource = &bundle["entry"][0]["resource"];
    assert_eq!(resource["resourceType"], "Observation");
    // The mapped value survived commit → reverse-map (magnitude 118).
    assert_eq!(
        resource["valueQuantity"]["value"].as_f64(),
        Some(118.0),
        "mapped magnitude reverse-mapped: {resource}"
    );
    assert_eq!(
        resource["valueQuantity"]["unit"], "kg",
        "unit reverse-mapped"
    );
    // The subject reference is reconstructed with strip_prefix re-applied.
    assert_eq!(resource["subject"]["reference"], "Patient/p-42");
    // The entry fullUrl is the composition's versioned-object uuid.
    assert!(
        bundle["entry"][0]["fullUrl"]
            .as_str()
            .unwrap()
            .starts_with("urn:uuid:"),
        "fullUrl urn: {bundle}"
    );
}

#[tokio::test]
async fn facade_empty_when_no_data_for_patient() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;
    // A mapping exists but nothing committed for this patient → empty searchset.
    let (status, _, _) = send(
        &router,
        req("POST", MAPPINGS, Some(mapping_body("bp", PROFILE_OK, "US"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, bundle) = send(
        &router,
        req(
            "GET",
            &format!("{BASE}/fhir/r4/Observation?patient=nobody"),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bundle["type"], "searchset");
    assert_eq!(bundle["total"], 0, "no data → empty bundle: {bundle}");
}

#[tokio::test]
async fn facade_missing_patient_is_400() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;
    let (status, _, oo) = send(
        &router,
        req("GET", &format!("{BASE}/fhir/r4/Observation"), None),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "no patient → 400: {oo}");
    assert_eq!(oo["issue"][0]["code"], "required");
}

#[tokio::test]
async fn invalid_mapped_content_is_422_with_validator_message() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;

    // A mapping whose context sets an invalid ISO-3166 territory ("ZZ") → the
    // built COMPOSITION fails the openEHR terminology validation on commit.
    let (status, _, _) = send(
        &router,
        req(
            "POST",
            MAPPINGS,
            Some(mapping_body("bp-bad", PROFILE_BAD, "ZZ")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/Observation"),
            Some(bp_observation(PROFILE_BAD)),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validator rejected: {oo}"
    );
    assert_eq!(oo["resourceType"], "OperationOutcome");
    assert_eq!(oo["issue"][0]["code"], "invalid");
    // The openEHR validator's message is carried verbatim (mentions the territory).
    let diag = oo["issue"][0]["diagnostics"]
        .as_str()
        .unwrap()
        .to_lowercase();
    assert!(
        diag.contains("territory") || diag.contains("country") || diag.contains("zz"),
        "validator message surfaced: {diag}"
    );
}

#[tokio::test]
async fn disabled_connector_is_404() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), false).await;
    let (status, _, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/Observation"),
            Some(bp_observation(PROFILE_OK)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(oo["resourceType"], "OperationOutcome");
}

#[tokio::test]
async fn unknown_resource_type_is_501() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;
    let (status, _, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/MedicationRequest"),
            Some(json!({ "resourceType": "MedicationRequest" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(oo["issue"][0]["code"], "not-supported");
}

#[tokio::test]
async fn no_mapping_for_type_is_404() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;
    // Patient is in the starter set (passes the 501 gate) but no Patient mapping
    // exists → the resolver misses → 404.
    let (status, _, oo) = send(
        &router,
        req(
            "POST",
            &format!("{BASE}/fhir/r4/Patient"),
            Some(json!({ "resourceType": "Patient", "id": "p-1" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no mapping → 404: {oo}");
    assert_eq!(oo["issue"][0]["code"], "not-found");
}

#[tokio::test]
async fn mapping_crud_over_http() {
    let db = testkit::db().await.expect("testkit database");
    let (_svc, router) = app_with_template(db.pool(), true).await;

    // Empty list.
    let (status, _, list) = send(&router, req("GET", MAPPINGS, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 0);

    // Create.
    let (status, _, created) = send(
        &router,
        req("POST", MAPPINGS, Some(mapping_body("bp", PROFILE_OK, "US"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["resource_type"], "Observation");
    assert_eq!(created["template_id"], TEMPLATE_ID);
    let id = created["id"].as_str().unwrap().to_owned();

    // Get.
    let (status, _, got) = send(&router, req("GET", &format!("{MAPPINGS}/{id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["name"], "bp");

    // Duplicate name → 409.
    let (status, _, _) = send(
        &router,
        req(
            "POST",
            MAPPINGS,
            Some(mapping_body("bp", PROFILE_BAD, "US")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Unknown template → 400 (FK).
    let mut bad = mapping_body("bad-tmpl", PROFILE_BAD, "US");
    bad["definition"]["template_id"] = json!("no.such.template.v0");
    let (status, _, _) = send(&router, req("POST", MAPPINGS, Some(bad))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Delete → 204, then gone → 404.
    let (status, _, _) = send(&router, req("DELETE", &format!("{MAPPINGS}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&router, req("GET", &format!("{MAPPINGS}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
