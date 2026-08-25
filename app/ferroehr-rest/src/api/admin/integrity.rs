// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The ADMIN **storage-integrity** wire — **our own extension**.
//!
//! No openEHR ITS-REST operation governs this route, and no SM interface
//! declares it either. The released Admin API is exactly two EHR deletes
//! (`specifications/admin.openapi.yaml`), and
//! `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc` declares
//! deletes, archival and reporting — nothing that inspects stored data for
//! damage. No openEHR spec governs storage mechanics at all, so both the
//! two-copy storage design and this route over it are ours.
//!
//! What it exposes: the storage keeps every version's content twice — the
//! materialized `vo_version.body` a point read serves, and the decomposed
//! `node` rows the AQL engine queries. Read-time signature verification (RM
//! common `master06-change_control_package.adoc` §Digital Signature) covers
//! the first copy. This route re-derives the second one and compares, so
//! tampering or corruption of either becomes visible.
//!
//! It is a `POST` because it is an action, not a resource: RFC 9110 §9.3.3
//! defines `POST` as "providing a block of data … to a data-handling process",
//! while a `GET` would present an expensive whole-repository scan as a
//! cacheable representation.
//!
//! Gating: mounted under `/admin/`, so it inherits the group's two gates
//! unchanged — the coarse RBAC `OperationClass::Admin` classifier (`401`
//! unauthenticated / `403` non-admin, our own authorization design; the
//! released admin operations carry `security: []`) and the
//! `AppConfig::admin.enabled` config gate, which answers `405` with an empty
//! `Allow` while the group is off (`crate::api::admin::dispatch`, whose ground
//! is the overview rule "If a method is recognized but not allowed for the
//! target resource, the response SHOULD be `405 Method Not Allowed` status
//! code" — `docs/overview/Requests_and_responses.md` §"HTTP Methods").

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 8): genuinely open operational JSON (the \
              storage-parity report is an operational document, not an RM resource)"
)]

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use openehr_its::rest::runtime::ApiError;

use ferroehr::service::admin::integrity::StorageParityReport;

use crate::api::{BoxResponse, RequestParts, guarded_dispatch};
use crate::negotiate;
use crate::overview::error::RestError;
use crate::state::AppState;

/// The storage-integrity extension route as a native `utoipa-axum` router —
/// **no ITS-REST contract** (see the module docs). Group-relative path (nested
/// under `base_path`); the operation runs through [`guarded_dispatch`] with
/// [`dispatch`].
pub(crate) fn integrity_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(admin_verify_storage_parity))
}

/// Sweep the stored versions for content-copy disagreement
/// (`POST /admin/integrity/verify`).
///
/// **Our own extension — no ITS-REST operation governs this, and it realizes
/// no SM operation either** (module docs). Every stored version in both
/// storage tiers is reassembled from its `node` rows and compared with its
/// materialized body; the response is the resulting report.
#[utoipa::path(
    post, path = "/admin/integrity/verify", tag = "admin-integrity",
    responses(
        (status = 200, description = "The sweep ran. The body is the report: \
                                      how many stored versions were read, how \
                                      many carried a body, how many mismatches \
                                      were found, and the mismatching versions \
                                      by identifier. `mismatch_count` is the \
                                      full count; `mismatches` is capped and \
                                      `truncated` says whether the cap was \
                                      reached (every mismatch is logged at \
                                      `warn` whatever the cap does). A finding \
                                      is NOT a request failure — the sweep \
                                      succeeded and is reporting what it saw, \
                                      so the status stays `200`.",
         body = serde_json::Value,
         example = json!({
             "versions_checked": 128,
             "versions_with_body": 126,
             "versions_without_body": 2,
             "mismatch_count": 1,
             "mismatches": [{
                 "vo_id": "8849182c-82ad-4088-a07f-48ead4180515",
                 "sys_version": 2,
                 "kind": "COMPOSITION",
                 "defect": "content_differs"
             }],
             "truncated": false,
             "elapsed_ms": 431
         })),
        (status = 401, description = "Unauthenticated (auth enabled, no valid \
                                      principal). Our own authorization design \
                                      — the released admin operations carry \
                                      `security: []` and declare no such \
                                      branch.",
         body = serde_json::Value),
        (status = 403, description = "Authenticated but not in the Admin class \
                                      (`OperationClass::Admin`, keyed off the \
                                      `/admin/` path). Our own authorization \
                                      design.",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this server \
                                      (`AppConfig::admin.enabled`, default \
                                      false), answered with an empty `Allow` \
                                      per RFC 9110 §10.2.1.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn admin_verify_storage_parity(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "admin_verify_storage_parity", parts, dispatch).await
}

// ── dispatch ─────────────────────────────────────────────────────────────────

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
    // The whole ADMIN group is opt-in; the gate and its two grounds are stated
    // once, in the group dispatcher.
    if let Some(refusal) = super::dispatch::admin_group_gate(&state) {
        return Ok(refusal);
    }
    match op {
        "admin_verify_storage_parity" => {
            let report = state.backend().verify_storage_parity().await?;
            Ok(negotiate::respond(
                &parts.headers,
                StatusCode::OK,
                &parity_report(&report),
            ))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted admin integrity operation: {other}"
        )))),
    }
}

/// Render the storage-parity report as the response body.
///
/// The shape is ours end to end (no openEHR spec governs storage mechanics),
/// so it is written out explicitly here rather than derived: the wire contract
/// is this function, not a serde attribute on a service type.
fn parity_report(report: &StorageParityReport) -> Value {
    json!({
        "versions_checked": report.versions_checked,
        "versions_with_body": report.versions_with_body,
        "versions_without_body": report.versions_without_body,
        "mismatch_count": report.mismatch_count,
        "mismatches": report
            .mismatches
            .iter()
            .map(|m| json!({
                "vo_id": m.vo_id.to_string(),
                "sys_version": m.sys_version,
                "kind": m.kind,
                "defect": m.defect.as_str(),
            }))
            .collect::<Vec<Value>>(),
        "truncated": report.truncated,
        "elapsed_ms": report.elapsed_ms,
    })
}
