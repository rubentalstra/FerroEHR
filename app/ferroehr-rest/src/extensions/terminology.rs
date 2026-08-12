// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! HTTP dispatch for the terminology extension API group over the
//! `ferroehr::service::TerminologyService` seam.
//!
//! **Operation semantics — SM `I_TERMINOLOGY_SERVICE`**
//! (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`,
//! which includes `docs/specs/openehr/SM/docs/UML/classes/i_terminology_service.adoc`):
//! the nine calls `get_terminology_ids`, `has_terminology`,
//! `get_terminology_description`, `has_term`, `get_term`, `subsumes`,
//! `value_set_validate`, `has_value_set`, `get_value_set` and their
//! `Pre_has_terminology` / `Pre_has_term` / `Pre_has_value_set` preconditions.
//! Every handler below realizes one of those calls with its precondition
//! mapping; the meaning of each is the master12 signature, cited inline.
//!
//! **Wire shape — no openEHR spec governs this; our own design/extension.**
//! Neither the Release-1.1.0 OAS set nor Release-1.0.3 defines a
//! terminology REST contract (there is no `terminology` group under
//! `crates/openehr-its/src/rest/generated/`), so this surface is ours: exposed
//! under the server's extension namespace (`/terminology`), shaped spec-first
//! from the master12 call semantics, and excluded from the ITS-REST drift
//! check. If/when openEHR publishes a contract, `emit-rest` takes over and these
//! routes migrate.
//!
//! NOTE (mount path): the extension groups (like the ADMIN group) are
//! mounted inside the ITS-REST API router, so the full path is
//! `{base_path}/terminology/...` — i.e. `/ferroehr/rest/openehr/v1/terminology`.
//! Nesting them here keeps the auth / ATNA-audit / ABAC middleware stack
//! uniform across the whole HTTP surface (our decision; the SM defines only the
//! abstract interface, not a URL layout).
//!
//! NOTE (existence calls): the boolean `has_terminology` / `has_term` /
//! `has_value_set` calls are surfaced implicitly through the `200`-vs-`404` of
//! their `get`/description counterparts (the idiomatic REST existence check)
//! rather than as separate boolean endpoints — mirroring the ADMIN group's
//! decision not to over-model the surface. The bundle provider maps a failed
//! precondition to `versioned_object_does_not_exist` (→ `404`), so no new SM
//! `CALL_STATUS_TYPE` is needed.
//!
//! The group is config-gated (`AppConfig::terminology_api_enabled`, default
//! `false`): when disabled every terminology route answers `404` without
//! touching the backend.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts, guarded_dispatch};
use crate::overview::error::RestError;

use crate::state::AppState;
use crate::{negotiate, params};

/// The terminology extension routes as a native `utoipa-axum` router: each
/// `#[utoipa::path]` handler single-sources its route and its `OpenAPI` path.
/// Group-relative paths (nested under the configured `base_path`); every
/// operation is served through [`guarded_dispatch`] → [`dispatch`], so the
/// wire behaviour is identical to the generated groups' `mount` adapter.
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    // One `routes!` per PATH (handlers in a single call must share the path;
    // mixing paths panics at router build with "Overlapping method route").
    OpenApiRouter::new()
        .routes(routes!(terminology_ids))
        .routes(routes!(terminology_description))
        .routes(routes!(terminology_get_term))
        .routes(routes!(terminology_subsumes))
        .routes(routes!(terminology_value_set))
        .routes(routes!(terminology_value_set_validate))
}

// ── Handlers (SM `I_TERMINOLOGY_SERVICE` semantics; our own wire shape) ───────
// Every handler snapshots the request into `RequestParts` (identical to the
// generated-group adapter) and runs it through the shared guarded dispatch, so
// the EHR_ACCESS gate, ABAC PEP, and ATNA audit tagging apply uniformly.

/// Every terminology id the server knows — SM `get_terminology_ids`
/// (`GET /terminology`).
///
/// Body: `{"terminology_ids": [..]}`. Config-gated: `404` when
/// `terminology_api_enabled` is off (the route stays mounted but the backend is
/// never consulted).
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology", tag = "terminology",
    responses(
        (status = 200, description = "The known terminology ids.", body = serde_json::Value),
        (status = 404, description = "The terminology extension is disabled (`terminology_api_enabled` off). With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_ids(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_ids", parts, dispatch).await
}

/// One terminology's descriptor — SM `get_terminology_description` (also the
/// `has_terminology` existence check) (`GET /terminology/{terminology_id}`).
///
/// A failed `Pre_has_terminology` maps to `404`.
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology/{terminology_id}", tag = "terminology",
    params(("terminology_id" = String, Path, description = "The terminology id.")),
    responses(
        (status = 200, description = "The terminology descriptor.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology (failed `Pre_has_terminology`), or the terminology extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_description(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_description", parts, dispatch).await
}

