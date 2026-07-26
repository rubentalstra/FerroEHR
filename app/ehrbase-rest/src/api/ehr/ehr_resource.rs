//! The `EHR` resource + EHR-level item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (EHR API) +
//! `specifications/operations/{ehr_get_by_subject,ehr_create,
//! ehr_create_with_id,ehr_get_by_id,ehr_tags_get}.yaml`.

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    EhrCreateParams, EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams,
    EhrTagsGetParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Ehr, EhrStatus};

use ehrbase::ids::EhrId;
use ehrbase::service::response::{ResourceMeta, ServiceResponse};

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::overview::version_id::parse_ehr_id;
use crate::state::AppState;
use crate::{negotiate, params};

pub(super) async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    // The configured base path, for building `Location` URLs.
    let base = state.config().server.base_path.clone();

    // EHR / EHR_STATUS have no Simplified-Formats mapping (they are not
    // templated) — a simplified Content-Type/Accept is rejected uniformly.
    crate::formats::dispatch::guard_non_templated(h)?;

    match op {
        "ehr_get_by_subject" => {
            let p = params::build::<EhrGetBySubjectParams>(&parts.path, q, h)?;
            let body = state
                .backend()
                .ehr_object_for_subject(&p.subject_id, &p.subject_namespace)
                .await?;
            // 200_EHR: the weak ETag off `EHR.ehr_id.value` — the ITS-REST
            // overview's own example source for a resource with a unique
            // state identifier (Requests_and_responses.md §"ETag and
            // Last-Modified": the value "is usually taken from e.g.
            // VERSIONED_OBJECT.uid.value, VERSION.uid.value,
            // EHR.ehr_id.value"). No Location on a GET (§Location).
            Ok(ehr_read_response(h, ok, &body))
        }
        "ehr_create" => {
            let _p = params::build::<EhrCreateParams>(&parts.path, q, h)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            // The service returns the created EHR's own resource metadata
            // (ehr_id + creation instant) — the write path never rebuilds a
            // metadata-less envelope, so `Last-Modified` survives to the wire.
            let (ehr_id, meta) = state.backend().create_ehr_meta(status).await?;
            ehr_write_response(&state, h, &base, ehr_id, meta).await
        }
        "ehr_create_with_id" => {
            let p = params::build::<EhrCreateWithIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let meta = state
                .backend()
                .create_ehr_with_id_meta(ehr_id, status)
                .await?;
            ehr_write_response(&state, h, &base, ehr_id, meta).await
        }
        "ehr_get_by_id" => {
            let p = params::build::<EhrGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().ehr_object(ehr_id).await?;
            Ok(ehr_read_response(h, ok, &body))
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

/// Render an EHR read (`200_EHR`) with the weak `ETag` carrying
/// `EHR.ehr_id.value` — the ITS-REST overview §"`ETag` and Last-Modified"
/// example source, and a resource with a unique state identifier, for which
/// the `ETag` SHOULD be present.
///
/// No `Last-Modified`: the RM `EHR` root is not a VERSION and has no
/// `commit_audit`, and the spec derives the header from
/// `VERSION.commit_audit.time_committed.value`. `EHR.time_created` is the
/// creation instant, not the last modification (the served body changes with
/// every `EHR_STATUS`/directory commit), so emitting it here would be wrong.
fn ehr_read_response(h: &http::HeaderMap, status: StatusCode, body: &Value) -> Response {
    let mut out = negotiate::respond_rm::<Ehr>(h, status, body, "ehr");
    if let Some(id) = body["ehr_id"]["value"].as_str() {
        negotiate::set_etag(&mut out, id);
    }
    out
}

/// Render an EHR create response (`201_EHR)`: `ETag(ehr_id)` +
/// `Last-Modified` + `Location`, with the RM `EHR` body only on
/// `Prefer: return=representation`.
///
/// `meta` comes straight from the service create (the `ehr_id` plus the
/// creating CONTRIBUTION's commit instant), so the `Last-Modified` the
/// ITS-REST overview §"`ETag` and Last-Modified" asks for is not lost between
/// the commit and the wire.
async fn ehr_write_response(
    state: &AppState,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: EhrId,
    meta: ResourceMeta,
) -> Result<Response, RestError> {
    let body = if negotiate::prefers_representation(h) {
        // `ehr_created_object` serves the just-committed EHR body from the
        // create-time stash (built from the commit results), avoiding the
        // `ehr_summary` re-read; it falls back to a full read on a stash miss.
        state.backend().ehr_created_object(ehr_id).await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, meta);
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
