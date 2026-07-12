//! The demographic CONTRIBUTION operations —
//! `operations/demographic_contribution_create.yaml`,
//! `demographic_contribution_get.yaml`. Canonical content negotiation; the
//! commit body is a `NewContribution` wrapper (`schemas/demographic/NewContribution.yaml`).

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::demographic::ContributionGetParams;
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::{Platform, ServiceResponse};

/// The `contribution_*` operations.
pub(super) async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().base_path.clone();

    match op {
        "contribution_create" => {
            // A CONTRIBUTION commit is a `NewContribution` wrapper DTO, JSON only.
            let body = negotiate::json_value(h, &parts.body)?;
            let resp = state
                .backend()
                .demographic_contribution_create(body)
                .await?;
            // 201_demographic_CONTRIBUTION + ETag(contribution_uid)/Location;
            // body per Prefer (oneOf[Contribution, Identifier]).
            Ok(write_shared(
                h,
                &base,
                "contribution",
                StatusCode::CREATED,
                StatusCode::CREATED,
                &resp,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .demographic_contribution_get(p.contribution_uid)
                .await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic contribution operation: {other}"
        )))),
    }
}

/// A create response for a JSON-only payload (CONTRIBUTION), honouring `Prefer`
/// and setting the demographic `ETag`/`Location`.
fn write_shared(
    h: &http::HeaderMap,
    base: &str,
    segment: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    resp: &ServiceResponse,
) -> Response {
    let mut out = if negotiate::prefers_representation(h) {
        negotiate::respond(h, repr_status, &resp.body)
    } else {
        negotiate::empty(minimal_status)
    };
    super::set_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}