/// A term definition — SM `get_term`
/// (`GET /terminology/{terminology_id}/term/{code}`).
///
/// The optional `at_date` selects an effective-dated definition. A failed
/// `Pre_has_term` (unknown terminology or code) maps to `404`.
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology/{terminology_id}/term/{code}", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("code" = String, Path, description = "The term code."),
        ("at_date" = Option<String>, Query, description = "Optional ISO-8601 effective date; absent means the current definition.")
    ),
    responses(
        (status = 200, description = "The term extract.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology or code (failed `Pre_has_term`), or the terminology extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_get_term(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_get_term", parts, dispatch).await
}

/// Strict subsumption test — SM `subsumes`
/// (`GET /terminology/{terminology_id}/subsumes`).
///
/// Body: `{"subsumes": bool}`. Both codes are required query parameters.
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology/{terminology_id}/subsumes", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("ref_code" = String, Query, description = "The reference (ancestor-candidate) code. Required."),
        ("candidate" = String, Query, description = "The candidate (descendant) code. Required.")
    ),
    responses(
        (status = 200, description = "The subsumption result.", body = serde_json::Value),
        (status = 400, description = "A required query parameter (`ref_code`/`candidate`) is missing or blank.", body = serde_json::Value),
        (status = 404, description = "The terminology extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_subsumes(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_subsumes", parts, dispatch).await
}

/// A value set's extract — SM `get_value_set` (also the `has_value_set`
/// existence check)
/// (`GET /terminology/{terminology_id}/value_set/{value_set_id}`).
///
/// A failed `Pre_has_value_set` maps to `404`.
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology/{terminology_id}/value_set/{value_set_id}", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("value_set_id" = String, Path, description = "The value set id.")
    ),
    responses(
        (status = 200, description = "The value set extract.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology or value set (failed `Pre_has_value_set`), or the terminology extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_value_set(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_value_set", parts, dispatch).await
}

/// Value-set membership test — SM `value_set_validate`
/// (`GET /terminology/{terminology_id}/value_set/{value_set_id}/validate`).
///
/// Body: `{"valid": bool}`. `candidate_code` is a required query parameter.
///
/// OUR OWN EXTENSION — no openEHR spec governs this wire shape: no ITS-REST
/// contract defines a terminology API. The operation semantics are the SM
/// `I_TERMINOLOGY_SERVICE` call cited above
/// (`docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`);
/// the URL layout, the JSON envelopes, and the existence-by-`404` convention
/// are ours.
#[utoipa::path(
    get, path = "/terminology/{terminology_id}/value_set/{value_set_id}/validate", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("value_set_id" = String, Path, description = "The value set id."),
        ("candidate_code" = String, Query, description = "The candidate code to test for membership. Required."),
        ("at_date" = Option<String>, Query, description = "Optional ISO-8601 effective date; absent means the current value set.")
    ),
    responses(
        (status = 200, description = "The membership result.", body = serde_json::Value),
        (status = 400, description = "The required `candidate_code` query parameter is missing or blank.", body = serde_json::Value),
        (status = 404, description = "The terminology extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn terminology_value_set_validate(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "terminology_value_set_validate", parts, dispatch).await
}

pub(crate) fn dispatch(state: AppState, op: &'static str, parts: RequestParts) -> BoxResponse {
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
    // Config gate: the terminology extension is opt-in. When disabled every
    // route answers 404 (as if unmounted) without consulting the backend.
    if !state.config().terminology_api_enabled {
        return Err(RestError(ApiError::NotFound(
            "terminology API is disabled".to_owned(),
        )));
    }

    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;

    match op {
        "terminology_ids" => {
            let ids = state.backend().get_terminology_ids()?;
            Ok(negotiate::respond(
                h,
                ok,
                &json!({ "terminology_ids": ids }),
            ))
        }
        "terminology_description" => {
            let tid = path_get(&parts, "terminology_id")?;
            // A failed `Pre_has_terminology` → NotFound (404).
            let desc = state.backend().get_terminology_description(&tid)?;
            Ok(negotiate::respond(h, ok, &desc))
        }
        "terminology_get_term" => {
            let tid = path_get(&parts, "terminology_id")?;
            let code = path_get(&parts, "code")?;
            let at_date = params::query_param(q, "at_date");
            // NOTE: the SM `get_term.attributes` allow-list is not surfaced on
            // the wire — its `Hash<String, String>` shape is ambiguous against
            // the SM's `List<String>`, and the bundle provider ignores it.
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
