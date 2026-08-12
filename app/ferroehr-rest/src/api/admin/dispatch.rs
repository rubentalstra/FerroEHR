// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! HTTP dispatch for the `admin` API group, over the concrete
//! `ferroehr::service::FerroEhrService` admin methods.
//!
//! Maturity: the ITS-REST Admin API is **`DEVELOPMENT`**
//! (`specifications/docs/admin/Description.md` §Status: "This specification is
//! in the `DEVELOPMENT` state"). It mounts exactly two operations — both
//! physical EHR delete — and both are dispatched here, alongside three of OUR
//! OWN extension routes (template delete, stored-query-version delete, the
//! redacted config read), which no ITS-REST operation governs. The remaining SM
//! admin capabilities (party delete, statistics, archive, dump/load) have no
//! ITS-REST binding and are not surfaced by this group at all.
//!
//! Spec grounding (both operations are vendored — `admin.openapi.yaml`,
//! `security: []` so auth is out of band, SM master02):
//! - `admin_ehr_delete` — `operations/admin_ehr_delete.yaml`: `DELETE
//!   /admin/ehr/{ehr_id}`, physical cascade of every owned resource (COMPOSITION,
//!   `EHR_STATUS`, `ITEM_TAG`, CONTRIBUTION + historical versions) "permanently and
//!   physically deleted … (e.g., the GDPR)"; sync success → `204 No Content`,
//!   async → `202 Accepted`, unknown id → `404` (`404_unknown_ehr_id.yaml`).
//!   Matches the abstract SM `I_ADMIN_SERVICE.physical_ehr_delete`
//!   (`SM/docs/UML/classes/i_admin_service.adoc` — precondition `has_ehr`, error
//!   `ehr_id_does_not_exist`) and the CNF Robot prior art
//!   (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`).
//! - `admin_ehr_delete_all` — `operations/admin_ehr_delete_all.yaml`: `DELETE
//!   /admin/ehr/all{?ehr_id*}`, "Deletes all or multiple EHRs, or a specified
//!   subset … identified using the `ehr_id` query parameter"; sync success →
//!   `204 No Content` (no body), async → `202 Accepted`; may respond `405` when
//!   disabled in production. This operation exists in the ITS-REST OAS but not
//!   in the abstract SM `i_admin_service.adoc` — a recorded spec-internal
//!   inconsistency.
//!
//! The group is config-gated (`AppConfig::admin.enabled`, default false): when
//! disabled every admin route answers **`405 Method Not Allowed`** with an
//! empty `Allow`, without touching the backend. The ground differs per route —
//! `admin_ehr_delete_all` has its own released NOTE + `responses/405.yaml`,
//! while the single-EHR delete and the three extensions rest on the
//! cross-cutting rule "If a method is recognized but not allowed for the target
//! resource, the response SHOULD be `405 Method Not Allowed` status code"
//! (`docs/overview/Requests_and_responses.md` §"HTTP Methods"). See the gate
//! comment in `run` below and the declarations in `super::openapi_routes`.

use axum::Json;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

use openehr_its::rest::generated::admin::AdminEhrDeleteParams;
use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts};
use crate::overview::error::RestError;

