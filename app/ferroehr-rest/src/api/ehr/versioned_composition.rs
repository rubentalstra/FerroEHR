// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
use openehr_rm::prelude::{Composition, RevisionHistory, Version, VersionedComposition};

use crate::api::RequestParts;
use crate::api::ehr::VERSION_ROOT_TAG;
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
            // VERSIONED_OBJECT container — canonical JSON or XML, with the
            // container-uid ETag (Requests_and_responses.md §ETag and
            // Last-Modified: "VERSIONED_OBJECT.uid.value") + Last-Modified from
            // the newest held version's commit instant (both headers SHOULD
            // accompany a VERSIONED_OBJECT response), carried in the service
            // metadata.
            let resp = state
                .backend()
                .versioned_composition_response(ehr_id, ferroehr::ids::VoId(vo_id))
                .await?;
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
            // ETag from the container uid + Last-Modified from the newest
            // version's commit instant (§ETag and Last-Modified SHOULD on
            // versioned reads), carried in the service metadata.
            let resp = state
                .backend()
                .composition_revision_history_response(ehr_id, ferroehr::ids::VoId(vo_id))
                .await?;
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
            let body = state
                .backend()
                .composition_version_at_time(ehr_id, ferroehr::ids::VoId(vo_id), p.version_at_time)
                .await?;
            // Version-uid ETag + Last-Modified from the envelope's
            // commit_audit.time_committed (§"ETag and Last-Modified": the
            // value "should be derived from
            // VERSION.commit_audit.time_committed.value"); no Location — a
            // GET never carries one (§Location: "It MUST NOT be used to
            // indicate an alternate representation of an existing resource").
            // The VERSION envelope carries the same externalized media the bare
            // resource does, so it re-inlines on the same request.
            let body = super::expand_multimedia_if_requested(&state, q, body).await?;
            let resp = super::read_resp(&p.ehr_id, body);
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML.
            Ok(negotiate::read_rm::<Version<Composition>>(
                h,
                &base,
                None,
                &resp,
                VERSION_ROOT_TAG,
            ))
        }
        "versioned_composition_version_get_by_id" => {
            let p = params::build::<VersionedCompositionVersionGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let ovid = parse_version_uid(&p.version_uid)?;
            // The addressed version must belong to the container the path
            // names: the version_uid's object_id "matches the
            // VERSIONED_OBJECT identifier" (ITS-REST overview Resources.md
            // §Identifier types; RM common version.adoc Owner_id_valid) — a
            // version of ANOTHER container is not a version of this resource,
            // so an incoherent pair names nothing here → 404.
            if crate::overview::version_id::object_id_uuid(&ovid) != Some(vo_id) {
                return Err(RestError(ApiError::NotFound(format!(
                    "version {} in versioned object {}",
                    p.version_uid, p.versioned_object_uid
                ))));
            }
            let body = state
                .backend()
                .composition_version_envelope(ehr_id, ovid)
                .await?;
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML, with the
            // version-uid ETag + Last-Modified from the envelope's
            // commit_audit.time_committed (§"ETag and Last-Modified": both
            // SHOULD accompany a VERSION response, and Last-Modified is
            // "derived from VERSION.commit_audit.time_committed.value").
            // The VERSION envelope carries the same externalized media the bare
            // resource does, so it re-inlines on the same request.
            let body = super::expand_multimedia_if_requested(&state, q, body).await?;
            let resp = super::read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<Version<Composition>>(
                h,
                &base,
                None,
                &resp,
                VERSION_ROOT_TAG,
            ))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
