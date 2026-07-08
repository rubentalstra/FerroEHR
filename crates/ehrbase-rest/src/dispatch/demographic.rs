//! HTTP dispatch for the `demographic` API group (PARTY / `VERSIONED_PARTY` /
//! demographic CONTRIBUTION / party tags) over the
//! [`DemographicService`](crate::backend::DemographicService) seam.
//!
//! TODO(port): the per-operation arms land with the demographic service; until
//! then every route answers 501 (unchanged wire behaviour).

use axum::response::IntoResponse;

use super::{BoxResponse, RequestParts};
use crate::state::AppState;

pub(super) fn dispatch(_state: AppState, op: &'static str, _parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        tracing::debug!(operation = op, "unimplemented demographic operation");
        crate::error::RestError(openehr_its::rest::runtime::ApiError::NotImplemented)
            .into_response()
    })
}
