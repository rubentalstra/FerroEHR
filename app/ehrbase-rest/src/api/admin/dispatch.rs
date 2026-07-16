//! HTTP dispatch for the `admin` API group (physical EHR delete) over the
//! [`AdminService`](ehrbase::service::AdminService) seam.
//!
//! Maturity: the ITS-REST Admin API is **`DEVELOPMENT`** (`admin.openapi.yaml`
//! `info.version: development`, `x-status: DEVELOPMENT`). It mounts exactly two
//! operations — both physical EHR delete — and both are dispatched here; there
//! are **no extension routes** in this group (the other SM admin capabilities —
//! party delete, statistics, archive, dump/load — have no ITS-REST binding and
//! stay native-API-only on the `ehrbase-sm` seam).
//!
//! Spec grounding (both operations are vendored — `admin.openapi.yaml:24-30`,
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
//! disabled every admin route answers `404` without touching the backend.

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
    // Config gate: the ADMIN API is opt-in. When disabled every admin route
    // answers 404 (as if unmounted) without consulting the backend.
    //
    // PORT NOTE: mirrors EHRbase's `ADMINAPI_ACTIVE` prior art — an inactive
    // admin surface simply has no such endpoint (a 404), never a 403.
    if !state.config().admin.enabled {
        return Err(RestError(ApiError::NotFound(
            "admin API is disabled".to_owned(),
        )));
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
        "admin_ehr_delete_all" => {
            // The generated `AdminEhrDeleteAllParams.ehr_id: Option<String>`
            // under-models the RFC 6570 `{?ehr_id*}` list: the type-directed
            // params deserializer (`crate::params`) collapses a repeated
            // `?ehr_id=a&ehr_id=b` to `Some("a")` for an `Option<String>` field.
            // So the full list is read straight from the raw query here, which
            // accepts BOTH the repeated form and a comma-separated single value
            // (`?ehr_id=a,b`).
            //
            // `ehr_id` is OPTIONAL (`parameters/query/ehr_id_Admin.yaml`: "An
            // optional parameter to perform the operation on a subset of EHRs").
            // An absent/empty list therefore means "delete ALL EHRs", per
            // `operations/admin_ehr_delete_all.yaml:5` ("Deletes all or multiple
            // EHRs, or a specified subset"). The list is passed through verbatim;
            // an empty vec is the "all EHRs" request.
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
