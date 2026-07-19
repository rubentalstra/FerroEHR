//! ITS-REST **ADL2 template** resource (`tags: ADL2`).
//!
//! Operations (`docs/specs/openehr/ITS-REST/specifications/operations/`):
//! `definition_template_adl2_list` / `_upload` / `_get` / `_example_get` /
//! `_version_get`. Governing spec text:
//! `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//!
//! ADL2 artefacts are served as `text/plain` source
//! (`200_Template_adl2_retrieved.yaml`: `text/plain` `OperationalTemplateV2 |
//! string`); the `get`/`upload` route through the SM-2 `adl2_artefact` store
//! (`DefinitionAdl2Service::get_artefact`) + the wire-shaped `DefinitionAdapter`
//! (`template_adl2_upload`/`template_adl2_list`). The `example` and `version`
//! operations stay `501` (they need an example generator / a cADL source parser
//! the tree lacks — deferred to the planned full-ADL2 work).

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, HeaderValue, StatusCode, header};

use openehr_its::rest::generated::definition::{
    DefinitionTemplateAdl2ExampleGetParams, DefinitionTemplateAdl2GetParams,
    DefinitionTemplateAdl2ListParams, DefinitionTemplateAdl2UploadParams,
    DefinitionTemplateAdl2VersionGetParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};

use super::dispatch::list_filter_and_page;

/// `GET …/definition/template/adl2` — list the stored ADL2 templates, with the
/// `template_id`/`concept`/`version`/`offset`/`fetch` filter + pagination the
/// wire decodes (`operations/definition_template_adl2_list.yaml`) threaded to
/// the adapter.
pub(super) async fn list(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl2ListParams>(&parts.path, parts.query.as_deref(), h)?;
    let (filter, page) =
        list_filter_and_page(p.template_id, p.concept, p.version, p.offset, p.fetch);
    Ok(negotiate::respond(
        h,
        StatusCode::OK,
        &state.backend().template_adl2_list(filter, page).await?,
    ))
}

/// `POST …/definition/template/adl2` — ingest an ADL2 operational-template
/// `text/plain` source (`operations/definition_template_adl2_upload.yaml`).
///
/// NOTE: the `at_version` (`version`) query parameter is
/// `deprecated: true` (`parameters/query/at_version.yaml`); dropping it is
/// spec-permitted, so only `Prefer` is read. Recorded as residue, not a defect.
pub(super) async fn upload(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DefinitionTemplateAdl2UploadParams>(
        &parts.path,
        parts.query.as_deref(),
        h,
    )?;
    let prefer = p.prefer.clone();
    // ADL2 arrives as text/plain source (dev-OAS: Content-Type text/plain, body
    // `OperationalTemplateV2` | string).
    let source = negotiate::text_body(&parts.body)?;
    // Store; the adapter returns the stored ARCHETYPE_HRID (an invalid artefact
    // is a 422, a duplicate is handled by replace — SM upload).
    let hrid = state.backend().template_adl2_upload(source.clone()).await?;
    let hrid = hrid.as_str();
    let location = format!(
        "{}/definition/template/adl2/{hrid}",
        state.config().server.base_path
    );
    // 201_Template_adl2_upload: body per `Prefer` — representation → the OPT
    // source (text/plain); identifier → `{template_id}` (JSON); missing/minimal
    // → empty. `Location` on every case.
    Ok(upload_response(prefer.as_deref(), &location, hrid, source))
}

/// `GET …/definition/template/adl2/{template_id}` — the stored ADL2 source.
///
/// Serves the source as `text/plain` via the SM `get_artefact` seam (an unknown
/// `template_id` → 404). `406`s an `Accept` outside `Accept_Template_adl2` that
/// this build cannot serve: the `application/json`
/// `OperationalTemplateV2` and `application/xml` forms need a cADL parser
/// (deferred to the planned full-ADL2 work), so a request that names *only* one of those is a
/// `406` rather than a wrong body.
pub(super) async fn get(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl2GetParams>(&parts.path, parts.query.as_deref(), h)?;
    if !accepts_text(h) {
        // NOTE: `Accept_Template_adl2` also enumerates
        // `application/json` (the `OperationalTemplateV2` JSON projection) and
        // `application/xml`; neither is produced until the cADL parser lands, so
        // an Accept naming only those is a `406`, not a wrong-format body.
        return Err(RestError(ApiError::NotAcceptable(
            "the ADL2 template is served as text/plain source; the application/json \
             OperationalTemplateV2 and application/xml projections are not yet available"
                .to_owned(),
        )));
    }
    let source = state.backend().get_artefact(p.template_id).await?;
    Ok(text_response(StatusCode::OK, None, source))
}

