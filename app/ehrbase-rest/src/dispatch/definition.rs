//! HTTP dispatch for the `definition` API group (templates + stored queries).
//!
//! Each arm rebuilds the operation's `*Params`, decodes any body, calls the
//! trait method on [`AppState`], and renders a negotiated response. ADL2
//! template `upload`/`list`/`get` are backed by the SM-2 `adl2_artefact` store
//! (ADL2 artefacts are served as text/plain source — the `get` routes through
//! the SM `DefinitionAdl2Service::get_artefact` seam). The ADL2 `example` and
//! `version` operations stay `501` (they need an example generator / a cADL
//! source parser — see `service::api::definition`).
//!
//! Note: the generated `ROUTES` operation ids carry dots (e.g.
//! `definition_template_adl1.4_list`, `definition_query_store.yaml`); the match
//! keys below are those exact strings, while the trait methods called are the
//! underscored names (`definition_template_adl1_4_list`, `definition_query_store_yaml`).

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, HeaderValue, StatusCode, header};

use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

// The generated `*Params` decode the wire path/query; the operation logic is
// dispatched onto the SM catalog traits (`DefinitionAdl14Service` /
// `DefinitionAdl2Service`) + the wire-shaped `DefinitionAdapter` extension —
// the generated `DefinitionApi` is no longer part of `Platform` (ADR-011).
use openehr_its::rest::generated::definition::{
    DefinitionQueryListParams, DefinitionQueryStoreYamlParams, DefinitionQueryVersionGetParams,
    DefinitionQueryVersionStoreYamlParams, DefinitionTemplateAdl2ExampleGetParams,
    DefinitionTemplateAdl2GetParams, DefinitionTemplateAdl2ListParams,
    DefinitionTemplateAdl2UploadParams, DefinitionTemplateAdl2VersionGetParams,
    DefinitionTemplateAdl14ExampleGetParams, DefinitionTemplateAdl14GetParams,
    DefinitionTemplateAdl14ListParams, DefinitionTemplateAdl14UploadParams,
};

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
// `DefinitionAdapter` + the SM `I_DEFINITION_*` methods called below resolve
// through the `Platform` supertrait bound (their methods are reachable with
// `Platform` in scope), so only `Platform` needs importing here.
use ehrbase_sm::Platform;

use crate::state::AppState;
use crate::{negotiate, params};

pub(super) fn dispatch<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

