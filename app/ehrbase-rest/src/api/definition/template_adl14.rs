//! ITS-REST **ADL 1.4 template** resource (`tags: ADL1.4`).
//!
//! Operations (`docs/specs/openehr/ITS-REST/specifications/operations/`):
//! `definition_template_adl1.4_list` / `_upload` / `_get` / `_example_get`.
//! Governing spec text: `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//! Register (gaps + target): `docs/design/its-rest/definition.md` (G-1/G-3/G-4/G-5).
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
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::Platform;

use super::dispatch::list_filter_and_page;

/// `GET …/definition/template/adl1.4` — list the stored OPT 1.4 templates.
///
/// The wire decodes `template_id` (glob), `concept` (glob), `version`
/// (version filter), `offset`, `fetch`
/// (`operations/definition_template_adl1.4_list.yaml`); they are threaded to the
/// adapter as a [`TemplateListFilter`](ehrbase_sm::extensions::adapters::TemplateListFilter)
/// + [`Page`](ehrbase_sm::Page) (G-1).
pub(super) async fn list<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
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
pub(super) async fn upload<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
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
        state.config().base_path,
        urlencoding::encode(&template_id)
    );
    Ok(upload_response(h, &location, &template_id, xml))
}

/// `GET …/definition/template/adl1.4/{template_id}` — retrieve the stored OPT.
///
/// Negotiates the `200_Template_adl1_4_retrieved` representations
/// (`application/xml` canonical OPT + the `application/openehr.wt+json` web
/// template EHRbase-compatible extension), sets the mandated `ETag` (G-4), and
/// returns `406` for an `Accept` outside `Accept_Template` (G-5). An unknown
/// template → `404` (checked first, before negotiation).
pub(super) async fn get<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionTemplateAdl14GetParams>(&parts.path, parts.query.as_deref(), h)?;
    let template_id = p.template_id.clone();
    // Resolve the `Accept` before touching storage so an unsupported one is a
    // clean `406` (`operations/definition_template_adl1.4_get.yaml` `406`).
    let accept = match negotiate_accept(h) {
        Some(a) => a,
        None => {
            return Err(RestError(ApiError::NotAcceptable(
                "the template is available as application/xml (canonical OPT) or \
                 application/openehr.wt+json (web template)"
                    .to_owned(),
            )));
        }
    };
    // Unknown template → 404, so the XML fetch runs regardless (it is the
    // existence probe as well as the canonical body).
    let xml = state
        .backend()
        .template_adl14_get(template_id.clone())
        .await?;
    match accept {
        TemplateAccept::WebTemplate => web_template_response(state, &template_id).await,
        TemplateAccept::Xml => {
            let mut resp = negotiate::xml_body(StatusCode::OK, xml);
            set_template_etag(&mut resp, &template_id);
            Ok(resp)
        }
    }
}

/// `GET …/definition/template/adl1.4/{template_id}/example` — a generated
/// example COMPOSITION, negotiated across the four `Accept_LOCATABLE` forms.
pub(super) async fn example_get<S: Platform>(
    state: &AppState<S>,
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
    if negotiate::wants_flat(h) {
        return super::flat::composition_flat_response(state, StatusCode::OK, &comp).await;
    }
    if negotiate::wants_structured(h) {
        return super::flat::composition_structured_response(state, StatusCode::OK, &comp).await;
    }
    if !example_accept_supported(h) {
        return Err(RestError(ApiError::NotAcceptable(
            "the template example is available as application/json, application/xml, \
             application/openehr.wt.flat+json, or application/openehr.wt.structured+json"
                .to_owned(),
        )));
    }
    // JSON (default) or canonical XML, via the single spec-typed COMPOSITION
    // path (`respond_rm` re-types the value so the generated `ToXml` runs).
    Ok(negotiate::respond_rm::<Composition>(
        h,
        StatusCode::OK,
        &comp,
        "composition",
    ))
}

/// The representation the ADL1.4 template GET should serve for a request's
/// `Accept`. `None` = an `Accept` outside `Accept_Template` → `406`.
enum TemplateAccept {
    /// The canonical `application/xml` OPT.
    Xml,
    /// The `application/openehr.wt+json` web template (EHRbase-compatible
    /// extension; also served for a bare `application/json`, the only JSON
    /// projection the server holds — the OPT canonical form is XML).
    WebTemplate,
}

