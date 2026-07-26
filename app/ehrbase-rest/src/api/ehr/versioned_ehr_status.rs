//! The `VERSIONED_EHR_STATUS` container (the versioned-object view of `EHR_STATUS`).
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` +
//! `specifications/operations/{versioned_ehr_status_get,
//! versioned_ehr_status_revision_history,
//! versioned_ehr_status_version_get_at_time,
//! versioned_ehr_status_version_get_by_id}.yaml`.

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::ehr::{
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{EhrStatus, OriginalVersion, RevisionHistory, VersionedEhrStatus};

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::overview::version_id::{parse_ehr_id, parse_version_uid};
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
    let base = state.config().server.base_path.clone();

    match op {
        "versioned_ehr_status_get" => {
            let p = params::build::<VersionedEhrStatusGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().get_versioned_ehr_status(ehr_id).await?;
            // VERSIONED_OBJECT container — canonical JSON or XML
            // (ITS-XML `Version.xsd`/`Common.xsd` define the shape; the generated
            // `ToXml` for the concrete `VERSIONED_*` class serves it).
            Ok(negotiate::respond_rm::<VersionedEhrStatus>(
                h,
                ok,
                &body,
                "versioned_ehr_status",
            ))
        }
        "versioned_ehr_status_revision_history" => {
            let p = params::build::<VersionedEhrStatusRevisionHistoryParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().ehr_status_revision_history(ehr_id).await?;
            Ok(negotiate::respond_rm::<RevisionHistory>(
                h,
                ok,
                &body,
                "revision_history",
            ))
        }
        "versioned_ehr_status_version_get_at_time" => {
            let p = params::build::<VersionedEhrStatusVersionGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .ehr_status_version_at_time(ehr_id, p.version_at_time)
                .await?;
            // 200_VERSION_at_time: ETag(version_uid) + Location of the VERSION;
            // body is an ORIGINAL_VERSION<EHR_STATUS> (JSON or canonical XML).
            let resp = super::read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<OriginalVersion<EhrStatus>>(
                h,
                &base,
                Some("versioned_ehr_status/version"),
                &resp,
                "original_version",
            ))
        }
        "versioned_ehr_status_version_get_by_id" => {
            let p = params::build::<VersionedEhrStatusVersionGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let (vo_id, version) = super::version_components(&parse_version_uid(&p.version_uid)?)?;
            let body = state
                .backend()
                .ehr_status_original_version(ehr_id, ehrbase::ids::VoId(vo_id), &version)
                .await?;
            // Full-identity check as on the ehr_status by-version read
            // (Resources.md §Identifier types; BASE master05 case rule).
            super::ensure_served_version(&p.version_uid, &body)?;
            Ok(negotiate::respond_rm::<OriginalVersion<EhrStatus>>(
                h,
                ok,
                &body,
                "original_version",
            ))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
