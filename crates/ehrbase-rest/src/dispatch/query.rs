//! HTTP dispatch for the `query` API group (AQL ad-hoc + stored queries).
//!
//! Each arm rebuilds the operation's `*Params`, decodes any JSON body, calls the
//! trait method on [`AppState`], and renders a negotiated `200 OK` response.
//! Handlers currently return `NotImplemented`; that surfaces here as a 501.

use axum::response::{IntoResponse, Response};
use http::StatusCode;

// QueryApi methods resolve through the `dyn Backend` trait object; import only params.
use openehr_its::rest::generated::query::{
    QueryExecuteAdhocQueryBodyParams, QueryExecuteAdhocQueryParams,
    QueryExecuteStoredQueryBodyParams, QueryExecuteStoredQueryParams,
    QueryExecuteStoredQueryVersionBodyParams, QueryExecuteStoredQueryVersionParams,
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

async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();

    match op {
        "query_execute_adhoc_query" => {
            let p = params::build::<QueryExecuteAdhocQueryParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().query_execute_adhoc_query(p).await?,
            ))
        }
        "query_execute_adhoc_query_body" => {
            let p = params::build::<QueryExecuteAdhocQueryBodyParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .query_execute_adhoc_query_body(p, body)
                    .await?,
            ))
        }
        "query_execute_stored_query" => {
            let p = params::build::<QueryExecuteStoredQueryParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state.backend().query_execute_stored_query(p).await?,
            ))
        }
        "query_execute_stored_query_body" => {
            let p = params::build::<QueryExecuteStoredQueryBodyParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .query_execute_stored_query_body(p, body)
                    .await?,
            ))
        }
        "query_execute_stored_query_version" => {
            let p = params::build::<QueryExecuteStoredQueryVersionParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .query_execute_stored_query_version(p)
                    .await?,
            ))
        }
        "query_execute_stored_query_version_body" => {
            let p = params::build::<QueryExecuteStoredQueryVersionBodyParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &state
                    .backend()
                    .query_execute_stored_query_version_body(p, body)
                    .await?,
            ))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted query operation: {other}"),
        ))),
    }
}
