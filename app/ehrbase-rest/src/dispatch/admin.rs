//! HTTP dispatch for the `admin` API group (physical EHR delete) over the
//! [`AdminService`](ehrbase_sm::services::AdminService) seam.
//!
//! Spec grounding: SM `I_ADMIN_SERVICE.physical_ehr_delete`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`) —
//! precondition `has_ehr`, error `ehr_id_does_not_exist`; and the CNF Robot
//! prior art (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot` +
//! `_resources/keywords/admin_keywords.robot`): `DELETE /admin/ehr/{ehr_id}` →
//! `204`, and every backing table returns to its baseline count (a full
//! physical cascade). The ADMIN API is dev-branch only in ITS-REST (no vendored
//! OAS; CNF master12 is TBD).
//!
//! The group is config-gated (`RestConfig::admin.enabled`, default false): when
//! disabled every admin route answers `404` without touching the backend.

use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::json;

use openehr_its::rest::generated::admin::AdminEhrDeleteParams;
use openehr_its::rest::runtime::ApiError;

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use ehrbase_sm::Platform;

use crate::state::AppState;
use crate::{negotiate, params};

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
            let ids = ehr_id_list(q);
            if ids.is_empty() {
                // PORT NOTE: delete-all is unspecified (not in the SM, not in any
                // OAS). We refuse an implicit delete-everything: an absent/empty
                // list is a 400 rather than a catastrophic wildcard.
                return Err(RestError(ApiError::BadRequest(
                    "DELETE /admin/ehr/all requires a non-empty ehr_id list".to_owned(),
                )));
            }
            let deleted = state.backend().admin_ehr_delete_all(ids).await?;
            // PORT NOTE: the response shape is unspecified; a small
            // `{"deleted": <n>}` body makes the idempotent partial-success
            // semantics (existing ids deleted, missing ids skipped) observable.
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &json!({ "deleted": deleted }),
            ))
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
