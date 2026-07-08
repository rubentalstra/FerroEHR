//! HTTP dispatch for the `admin` API group (physical EHR delete) over the
//! [`AdminService`](crate::backend::AdminService) seam.
//!
//! TODO(port): the per-operation arms land with the admin service; until then
//! every route answers 501 (unchanged wire behaviour).

use axum::response::IntoResponse;

use super::{BoxResponse, RequestParts};
use crate::state::AppState;

pub(super) fn dispatch(_state: AppState, op: &'static str, _parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        tracing::debug!(operation = op, "unimplemented admin operation");
        crate::error::RestError(openehr_its::rest::runtime::ApiError::NotImplemented)
            .into_response()
    })
}
