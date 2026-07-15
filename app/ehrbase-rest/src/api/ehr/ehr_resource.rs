//! The `EHR` resource + EHR-level item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (EHR API) +
//! `specifications/operations/{ehr_get_by_subject,ehr_create,
//! ehr_create_with_id,ehr_get_by_id,ehr_tags_get}.yaml`.

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use openehr_its::rest::generated::ehr::{
    EhrCreateParams, EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams,
    EhrTagsGetParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Ehr, EhrStatus};

use ehrbase_sm::Platform;
use ehrbase_sm::{ResourceMeta, ServiceResponse};

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::overview::version_id::parse_ehr_id;
use crate::state::AppState;
use crate::{negotiate, params};

pub(super) async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    // The configured base path, for building `Location` URLs.
    let base = state.config().base_path.clone();

    match op {
        "ehr_get_by_subject" => {
            let p = params::build::<EhrGetBySubjectParams>(&parts.path, q, h)?;
            let body = state
                .backend()
                .ehr_object_for_subject(&p.subject_id, &p.subject_namespace)
                .await?;
            // 200_EHR: no ETag/Location declared for EHR retrieval.
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &body, "ehr"))
        }
        "ehr_create" => {
            let _p = params::build::<EhrCreateParams>(&parts.path, q, h)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let ehr_id = state.backend().create_ehr(status).await?;
            ehr_write_response(&state, h, &base, ehr_id).await
        }
        "ehr_create_with_id" => {
            let p = params::build::<EhrCreateWithIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let ehr_id = state.backend().create_ehr_with_id(ehr_id, status).await?;
            ehr_write_response(&state, h, &base, ehr_id).await
        }
        "ehr_get_by_id" => {
            let p = params::build::<EhrGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().ehr_object(ehr_id).await?;
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &body, "ehr"))
        }
        "ehr_tags_get" => {
            let p = params::build::<EhrTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .ehr_tags_get(ehr_id, p.tag_key, p.tag_value, p.tag_target_path)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}

/// Render an EHR create response (`201_EHR)`: `ETag(ehr_id)` + `Location`, with the
/// RM `EHR` body only on `Prefer: return=representation`.
async fn ehr_write_response<S: Platform>(
    state: &AppState<S>,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: Uuid,
) -> Result<Response, RestError> {
    let ehr_id_str = ehr_id.to_string();
    let body = if negotiate::prefers_representation(h) {
        // `ehr_created_object` serves the just-committed EHR body from the
        // create-time stash (built from the commit results), avoiding the
        // `ehr_summary` re-read; it falls back to a full read on a stash miss.
        state.backend().ehr_created_object(ehr_id).await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, ResourceMeta::new(ehr_id_str.clone(), ehr_id_str));
    Ok(negotiate::write_rm::<Ehr>(
        h,
        base,
        StatusCode::CREATED,
        StatusCode::CREATED,
        None,
        &resp,
        "ehr",
    ))
}
