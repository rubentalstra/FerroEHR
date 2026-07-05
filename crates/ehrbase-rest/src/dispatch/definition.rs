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
            // The service returns the stored OPT XML as a JSON string; serve it
            // verbatim as application/xml (the canonical template artifact).
            match state.backend().definition_template_adl1_4_get(p).await? {
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
            // TODO(port): P12 — parse OPT XML into the template model.
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
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
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
            let body = negotiate::text_body(&parts.body)?;
            state
                .backend()
                .definition_query_version_store_yaml(p, body)
                .await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted definition operation: {other}"),
        ))),
    }
}
