//! HTTP dispatch for the **FHIR R4 inbound connector** + mapping-store CRUD
//! over the [`FhirConnectorAdapter`](ehrbase_sm::FhirConnectorAdapter) seam
//! (our own extension — no openEHR spec governs this; E3).
//!
//! Two surfaces, both config-gated (`RestConfig::fhir.enabled`, default
//! `false`): when disabled every route answers `404` (an `OperationOutcome`)
//! without touching the backend.
//!
//! * `POST /fhir/r4/{resource_type}` — the inbound connector. A FHIR R4
//!   resource is accepted, its mapping resolved by type + `meta.profile`, a
//!   COMPOSITION built and committed through the NORMAL validated path with
//!   `FEEDER_AUDIT` provenance. Only the starter resource
//!   set ([`STARTER_RESOURCES`]) is supported; anything else is a typed
//!   `501 OperationOutcome`.
//! * `GET /fhir/r4/{resource_type}?patient=<ehr-subject-or-id>[&_count=N]` — the
//!   read façade. Resolves the enabled mappings for the
//!   type, runs the template-bound COMPOSITION query scoped to the patient, and
//!   returns a FHIR `searchset` Bundle of reverse-mapped resources. The
//!   `patient` scope is mandatory — a missing one is a typed `400` (explicit
//!   params only; never generic FHIR Search). An
//!   out-of-scope type is the same typed `501` as inbound.
//! * `/admin/fhir_mapping[/{id}]` — CRUD over the deployable mapping artefacts
//!   ("mapping-as-data"). Mounted under `/admin/` like the
//!   event-subscription/tenant extensions (the coarse RBAC gate classes it
//!   `Admin`).
//!
//! PORT NOTE: every error on this surface is a FHIR
//! `OperationOutcome` (`severity`/`code`/`diagnostics`), NOT the openEHR error
//! body — this is the FHIR boundary. Validator rejections surface the openEHR
//! validator's message verbatim in `diagnostics` (the CDR's rules win: a
//! resource that maps to an invalid COMPOSITION is rejected `422`, not partially
//! stored). FHIR↔openEHR mapping is spec-silent — our own extension,
//! so this is our own surface, excluded from the ITS-REST drift check.

use axum::Json;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::header::{CONTENT_TYPE, HeaderValue};
use serde_json::{Value, json};
use uuid::Uuid;

use ehrbase_sm::Platform;
use ehrbase_sm::SmError;
use ehrbase_sm::ServiceResponse;
use openehr_its::rest::runtime::ApiError;

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use crate::negotiate;
use crate::state::AppState;

/// FHIR R4 media type for the `OperationOutcome` / connector responses.
const FHIR_JSON: &str = "application/fhir+json";

/// The starter resource set the inbound connector maps;
/// anything else is a typed `501 OperationOutcome`.
pub(crate) const STARTER_RESOURCES: &[&str] =
    &["Patient", "Observation", "Condition", "DocumentReference"];

/// The FHIR-connector routes — our own extension (no ITS-REST contract), mounted
/// alongside the generated `ROUTES`. Group-relative paths (nested under the
/// configured `base_path`).
pub(crate) const FHIR_ROUTES: &[(&str, &str, &str)] = &[
    // Inbound: accept a FHIR R4 resource of {resource_type} and commit it.
    ("POST", "/fhir/r4/{resource_type}", "fhir_ingest"),
    // Read façade: patient-scoped searchset Bundle of reverse-mapped resources.
    ("GET", "/fhir/r4/{resource_type}", "fhir_search"),
    // Mapping-store CRUD (mapping-as-data).
    ("GET", "/admin/fhir_mapping", "fhir_mapping_list"),
    ("POST", "/admin/fhir_mapping", "fhir_mapping_create"),
    (
        "GET",
        "/admin/fhir_mapping/{mapping_id}",
        "fhir_mapping_get",
    ),
    (
        "PUT",
        "/admin/fhir_mapping/{mapping_id}",
        "fhir_mapping_update",
    ),
    (
        "DELETE",
        "/admin/fhir_mapping/{mapping_id}",
        "fhir_mapping_delete",
    ),
];

pub(super) fn dispatch<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move { run(state, op, parts).await })
}

