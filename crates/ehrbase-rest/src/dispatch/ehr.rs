//! HTTP dispatch for the `ehr` API group.
//!
//! Each arm rebuilds the operation's `*Params`, decodes any body (RM-typed
//! bodies accept JSON or canonical XML), calls the [`EhrService`] backend, and
//! renders a negotiated response from the returned [`ServiceResponse`]
//! (RM payload + typed [`ResourceMeta`]). The whole group is served through the
//! envelope seam (W2-A) — the generated `EhrApi` returned a bare `Value` that
//! could not carry the spec's `ETag`/`Location` headers or drive `Prefer`.
//!
//! Header policy is per operation, per the ITS-REST 1.0.3 response definitions:
//! writes honour `Prefer` (`return=minimal` default → header-only, vs
//! `return=representation` → full body) and set `ETag`/`Location`; the
//! `COMPOSITION/EHR_STATUS` reads set `ETag`/`Location` too; header-free reads
//! (VERSION wrappers, revision histories, EHR/FOLDER retrieval, item tags,
//! CONTRIBUTION retrieval) render the body alone. On a `409`/`412` the write
//! arms decorate the error with the current `version_uid` in `ETag`/`Location`.

use axum::response::{IntoResponse, Response};
use http::StatusCode;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrCreateParams, EhrCreateWithIdParams,
    EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Composition, Ehr, EhrStatus, Folder};

use super::{BoxResponse, RequestParts};
use crate::backend::EhrService;
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

/// Whether an error is the optimistic-concurrency precondition failure
/// (`If-Match` mismatch → `412`).
fn is_precondition(e: &ApiError) -> bool {
    matches!(e, ApiError::PreconditionFailed(_))
}

