//! The `EHR_STATUS` resource + its item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (`EHR_STATUS`) +
//! `specifications/operations/{ehr_status_get_by_version_id,
//! ehr_status_get_at_time,ehr_status_update,ehr_status_tags_get,
//! ehr_status_tags_update,ehr_status_tags_delete}.yaml`.

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    EhrStatusGetAtTimeParams, EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams,
    EhrStatusTagsGetParams, EhrStatusTagsUpdateParams, EhrStatusUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::EhrStatus;

use ehrbase::service::response::{ResourceMeta, ServiceResponse};
use ehrbase::service::status::CallStatusType;

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::overview::version_id::{parse_ehr_id, parse_version_uid, require_if_match};
use crate::state::AppState;
use crate::{negotiate, params};

#[allow(clippy::too_many_lines)] // one arm per EHR_STATUS operation; a flat match is clearest
pub(super) async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let no_content = StatusCode::NO_CONTENT;
    let base = state.config().server.base_path.clone();

    // EHR_STATUS is not templated → no Simplified-Formats mapping; reject a
    // simplified Content-Type/Accept uniformly (see `formats::dispatch`).
    crate::formats::dispatch::guard_non_templated(h)?;

    match op {
        "ehr_status_get_by_version_id" => {
            let p = params::build::<EhrStatusGetByVersionIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let (vo_id, version) = super::version_components(&parse_version_uid(&p.version_uid)?)?;
            // the bare EHR_STATUS at that version (not ORIGINAL_VERSION);
            // 200_EHR_STATUS_retrieved: ETag(version_uid) + Location.
            let body = state
                .backend()
                .get_ehr_status_at_version(ehr_id, ehrbase::ids::VoId(vo_id), &version)
                .await?;
            let resp = ServiceResponse::new(body, ResourceMeta::new(p.ehr_id, p.version_uid));
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
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .get_ehr_status_at_time(ehr_id, p.version_at_time)
                .await?;
            let resp = super::read_resp(&p.ehr_id, body);
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
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<EhrStatus>(h, &parts.body)?;
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_MODIFICATION,
                "EHR_STATUS update",
                Some(require_if_match(&p.if_match)?),
            );
            // 204_EHR_STATUS (default minimal) / 200_EHR_STATUS_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().replace_ehr_status(ehr_id, uv).await {
                Ok(uid) => {
                    // apply the openehr-item-tag / openehr-version-item-tag
                    // write-wrapper headers to the committed target
                    // (Requests_and_responses.md §…§Usage in Requests).
                    let stored_tags =
                        super::apply_item_tag_headers(&state, ehr_id, "EHR_STATUS", &uid, h)
                            .await?;
                    let repr = if negotiate::prefers_representation(h) {
                        state.backend().get_ehr_status(ehr_id).await?
                    } else {
                        Value::Null
                    };
                    let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
                    let mut resp = negotiate::write_rm::<EhrStatus>(
                        h,
                        &base,
                        no_content,
                        ok,
                        Some("ehr_status"),
                        &resp,
                        "ehr_status",
                    );
                    if let Some((names, tags)) = stored_tags {
                        super::echo_item_tags(&mut resp, &names, &tags);
                    }
                    Ok(resp)
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .ehr_status_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("ehr_status"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "ehr_status_tags_get" => {
            let p = params::build::<EhrStatusTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .target_tags_get(ehr_id, p.uid_based_id)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "ehr_status_tags_update" => {
            let p = params::build::<EhrStatusTagsUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let tags = state
                .backend()
                .target_tags_replace(ehr_id, p.uid_based_id, "EHR_STATUS", body)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "ehr_status_tags_delete" => {
            let p = params::build::<EhrStatusTagsDeleteParams>(&parts.path, q, h)?;
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
