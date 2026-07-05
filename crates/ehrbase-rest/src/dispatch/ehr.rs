//! HTTP dispatch for the `ehr` API group.
//!
//! Each arm rebuilds the operation's `*Params`, decodes any body (RM-typed
//! bodies accept JSON or canonical XML), calls the backend, and renders a
//! negotiated response. The `ehr` group is served by the `ehrbase` service
//! (P12); operations a backend does not implement surface as a 501.
//!
//! Response negotiation splits by payload kind: arms returning a single
//! spec-typed RM object (COMPOSITION / `EHR_STATUS` / EHR / FOLDER) use
//! [`negotiate::respond_rm`] and honour an XML `Accept`; arms returning a
//! VERSION-family wrapper, revision history, item tags, or a CONTRIBUTION DTO
//! use [`negotiate::respond`] and stay JSON-only, since those payloads have no
//! spec-defined canonical-XML shape (a future typed-VERSION codegen effort).

use axum::response::{IntoResponse, Response};
use http::StatusCode;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrApi, EhrCreateParams,
    EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_rm::prelude::{Composition, Ehr, EhrStatus, Folder};

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

#[allow(clippy::too_many_lines)] // one arm per operation; a flat match is clearest
async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;

    match op {
        "ehr_get_by_subject" => {
            let p = params::build::<EhrGetBySubjectParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<Ehr>(
                h,
                ok,
                &state.backend().ehr_get_by_subject(p).await?,
                "ehr",
            ))
        }
        "ehr_create" => {
            let p = params::build::<EhrCreateParams>(&parts.path, q, h)?;
            let body = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            Ok(negotiate::respond_rm::<Ehr>(
                h,
                StatusCode::CREATED,
                &state.backend().ehr_create(p, body).await?,
                "ehr",
            ))
        }
        "ehr_get_by_id" => {
            let p = params::build::<EhrGetByIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<Ehr>(
                h,
                ok,
                &state.backend().ehr_get_by_id(p).await?,
                "ehr",
            ))
        }
        "ehr_create_with_id" => {
            let p = params::build::<EhrCreateWithIdParams>(&parts.path, q, h)?;
            let body = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            Ok(negotiate::respond_rm::<Ehr>(
                h,
                StatusCode::CREATED,
                &state.backend().ehr_create_with_id(p, body).await?,
                "ehr",
            ))
        }
        "ehr_status_get_by_version_id" => {
            let p = params::build::<EhrStatusGetByVersionIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<EhrStatus>(
                h,
                ok,
                &state.backend().ehr_status_get_by_version_id(p).await?,
                "ehr_status",
            ))
        }
        "ehr_status_get_at_time" => {
            let p = params::build::<EhrStatusGetAtTimeParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<EhrStatus>(
                h,
                ok,
                &state.backend().ehr_status_get_at_time(p).await?,
                "ehr_status",
            ))
        }
        "ehr_status_update" => {
            let p = params::build::<EhrStatusUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<EhrStatus>(h, &parts.body)?;
            Ok(negotiate::respond_rm::<EhrStatus>(
                h,
                ok,
                &state.backend().ehr_status_update(p, body).await?,
                "ehr_status",
            ))
        }
        "versioned_ehr_status_get" => {
            let p = params::build::<VersionedEhrStatusGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().versioned_ehr_status_get(p).await?,
            ))
        }
        "versioned_ehr_status_revision_history" => {
            let p = params::build::<VersionedEhrStatusRevisionHistoryParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_ehr_status_revision_history(p)
                    .await?,
            ))
        }
        "versioned_ehr_status_version_get_at_time" => {
            let p = params::build::<VersionedEhrStatusVersionGetAtTimeParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_ehr_status_version_get_at_time(p)
                    .await?,
            ))
        }
        "versioned_ehr_status_version_get_by_id" => {
            let p = params::build::<VersionedEhrStatusVersionGetByIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_ehr_status_version_get_by_id(p)
                    .await?,
            ))
        }
        "composition_create" => {
            let p = params::build::<CompositionCreateParams>(&parts.path, q, h)?;
            // A FLAT/STRUCTURED (wt.flat/structured+json) body is rebuilt into a
            // canonical composition.
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            let created = state.backend().composition_create(p, body).await?;
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(
                    &state,
                    StatusCode::CREATED,
                    &created,
                )
                .await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(
                    &state,
                    StatusCode::CREATED,
                    &created,
                )
                .await;
            }
            Ok(negotiate::respond_rm::<Composition>(
                h,
                StatusCode::CREATED,
                &created,
                "composition",
            ))
        }
        "composition_get" => {
            let p = params::build::<CompositionGetParams>(&parts.path, q, h)?;
            let comp = state.backend().composition_get(p).await?;
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, ok, &comp).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, ok, &comp).await;
            }
            Ok(negotiate::respond_rm::<Composition>(
                h,
                ok,
                &comp,
                "composition",
            ))
        }
        "composition_update" => {
            let p = params::build::<CompositionUpdateParams>(&parts.path, q, h)?;
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            let updated = state.backend().composition_update(p, body).await?;
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, ok, &updated).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, ok, &updated).await;
            }
            Ok(negotiate::respond_rm::<Composition>(
                h,
                ok,
                &updated,
                "composition",
            ))
        }
        "composition_delete" => {
            let p = params::build::<CompositionDeleteParams>(&parts.path, q, h)?;
            state.backend().composition_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "versioned_composition_get" => {
            let p = params::build::<VersionedCompositionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().versioned_composition_get(p).await?,
            ))
        }
        "versioned_composition_revision_history" => {
            let p = params::build::<VersionedCompositionRevisionHistoryParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_composition_revision_history(p)
                    .await?,
            ))
        }
        "versioned_composition_version_get_at_time" => {
            let p = params::build::<VersionedCompositionVersionGetAtTimeParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_composition_version_get_at_time(p)
                    .await?,
            ))
        }
        "versioned_composition_version_get_by_id" => {
            let p = params::build::<VersionedCompositionVersionGetByIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_composition_version_get_by_id(p)
                    .await?,
            ))
        }
        "directory_get_at_time" => {
            let p = params::build::<DirectoryGetAtTimeParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<Folder>(
                h,
                ok,
                &state.backend().directory_get_at_time(p).await?,
                "folder",
            ))
        }
        "directory_update" => {
            let p = params::build::<DirectoryUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            Ok(negotiate::respond_rm::<Folder>(
                h,
                ok,
                &state.backend().directory_update(p, body).await?,
                "folder",
            ))
        }
        "directory_create" => {
            let p = params::build::<DirectoryCreateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            Ok(negotiate::respond_rm::<Folder>(
                h,
                StatusCode::CREATED,
                &state.backend().directory_create(p, body).await?,
                "folder",
            ))
        }
        "directory_delete" => {
            let p = params::build::<DirectoryDeleteParams>(&parts.path, q, h)?;
            state.backend().directory_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "directory_get_by_version_id" => {
            let p = params::build::<DirectoryGetByVersionIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond_rm::<Folder>(
                h,
                ok,
                &state.backend().directory_get_by_version_id(p).await?,
                "folder",
            ))
        }
        "contribution_create" => {
            let p = params::build::<ContributionCreateParams>(&parts.path, q, h)?;
            // A CONTRIBUTION's wire DTO differs from the RM type; JSON only for now.
            // TODO(port): P12 — typed XML contribution bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                // `contribution_*` is defined on both EhrApi and DemographicApi
                // (shared method names); disambiguate on the trait-object backend.
                &EhrApi::contribution_create(state.backend(), p, body).await?,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &EhrApi::contribution_get(state.backend(), p).await?,
            ))
        }
        "ehr_tags_get" => {
            let p = params::build::<EhrTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().ehr_tags_get(p).await?,
            ))
        }
        "composition_tags_get" => {
            let p = params::build::<CompositionTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().composition_tags_get(p).await?,
            ))
        }
        "composition_tags_update" => {
            let p = params::build::<CompositionTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().composition_tags_update(p, body).await?,
            ))
        }
        "composition_tags_delete" => {
            let p = params::build::<CompositionTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().composition_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "ehr_status_tags_get" => {
            let p = params::build::<EhrStatusTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().ehr_status_tags_get(p).await?,
            ))
        }
        "ehr_status_tags_update" => {
            let p = params::build::<EhrStatusTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().ehr_status_tags_update(p, body).await?,
            ))
        }
        "ehr_status_tags_delete" => {
            let p = params::build::<EhrStatusTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().ehr_status_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted ehr operation: {other}"),
        ))),
    }
}
