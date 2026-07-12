//! The `COMPOSITION` resource + its item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (COMPOSITION) +
//! `specifications/operations/{composition_create,composition_get,
//! composition_update,composition_delete,composition_tags_get,
//! composition_tags_update,composition_tags_delete}.yaml`.
//!
//! The FLAT/STRUCTURED converters are reached through the group-level
//! `super::flat` alias onto the `pub(crate)` `crate::formats::dispatch`
//! converters (see the parent module).

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use ehrbase_sm::{CallStatusType, Platform};
use ehrbase_sm::{ResourceMeta, ServiceResponse};

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::overview::version_id::{
    object_id_uuid, parse_ehr_id, parse_uid_based_id, parse_version_uid, require_if_match,
};
use crate::state::AppState;
use crate::{negotiate, params};

#[allow(clippy::too_many_lines)] // one arm per COMPOSITION operation; a flat match is clearest
pub(super) async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let created = StatusCode::CREATED;
    let no_content = StatusCode::NO_CONTENT;
    let base = state.config().base_path.clone();

    match op {
        "composition_create" => {
            let p = params::build::<CompositionCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // A FLAT/STRUCTURED (wt.flat/structured+json) body is rebuilt into a
            // canonical composition.
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_CREATION,
                "COMPOSITION creation",
                None,
            );
            let uid = state.backend().create_composition(ehr_id, uv).await?;
            // G-4: apply the openehr-item-tag / openehr-version-item-tag
            // write-wrapper headers to the committed COMPOSITION
            // (Requests_and_responses.md §…§Usage in Requests).
            let stored_tags =
                super::apply_item_tag_headers(&state, ehr_id, "COMPOSITION", &uid, h).await?;
            let mut resp =
                composition_write_response(&state, h, &base, ehr_id, uid, created, created).await?;
            if let Some((names, tags)) = stored_tags {
                super::echo_item_tags(&mut resp, &names, &tags);
            }
            Ok(resp)
        }
        "composition_get" => {
            let p = params::build::<CompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let uid = parse_uid_based_id(&p.uid_based_id)?;
            let body = if let Some(ovid) = uid.version {
                state
                    .backend()
                    .get_composition_at_version(ehr_id, ovid)
                    .await?
            } else if p.version_at_time.is_some() {
                state
                    .backend()
                    .get_composition_at_time(ehr_id, uid.vo_id, p.version_at_time)
                    .await?
            } else {
                state
                    .backend()
                    .get_composition_latest(ehr_id, uid.vo_id)
                    .await?
            };
            // A deleted version resolves to a null body → 204 No Content
            // (composition_get.yaml 204_because_deleted*; F-02-01).
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            // `?expand_multimedia=true`: transparently re-inline any
            // externalized DV_MULTIMEDIA blobs, verifying integrity. A no-op
            // when externalization is off or the body has no external media.
            // Not an openEHR spec parameter, so read off the raw query string
            // (the `template_id` precedent), never a generated params struct.
            let body = if params::query_param(q, "expand_multimedia").as_deref() == Some("true") {
                state.backend().expand_multimedia(body).await?
            } else {
                body
            };
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, ok, &body).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, ok, &body).await;
            }
            // 200_COMPOSITION_retrieved: ETag(version_uid) + Location.
            let resp = super::read_resp(&p.ehr_id, body);
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
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let uid = parse_uid_based_id(&p.uid_based_id)?;
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            // A body-supplied COMPOSITION.uid must identify the same
            // versioned object as the path `uid_based_id` (ITS-REST
            // `composition_update`: "the uid, if present, must match") —
            // a mismatched body uid is a 400, never a silent write to the
            // path's object.
            if let Some(body_uid) = body
                .get("uid")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str)
            {
                let body_vo = body_uid.split("::").next().unwrap_or(body_uid);
                if body_vo.parse::<Uuid>() != Ok(uid.vo_id) {
                    return Err(ApiError::BadRequest(format!(
                        "the body COMPOSITION.uid {body_uid:?} does not identify the \
                         versioned object addressed by the request path ({})",
                        uid.vo_id
                    ))
                    .into());
                }
            }
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_MODIFICATION,
                "COMPOSITION update",
                Some(require_if_match(&p.if_match)?),
            );
            match state
                .backend()
                .update_composition(ehr_id, uid.vo_id, uv)
                .await
            {
                Ok(new_uid) => {
                    // G-4: apply item-tag write-wrapper headers to the new version.
                    let stored_tags =
                        super::apply_item_tag_headers(&state, ehr_id, "COMPOSITION", &new_uid, h)
                            .await?;
                    let mut resp =
                        composition_write_response(&state, h, &base, ehr_id, new_uid, ok, ok)
                            .await?;
                    if let Some((names, tags)) = stored_tags {
                        super::echo_item_tags(&mut resp, &names, &tags);
                    }
                    Ok(resp)
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, uid.vo_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "composition_delete" => {
            let p = params::build::<CompositionDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // composition_delete.yaml: the uid_based_id MUST be an OBJECT_VERSION_ID
            // (the preceding_version_uid to delete); a bare HIER_OBJECT_ID → 400.
            let ovid = parse_version_uid(&p.uid_based_id)?;
            let vo_id = object_id_uuid(&ovid).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "OBJECT_VERSION_ID object_id is not a UUID: {}",
                    p.uid_based_id
                ))
            })?;
            match state.backend().delete_composition(ehr_id, ovid).await {
                Ok(uid) => {
                    // 204_COMPOSITION_deleted: ETag + Location of the deleted version.
                    let resp = ServiceResponse::deleted(ResourceMeta::new(p.ehr_id, uid));
                    Ok(negotiate::deleted_with_headers(
                        &base,
                        Some("composition"),
                        &resp,
                    ))
                }
                // 409_COMPOSITION_with_uid_based_id (stale) → latest version_uid.
                Err(e) if e.status == CallStatusType::CompositionAlreadyExists => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, vo_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "composition_tags_get" => {
            let p = params::build::<CompositionTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .target_tags_get(ehr_id, p.uid_based_id)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "composition_tags_update" => {
            let p = params::build::<CompositionTagsUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let tags = state
                .backend()
                .target_tags_replace(ehr_id, p.uid_based_id, "COMPOSITION", body)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "composition_tags_delete" => {
            let p = params::build::<CompositionTagsDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            state
                .backend()
                .target_tag_delete(ehr_id, p.uid_based_id, p.key)
                .await?;
            Ok(negotiate::empty(no_content))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}

/// Render a COMPOSITION create/update response: FLAT/STRUCTURED interop bodies
/// when requested (always the representation), else the canonical
/// `ETag`/`Location` + `Prefer` write response.
async fn composition_write_response<S: Platform>(
    state: &AppState<S>,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: Uuid,
    uid: String,
    minimal: StatusCode,
    repr: StatusCode,
) -> Result<Response, RestError> {
    let ehr_id_str = ehr_id.to_string();
    // FLAT/STRUCTURED Accept returns the Better representation (interop format),
    // which needs the stored body regardless of Prefer.
    if negotiate::wants_flat(h) || negotiate::wants_structured(h) {
        let ovid = parse_version_uid(&uid)?;
        let body = state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?;
        if negotiate::wants_flat(h) {
            return super::flat::composition_flat_response(state, repr, &body).await;
        }
        return super::flat::composition_structured_response(state, repr, &body).await;
    }
    let body = if negotiate::prefers_representation(h) {
        let ovid = parse_version_uid(&uid)?;
        state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, ResourceMeta::new(ehr_id_str, uid));
    Ok(negotiate::write_rm::<Composition>(
        h,
        base,
        minimal,
        repr,
        Some("composition"),
        &resp,
        "composition",
    ))
}