async fn run<S: Platform>(state: AppState<S>, op: &'static str, parts: RequestParts) -> Response {
    // Config gate: opt-in. When disabled every route answers 404 (as an
    // `OperationOutcome`) without consulting the backend.
    if !state.config().fhir.enabled {
        return operation_outcome(
            StatusCode::NOT_FOUND,
            "not-supported",
            "the FHIR connector is disabled",
        );
    }

    let h = &parts.headers;
    match op {
        "fhir_ingest" => ingest(&state, &parts).await,
        "fhir_search" => search(&state, &parts).await,
        "fhir_mapping_list" => match state.backend().fhir_mapping_list().await {
            Ok(items) => negotiate::respond(h, StatusCode::OK, &items),
            Err(e) => sm_error_outcome(e),
        },
        "fhir_mapping_create" => {
            let body = match negotiate::json_value(h, &parts.body) {
                Ok(b) => b,
                Err(e) => return api_error_outcome(&e),
            };
            match state.backend().fhir_mapping_create(body).await {
                Ok(created) => negotiate::respond(h, StatusCode::CREATED, &created),
                Err(e) => sm_error_outcome(e),
            }
        }
        "fhir_mapping_get" => match mapping_id(&parts) {
            Ok(id) => match state.backend().fhir_mapping_get(id).await {
                Ok(item) => negotiate::respond(h, StatusCode::OK, &item),
                Err(e) => sm_error_outcome(e),
            },
            Err(e) => api_error_outcome(&e.0),
        },
        "fhir_mapping_update" => match mapping_id(&parts) {
            Ok(id) => {
                let body = match negotiate::json_value(h, &parts.body) {
                    Ok(b) => b,
                    Err(e) => return api_error_outcome(&e),
                };
                match state.backend().fhir_mapping_update(id, body).await {
                    Ok(updated) => negotiate::respond(h, StatusCode::OK, &updated),
                    Err(e) => sm_error_outcome(e),
                }
            }
            Err(e) => api_error_outcome(&e.0),
        },
        "fhir_mapping_delete" => match mapping_id(&parts) {
            Ok(id) => match state.backend().fhir_mapping_delete(id).await {
                Ok(()) => negotiate::empty(StatusCode::NO_CONTENT),
                Err(e) => sm_error_outcome(e),
            },
            Err(e) => api_error_outcome(&e.0),
        },
        other => operation_outcome(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            &format!("unrouted FHIR connector operation: {other}"),
        ),
    }
}

/// Resolve the `{resource_type}` path param + enforce the starter-scope gate
///: a missing param is a routing bug (`500`), an out-of-scope type
/// is a typed `501` before the backend is touched. Shared by inbound + façade.
#[allow(clippy::result_large_err)] // the Err is a ready axum Response (large by nature)
fn scoped_resource_type(parts: &RequestParts) -> Result<String, Response> {
    let Some(resource_type) = parts.path.get("resource_type").cloned() else {
        return Err(operation_outcome(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            "missing path parameter `resource_type`",
        ));
    };
    if !STARTER_RESOURCES.contains(&resource_type.as_str()) {
        return Err(operation_outcome(
            StatusCode::NOT_IMPLEMENTED,
            "not-supported",
            &format!(
                "FHIR resource type '{resource_type}' is not supported by the connector \
                 (starter set: {})",
                STARTER_RESOURCES.join(", ")
            ),
        ));
    }
    Ok(resource_type)
}

/// `POST /fhir/r4/{resource_type}` — the inbound connector.
async fn ingest<S: Platform>(state: &AppState<S>, parts: &RequestParts) -> Response {
    let resource_type = match scoped_resource_type(parts) {
        Ok(rt) => rt,
        Err(resp) => return resp,
    };
    let body = match negotiate::json_value(&parts.headers, &parts.body) {
        Ok(b) => b,
        Err(e) => return api_error_outcome(&e),
    };
    let profile = first_profile(&body);
    match state
        .backend()
        .fhir_ingest(resource_type, profile, body)
        .await
    {
        Ok(resp) => ingest_created(state, &resp),
        Err(e) => sm_error_outcome(e),
    }
}

