//! ITS-REST **ADL2 template** resource (`tags: ADL2`).
//!
//! Operations (`docs/specs/openehr/ITS-REST/specifications/operations/`):
//! `definition_template_adl2_list` / `_upload` / `_get` / `_example_get` /
//! `_version_get`. Governing spec text:
//! `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//!
//! ADL2 artefacts are validated + compiled by the `openehr-adl` engine through
//! the service layer (`EhrbaseService::adl2_*`). The `_get`/`_version_get`
//! operations serve two representations of the stored operational template:
//! `text/plain` — the stored ADL2 **source** verbatim
//! (`200_Template_adl2_retrieved.yaml` body `oneOf: [OperationalTemplateV2,
//! string]`, whose example is ADL2 source) — and `application/json` — the
//! `OperationalTemplateV2` canonical-JSON projection (the OAS declares that
//! schema as an opaque `type: object`, `schemas/aom/OperationalTemplateV2.yaml`,
//! so the AOM2 canonical JSON of the OPT satisfies it). `application/xml` is
//! enumerated by the `Accept` header but the response declares **no**
//! `application/xml` body content, so an `Accept` naming only XML is a `406`.

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

/// The representation a `GET …/adl2/{template_id}[/{version}]` request resolves
/// to, per its `Accept` header (`parameters/header/Accept_Template_adl2.yaml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Adl2Repr {
    /// `text/plain` — the stored ADL2 source verbatim (also the default for
    /// absent / `*/*` / `text/*`).
    Source,
    /// `application/json` — the `OperationalTemplateV2` canonical-JSON
    /// projection.
    Json,
}

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
/// `text/plain` source (`operations/definition_template_adl2_upload.yaml`),
/// validated by the `openehr-adl` engine.
///
/// NOTE: the `at_version` (`version`) query parameter is `deprecated: true`
/// (`parameters/query/at_version.yaml`); dropping it is spec-permitted, so only
/// `Prefer` is read. Recorded as residue, not a defect.
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
    // The engine validates: an unparseable source is a `400` (BadRequest), an
    // AOM2-invalid one a `422` carrying the rule codes (ValidationFailed), a
    // duplicate HRID a `409` (Conflict) — all via `ServiceError` so the `422`
    // renders the ITS-REST `Error` object with per-code `validationErrors`.
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

/// `GET …/definition/template/adl2/{template_id}` — the stored operational
/// template.
///
/// Resolves `template_id` (full HRID, or a partial that selects the latest
/// matching version) and serves the representation `Accept` negotiates:
/// `text/plain` source, `application/json` `OperationalTemplateV2`, or `406`
/// when only `application/xml` (which the response declares no body for) is
/// acceptable. Unknown `template_id` → `404`.
pub(super) async fn get(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl2GetParams>(&parts.path, parts.query.as_deref(), h)?;
    render(state, h, p.template_id, None).await
}

/// `GET …/definition/template/adl2/{template_id}/{version}` — the stored
/// operational template at an explicit SEMVER `version`
/// (`operations/definition_template_adl2_version_get.yaml`; `deprecated: true`
/// in the OAS but spec-declared, so served). `version` is an exact version or a
/// `{major}`/`{major}.{minor}` prefix resolving to the highest match; a missing
/// template/version → `404`.
pub(super) async fn version_get(
    state: &AppState,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DefinitionTemplateAdl2VersionGetParams>(
        &parts.path,
        parts.query.as_deref(),
        h,
    )?;
    render(state, h, p.template_id, Some(p.version)).await
}

/// `GET …/definition/template/adl2/{template_id}/example` — `501`.
///
/// TODO: generate a spec-valid example instance (COMPOSITION/…) from the ADL2
/// operational template — the same generator issue #94 builds by walking a
/// `WebTemplate`; it needs an `am24`-OPT → `WebTemplate` builder the tree does not
/// have yet, so this is not a bounded add here. ADL2 is OPTIONAL for CNF.
pub(super) fn example_get(parts: &RequestParts) -> Result<Response, RestError> {
    params::build::<DefinitionTemplateAdl2ExampleGetParams>(
        &parts.path,
        parts.query.as_deref(),
        &parts.headers,
    )?;
    Err(RestError(ApiError::NotImplemented))
}

/// Resolve + render one ADL2 template in the `Accept`-negotiated representation.
async fn render(
    state: &AppState,
    headers: &HeaderMap,
    template_id: String,
    version: Option<String>,
) -> Result<Response, RestError> {
    match negotiate_get(headers) {
        Some(Adl2Repr::Source) => {
            let source = state
                .backend()
                .template_adl2_source(template_id, version)
                .await?;
            Ok(text_response(StatusCode::OK, None, source))
        }
        Some(Adl2Repr::Json) => {
            let json = state
                .backend()
                .template_adl2_opt_json(template_id, version)
                .await?;
            Ok(json_response(StatusCode::OK, json))
        }
        None => Err(RestError(ApiError::NotAcceptable(
            "the ADL2 template is served as text/plain source or application/json \
             OperationalTemplateV2; application/xml has no declared response body"
                .to_owned(),
        ))),
    }
}

/// Choose the `GET` representation from `Accept`
/// (`parameters/header/Accept_Template_adl2.yaml` enumerates `text/plain`,
/// `application/json`, `application/xml`). Absent / `*/*` / `text/*` /
/// `text/plain` → [`Adl2Repr::Source`]; `application/json` (or `application/*`)
/// with text not acceptable → [`Adl2Repr::Json`]; only `application/xml` → `None`
/// (→ `406`, the response declares no XML body).
fn negotiate_get(headers: &HeaderMap) -> Option<Adl2Repr> {
    let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) else {
        return Some(Adl2Repr::Source); // absent → text/plain source
    };
    if accept.trim().is_empty() {
        return Some(Adl2Repr::Source);
    }
    let mut source_ok = false;
    let mut json_ok = false;
    for range in accept.split(',') {
        // A `;q=0` explicitly rejects the range (RFC 9110 §12.5.1).
        let mut parts = range.split(';');
        let media = parts.next().unwrap_or(range).trim();
        if parts.any(|p| p.trim().eq_ignore_ascii_case("q=0")) {
            continue;
        }
        match media {
            "" | "*/*" | "text/*" | "text/plain" => source_ok = true,
            "application/json" | "application/*" => json_ok = true,
            _ => {}
        }
    }
    // text/plain is the canonical ADL2 interchange form, so it wins when both
    // are acceptable; JSON only when text is not.
    if source_ok {
        Some(Adl2Repr::Source)
    } else if json_ok {
        Some(Adl2Repr::Json)
    } else {
        None
    }
}

/// A `text/plain` response for ADL2 source (the artefact interchange form), with
/// an optional `Location` header.
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

/// An `application/json` response carrying a pre-serialized
/// `OperationalTemplateV2` canonical-JSON body.
fn json_response(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
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