/// `GET …/definition/template/adl2/{template_id}/example` — `501`.
///
/// NOTE: needs an example generator over a cADL/AOM2 source
/// model (none in the tree). ADL2 is OPTIONAL for CNF; the example generator
/// lands with the planned full-ADL2 work.
pub(super) fn example_get(parts: &RequestParts) -> Result<Response, RestError> {
    params::build::<DefinitionTemplateAdl2ExampleGetParams>(
        &parts.path,
        parts.query.as_deref(),
        &parts.headers,
    )?;
    Err(RestError(ApiError::NotImplemented))
}

/// `GET …/definition/template/adl2/{template_id}/version/{version}` — `501`.
///
/// NOTE: needs a cADL source parser for the JSON
/// `OperationalTemplateV2` form; the operation is `deprecated: true`
/// (`operations/definition_template_adl2_version_get.yaml`) and ADL2 is OPTIONAL
/// for CNF.
pub(super) fn version_get(parts: &RequestParts) -> Result<Response, RestError> {
    params::build::<DefinitionTemplateAdl2VersionGetParams>(
        &parts.path,
        parts.query.as_deref(),
        &parts.headers,
    )?;
    Err(RestError(ApiError::NotImplemented))
}

/// Whether the request's `Accept` can be served by the `text/plain` ADL2 source
/// (`parameters/header/Accept_Template_adl2.yaml`). Absent / `*/*` / `text/*`
/// → yes; an Accept naming only `application/json`/`application/xml` (the
/// not-yet-produced projections) → no (→ `406`).
fn accepts_text(headers: &HeaderMap) -> bool {
    let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) else {
        return true; // absent → text/plain source
    };
    accept.split(',').any(|range| {
        let media = range.split(';').next().unwrap_or(range).trim();
        matches!(media, "" | "*/*" | "text/*" | "text/plain")
    })
}

/// A `text/plain` response for ADL2 source (the artefact interchange form), with
/// an optional `Location` header. ADL2 artefacts are text, so both the `adl2`
/// GET body and the `Prefer: return=representation` upload body are served this
/// way (dev-OAS: `text/plain` for the ADL2 OPT).
fn text_response(status: StatusCode, location: Option<&str>, source: String) -> Response {
    // axum's `String` responder already sets `Content-Type: text/plain; charset=utf-8`.
    let mut resp = (status, source).into_response();
    if let Some(loc) = location
        && let Ok(value) = HeaderValue::from_str(loc)
    {
        resp.headers_mut().insert(header::LOCATION, value);
    }
    resp
}

/// Render the `201 Created` for an ADL2 template upload per `Prefer`
/// (`201_Template_adl2_upload`): `return=representation` → the OPT source
/// (text/plain); `return=identifier` → a `{template_id}` JSON body
/// (`schemas/others/TemplateIdentifier.yaml`); missing/`return=minimal` → an
/// empty body. `Location` is set on every case.
fn upload_response(prefer: Option<&str>, location: &str, hrid: &str, source: String) -> Response {
    let has = |token: &str| {
        prefer.is_some_and(|p| p.split(',').any(|t| t.trim().eq_ignore_ascii_case(token)))
    };
    if has("return=representation") {
        text_response(StatusCode::CREATED, Some(location), source)
    } else if has("return=identifier") {
        let body = serde_json::json!({ "template_id": hrid });
        let mut resp = (StatusCode::CREATED, axum::Json(body)).into_response();
        if let Ok(value) = HeaderValue::from_str(location) {
            resp.headers_mut().insert(header::LOCATION, value);
        }
        resp
    } else {
        negotiate::empty_with_location(StatusCode::CREATED, location)
    }
}
