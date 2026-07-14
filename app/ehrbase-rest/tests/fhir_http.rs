//! End-to-end HTTP tests for the FHIR R4 connector API group (our own extension — no openEHR spec governs this; E3):
//! the config gate (`RestConfig::fhir.enabled`), the starter-scope `501`, the
//! inbound `POST /fhir/r4/{resourceType}` outcomes (`201`/`404`/`422`), the
//! `/admin/fhir_mapping` CRUD, and the FHIR `OperationOutcome` error shape —
//! driven through the assembled router with the shared [`Mock`] platform whose
//! `fhir_*` hooks back an in-memory store.
//!
//! Design: FHIR↔openEHR mapping is spec-silent — our own extension;
//! the surface is our own, config-gated like the event-subscription group,
//! dispatching to the `FhirConnectorAdapter` extension. Every error is a FHIR
//! `OperationOutcome`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::type_complexity)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{
    AdminConfig, EventSubscriptionConfig, FhirConfig, RestConfig, TerminologyConfig,
};
use ehrbase_sm::{CallStatusType, SmError};
use ehrbase_sm::{ResourceMeta, ServiceResponse};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const MAPPINGS: &str = "/ehrbase/rest/openehr/v1/admin/fhir_mapping";
const INGEST_OBS: &str = "/ehrbase/rest/openehr/v1/fhir/r4/Observation";

type Store = Arc<Mutex<BTreeMap<Uuid, Value>>>;

fn record(id: Uuid, body: &Value) -> Value {
    json!({
        "id": id.to_string(),
        "name": body.get("name").cloned().unwrap_or(Value::Null),
        "resource_type": body.pointer("/definition/resource_type").cloned().unwrap_or(Value::Null),
        "template_id": body.pointer("/definition/template_id").cloned().unwrap_or(Value::Null),
        "profile_url": body.pointer("/definition/profile_url").cloned().unwrap_or(Value::Null),
        "definition": body.get("definition").cloned().unwrap_or(Value::Null),
        "enabled": body.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "created_at": "2026-07-11T00:00:00Z",
    })
}

fn not_found(id: Uuid) -> SmError {
    SmError::new(
        CallStatusType::VersionedObjectDoesNotExist,
        format!("FHIR mapping {id} does not exist"),
    )
}

/// Mapping-store hooks backed by a shared in-memory store, plus an `fhir_ingest`
/// hook that emulates the backend's behaviour (a mapping named `reject` maps to
/// an invalid COMPOSITION → 422; an absent mapping → 404; otherwise a committed
/// `ServiceResponse` → 201).
#[allow(clippy::too_many_lines)] // linear hook wiring; splitting obscures it
fn hooks(store: Store) -> Hooks {
    let (s_list, s_create, s_get, s_update, s_delete) = (
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        store,
    );
    Hooks {
        fhir_mapping_list: Some(Arc::new(move || {
            Ok(s_list.lock().unwrap().values().cloned().collect())
        })),
        fhir_mapping_create: Some(Arc::new(move |body: Value| {
            let name = body
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    SmError::new(CallStatusType::PreconditionViolation, "name required")
                })?;
            let mut map = s_create.lock().unwrap();
            if map.values().any(|v| v["name"] == json!(name)) {
                return Err(SmError::new(
                    CallStatusType::CompositionAlreadyExists,
                    "duplicate name",
                ));
            }
            let id = Uuid::now_v7();
            let rec = record(id, &body);
            map.insert(id, rec.clone());
            Ok(rec)
        })),
        fhir_mapping_get: Some(Arc::new(move |id: Uuid| {
            s_get
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| not_found(id))
        })),
        fhir_mapping_update: Some(Arc::new(move |id: Uuid, body: Value| {
            let mut map = s_update.lock().unwrap();
            if !map.contains_key(&id) {
                return Err(not_found(id));
            }
            let rec = record(id, &body);
            map.insert(id, rec.clone());
            Ok(rec)
        })),
        fhir_mapping_delete: Some(Arc::new(move |id: Uuid| {
            if s_delete.lock().unwrap().remove(&id).is_some() {
                Ok(())
            } else {
                Err(not_found(id))
            }
        })),
        fhir_search: Some(Arc::new(
            |resource_type: String, patient: String, _count: Option<i64>| {
                // Emulate the façade: Observation returns a one-entry searchset;
                // any other (in-scope) type has no mapping → an empty Bundle.
                let entries = if resource_type == "Observation" {
                    vec![json!({
                        "fullUrl": "urn:uuid:7f4c8e1a-0000-4000-8000-000000000001",
                        "resource": {
                            "resourceType": "Observation",
                            "id": "7f4c8e1a-0000-4000-8000-000000000001",
                            "subject": { "reference": format!("Patient/{patient}") },
                            "component": [ { "valueQuantity": { "value": 118, "unit": "mm[Hg]" } } ]
                        }
                    })]
                } else {
                    vec![]
                };
                Ok(json!({
                    "resourceType": "Bundle",
                    "type": "searchset",
                    "total": entries.len(),
                    "entry": entries,
                }))
            },
        )),
        fhir_ingest: Some(Arc::new(
            |resource_type: String, _profile: Option<String>, resource: Value| {
                // No mapping for a "Condition" (emulates the resolver miss → 404).
                if resource_type == "Condition" {
                    return Err(SmError::new(
                        CallStatusType::VersionedObjectDoesNotExist,
                        "no enabled FHIR mapping for resource type 'Condition'",
                    ));
                }
                // A resource whose subject is "invalid" maps to a COMPOSITION the
                // validator rejects (emulates content_invalid → 422), carrying the
                // validator message verbatim.
                if resource
                    .pointer("/subject/reference")
                    .and_then(Value::as_str)
                    == Some("Patient/invalid")
                {
                    return Err(SmError::new(
                        CallStatusType::ContentInvalid,
                        "/content[0]/data: missing mandatory element 'systolic'",
                    ));
                }
                Ok(ServiceResponse::new(
                    json!({ "_type": "COMPOSITION" }),
                    ResourceMeta::new("7f4c8e1a-0000-4000-8000-000000000001", "8d2b::local::1"),
                ))
            },
        )),
        ..Default::default()
    }
}

