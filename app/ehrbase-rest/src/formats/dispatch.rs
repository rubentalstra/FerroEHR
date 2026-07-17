//! FLAT (simSDT) + STRUCTURED (structSDT) glue for the COMPOSITION endpoints.
//!
//! Both are Better/EHRbase interop formats (`openehr-flat`), served as
//! `application/openehr.wt.flat+json` and `application/openehr.wt.structured+json`.
//! STRUCTURED is the pure nesting of the FLAT map (`openehr_flat::to_structured`
//! / `from_structured`); the template-id resolution is shared with FLAT.
//!
//! `WebTemplate` resolution is **not** this layer's concern: the service owns
//! the one cache and exposes it through the
//! [`WebTemplateService`](ehrbase::service::WebTemplateService) seam
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

use crate::overview::error::RestError;

use crate::state::AppState;
use crate::{negotiate, params};

fn internal(msg: impl Into<String>) -> RestError {
    RestError(ApiError::Internal(msg.into()))
}

fn bad_request(msg: impl Into<String>) -> RestError {
    RestError(ApiError::BadRequest(msg.into()))
}

/// A FLAT/STRUCTURED **output** conversion failure: the server failed to render
/// a *stored* canonical composition into the requested simplified format. Stored
/// data is the server's own and should always convert, so a failure here is a
/// server fault → `500 Internal Server Error` (ITS-REST
/// `Requests_and_responses.md` §HTTP status codes, row `500`).
fn flat_err(e: &openehr_flat::FlatError) -> RestError {
    RestError(ApiError::Internal(format!("FLAT conversion failed: {e}")))
}

/// A FLAT/STRUCTURED **input** conversion failure: the request body parsed as
/// JSON but does not conform to the target template's simplified-data-template
/// shape (the simSDT/structSDT formats, SM `simplified_im_b`). That is
/// well-formed-but-semantically-invalid *client* content, not a server fault →
/// `422 Unprocessable Entity` (ITS-REST `Requests_and_responses.md` §HTTP status
/// codes, row `422` — "The request was well-formed but was unable to be followed
/// due to semantic errors"; syntactically-invalid JSON is caught earlier as a
/// `400`).
fn flat_input_err(e: &openehr_flat::FlatError) -> RestError {
    RestError(ApiError::Unprocessable(format!(
        "FLAT conversion failed: {e}"
    )))
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
pub(crate) async fn composition_from_flat(
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
        .map_err(RestError::from)?;
    // Enforce the `|other` open-value-set MUST-rules on the FLAT input before
    // conversion (master02/master04 §"Open Value-Sets and the `|other` Suffix";
    // master05 §"When a `DV_CODED_TEXT` becomes a `DV_TEXT`"): `|other` must not
    // co-occur with `|code`/`|value`/`|terminology`/`|preferred_term`, and must
    // be rejected on a closed value-set.
    if let Some(v) = openehr_flat::validate_flat_other(&flat, &wt).first() {
        return Err(bad_request(format!("{}: {}", v.path, v.message)));
    }
    openehr_flat::from_flat(&flat, &wt).map_err(|e| flat_input_err(&e))
}

/// Render a canonical-JSON composition as a FLAT `application/openehr.wt.flat+json`
/// response (its template id read from `archetype_details/template_id`).
pub(crate) async fn composition_flat_response(
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
        .map_err(RestError::from)?;
    let flat = openehr_flat::to_flat(comp, &wt).map_err(|e| flat_err(&e))?;
    let json =
        serde_json::to_string(&flat).map_err(|e| internal(format!("FLAT serialization: {e}")))?;
    Ok(negotiate::flat_json_body(status, json))
}

/// Parse a STRUCTURED (structSDT) request body into a canonical-JSON
/// `COMPOSITION` via `openehr_flat::from_structured` (template id resolved as
/// for FLAT: query param or `openEHR-TEMPLATE_ID` header).
pub(crate) async fn composition_from_structured(
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
        .map_err(RestError::from)?;
    openehr_flat::from_structured(&structured, &wt).map_err(|e| flat_input_err(&e))
}

/// Render a canonical-JSON composition as a STRUCTURED
/// `application/openehr.wt.structured+json` response.
pub(crate) async fn composition_structured_response(
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
        .map_err(RestError::from)?;
    let structured = openehr_flat::to_structured(comp, &wt).map_err(|e| flat_err(&e))?;
    let json = serde_json::to_string(&structured)
        .map_err(|e| internal(format!("STRUCTURED serialization: {e}")))?;
    Ok(negotiate::structured_json_body(status, json))
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use axum::response::IntoResponse;
    use http::StatusCode;

    use super::{flat_err, flat_input_err};

    /// a FLAT/STRUCTURED **input** conversion failure is client data →
    /// `422`, not a `500` server fault (ITS-REST `Requests_and_responses.md`
    /// §HTTP status codes). `openehr_flat::from_flat` is presently infallible in
    /// practice, so this asserts the mapping directly at the seam.
    #[test]
    fn input_conversion_failure_maps_to_422() {
        let e = openehr_flat::FlatError::Conversion("bad leaf".to_owned());
        let status = flat_input_err(&e).into_response().status();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// an **output** conversion failure (rendering stored server data)
    /// stays a `500` — the server should always be able to convert its own data.
    #[test]
    fn output_conversion_failure_stays_500() {
        let e = openehr_flat::FlatError::Conversion("bad leaf".to_owned());
        let status = flat_err(&e).into_response().status();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
