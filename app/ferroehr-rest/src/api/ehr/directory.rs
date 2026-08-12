// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `DIRECTORY` (FOLDER) resource.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (DIRECTORY) +
//! `specifications/operations/{directory_get_at_time,directory_update,
//! directory_create,directory_delete,directory_get_by_version_id}.yaml`.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the handler carries the canonical fragment the \
              negotiate seam produced once (stored-content serving / commit interior)"
)]

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Folder;

use ferroehr::service::response::ServiceResponse;
use ferroehr::service::status::CallStatusType;

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::overview::version_id::{parse_ehr_id, parse_version_uid, require_if_match};
use crate::state::AppState;
use crate::{negotiate, params};

#[expect(
    clippy::too_many_lines,
    reason = "one arm per DIRECTORY operation: a flat match keeps every \
              operation's wire behaviour readable in one place"
)]
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

    // DIRECTORY (FOLDER) is not templated → no Simplified-Formats mapping;
    // reject a simplified Content-Type/Accept uniformly (see `formats::dispatch`).
    crate::formats::dispatch::guard_non_templated(h)?;

    match op {
        "directory_get_at_time" => {
            let p = params::build::<DirectoryGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let resp = state
                .backend()
                .get_directory_at_time(ehr_id, p.version_at_time, p.path)
                .await?;
            // Deleted directory → 204 (directory_get_at_time.yaml 204_because_deleted_at_time).
            if resp.body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            // ETag + Last-Modified on the read too (overview §"ETag and
            // Last-Modified": both SHOULD accompany versioned resources);
            // no Location on GET (overview §Location).
            // A FOLDER's DV_MULTIMEDIA is externalized by the same generic
            // versioning path as a COMPOSITION's, so it re-inlines here.
            let mut resp = resp;
            resp.body = super::expand_multimedia_if_requested(&state, q, resp.body).await?;
            let mut out = negotiate::respond_rm::<Folder>(h, ok, &resp.body, "folder");
            if let Some(meta) = &resp.meta {
                negotiate::set_versioning_headers(&mut out, meta);
            }
            Ok(out)
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
            )?;
            // 204_directory_updated (default) / 200_directory_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            // Judge the wrapper-header tags before the commit (see
            // `crate::api::ehr::pending_item_tags`); the write stays after it.
            let pending_tags = super::pending_item_tags(h)?;
            match state.backend().update_directory(ehr_id, uv).await {
                Ok(meta) => {
                    // apply item-tag write-wrapper headers to the new version.
                    let stored_tags = super::apply_item_tag_headers(
                        &state,
                        ehr_id,
                        "FOLDER",
                        &meta.uid,
                        pending_tags,
                    )
                    .await?;
                    let repr = if negotiate::prefers_representation(h) {
                        state
                            .backend()
                            .get_directory_at_time(ehr_id, None, None)
                            .await?
                            .body
                    } else {
                        Value::Null
                    };
                    let resp = ServiceResponse::new(repr, meta);
                    let mut resp = negotiate::write_rm::<Folder>(
                        h,
                        &base,
                        no_content,
                        ok,
                        Some("directory"),
                        &resp,
                        "folder",
                    );
                    super::echo_item_tags(&mut resp, &stored_tags);
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
            )?;
            // Judge the wrapper-header tags before the commit (see
            // `crate::api::ehr::pending_item_tags`); the write stays after it.
            let pending_tags = super::pending_item_tags(h)?;
            let meta = state.backend().create_directory(ehr_id, uv).await?;
            // apply item-tag write-wrapper headers to the committed FOLDER.
            let stored_tags =
                super::apply_item_tag_headers(&state, ehr_id, "FOLDER", &meta.uid, pending_tags)
                    .await?;
            let repr = if negotiate::prefers_representation(h) {
                state
                    .backend()
                    .get_directory_at_time(ehr_id, None, None)
                    .await?
                    .body
            } else {
                Value::Null
            };
            let resp = ServiceResponse::new(repr, meta);
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
            super::echo_item_tags(&mut resp, &stored_tags);
            Ok(resp)
        }
        "directory_delete" => {
            let p = params::build::<DirectoryDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // 204_because_deleted declares no headers; 412_directory → latest version_uid.
            // A DELETE commits a `523|deleted|` version, so the committal
            // request headers are accepted and merged here too (overview
            // §"openehr-version and openehr-audit-details": PUT, POST and
            // DELETE).
            let update_audit = crate::overview::committal::committal_audit_for_delete(
                h,
                super::committer_proxy(),
            )?;
            match state
                .backend()
                .delete_directory(
                    ehr_id,
                    Some(require_if_match(&p.if_match)?),
                    update_audit.as_ref(),
                )
                .await
            {
                // 204 with the NEW deleted version's weak ETag + Last-Modified
                // (overview §"ETag and Last-Modified": both SHOULD accompany
                // versioned resources; the delete commits a 523|deleted|
                // version — RM common master06 §Logical Deletion).
                Ok(resp) => Ok(negotiate::deleted_with_headers(
                    &base,
                    Some("directory"),
                    &resp,
                )),
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
            let resp = state
                .backend()
                .get_directory_at_version(ehr_id, ovid, p.path.as_deref())
                .await?;
            if resp.body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            // ETag + Last-Modified on the version read (overview §"ETag and
            // Last-Modified"); no Location on GET (overview §Location).
            // A FOLDER's DV_MULTIMEDIA is externalized by the same generic
            // versioning path as a COMPOSITION's, so it re-inlines here.
            let mut resp = resp;
            resp.body = super::expand_multimedia_if_requested(&state, q, resp.body).await?;
            let mut out = negotiate::respond_rm::<Folder>(h, ok, &resp.body, "folder");
            if let Some(meta) = &resp.meta {
                negotiate::set_versioning_headers(&mut out, meta);
            }
            Ok(out)
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
