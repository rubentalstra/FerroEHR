//! HTTP dispatch for the `definition` API group (templates + stored queries).
//!
//! Each arm rebuilds the operation's `*Params`, decodes any body, calls the
//! trait method on [`AppState`], and renders a negotiated response. Handlers
//! currently return `NotImplemented`; that surfaces here as a 501 response.
//!
//! Note: the generated `ROUTES` operation ids carry dots (e.g.
//! `definition_template_adl1.4_list`, `definition_query_store.yaml`); the match
//! keys below are those exact strings, while the trait methods called are the
//! underscored names (`definition_template_adl1_4_list`, `definition_query_store_yaml`).

use axum::response::{IntoResponse, Response};
use http::StatusCode;

// DefinitionApi methods resolve through the `dyn Backend` trait object; import only params.
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
use crate::state::AppState;
use crate::{negotiate, params};

pub(super) fn dispatch(state: AppState, op: &'static str, parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

#[allow(clippy::too_many_lines)] // one arm per operation; a flat match is clearest
async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();

    match op {
        "definition_template_adl1.4_list" => {
            let p = params::build::<DefinitionTemplateAdl14ListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().definition_template_adl1_4_list(p).await?,
            ))
        }
        "definition_template_adl1.4_upload" => {
            let p = params::build::<DefinitionTemplateAdl14UploadParams>(&parts.path, q, h)?;
            // The OPT 1.4 template arrives as canonical XML; the lenient reader
            // hands it to the service as a JSON string, which it parses (opt14).
            let body = negotiate::lenient_value(&parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state
                    .backend()
                    .definition_template_adl1_4_upload(p, body)
                    .await?,
            ))
        }
        "definition_template_adl1.4_get" => {
            let p = params::build::<DefinitionTemplateAdl14GetParams>(&parts.path, q, h)?;
            let template_id = p.template_id.clone();
            // The service returns the stored OPT XML as a JSON string. For a Better
            // `wt+json` Accept, build the WebTemplate (cached per template id) from
            // that OPT; otherwise serve the OPT XML verbatim (the canonical artifact).
            match state.backend().definition_template_adl1_4_get(p).await? {
                serde_json::Value::String(xml) if negotiate::wants_web_template(h) => {
                    web_template_response(&state, &template_id, &xml).await
                }
                serde_json::Value::String(xml) => Ok(negotiate::xml_body(StatusCode::OK, xml)),
                other => Ok(negotiate::respond(h, StatusCode::OK, &other)),
            }
        }
        "definition_template_adl1.4_example_get" => {
            let p = params::build::<DefinitionTemplateAdl14ExampleGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .definition_template_adl1_4_example_get(p)
                    .await?,
            ))
        }
        "definition_template_adl2_list" => {
            let p = params::build::<DefinitionTemplateAdl2ListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().definition_template_adl2_list(p).await?,
            ))
        }
        "definition_template_adl2_upload" => {
            let p = params::build::<DefinitionTemplateAdl2UploadParams>(&parts.path, q, h)?;
            // PORT NOTE: ADL2/OPT2 ingestion is deferred (optional for CNF, untested;
            // its upload wire is ADL2 *text* needing a cADL parser — a later phase).
            // The backend 501s adl2_upload, so the body is passed through untyped.
            let body = negotiate::lenient_value(&parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state
                    .backend()
                    .definition_template_adl2_upload(p, body)
                    .await?,
            ))
        }
        "definition_template_adl2_get" => {
            let p = params::build::<DefinitionTemplateAdl2GetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().definition_template_adl2_get(p).await?,
            ))
        }
        "definition_template_adl2_example_get" => {
            let p = params::build::<DefinitionTemplateAdl2ExampleGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .definition_template_adl2_example_get(p)
                    .await?,
            ))
        }
        "definition_template_adl2_version_get" => {
            let p = params::build::<DefinitionTemplateAdl2VersionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .definition_template_adl2_version_get(p)
                    .await?,
            ))
        }
        "definition_query_list" => {
            let p = params::build::<DefinitionQueryListParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().definition_query_list(p).await?,
            ))
        }
        "definition_query_store.yaml" => {
            let p = params::build::<DefinitionQueryStoreYamlParams>(&parts.path, q, h)?;
            let body = negotiate::text_body(&parts.body)?;
            state.backend().definition_query_store_yaml(p, body).await?;
            // Spec: the store success is `200 OK` (not `204`).
            // TODO(port): the no-version store auto-assigns the version, but the
            // generated trait method is bodyless (`()`), so the assigned version
            // is not available here to build the `Location` header (a coherent
            // no-version auto-increment + Location design is deferred — see the
            // finding 03 hygiene note). The versioned store arm below sets it.
            Ok(negotiate::empty(StatusCode::OK))
        }
        "definition_query_version_get" => {
            let p = params::build::<DefinitionQueryVersionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().definition_query_version_get(p).await?,
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
                .definition_query_version_store_yaml(p, body)
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

/// Build (or reuse the cached) Better `WebTemplate` for `template_id` from the
/// stored OPT 1.4 `xml`, and serve it as `application/openehr.wt+json`.
///
/// Serving `wt+json` on the spec `adl1.4/{id}` GET endpoint is a deliberate
/// EHRbase-compatible extension (openEHR ITS-REST returns only the OPT itself).
async fn web_template_response(
    state: &AppState,
    template_id: &str,
    xml: &str,
) -> Result<Response, RestError> {
    let built = state
        .web_templates()
        .get_or_build(template_id, || {
            let opt = openehr_its::opt14::from_xml(xml)
                .map_err(|e| openehr_flat::FlatError::OptParse(e.to_string()))?;
            openehr_flat::build_web_template(&opt)
        })
        .await
        .map_err(|e| {
            RestError(openehr_its::rest::runtime::ApiError::Internal(format!(
                "WebTemplate build failed: {e}"
            )))
        })?;

    let json = serde_json::to_string(&*built).map_err(|e| {
        RestError(openehr_its::rest::runtime::ApiError::Internal(format!(
            "WebTemplate JSON serialization failed: {e}"
        )))
    })?;
    Ok(negotiate::wt_json_body(StatusCode::OK, json))
}