/// Whether an error is a state conflict (a stale `uid_based_id` on delete → `409`).
fn is_conflict(e: &ApiError) -> bool {
    matches!(e, ApiError::Conflict(_))
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
    let created = StatusCode::CREATED;
    let no_content = StatusCode::NO_CONTENT;
    // The configured base path, for building `Location` URLs.
    let base = state.config().base_path.clone();

    match op {
        // ── EHR ──────────────────────────────────────────────────────────────
        "ehr_get_by_subject" => {
            let p = params::build::<EhrGetBySubjectParams>(&parts.path, q, h)?;
            let resp = state.backend().ehr_get_by_subject(p).await?;
            // 200_EHR: no ETag/Location declared for EHR retrieval.
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &resp.body, "ehr"))
        }
        "ehr_create" => {
            let p = params::build::<EhrCreateParams>(&parts.path, q, h)?;
            let body = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            // 201_EHR: ETag(ehr_id) + Location; body only on return=representation.
            let resp = state.backend().ehr_create(p, body).await?;
            Ok(negotiate::write_rm::<Ehr>(
                h, &base, created, created, None, &resp, "ehr",
            ))
        }
        "ehr_get_by_id" => {
            let p = params::build::<EhrGetByIdParams>(&parts.path, q, h)?;
            let resp = state.backend().ehr_get_by_id(p).await?;
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &resp.body, "ehr"))
        }
        "ehr_create_with_id" => {
            let p = params::build::<EhrCreateWithIdParams>(&parts.path, q, h)?;
            let body = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let resp = state.backend().ehr_create_with_id(p, body).await?;
            Ok(negotiate::write_rm::<Ehr>(
                h, &base, created, created, None, &resp, "ehr",
            ))
        }
        // ── EHR_STATUS ───────────────────────────────────────────────────────
        "ehr_status_get_by_version_id" => {
            let p = params::build::<EhrStatusGetByVersionIdParams>(&parts.path, q, h)?;
            // F-01-03: the bare EHR_STATUS at that version (not ORIGINAL_VERSION);
            // 200_EHR_STATUS_retrieved: ETag(version_uid) + Location.
            let resp = state.backend().ehr_status_get_by_version_id(p).await?;
            Ok(negotiate::read_rm::<EhrStatus>(
                h,
                &base,
                Some("ehr_status"),
                &resp,
                "ehr_status",
            ))
        }
        "ehr_status_get_at_time" => {
            let p = params::build::<EhrStatusGetAtTimeParams>(&parts.path, q, h)?;
            let resp = state.backend().ehr_status_get_at_time(p).await?;
            Ok(negotiate::read_rm::<EhrStatus>(
                h,
                &base,
                Some("ehr_status"),
                &resp,
                "ehr_status",
            ))
        }
        "ehr_status_update" => {
            let p = params::build::<EhrStatusUpdateParams>(&parts.path, q, h)?;
            let ehr_id = p.ehr_id.clone();
            let body = negotiate::rm_value::<EhrStatus>(h, &parts.body)?;
            // 204_EHR_STATUS (default minimal) / 200_EHR_STATUS_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().ehr_status_update(p, body).await {
                Ok(resp) => Ok(negotiate::write_rm::<EhrStatus>(
                    h,
                    &base,
                    no_content,
                    ok,
                    Some("ehr_status"),
                    &resp,
                    "ehr_status",
                )),
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .ehr_status_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        e,
                        &base,
                        Some("ehr_status"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "versioned_ehr_status_get" => {
            let p = params::build::<VersionedEhrStatusGetParams>(&parts.path, q, h)?;
            let resp = state.backend().versioned_ehr_status_get(p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_ehr_status_revision_history" => {
            let p = params::build::<VersionedEhrStatusRevisionHistoryParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_ehr_status_revision_history(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_ehr_status_version_get_at_time" => {
            let p = params::build::<VersionedEhrStatusVersionGetAtTimeParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_ehr_status_version_get_at_time(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_ehr_status_version_get_by_id" => {
            let p = params::build::<VersionedEhrStatusVersionGetByIdParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_ehr_status_version_get_by_id(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        // ── COMPOSITION ──────────────────────────────────────────────────────
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
            let resp = state.backend().composition_create(p, body).await?;
            // FLAT/STRUCTURED Accept returns the Better representation (interop
            // format); the canonical path honours ETag/Location + Prefer.
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, created, &resp.body).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, created, &resp.body)
                    .await;
            }
            Ok(negotiate::write_rm::<Composition>(
                h,
                &base,
                created,
                created,
                Some("composition"),
                &resp,
                "composition",
            ))
        }
        "composition_get" => {
            let p = params::build::<CompositionGetParams>(&parts.path, q, h)?;
            let resp = state.backend().composition_get(p).await?;
            // A deleted version resolves to a null body → 204 No Content
            // (composition_get.yaml 204_because_deleted*; F-02-01).
            if resp.is_empty() {
                return Ok(negotiate::empty(no_content));
            }
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, ok, &resp.body).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, ok, &resp.body).await;
            }
            // 200_COMPOSITION_retrieved: ETag(version_uid) + Location.
            Ok(negotiate::read_rm::<Composition>(
                h,
                &base,
                Some("composition"),
                &resp,
                "composition",
            ))
        }
        "composition_update" => {
            let p = params::build::<CompositionUpdateParams>(&parts.path, q, h)?;
            let ehr_id = p.ehr_id.clone();
            let uid = p.uid_based_id.clone();
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            match state.backend().composition_update(p, body).await {
                Ok(resp) => {
                    if negotiate::wants_flat(h) {
                        return super::flat::composition_flat_response(&state, ok, &resp.body)
                            .await;
                    }
                    if negotiate::wants_structured(h) {
                        return super::flat::composition_structured_response(
                            &state, ok, &resp.body,
                        )
                        .await;
                    }
                    // 200_COMPOSITION_updated: body only on return=representation.
                    Ok(negotiate::write_rm::<Composition>(
                        h,
                        &base,
                        ok,
                        ok,
                        Some("composition"),
                        &resp,
                        "composition",
                    ))
                }
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        e,
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "composition_delete" => {
            let p = params::build::<CompositionDeleteParams>(&parts.path, q, h)?;
            let ehr_id = p.ehr_id.clone();
            let uid = p.uid_based_id.clone();
            // 204_COMPOSITION_deleted: ETag + Location of the deleted version.
            // 409_COMPOSITION_with_uid_based_id (stale) → latest version_uid.
            match state.backend().composition_delete(p).await {
                Ok(resp) => Ok(negotiate::deleted_with_headers(
                    &base,
                    Some("composition"),
                    &resp,
                )),
                Err(e) if is_conflict(&e) => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        e,
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "versioned_composition_get" => {
            let p = params::build::<VersionedCompositionGetParams>(&parts.path, q, h)?;
            let resp = state.backend().versioned_composition_get(p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_composition_revision_history" => {
            let p = params::build::<VersionedCompositionRevisionHistoryParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_composition_revision_history(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_composition_version_get_at_time" => {
            let p = params::build::<VersionedCompositionVersionGetAtTimeParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_composition_version_get_at_time(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_composition_version_get_by_id" => {
            let p = params::build::<VersionedCompositionVersionGetByIdParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_composition_version_get_by_id(p)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────
        "directory_get_at_time" => {
            let p = params::build::<DirectoryGetAtTimeParams>(&parts.path, q, h)?;
            let resp = state.backend().directory_get_at_time(p).await?;
            // Deleted directory → 204 (directory_get_at_time.yaml 204_because_deleted_at_time).
            // 200_FOLDER_retrieved declares no ETag/Location.
            if resp.is_empty() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &resp.body, "folder"))
        }
        "directory_update" => {
            let p = params::build::<DirectoryUpdateParams>(&parts.path, q, h)?;
            let ehr_id = p.ehr_id.clone();
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            // 204_directory_updated (default) / 200_directory_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().directory_update(p, body).await {
                Ok(resp) => Ok(negotiate::write_rm::<Folder>(
                    h,
                    &base,
                    no_content,
                    ok,
                    Some("directory"),
                    &resp,
                    "folder",
                )),
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        e,
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "directory_create" => {
            let p = params::build::<DirectoryCreateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            let resp = state.backend().directory_create(p, body).await?;
            // 201_directory: ETag + Location; body only on return=representation.
            Ok(negotiate::write_rm::<Folder>(
                h,
                &base,
                created,
                created,
                Some("directory"),
                &resp,
                "folder",
            ))
        }
        "directory_delete" => {
            let p = params::build::<DirectoryDeleteParams>(&parts.path, q, h)?;
            let ehr_id = p.ehr_id.clone();
            // 204_because_deleted declares no headers; 412_directory → latest version_uid.
            match state.backend().directory_delete(p).await {
                Ok(_) => Ok(negotiate::empty(no_content)),
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        e,
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "directory_get_by_version_id" => {
            let p = params::build::<DirectoryGetByVersionIdParams>(&parts.path, q, h)?;
            let resp = state.backend().directory_get_by_version_id(p).await?;
            if resp.is_empty() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &resp.body, "folder"))
        }
        // ── CONTRIBUTION ─────────────────────────────────────────────────────
        "contribution_create" => {
            let p = params::build::<ContributionCreateParams>(&parts.path, q, h)?;
            // PORT NOTE: a CONTRIBUTION commit is a wrapper DTO (a version set +
            // audit), not a single canonical RM value with a defined canonical-XML
            // shape — so it is accepted as JSON only.
            let body = negotiate::json_value(h, &parts.body)?;
            // `contribution_*` is defined on EhrService and DemographicApi (shared
            // method names); disambiguate on the trait-object backend.
            let resp = EhrService::contribution_create(state.backend(), p, body).await?;
            // 201_CONTRIBUTION: ETag(contribution_uid) + Location; body per Prefer.
            Ok(negotiate::write_json(
                h,
                &base,
                created,
                created,
                Some("contribution"),
                &resp,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            let resp = EhrService::contribution_get(state.backend(), p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        // ── item tags ────────────────────────────────────────────────────────
        "ehr_tags_get" => {
            let p = params::build::<EhrTagsGetParams>(&parts.path, q, h)?;
            let resp = state.backend().ehr_tags_get(p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "composition_tags_get" => {
            let p = params::build::<CompositionTagsGetParams>(&parts.path, q, h)?;
            let resp = state.backend().composition_tags_get(p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "composition_tags_update" => {
            let p = params::build::<CompositionTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let resp = state.backend().composition_tags_update(p, body).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "composition_tags_delete" => {
            let p = params::build::<CompositionTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().composition_tags_delete(p).await?;
            Ok(negotiate::empty(no_content))
        }
        "ehr_status_tags_get" => {
            let p = params::build::<EhrStatusTagsGetParams>(&parts.path, q, h)?;
            let resp = state.backend().ehr_status_tags_get(p).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "ehr_status_tags_update" => {
            let p = params::build::<EhrStatusTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let resp = state.backend().ehr_status_tags_update(p, body).await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "ehr_status_tags_delete" => {
            let p = params::build::<EhrStatusTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().ehr_status_tags_delete(p).await?;
            Ok(negotiate::empty(no_content))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted ehr operation: {other}"),
        ))),
    }
}