fn config(enabled: bool) -> RestConfig {
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
        admin: AdminConfig::default(),
        terminology: TerminologyConfig::default(),
        event_subscription: EventSubscriptionConfig::default(),
        tenancy: ehrbase_rest::TenancyConfig::default(),
        fhir: FhirConfig { enabled },
    }
}

fn app(enabled: bool) -> Router {
    let backend = Arc::new(Mock::with(hooks(Store::default())));
    ehrbase_rest::build_with(config(enabled), backend).expect("router builds")
}

fn app_unhooked() -> Router {
    let backend = Arc::new(Mock::new());
    ehrbase_rest::build_with(config(true), backend).expect("router builds")
}

/// Drive one request, returning `(status, Location header, JSON body)`.
async fn send(app: Router, req: Request<Body>) -> (StatusCode, Option<String>, Value) {
    let resp = app.oneshot(req).await.expect("response");
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

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder().method(method).uri(uri);
    match body {
        Some(v) => builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// Assert a body is a FHIR `OperationOutcome` with the given issue code.
fn assert_operation_outcome(body: &Value, code: &str) {
    assert_eq!(
        body["resourceType"], "OperationOutcome",
        "body is an OperationOutcome"
    );
    assert_eq!(body["issue"][0]["code"], code, "issue code");
    assert!(
        body["issue"][0]["diagnostics"].is_string(),
        "has diagnostics"
    );
}

// ── config gate ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn disabled_connector_is_404_operation_outcome() {
    // Mapping CRUD off.
    let (status, _, body) = send(app(false), req("GET", MAPPINGS, None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_operation_outcome(&body, "not-supported");
    // Inbound off.
    let obs = json!({ "resourceType": "Observation", "subject": { "reference": "Patient/x" } });
    let (status, _, body) = send(app(false), req("POST", INGEST_OBS, Some(obs))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_operation_outcome(&body, "not-supported");
}

// ── starter scope ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn unknown_resource_type_is_501_before_backend() {
    // MedicationRequest is outside the starter set → typed 501, even though the
    // backend has an fhir_ingest hook (the scope check is at the protocol edge).
    let uri = format!("{BASE}/fhir/r4/MedicationRequest");
    let body = json!({ "resourceType": "MedicationRequest" });
    let (status, _, oo) = send(app(true), req("POST", &uri, Some(body))).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_operation_outcome(&oo, "not-supported");
}

// ── read façade ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn search_returns_searchset_bundle() {
    let uri = format!("{INGEST_OBS}?patient=p-1&_count=10");
    let resp = app(true)
        .oneshot(req("GET", &uri, None))
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    // The façade renders as FHIR JSON.
    assert_eq!(
        resp.headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/fhir+json"),
    );
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let bundle: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(bundle["resourceType"], "Bundle");
    assert_eq!(bundle["type"], "searchset");
    assert_eq!(bundle["total"], 1);
    assert_eq!(
        bundle["entry"][0]["resource"]["resourceType"],
        "Observation"
    );
    assert!(
        bundle["entry"][0]["fullUrl"]
            .as_str()
            .unwrap()
            .starts_with("urn:uuid:"),
        "fullUrl is a urn:uuid"
    );
    // The subject scope round-trips into the reconstructed reference.
    assert_eq!(
        bundle["entry"][0]["resource"]["subject"]["reference"],
        "Patient/p-1"
    );
}

#[tokio::test]
async fn search_missing_patient_is_400() {
    // No patient param → 400 (explicit scope only; never generic Search).
    let (status, _, oo) = send(app(true), req("GET", INGEST_OBS, None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_operation_outcome(&oo, "required");
}

#[tokio::test]
async fn search_unknown_type_is_501() {
    let uri = format!("{BASE}/fhir/r4/MedicationRequest?patient=p-1");
    let (status, _, oo) = send(app(true), req("GET", &uri, None)).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_operation_outcome(&oo, "not-supported");
}

#[tokio::test]
async fn search_disabled_is_404() {
    let uri = format!("{INGEST_OBS}?patient=p-1");
    let (status, _, oo) = send(app(false), req("GET", &uri, None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_operation_outcome(&oo, "not-supported");
}

#[tokio::test]
async fn search_unhooked_is_501() {
    let uri = format!("{INGEST_OBS}?patient=p-1");
    let (status, _, _) = send(app_unhooked(), req("GET", &uri, None)).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

// ── inbound ingest outcomes ───────────────────────────────────────────────────
#[tokio::test]
async fn ingest_success_is_201_with_location() {
    let obs = json!({
        "resourceType": "Observation",
        "id": "bp-1",
        "subject": { "reference": "Patient/p-1" },
        "component": [ { "valueQuantity": { "value": 120, "unit": "mm[Hg]" } } ]
    });
    let (status, location, body) = send(app(true), req("POST", INGEST_OBS, Some(obs))).await;
    assert_eq!(status, StatusCode::CREATED);
    // The Location points at the committed openEHR COMPOSITION (readable via the
    // openEHR surface).
    let loc = location.expect("Location header present");
    assert!(
        loc.contains("/composition/"),
        "location targets the composition: {loc}"
    );
    assert_operation_outcome(&body, "informational");
}

#[tokio::test]
async fn ingest_validation_rejection_is_422_with_validator_message() {
    let obs = json!({
        "resourceType": "Observation",
        "subject": { "reference": "Patient/invalid" }
    });
    let (status, _, body) = send(app(true), req("POST", INGEST_OBS, Some(obs))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_operation_outcome(&body, "invalid");
    // The openEHR validator's message is carried verbatim.
    assert!(
        body["issue"][0]["diagnostics"]
            .as_str()
            .unwrap()
            .contains("missing mandatory element 'systolic'"),
        "validator message surfaced verbatim"
    );
}

#[tokio::test]
async fn ingest_no_mapping_is_404() {
    let uri = format!("{BASE}/fhir/r4/Condition");
    let body = json!({ "resourceType": "Condition", "subject": { "reference": "Patient/p" } });
    let (status, _, oo) = send(app(true), req("POST", &uri, Some(body))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_operation_outcome(&oo, "not-found");
}

#[tokio::test]
async fn ingest_unhooked_is_501() {
    let obs = json!({ "resourceType": "Observation", "subject": { "reference": "Patient/p" } });
    let (status, _, _) = send(app_unhooked(), req("POST", INGEST_OBS, Some(obs))).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

// ── mapping CRUD ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn mapping_crud_round_trip() {
    let app = app(true);

    let (status, _, list) = send(app.clone(), req("GET", MAPPINGS, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 0);

    let create = json!({
        "name": "obs-bp",
        "definition": {
            "resource_type": "Observation",
            "template_id": "ehrbase_blood_pressure_simple.de.v0",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir" },
            "entries": []
        }
    });
    let (status, _, created) = send(app.clone(), req("POST", MAPPINGS, Some(create))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "obs-bp");
    assert_eq!(created["resource_type"], "Observation");
    let id = created["id"].as_str().unwrap().to_owned();

    let (status, _, got) = send(app.clone(), req("GET", &format!("{MAPPINGS}/{id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["id"], id);

    let (status, _, _) = send(
        app.clone(),
        req("DELETE", &format!("{MAPPINGS}/{id}"), None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, oo) = send(app, req("GET", &format!("{MAPPINGS}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_operation_outcome(&oo, "not-found");
}

#[tokio::test]
async fn mapping_duplicate_name_is_409() {
    let app = app(true);
    let body = json!({
        "name": "dup",
        "definition": {
            "resource_type": "Observation", "template_id": "t",
            "subject": { "reference_path": "id", "namespace": "fhir" }, "entries": []
        }
    });
    let (status, _, _) = send(app.clone(), req("POST", MAPPINGS, Some(body.clone()))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, oo) = send(app, req("POST", MAPPINGS, Some(body))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_operation_outcome(&oo, "conflict");
}

#[tokio::test]
async fn mapping_malformed_id_is_400() {
    let (status, _, oo) = send(
        app(true),
        req("GET", &format!("{MAPPINGS}/not-a-uuid"), None),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_operation_outcome(&oo, "invalid");
}

#[tokio::test]
async fn mapping_unhooked_is_501() {
    let (status, _, _) = send(app_unhooked(), req("GET", MAPPINGS, None)).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}
