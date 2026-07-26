//! The `VERSIONED_COMPOSITION` container (the versioned-object view of a COMPOSITION).
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` +
//! `specifications/operations/{versioned_composition_get,
//! versioned_composition_revision_history,
//! versioned_composition_version_get_at_time,
//! versioned_composition_version_get_by_id}.yaml`.

use axum::response::Response;

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
    let base = state.config().server.base_path.clone();

    match op {
        "versioned_composition_get" => {
            let p = params::build::<VersionedCompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .get_versioned_composition(ehr_id, ehrbase::ids::VoId(vo_id))
                .await?;
            // VERSIONED_OBJECT container — canonical JSON or XML, with the
            // container-uid ETag (Requests_and_responses.md §ETag and
            // Last-Modified: "VERSIONED_OBJECT.uid.value"); the container body
            // carries no commit instant, so Last-Modified is honestly absent.
            let resp = super::read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<VersionedComposition>(
                h,
                &base,
                None,
                &resp,
                "versioned_composition",
            ))
        }
        "versioned_composition_revision_history" => {
            let p = params::build::<VersionedCompositionRevisionHistoryParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .composition_revision_history(ehr_id, ehrbase::ids::VoId(vo_id))
                .await?;
            // ETag from the container uid + Last-Modified from the most
            // recent item (§ETag and Last-Modified SHOULD on versioned reads).
            let resp = super::revision_history_resp(&p.ehr_id, &p.versioned_object_uid, body);
            Ok(negotiate::read_rm::<RevisionHistory>(
                h,
                &base,
                None,
                &resp,
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
                .composition_version_at_time(ehr_id, ehrbase::ids::VoId(vo_id), p.version_at_time)
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
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML, with the
            // version-uid ETag + commit-time Last-Modified (§ETag and
            // Last-Modified SHOULD on VERSION reads).
            let resp = super::read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<OriginalVersion<Composition>>(
                h,
                &base,
                None,
                &resp,
                "original_version",
            ))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
