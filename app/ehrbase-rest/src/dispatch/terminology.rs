//! HTTP dispatch for the terminology extension API group over the
//! [`TerminologyService`](ehrbase_sm::services::TerminologyService) seam.
//!
//! Spec grounding: SM `I_TERMINOLOGY_SERVICE`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_terminology_service.adoc`,
//! `master12-terminology_service.adoc`) — the nine calls `get_terminology_ids`,
//! `has_terminology`, `get_terminology_description`, `has_term`, `get_term`,
//! `subsumes`, `value_set_validate`, `has_value_set`, `get_value_set` and their
//! `Pre_has_terminology` / `Pre_has_term` / `Pre_has_value_set` preconditions.
//!
//! Wire design (`docs/design/sm-platform/08-target-architecture.md` §7):
//! ITS-REST 1.0.3 defines **no** terminology contract, so this surface is our
//! own, exposed under the server's extension namespace (`/terminology`), spec-
//! first from the SM call semantics and excluded from the ITS-REST drift check.
//! If/when openEHR publishes a contract, `emit-rest` takes over (ADR-005) and
//! these routes migrate.
//!
//! PORT NOTE (mount path, §7): §7 names the namespace `/rest/terminology`; in
//! this server the extension groups (like the ADMIN group) are mounted inside
//! the ITS-REST API router, so the full path is
//! `{base_path}/terminology/...` — i.e. `/ehrbase/rest/openehr/v1/terminology`.
//! Nesting them here keeps the auth / ATNA-audit / ABAC middleware stack
//! uniform across the whole HTTP surface.
//!
//! PORT NOTE (existence calls): the boolean `has_terminology` / `has_term` /
//! `has_value_set` calls are surfaced implicitly through the `200`-vs-`404` of
//! their `get`/description counterparts (the idiomatic REST existence check)
//! rather than as separate boolean endpoints — mirroring the ADMIN group's
//! decision not to over-model the surface. The bundle provider maps a failed
//! precondition to `versioned_object_does_not_exist` (→ `404`), so no new SM
//! `CALL_STATUS_TYPE` is needed.
//!
//! The group is config-gated (`RestConfig::terminology.enabled`, default
//! `false`): when disabled every terminology route answers `404` without
//! touching the backend.

use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::json;

use openehr_its::rest::runtime::ApiError;

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use ehrbase_sm::Platform;

use crate::state::AppState;
use crate::{negotiate, params};

/// The terminology extension routes — our own design (no ITS-REST contract;
/// §7), mounted alongside the generated `ROUTES` and served by [`dispatch`].
/// Group-relative paths (nested under the configured `base_path`).
pub(crate) const TERMINOLOGY_ROUTES: &[(&str, &str, &str)] = &[
    // `get_terminology_ids` — the identifiers of every terminology this server
    // knows.
    ("GET", "/terminology", "terminology_ids"),
    // `get_terminology_description` — descriptor for one terminology
    // (`Pre_has_terminology`; also the existence check for `has_terminology`).
    (
        "GET",
        "/terminology/{terminology_id}",
        "terminology_description",
    ),
    // `get_term` — a term definition (lookup); optional `?at_date=`.
    (
        "GET",
        "/terminology/{terminology_id}/term/{code}",
        "terminology_get_term",
    ),
    // `subsumes` — strict subsumption test; `?ref_code=&candidate=`.
    (
        "GET",
        "/terminology/{terminology_id}/subsumes",
        "terminology_subsumes",
    ),
    // `get_value_set` — the value set's extract (expand;
    // `Pre_has_value_set`, also the existence check for `has_value_set`).
    (
        "GET",
        "/terminology/{terminology_id}/value_set/{value_set_id}",
        "terminology_value_set",
    ),
    // `value_set_validate` — membership test; `?candidate_code=&at_date=`.
    (
        "GET",
        "/terminology/{terminology_id}/value_set/{value_set_id}/validate",
        "terminology_value_set_validate",
    ),
];

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

async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    // Config gate: the terminology extension is opt-in. When disabled every
    // route answers 404 (as if unmounted) without consulting the backend.
    if !state.config().terminology.enabled {
        return Err(RestError(ApiError::NotFound(
            "terminology API is disabled".to_owned(),
        )));
    }

    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;

    match op {
        "terminology_ids" => {
            let ids = state.backend().get_terminology_ids().await?;
            Ok(negotiate::respond(
                h,
                ok,
                &json!({ "terminology_ids": ids }),
            ))
        }
        "terminology_description" => {
            let tid = path_get(&parts, "terminology_id")?;
            // A failed `Pre_has_terminology` → NotFound (404).
            let desc = state.backend().get_terminology_description(&tid).await?;
            Ok(negotiate::respond(h, ok, &desc))
        }
        "terminology_get_term" => {
            let tid = path_get(&parts, "terminology_id")?;
            let code = path_get(&parts, "code")?;
            let at_date = params::query_param(q, "at_date");
            // PORT NOTE: the SM `get_term.attributes` allow-list is not surfaced
            // on the wire — its `Hash<String, String>` shape is ambiguous against
            // the SM's `List<String>`, and the bundle provider ignores it
            // (`ehrbase::service::api::terminology`). Passed as `None`; add a
            // query encoding when a consumer needs the filter.
            let extract = state.backend().get_term(&tid, &code, None, at_date).await?;
            Ok(negotiate::respond(h, ok, &extract))
        }
        "terminology_subsumes" => {
            let tid = path_get(&parts, "terminology_id")?;
            let ref_code = require_query(q, "ref_code")?;
            let candidate = require_query(q, "candidate")?;
            let subsumes = state
                .backend()
                .subsumes(&tid, &ref_code, &candidate)
                .await?;
            Ok(negotiate::respond(h, ok, &json!({ "subsumes": subsumes })))
        }
        "terminology_value_set" => {
            let tid = path_get(&parts, "terminology_id")?;
            let value_set_id = path_get(&parts, "value_set_id")?;
            // A failed `Pre_has_value_set` → NotFound (404).
            let extract = state.backend().get_value_set(&tid, &value_set_id).await?;
            Ok(negotiate::respond(h, ok, &extract))
        }
        "terminology_value_set_validate" => {
            let tid = path_get(&parts, "terminology_id")?;
            let value_set_id = path_get(&parts, "value_set_id")?;
            let candidate_code = require_query(q, "candidate_code")?;
            let at_date = params::query_param(q, "at_date");
            let valid = state
                .backend()
                .value_set_validate(&tid, &value_set_id, &candidate_code, at_date)
                .await?;
            Ok(negotiate::respond(h, ok, &json!({ "valid": valid })))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted terminology operation: {other}"
        )))),
    }
}

/// Read a matched path parameter (guaranteed present by the route template; an
/// absence is a routing bug → `500`).
fn path_get(parts: &RequestParts, key: &str) -> Result<String, RestError> {
    parts.path.get(key).cloned().ok_or_else(|| {
        RestError(ApiError::Internal(format!(
            "missing path parameter `{key}`"
        )))
    })
}

/// Read a required, non-empty query parameter → `400` when absent/blank.
fn require_query(query: Option<&str>, key: &str) -> Result<String, RestError> {
    params::query_param(query, key)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            RestError(ApiError::BadRequest(format!(
                "missing required query parameter `{key}`"
            )))
        })
}