/// Resolve a request `Accept` against the `Accept_Template` enum
/// (`parameters/header/Accept_Template.yaml`: `application/json`,
/// `application/xml`, `application/openehr.wt+json`) plus the `200`-response
/// content (`200_Template_adl1_4_retrieved.yaml`: xml + wt+json). Absent / `*/*`
/// / `application/*` default to the canonical XML; anything outside the enum is
/// `None` (→ `406`).
fn negotiate_accept(headers: &HeaderMap) -> Option<TemplateAccept> {
    let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) else {
        return Some(TemplateAccept::Xml); // absent → canonical XML
    };
    let mut acceptable: Option<TemplateAccept> = None;
    for range in accept.split(',') {
        let media = range.split(';').next().unwrap_or(range).trim();
        match media {
            "" | "*/*" | "application/*" | "application/xml" | "text/xml" => {
                return Some(TemplateAccept::Xml);
            }
            "application/openehr.wt+json" | "application/json" => {
                // Keep looking for a canonical-XML preference, but remember the
                // web-template match so a pure JSON/wt Accept still resolves.
                acceptable.get_or_insert(TemplateAccept::WebTemplate);
            }
            _ => {}
        }
    }
    acceptable
}

/// Set the weak `ETag` the retrieved/uploaded template responses mandate
/// (`headers/ETag_Template_adl1_4.yaml`: `W/"<id>"`). Keyed on the
/// `template_id` string, matching the upload path so a client's `If-None-Match`
/// round-trips.
///
// TODO(w3e-integrate): fold this weak-ETag construction into the central
// negotiate helper (`overview::negotiate`) so the upload
// (`template_upload_response`) and this GET share one implementation instead of
// two copies of the `W/"…"` format.
fn set_template_etag(resp: &mut Response, template_id: &str) {
    if let Ok(v) = HeaderValue::from_str(&format!("W/\"{template_id}\"")) {
        resp.headers_mut().insert(header::ETAG, v);
    }
}

/// Render the `201_Template_adl1_4_upload` response per `Prefer`:
/// `return=representation` → the OPT XML; `return=identifier` → the JSON
/// `TemplateIdentifier` object `{"template_id": <id>}` (G-3 — matching the ADL2
/// upload, `schemas/others/TemplateIdentifier.yaml`); missing / `return=minimal`
/// → an empty body. `Location` + the weak `ETag` are set on every case.
///
// TODO(w3e-integrate): the old shared `overview::negotiate::template_upload_response`
// returned the `return=identifier` body as a `text/plain` scalar (the G-3
// defect). This ADL1.4-local responder supersedes it; once no other caller
// depends on the old helper, remove it from `overview::negotiate` and route
// both upload paths through one shared builder.
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

/// Whether the request's `Accept` names one of the four representations the
/// `adl1.4/{id}/example` endpoint supports (dev-OAS `Accept_LOCATABLE`:
/// `application/json`, `application/xml`, `application/openehr.wt.flat+json`,
/// `application/openehr.wt.structured+json`). An absent `Accept` (or `*/*`)
/// defaults to JSON; anything else is a `406`.
///
/// The FLAT/STRUCTURED media types are resolved before this call, so here they
/// only need to keep a mixed `Accept` from being rejected.
fn example_accept_supported(headers: &HeaderMap) -> bool {
    let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) else {
        return true; // absent → canonical JSON
    };
    accept.split(',').any(|range| {
        let media = range.split(';').next().unwrap_or(range).trim();
        matches!(
            media,
            "" | "*/*"
                | "application/*"
                | "application/json"
                | "application/xml"
                | "text/xml"
                | "application/openehr.wt.flat+json"
                | "application/openehr.wt.structured+json"
        )
    })
}

/// Serve the service-owned Better `WebTemplate` for `template_id` as
/// `application/openehr.wt+json` (single resolution seam:
/// [`ehrbase_sm::services::WebTemplateService`] — W2-K/F-13-02).
///
/// Serving `wt+json` on the spec `adl1.4/{id}` GET endpoint is a deliberate
/// EHRbase-compatible extension (openEHR ITS-REST returns only the OPT itself).
async fn web_template_response<S: Platform>(
    state: &AppState<S>,
    template_id: &str,
) -> Result<Response, RestError> {
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