#[allow(clippy::too_many_lines)] // one arm per operation; a flat match is clearest
async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();

    match op {
        "definition_template_adl1.4_list" => {
            params::build::<DefinitionTemplateAdl14ListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().template_adl14_list().await?,
            ))
        }
        "definition_template_adl1.4_upload" => {
            params::build::<DefinitionTemplateAdl14UploadParams>(&parts.path, q, h)?;
            // The OPT 1.4 template arrives as canonical XML; the lenient reader
            // hands it back as a JSON string, which the service parses (opt14).
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
            // `201_Template_adl1_4_upload`: the endpoint produces
            // `application/xml` only — a `return=representation` body is the
            // OPT itself; `return=identifier` → the template id; missing/
            // `return=minimal` → empty. `Location` + `ETag` carry the
            // template id on every case.
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
            Ok(negotiate::template_upload_response(
                h,
                &location,
                &template_id,
                xml,
            ))
        }
        "definition_template_adl1.4_get" => {
            let p = params::build::<DefinitionTemplateAdl14GetParams>(&parts.path, q, h)?;
            let template_id = p.template_id.clone();
            // The wire addresses the OPT by its `template_id` string, so this
            // runs through the adapter's `template_adl14_get` (the SM `get_opt`
            // is UUID-keyed — see the trait PORT NOTE). An unknown template →
            // 404, so it runs first. For a Better `wt+json` Accept, serve the
            // service-owned WebTemplate (the one cache, shared with validation +
            // FLAT — W2-K/F-13-02); otherwise serve the OPT XML verbatim (the
            // canonical artifact).
            let xml = state
                .backend()
                .template_adl14_get(template_id.clone())
                .await?;
            if negotiate::wants_web_template(h) {
                web_template_response(&state, &template_id).await
            } else {
                Ok(negotiate::xml_body(StatusCode::OK, xml))
            }
        }
        "definition_template_adl1.4_example_get" => {
            let p = params::build::<DefinitionTemplateAdl14ExampleGetParams>(&parts.path, q, h)?;
            // The backend generates the canonical example COMPOSITION (an unknown
            // template → 404; an invalid `type`/`detail_level` → 400).
            let comp = state
                .backend()
                .template_adl14_example(p.template_id, p.detail_level, p.r#type)
                .await?;
            // Negotiate the four representations the dev-OAS `Accept_LOCATABLE`
            // enumerates (json / xml / wt.flat+json / wt.structured+json). The
            // FLAT/STRUCTURED converters reach the WebTemplate through the same
            // `WebTemplateService` seam as `dispatch::flat` (the example carries
            // its own `archetype_details/template_id`). Any other media type is a
            // `406` (the endpoint's `406` response).
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, StatusCode::OK, &comp).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, StatusCode::OK, &comp)
                    .await;
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
        "definition_template_adl2_list" => {
            params::build::<DefinitionTemplateAdl2ListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().template_adl2_list().await?,
            ))
        }
        "definition_template_adl2_upload" => {
            let p = params::build::<DefinitionTemplateAdl2UploadParams>(&parts.path, q, h)?;
            let prefer = p.prefer.clone();
            // ADL2 arrives as text/plain source (definition-codegen.openapi.yaml:
            // Content-Type text/plain, body `OperationalTemplateV2` | string).
            let source = negotiate::text_body(&parts.body)?;
            // Store; the adapter returns the stored ARCHETYPE_HRID (an invalid
            // artefact is a 422, a duplicate is handled by replace — SM upload).
            let hrid = state.backend().template_adl2_upload(source.clone()).await?;
            let hrid = hrid.as_str();
            let location = format!(
                "{}/definition/template/adl2/{hrid}",
                state.config().base_path
            );
            // 201_Template_adl2_upload: body per `Prefer` — representation → the
            // OPT source (text/plain); identifier → `{template_id}` (JSON);
            // missing/minimal → empty. `Location` on every case.
            Ok(adl2_upload_response(
                prefer.as_deref(),
                &location,
                hrid,
                source,
            ))
        }
        "definition_template_adl2_get" => {
            let p = params::build::<DefinitionTemplateAdl2GetParams>(&parts.path, q, h)?;
            // ADL2 artefacts are text; serve the stored source as text/plain via
            // the SM `get_artefact` seam (an unknown `template_id` → 404). The
            // generated map-returning op models the JSON `OperationalTemplateV2`
            // form, which needs a cADL parser (deferred) — see
            // `service::api::definition`.
            let source = state.backend().get_artefact(p.template_id).await?;
            Ok(adl2_text_response(StatusCode::OK, None, source))
        }
        "definition_template_adl2_example_get" => {
            params::build::<DefinitionTemplateAdl2ExampleGetParams>(&parts.path, q, h)?;
            // Needs an example generator over a cADL/AOM2 source model (none in
            // the tree) — stays `501` (ADL2 is OPTIONAL for CNF, untested).
            Err(RestError(ApiError::NotImplemented))
        }
        "definition_template_adl2_version_get" => {
            params::build::<DefinitionTemplateAdl2VersionGetParams>(&parts.path, q, h)?;
            // Needs a cADL source parser for the JSON `OperationalTemplateV2`
            // form — stays `501` (ADL2 is OPTIONAL for CNF, untested).
            Err(RestError(ApiError::NotImplemented))
        }
        "definition_query_list" => {
            let p = params::build::<DefinitionQueryListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().query_list(p.qualified_query_name).await?,
            ))
        }
        "definition_query_store.yaml" => {
            let p = params::build::<DefinitionQueryStoreYamlParams>(&parts.path, q, h)?;
            let name = p.qualified_query_name.clone();
            let body = negotiate::text_body(&parts.body)?;
            state
                .backend()
                .query_store(name.clone(), None, body)
                .await?;
            // Spec: the store success is `200 OK` (not `204`), with a `Location`
            // for the stored resource (`responses/200_StoredQuery_stored.yaml` +
            // `headers/Location_Query.yaml`). The no-version store auto-assigns
            // the SEMVER but the generated trait method is bodyless (`()`), so
            // the assigned version is recovered through the list seam: exact-name
            // rows come back ordered by version ascending, so the last one is the
            // version this store just wrote (or upserted).
            match stored_version_of(&state, &name, h).await {
                Some(version) => {
                    let location = format!(
                        "{}/definition/query/{name}/{version}",
                        state.config().base_path
                    );
                    Ok(negotiate::empty_with_location(StatusCode::OK, &location))
                }
                None => Ok(negotiate::empty(StatusCode::OK)),
            }
        }
        "definition_query_version_get" => {
            let p = params::build::<DefinitionQueryVersionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .query_version_get(p.qualified_query_name, p.version)
                    .await?,
            ))
        }
        "definition_query_version_store.yaml" => {
            let p = params::build::<DefinitionQueryVersionStoreYamlParams>(&parts.path, q, h)?;
            // The version is stored verbatim, so it is the effective SEMVER the
            // `Location` header points at.
            let name = p.qualified_query_name.clone();
            let version = p.version.clone();
            let body = negotiate::text_body(&parts.body)?;
            state
                .backend()
                .query_store(name.clone(), Some(version.clone()), body)
                .await?;
            // Spec: `200 OK` with a `Location` header for the stored resource
            // (`responses/200_StoredQuery_stored.yaml` + `headers/Location_Query.yaml`).
            let location = format!(
                "{}/definition/query/{name}/{version}",
                state.config().base_path
            );
            Ok(negotiate::empty_with_location(StatusCode::OK, &location))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted definition operation: {other}"),
        ))),
    }
}

