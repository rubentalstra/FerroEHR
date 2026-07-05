//! FLAT (simSDT) glue for the COMPOSITION endpoints.
//!
//! The FLAT format is a Better/EHRbase interop format (`openehr-flat`), served
//! as `application/openehr.wt.flat+json`:
//!
//! * **input** (`Content-Type` FLAT on create/update): the flat map is rebuilt
//!   into a canonical-JSON `COMPOSITION` via `openehr_flat::from_flat`, driven
//!   by the target template's `WebTemplate`. The template id — which a flat body
//!   does not carry — comes from the `template_id`/`templateId` query parameter
//!   or the `openEHR-TEMPLATE_ID` header (EHRbase-compatible), and the OPT 1.4 is
//!   fetched from the DEFINITION store and cached as a `WebTemplate`.
//! * **output** (`Accept` FLAT on get/create/update): the stored canonical
//!   composition is converted via `openehr_flat::to_flat` (its template id is
//!   read from `archetype_details/template_id`).

use std::sync::Arc;

use axum::response::Response;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use serde_json::{Map, Value};

use openehr_flat::WebTemplate;
use openehr_its::rest::generated::definition::DefinitionTemplateAdl14GetParams;
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

/// Fetch the OPT 1.4 for `template_id` from the DEFINITION store and build (or
/// reuse the cached) Better [`WebTemplate`].
async fn web_template_for(
    state: &AppState,
    template_id: &str,
) -> Result<Arc<WebTemplate>, RestError> {
    let fetched = state
        .backend()
        .definition_template_adl1_4_get(DefinitionTemplateAdl14GetParams {
            template_id: template_id.to_owned(),
            accept: None,
        })
        .await
        .map_err(RestError)?;
    let Value::String(xml) = fetched else {
        return Err(internal("stored template is not OPT 1.4 XML"));
    };
    state
        .web_templates()
        .get_or_build(template_id, || {
            let opt = openehr_its::opt14::from_xml(&xml)
                .map_err(|e| openehr_flat::FlatError::OptParse(e.to_string()))?;
            openehr_flat::build_web_template(&opt)
        })
        .await
        .map_err(|e| internal(format!("WebTemplate build failed: {e}")))
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
    let wt = web_template_for(state, &template_id).await?;
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
    let wt = web_template_for(state, &template_id).await?;
    let flat = openehr_flat::to_flat(comp, &wt).map_err(|e| flat_err(&e))?;
    let json =
        serde_json::to_string(&flat).map_err(|e| internal(format!("FLAT serialization: {e}")))?;
    Ok(negotiate::flat_json_body(status, json))
}
