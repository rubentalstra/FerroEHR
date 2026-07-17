//! ITS-REST **ADL 1.4 template** resource (`tags: ADL1.4`).
//!
//! Operations (`docs/specs/openehr/ITS-REST/specifications/operations/`):
//! `definition_template_adl1.4_list` / `_upload` / `_get` / `_example_get`.
//! Governing spec text: `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//! Governing spec: `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//!
//! The wire addresses OPTs by their `template_id` string; the SM `get_opt` is
//! UUID-keyed, so retrieval runs through the `DefinitionAdapter` extension
//! (`template_adl14_get`), while list/upload/example likewise route through the
//! wire-shaped adapter (rich summary objects + a generated example COMPOSITION
//! the SM `I_DEFINITION_ADL14` interface does not express).

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, HeaderValue, StatusCode, header};

use openehr_its::rest::generated::definition::{
    DefinitionTemplateAdl14ExampleGetParams, DefinitionTemplateAdl14GetParams,
    DefinitionTemplateAdl14ListParams, DefinitionTemplateAdl14UploadParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use crate::api::RequestParts;
use crate::negotiate::WireFormat;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};

use super::dispatch::list_filter_and_page;

/// The four `Accept_LOCATABLE` representations of a generated example
/// COMPOSITION: canonical JSON/XML + FLAT/STRUCTURED.
const EXAMPLE_FORMATS: &[WireFormat] = &[
    WireFormat::CanonicalJson,
    WireFormat::CanonicalXml,
    WireFormat::Flat,
    WireFormat::Structured,
];

/// The template-definition GET representations (`Accept_template` +
/// `200_Template_adl1_4_retrieved`): the canonical OPT is `application/xml`;
/// `application/openehr.wt+json` (and a bare `application/json`, the only JSON
/// projection the server holds) return the Web Template document.
const TEMPLATE_DEF_FORMATS: &[WireFormat] = &[
    WireFormat::CanonicalXml,
    WireFormat::WebTemplate,
    WireFormat::CanonicalJson,
];

/// `GET …/definition/template/adl1.4` — list the stored OPT 1.4 templates.
///
/// The wire decodes `template_id` (glob), `concept` (glob), `version`
/// (version filter), `offset`, `fetch`
/// (`operations/definition_template_adl1.4_list.yaml`); they are threaded to the
/// adapter as a [`TemplateListFilter`](ehrbase::service::adapters::TemplateListFilter)
/// + [`Page`](ehrbase::service::list::Page).
pub(super) async fn list(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl14ListParams>(&parts.path, parts.query.as_deref(), h)?;
    let (filter, page) =
        list_filter_and_page(p.template_id, p.concept, p.version, p.offset, p.fetch);
    Ok(negotiate::respond(
        h,
        StatusCode::OK,
        &state.backend().template_adl14_list(filter, page).await?,
    ))
}

/// `POST …/definition/template/adl1.4` — ingest an OPT 1.4 canonical-XML
/// template (`operations/definition_template_adl1.4_upload.yaml`).
pub(super) async fn upload(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    params::build::<DefinitionTemplateAdl14UploadParams>(&parts.path, parts.query.as_deref(), h)?;
    // The OPT 1.4 template arrives as canonical XML; the lenient reader hands it
    // back as a JSON string, which the service parses (opt14).
    let body = negotiate::lenient_value(&parts.body)?;
    let xml = body.as_str().ok_or_else(|| {
        RestError(ApiError::BadRequest(
            "expected an OPT 1.4 XML template body".to_owned(),
        ))
    })?;
    let meta = state
        .backend()
        .template_adl14_upload(xml.to_owned())
        .await?;
    let template_id = meta
        .get("template_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let location = format!(
        "{}/definition/template/adl1.4/{}",
        state.config().server.base_path,
        urlencoding::encode(&template_id)
    );
    Ok(upload_response(h, &location, &template_id, xml))
}

/// `GET …/definition/template/adl1.4/{template_id}` — retrieve the stored OPT.
///
/// Negotiates the `200_Template_adl1_4_retrieved` representations
/// (`application/xml` canonical OPT + the `application/openehr.wt+json` web
/// template EHRbase-compatible extension), sets the mandated `ETag`, and
/// returns `406` for an `Accept` outside `Accept_Template`. An unknown
/// template → `404` (checked first, before negotiation).
pub(super) async fn get(state: &AppState, parts: &RequestParts) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl14GetParams>(&parts.path, parts.query.as_deref(), h)?;
    let template_id = p.template_id.clone();
    // Resolve the `Accept` before touching storage so an unsupported one is a
    // clean `406` (`operations/definition_template_adl1.4_get.yaml` `406`).
    // Absent / `*/*` default to the canonical OPT (application/xml).
    let Some(fmt) = negotiate::resolve_accept(h, TEMPLATE_DEF_FORMATS, WireFormat::CanonicalXml)
    else {
        return Err(RestError(ApiError::NotAcceptable(
            "the template is available as application/xml (canonical OPT), \
             application/openehr.wt+json, or application/json (web template)"
                .to_owned(),
        )));
    };
    // Unknown template → 404, so the XML fetch runs regardless (it is the
    // existence probe as well as the canonical body).
    let xml = state
        .backend()
        .template_adl14_get(template_id.clone())
        .await?;
    match fmt {
        WireFormat::CanonicalXml => {
            let mut resp = negotiate::xml_body(StatusCode::OK, xml);
            set_template_etag(&mut resp, &template_id);
            Ok(resp)
        }
        // `application/openehr.wt+json` and a bare `application/json` both serve
        // the Web Template document (the only JSON projection of an OPT).
        _ => web_template_response(state, &template_id).await,
    }
}