/// A `text/plain` response for ADL2 source (the artefact interchange form),
/// with an optional `Location` header. ADL2 artefacts are text, so both the
/// `adl2` GET body and the `Prefer: return=representation` upload body are served
/// this way (`definition-codegen.openapi.yaml`: `text/plain` for the ADL2 OPT).
fn adl2_text_response(status: StatusCode, location: Option<&str>, source: String) -> Response {
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
/// (text/plain); `return=identifier` → a `{template_id}` JSON body;
/// missing/`return=minimal` → an empty body. `Location` is set on every case.
fn adl2_upload_response(
    prefer: Option<&str>,
    location: &str,
    hrid: &str,
    source: String,
) -> Response {
    let representation = prefer.is_some_and(|p| {
        p.split(',')
            .any(|t| t.trim().eq_ignore_ascii_case("return=representation"))
    });
    let identifier = prefer.is_some_and(|p| {
        p.split(',')
            .any(|t| t.trim().eq_ignore_ascii_case("return=identifier"))
    });
    if representation {
        adl2_text_response(StatusCode::CREATED, Some(location), source)
    } else if identifier {
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

/// The stored SEMVER of the stored query `name` after a no-version store: the
/// exact-name entries from the list seam (ordered by version ascending), taking
/// the highest. `None` when the lookup fails or finds nothing — the store
/// itself already succeeded, so the response degrades to Location-less rather
/// than failing the request.
async fn stored_version_of<S: Platform>(
    state: &AppState<S>,
    name: &str,
    _headers: &HeaderMap,
) -> Option<String> {
    let list = state.backend().query_list(name.to_owned()).await.ok()?;
    list.iter()
        .filter(|entry| entry.get("name").and_then(|n| n.as_str()) == Some(name))
        .filter_map(|entry| entry.get("version").and_then(|v| v.as_str()))
        .next_back()
        .map(str::to_owned)
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
        RestError(openehr_its::rest::runtime::ApiError::Internal(format!(
            "WebTemplate JSON serialization failed: {e}"
        )))
    })?;
    Ok(negotiate::wt_json_body(StatusCode::OK, json))
}
