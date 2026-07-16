//! The `VERSIONED_COMPOSITION` container (the versioned-object view of a COMPOSITION).
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` +
//! `specifications/operations/{versioned_composition_get,
//! versioned_composition_revision_history,
//! versioned_composition_version_get_at_time,
//! versioned_composition_version_get_by_id}.yaml`.

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::ehr::{
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Composition, OriginalVersion, RevisionHistory, VersionedComposition};

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::overview::version_id::{parse_ehr_id, parse_uuid, parse_version_uid};
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
        "versioned_composition_get" => {
            let p = params::build::<VersionedCompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .get_versioned_composition(ehr_id, vo_id)
                .await?;
            // VERSIONED_OBJECT container — canonical JSON or XML.
            Ok(negotiate::respond_rm::<VersionedComposition>(
                h,
                ok,
                &body,
                "versioned_composition",
            ))
        }
        "versioned_composition_revision_history" => {
            let p = params::build::<VersionedCompositionRevisionHistoryParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .composition_revision_history(ehr_id, vo_id)
                .await?;
            Ok(negotiate::respond_rm::<RevisionHistory>(
                h,
                ok,
                &body,
                "revision_history",
            ))
        }
        "versioned_composition_version_get_at_time" => {
            let p = params::build::<VersionedCompositionVersionGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            // 200_VERSION_of_COMPOSITION_at_time: Location is
            // …/versioned_composition/{versioned_object_uid}/version/{version_uid}.
            let segment = format!("versioned_composition/{}/version", p.versioned_object_uid);
            let body = state
                .backend()
                .composition_version_at_time(ehr_id, vo_id, p.version_at_time)
                .await?;
            let resp = super::read_resp(&p.ehr_id, body);
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML.
            Ok(negotiate::read_rm::<OriginalVersion<Composition>>(
                h,
                &base,
                Some(&segment),
                &resp,
                "original_version",
            ))
        }
        "versioned_composition_version_get_by_id" => {
            let p = params::build::<VersionedCompositionVersionGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let ovid = parse_version_uid(&p.version_uid)?;
            let body = state
                .backend()
                .composition_original_version(ehr_id, ovid)
                .await?;
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML; carries the
            // version `<signature>` (ECC-SIG-001, version-signing.md §4.4).
            Ok(negotiate::respond_rm::<OriginalVersion<Composition>>(
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