use crate::state::AppState;
use crate::{negotiate, params};

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
    // Config gate: the ADMIN API is opt-in, and every admin route answers
    // `405 Method Not Allowed` with the openEHR error body while it is off —
    // `operations/admin_ehr_delete_all.yaml` states it for the bulk delete, and
    // `docs/overview/Requests_and_responses.md` §"HTTP Methods" covers the
    // rest. This `405` comes from a MATCHED handler, so axum's allow-header
    // machinery never runs and the `Allow` RFC 9110 §15.5.6 mandates is stated
    // here as the EMPTY field value — RFC 9110 §10.2.1's exact case for a
    // resource "temporarily disabled by configuration".
    if let Some(refusal) = admin_group_gate(&state) {
        return Ok(refusal);
    }
    let h = &parts.headers;
    let q = parts.query.as_deref();

    match op {
        "admin_ehr_delete" => {
            let p = params::build::<AdminEhrDeleteParams>(&parts.path, q, h)?;
            // SM physical_ehr_delete → 204 No Content; unknown EHR → 404
            // (the service maps `ehr_id_does_not_exist` to NotFound).
            state.backend().admin_ehr_delete(p.ehr_id).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        // Admin extensions (our own design — the ITS-REST Admin API defines only
        // EHR deletes; see the module NOTE). Both mirror `admin_ehr_delete`:
        // 204 on success, 404 for an unknown id, same admin gate.
        "admin_template_delete" => {
            let template_id = path_segment(&parts, "template_id")?;
            // Physical template delete: 204; unknown id → 404; a template still
            // referenced by a committed version → 409 (never orphan clinical
            // data). The service maps those outcomes.
            state.backend().admin_template_delete(template_id).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "admin_query_delete" => {
            let qualified_name = path_segment(&parts, "qualified_query_name")?;
            let version = path_segment(&parts, "version")?;
            // Single stored-query-version delete: 204; unknown (name, version) →
            // 404.
            state
                .backend()
                .admin_query_delete(qualified_name, version)
                .await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "admin_config" => {
            // The redacted effective configuration (OUR OWN EXTENSION — no
            // openEHR spec governs configuration). The binary builds this
            // snapshot as `FerroEhrConfig::to_redacted_json` at boot, so every
            // secret leaf is already `***`/`scheme://***@…` by its own
            // `Secret`/`SecretUrl` type — no secret substring is present here to
            // leak. Serving the pre-built snapshot (never the raw config) keeps
            // the redaction a structural property of the config tree.
            let snapshot = state.observability().env_snapshot.as_ref().clone();
            Ok((StatusCode::OK, Json(snapshot)).into_response())
        }
        "admin_ehr_delete_all" => {
            // The generated `AdminEhrDeleteAllParams.ehr_id: Option<String>`
            // under-models the RFC 6570 `{?ehr_id*}` list — the params
            // deserializer collapses a repeated `?ehr_id=a&ehr_id=b` to
            // `Some("a")` — so the full list is read from the raw query, which
            // accepts both the repeated and comma-separated forms. `ehr_id` is
            // OPTIONAL (`parameters/query/ehr_id_Admin.yaml`), so an
            // absent/empty list means "delete ALL EHRs"
            // (`operations/admin_ehr_delete_all.yaml`).
            let ids = ehr_id_list(q);
            // The `AdminService::admin_ehr_delete_all` seam honours the
            // empty-list = all-EHRs semantics: per
            // `operations/admin_ehr_delete_all.yaml:5` +
            // `parameters/query/ehr_id_Admin.yaml` an absent `ehr_id` means
            // "delete ALL EHRs", so the empty list forwarded here targets the
            // full EHR set; a non-empty list targets that subset.
            state.backend().admin_ehr_delete_all(ids).await?;
            // `operations/admin_ehr_delete_all.yaml:18-26`: the only declared
            // success responses are `204 No Content` (sync,
            // `responses/204_deleted_hard.yaml`) and `202 Accepted` (async,
            // `responses/202.yaml`) — both bodyless. We are synchronous → `204`.
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted admin operation: {other}"
        )))),
    }
}

/// The ADMIN group's config gate, shared by every dispatcher mounted under
/// `/admin/` (this one plus the [`archive`](super::archive) and
/// [`report`](super::report) extension groups): `Some(refusal)` while
/// `AppConfig::admin.enabled` is off, `None` when the group serves.
///
/// The grounds and the empty `Allow` are stated at the call site in [`run`];
/// this is that one decision, in one place, so no route under `/admin/` can
/// drift out of the gate.
pub(super) fn admin_group_gate(state: &AppState) -> Option<Response> {
    (!state.config().admin.enabled).then(|| {
        crate::overview::error::method_not_allowed_response(
            "",
            "the admin API is disabled on this server",
        )
    })
}

/// Read a required path segment for the admin extension routes (not modelled by
/// a generated params type). A missing segment is impossible for a matched route
/// but is mapped to a `400` rather than panicking.
fn path_segment(parts: &RequestParts, key: &str) -> Result<String, RestError> {
    parts.path.get(key).cloned().ok_or_else(|| {
        RestError(ApiError::BadRequest(format!(
            "missing path parameter '{key}'"
        )))
    })
}

/// Collect every `ehr_id` value from the raw query string, splitting each on
/// commas — so both `?ehr_id=a&ehr_id=b` (repeated) and `?ehr_id=a,b`
/// (comma-separated) yield the full list. Blank entries are dropped.
///
/// Kept a plain query walk rather than percent-decoding: `ehr_id`s are UUIDs
/// (ASCII hex + hyphens, no reserved characters), so no decoding is needed.
fn ehr_id_list(query: Option<&str>) -> Vec<String> {
    let Some(q) = query else {
        return Vec::new();
    };
    q.split('&')
        .filter_map(|pair| pair.split_once('='))
        .filter(|(k, _)| *k == "ehr_id")
        .flat_map(|(_, v)| v.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::ehr_id_list;

    #[test]
    fn parses_comma_separated_and_repeated() {
        assert!(ehr_id_list(None).is_empty());
        assert!(ehr_id_list(Some("")).is_empty());
        assert_eq!(ehr_id_list(Some("ehr_id=a")), vec!["a".to_owned()]);
        assert_eq!(
            ehr_id_list(Some("ehr_id=a,b")),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(
            ehr_id_list(Some("ehr_id=a&ehr_id=b")),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(
            ehr_id_list(Some("ehr_id=a, b &other=x&ehr_id=c")),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
        // A present-but-empty value contributes nothing.
        assert!(ehr_id_list(Some("ehr_id=")).is_empty());
    }
}
