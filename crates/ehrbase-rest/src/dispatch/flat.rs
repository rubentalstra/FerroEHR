//! FLAT (simSDT) + STRUCTURED (structSDT) glue for the COMPOSITION endpoints.
//!
//! Both are Better/EHRbase interop formats (`openehr-flat`), served as
//! `application/openehr.wt.flat+json` and `application/openehr.wt.structured+json`.
//! STRUCTURED is the pure nesting of the FLAT map (`openehr_flat::to_structured`
//! / `from_structured`); the template-id resolution is shared with FLAT.
//!
//! `WebTemplate` resolution is **not** this layer's concern: the service owns
//! the one cache and exposes it through the
//! [`WebTemplateService`](crate::backend::WebTemplateService) seam
//! (W2-K / finding F-13-02) — the same `WebTemplate` composition validation
//! uses. For FLAT specifically:
//!
//! * **input** (`Content-Type` FLAT on create/update): the flat map is rebuilt
//!   into a canonical-JSON `COMPOSITION` via `openehr_flat::from_flat`, driven
//!   by the target template's `WebTemplate`. The template id — which a flat body
//!   does not carry — comes from the `template_id`/`templateId` query parameter
//!   or the `openEHR-TEMPLATE_ID` header (EHRbase-compatible).
//! * **output** (`Accept` FLAT on get/create/update): the stored canonical
//!   composition is converted via `openehr_flat::to_flat` (its template id is
//!   read from `archetype_details/template_id`).

use axum::response::Response;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use serde_json::{Map, Value};

use openehr_its::rest::runtime::ApiError;

use crate::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};

fn internal(msg: impl Into<String>) -> RestError {
    RestError(ApiError::Internal(msg.into()))
}

fn bad_request(msg: impl Into<String>) -> RestError {
    RestError(ApiError::BadRequest(msg.into()))
}

fn flat_err(e: &openehr_flat::FlatError) -> RestError {
    RestError(ApiError::Internal(format!("FLAT conversion failed: {e}")))
}

/// The template id for a FLAT request: the `template_id` (or `templateId`) query
/// parameter, else the `openEHR-TEMPLATE_ID` header.
pub(super) fn request_template_id(query: Option<&str>, headers: &HeaderMap) -> Option<String> {
    params::query_param(query, "template_id")
        .or_else(|| params::query_param(query, "templateId"))
        .or_else(|| {
            headers
                .get("openEHR-TEMPLATE_ID")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        })
}

/// Parse a FLAT request body into a canonical-JSON `COMPOSITION`.
pub(super) async fn composition_from_flat(
    state: &AppState,
    query: Option<&str>,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Value, RestError> {
    let template_id = request_template_id(query, headers).ok_or_else(|| {
        bad_request(
            "FLAT composition input requires a template id via the `template_id` \
             (or `templateId`) query parameter or the `openEHR-TEMPLATE_ID` header",
        )
    })?;
    let flat: Map<String, Value> =
        serde_json::from_slice(body).map_err(|e| bad_request(format!("invalid FLAT JSON: {e}")))?;
    let wt = state
        .backend()
        .web_template(&template_id)
        .await
        .map_err(RestError)?;
    openehr_flat::from_flat(&flat, &wt).map_err(|e| flat_err(&e))
}

/// Render a canonical-JSON composition as a FLAT `application/openehr.wt.flat+json`
/// response (its template id read from `archetype_details/template_id`).
pub(super) async fn composition_flat_response(
    state: &AppState,
    status: StatusCode,
    comp: &Value,
) -> Result<Response, RestError> {
    let template_id = comp
        .pointer("/archetype_details/template_id/value")
        .and_then(Value::as_str)
        .ok_or_else(|| internal("composition has no archetype_details/template_id"))?
        .to_owned();
    let wt = state
        .backend()
        .web_template(&template_id)
        .await
        .map_err(RestError)?;
    let flat = openehr_flat::to_flat(comp, &wt).map_err(|e| flat_err(&e))?;
    let json =
        serde_json::to_string(&flat).map_err(|e| internal(format!("FLAT serialization: {e}")))?;
    Ok(negotiate::flat_json_body(status, json))
}

/// Parse a STRUCTURED (structSDT) request body into a canonical-JSON
/// `COMPOSITION` via `openehr_flat::from_structured` (template id resolved as
/// for FLAT: query param or `openEHR-TEMPLATE_ID` header).
pub(super) async fn composition_from_structured(
    state: &AppState,
    query: Option<&str>,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Value, RestError> {
    let template_id = request_template_id(query, headers).ok_or_else(|| {
        bad_request(
            "STRUCTURED composition input requires a template id via the `template_id` \
             (or `templateId`) query parameter or the `openEHR-TEMPLATE_ID` header",
        )
    })?;
    let structured: Value = serde_json::from_slice(body)
        .map_err(|e| bad_request(format!("invalid STRUCTURED JSON: {e}")))?;
    let wt = state
        .backend()
        .web_template(&template_id)
        .await
        .map_err(RestError)?;
    openehr_flat::from_structured(&structured, &wt).map_err(|e| flat_err(&e))
}

/// Render a canonical-JSON composition as a STRUCTURED
/// `application/openehr.wt.structured+json` response.
pub(super) async fn composition_structured_response(
    state: &AppState,
    status: StatusCode,
    comp: &Value,
) -> Result<Response, RestError> {
    let template_id = comp
        .pointer("/archetype_details/template_id/value")
        .and_then(Value::as_str)
        .ok_or_else(|| internal("composition has no archetype_details/template_id"))?
        .to_owned();
    let wt = state
        .backend()
        .web_template(&template_id)
        .await
        .map_err(RestError)?;
    let structured = openehr_flat::to_structured(comp, &wt).map_err(|e| flat_err(&e))?;
    let json = serde_json::to_string(&structured)
        .map_err(|e| internal(format!("STRUCTURED serialization: {e}")))?;
    Ok(negotiate::structured_json_body(status, json))
}