/// `GET …/definition/template/adl1.4/{template_id}/example` — a generated
/// example COMPOSITION, negotiated across the four `Accept_LOCATABLE` forms.
pub(super) async fn example_get(
    state: &AppState,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DefinitionTemplateAdl14ExampleGetParams>(
        &parts.path,
        parts.query.as_deref(),
        h,
    )?;
    // The backend generates the canonical example COMPOSITION (an unknown
    // template → 404; an invalid `type`/`detail_level` → 400).
    let comp = state
        .backend()
        .template_adl14_example(p.template_id, p.detail_level, p.r#type)
        .await?;
    // Negotiate the four representations the dev-OAS `Accept_LOCATABLE`
    // enumerates (json / xml / wt.flat+json / wt.structured+json). Any other
    // media type is a `406` (the endpoint's `406` response).
    match negotiate::resolve_accept(h, EXAMPLE_FORMATS, WireFormat::CanonicalJson) {
        Some(WireFormat::Flat) => {
            crate::formats::dispatch::composition_flat_response(state, StatusCode::OK, &comp).await
        }
        Some(WireFormat::Structured) => {
            crate::formats::dispatch::composition_structured_response(state, StatusCode::OK, &comp)
                .await
        }
        // JSON (default) or canonical XML, via the single spec-typed COMPOSITION
        // path (`respond_rm` re-types the value so the generated `ToXml` runs).
        Some(WireFormat::CanonicalJson | WireFormat::CanonicalXml) => {
            Ok(negotiate::respond_rm::<Composition>(
                h,
                StatusCode::OK,
                &comp,
                "composition",
            ))
        }
        _ => Err(RestError(ApiError::NotAcceptable(
            "the template example is available as application/json, application/xml, \
             application/openehr.wt.flat+json, or application/openehr.wt.structured+json"
                .to_owned(),
        ))),
    }
}

/// Set the weak `ETag` the retrieved/uploaded template responses mandate
/// (`headers/ETag_Template_adl1_4.yaml`: `W/"<id>"`). Keyed on the
/// `template_id` string, matching the upload path so a client's `If-None-Match`
/// round-trips. The weak-form construction goes through the shared
/// [`negotiate::resource_etag`] helper (overview §"Deprecated headers": a
/// resource-identifier `ETag` MUST carry the `W/` weakness indicator), so the
/// upload and this GET share one implementation of the `W/"…"` format.
fn set_template_etag(resp: &mut Response, template_id: &str) {
    if let Some(v) = negotiate::resource_etag(template_id) {
        resp.headers_mut().insert(header::ETAG, v);
    }
}

/// Render the `201_Template_adl1_4_upload` response per `Prefer`:
/// `return=representation` → the OPT XML; `return=identifier` → the JSON
/// `TemplateIdentifier` object `{"template_id": <id>}`
/// (`schemas/others/TemplateIdentifier.yaml`); missing / `return=minimal` → an
/// empty body. `Location` + the weak `ETag` are set on every case.
fn upload_response(
    headers: &HeaderMap,
    location: &str,
    template_id: &str,
    opt_xml: &str,
) -> Response {
    let prefer = headers
        .get("prefer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let has = |token: &str| {
        prefer
            .split(',')
            .any(|t| t.trim().eq_ignore_ascii_case(token))
    };
    let mut resp = if has("return=representation") {
        negotiate::xml_body(StatusCode::CREATED, opt_xml.to_owned())
    } else if has("return=identifier") {
        let body = serde_json::json!({ "template_id": template_id });
        (StatusCode::CREATED, axum::Json(body)).into_response()
    } else {
        StatusCode::CREATED.into_response()
    };
    if let Ok(v) = HeaderValue::from_str(location) {
        resp.headers_mut().insert(header::LOCATION, v);
    }
    set_template_etag(&mut resp, template_id);
    resp
}

/// Serve the service-owned Better `WebTemplate` for `template_id` as
/// `application/openehr.wt+json` (single resolution seam via
/// `state.backend().web_template(..)`).
///
/// Serving `wt+json` on the spec `adl1.4/{id}` GET endpoint is a deliberate
/// EHRbase-compatible extension (openEHR ITS-REST returns only the OPT itself).
async fn web_template_response(state: &AppState, template_id: &str) -> Result<Response, RestError> {
    let built = state
        .backend()
        .web_template(template_id)
        .await
        .map_err(RestError::from)?;
    let json = serde_json::to_string(&*built).map_err(|e| {
        RestError(ApiError::Internal(format!(
            "WebTemplate JSON serialization failed: {e}"
        )))
    })?;
    let mut resp = negotiate::wt_json_body(StatusCode::OK, json);
    set_template_etag(&mut resp, template_id);
    Ok(resp)
}