/// `GET /fhir/r4/{resource_type}?patient=…[&_count=N]` — the read façade.
async fn search<S: Platform>(state: &AppState<S>, parts: &RequestParts) -> Response {
    let resource_type = match scoped_resource_type(parts) {
        Ok(rt) => rt,
        Err(resp) => return resp,
    };
    let q = parts.query.as_deref();
    // `patient` is mandatory: the façade serves only this explicit scope,
    // never generic FHIR Search.
    let Some(patient) = crate::params::query_param(q, "patient").filter(|p| !p.is_empty()) else {
        return operation_outcome(
            StatusCode::BAD_REQUEST,
            "required",
            "the `patient` query parameter is required (the FHIR read façade serves only \
             the explicit patient scope, not generic Search)",
        );
    };
    let count = crate::params::query_param(q, "_count").and_then(|c| c.parse::<i64>().ok());
    match state
        .backend()
        .fhir_search(resource_type, patient, count)
        .await
    {
        Ok(bundle) => fhir_json(StatusCode::OK, &bundle),
        Err(e) => sm_error_outcome(e),
    }
}

/// The first `meta.profile` canonical URL on the resource, if any.
fn first_profile(resource: &Value) -> Option<String> {
    resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.get(0))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The `201` response for a committed resource: an information `OperationOutcome`
/// plus the `ETag`/`Location` headers pointing at the openEHR COMPOSITION (so a
/// client can read it back through the openEHR surface).
fn ingest_created<S: Platform>(state: &AppState<S>, resp: &ServiceResponse) -> Response {
    let mut out = operation_outcome(
        StatusCode::CREATED,
        "informational",
        "the FHIR resource was committed as an openEHR COMPOSITION",
    );
    if let Some(meta) = &resp.meta {
        negotiate::set_resource_headers(
            &mut out,
            &state.config().base_path,
            Some("composition"),
            meta,
        );
    }
    out
}

/// Parse the `{mapping_id}` path parameter as a UUID → `400` when malformed (a
/// missing param is a routing bug → `500`).
fn mapping_id(parts: &RequestParts) -> Result<Uuid, RestError> {
    let raw = parts.path.get("mapping_id").ok_or_else(|| {
        RestError(ApiError::Internal(
            "missing path parameter `mapping_id`".to_owned(),
        ))
    })?;
    raw.parse::<Uuid>().map_err(|_| {
        RestError(ApiError::BadRequest(format!(
            "invalid FHIR mapping id `{raw}`"
        )))
    })
}

/// Render a FHIR resource (the read-façade Bundle) as `application/fhir+json`.
fn fhir_json(status: StatusCode, body: &Value) -> Response {
    let mut resp = (status, Json(body.clone())).into_response();
    resp.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(FHIR_JSON));
    resp
}

/// Build a FHIR `OperationOutcome` response (`severity`/`code`/`diagnostics`)
/// with `Content-Type: application/fhir+json` and the given status.
fn operation_outcome(status: StatusCode, code: &str, diagnostics: &str) -> Response {
    let severity = if status.is_success() {
        "information"
    } else {
        "error"
    };
    let body = json!({
        "resourceType": "OperationOutcome",
        "issue": [{ "severity": severity, "code": code, "diagnostics": diagnostics }],
    });
    let mut resp = (status, Json(body)).into_response();
    resp.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(FHIR_JSON));
    resp
}

/// Map an [`SmError`] to a FHIR `OperationOutcome`, reusing the maintained SM →
/// HTTP table ([`RestError`]) for the status and surfacing the message (e.g. a
/// validator rejection) verbatim in `diagnostics`.
fn sm_error_outcome(e: SmError) -> Response {
    let message = e.message.clone();
    let status = RestError::from(e).0.status();
    operation_outcome(status, fhir_issue_code(status), &message)
}

/// Map an [`ApiError`] (a wire-parse failure) to a FHIR `OperationOutcome`.
fn api_error_outcome(e: &ApiError) -> Response {
    let status = e.status();
    operation_outcome(status, fhir_issue_code(status), &e.to_string())
}

/// The FHIR `issue.code` (`IssueType`) for an HTTP status.
fn fhir_issue_code(status: StatusCode) -> &'static str {
    match status {
        StatusCode::NOT_FOUND => "not-found",
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST => "invalid",
        StatusCode::CONFLICT | StatusCode::PRECONDITION_FAILED => "conflict",
        StatusCode::NOT_IMPLEMENTED
        | StatusCode::UNSUPPORTED_MEDIA_TYPE
        | StatusCode::NOT_ACCEPTABLE => "not-supported",
        StatusCode::UNAUTHORIZED => "login",
        StatusCode::FORBIDDEN => "forbidden",
        _ => "exception",
    }
}
