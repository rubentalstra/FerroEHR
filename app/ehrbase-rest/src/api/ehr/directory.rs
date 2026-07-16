//! The `DIRECTORY` (FOLDER) resource.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (DIRECTORY) +
//! `specifications/operations/{directory_get_at_time,directory_update,
//! directory_create,directory_delete,directory_get_by_version_id}.yaml`.

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Folder;

use ehrbase::service::response::{ResourceMeta, ServiceResponse};
use ehrbase::service::status::CallStatusType;

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::overview::version_id::{parse_ehr_id, parse_version_uid, require_if_match};
use crate::state::AppState;
use crate::{negotiate, params};

#[allow(clippy::too_many_lines)] // one arm per DIRECTORY operation; a flat match is clearest
pub(super) async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let created = StatusCode::CREATED;
    let no_content = StatusCode::NO_CONTENT;
    let base = state.config().server.base_path.clone();

    match op {
        "directory_get_at_time" => {
            let p = params::build::<DirectoryGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .get_directory_at_time(ehr_id, p.version_at_time, p.path)
                .await?;
            // Deleted directory → 204 (directory_get_at_time.yaml 204_because_deleted_at_time).
            // 200_FOLDER_retrieved declares no ETag/Location.
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &body, "folder"))
        }
        "directory_update" => {
            let p = params::build::<DirectoryUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_MODIFICATION,
                "DIRECTORY update",
                Some(require_if_match(&p.if_match)?),
            );
            // 204_directory_updated (default) / 200_directory_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().update_directory(ehr_id, uv).await {
                Ok(uid) => {
                    // apply item-tag write-wrapper headers to the new version.
                    let stored_tags =
                        super::apply_item_tag_headers(&state, ehr_id, "FOLDER", &uid, h).await?;
                    let repr = if negotiate::prefers_representation(h) {
                        state
                            .backend()
                            .get_directory_at_time(ehr_id, None, None)
                            .await?
                    } else {
                        Value::Null
                    };
                    let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
                    let mut resp = negotiate::write_rm::<Folder>(
                        h,
                        &base,
                        no_content,
                        ok,
                        Some("directory"),
                        &resp,
                        "folder",
                    );
                    if let Some((names, tags)) = stored_tags {
                        super::echo_item_tags(&mut resp, &names, &tags);
                    }
                    Ok(resp)
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "directory_create" => {
            let p = params::build::<DirectoryCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_CREATION,
                "DIRECTORY creation",
                None,
            );
            let uid = state.backend().create_directory(ehr_id, uv).await?;
            // apply item-tag write-wrapper headers to the committed FOLDER.
            let stored_tags =
                super::apply_item_tag_headers(&state, ehr_id, "FOLDER", &uid, h).await?;
            let repr = if negotiate::prefers_representation(h) {
                state
                    .backend()
                    .get_directory_at_time(ehr_id, None, None)
                    .await?
            } else {
                Value::Null
            };
            let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
            // 201_directory: ETag + Location; body only on return=representation.
            let mut resp = negotiate::write_rm::<Folder>(
                h,
                &base,
                created,
                created,
                Some("directory"),
                &resp,
                "folder",
            );
            if let Some((names, tags)) = stored_tags {
                super::echo_item_tags(&mut resp, &names, &tags);
            }
            Ok(resp)
        }
        "directory_delete" => {
            let p = params::build::<DirectoryDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // 204_because_deleted declares no headers; 412_directory → latest version_uid.
            match state
                .backend()
                .delete_directory(ehr_id, Some(require_if_match(&p.if_match)?))
                .await
            {
                Ok(()) => Ok(negotiate::empty(no_content)),
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "directory_get_by_version_id" => {
            let p = params::build::<DirectoryGetByVersionIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let ovid = parse_version_uid(&p.version_uid)?;
            let body = state
                .backend()
                .get_directory_at_version(ehr_id, ovid)
                .await?;
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &body, "folder"))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
