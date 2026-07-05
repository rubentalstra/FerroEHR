//! HTTP dispatch for the `admin` API group (`EHRbase` admin extension).

use axum::response::{IntoResponse, Response};
use http::StatusCode;

// The `AdminApi` methods resolve through the `dyn Backend` trait object, so only
// the param types are imported here.
use openehr_its::rest::generated::admin::{AdminEhrDeleteAllParams, AdminEhrDeleteParams};
use openehr_its::rest::runtime::ApiError;

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
        "admin_ehr_delete" => {
            let p = params::build::<AdminEhrDeleteParams>(&parts.path, q, h)?;
            state.backend().admin_ehr_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "admin_ehr_delete_all" => {
            let p = params::build::<AdminEhrDeleteAllParams>(&parts.path, q, h)?;
            state.backend().admin_ehr_delete_all(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted admin operation: {other}"
        )))),
    }
}
