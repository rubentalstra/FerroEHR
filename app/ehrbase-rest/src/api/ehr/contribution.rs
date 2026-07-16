//! The `CONTRIBUTION` resource.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (CONTRIBUTION) +
//! `specifications/operations/{contribution_create,contribution_get}.yaml`.

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::ehr::{ContributionCreateParams, ContributionGetParams};
use openehr_its::rest::runtime::ApiError;

// The contribution trait is named explicitly (its call names collide with other
// groups, so a trait-path call disambiguates).

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::overview::version_id::{parse_ehr_id, parse_uuid};
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
    let created = StatusCode::CREATED;
    let base = state.config().server.base_path.clone();

    match op {
        "contribution_create" => {
            let p = params::build::<ContributionCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // PORT NOTE: a CONTRIBUTION commit is a wrapper DTO (a version set +
            // audit), not a single canonical RM value with a defined canonical-XML
            // shape — so it is accepted as JSON only.
            //
            // Committed as the *raw wire body* through the `ContributionAdapter`
            // seam, not the typed SM `commit_contribution`: the typed
            // `UpdateVersion` envelope cannot represent attestation-only (666) or
            // delete (523) members, or committer/system_id inheritance from the
            // CONTRIBUTION audit (see the trait's PORT NOTE; RM common master06
            // §Committal m4).
            let body = negotiate::json_value(h, &parts.body)?;
            // Under `return=minimal` the response is headers-only (ETag +
            // Location), so the composite CONTRIBUTION body is not built and its
            // post-commit re-read is skipped; `return=representation` assembles
            // it (ITS-REST `Requests_and_responses` §Representation details).
            let resp = state
                .backend()
                .ehr_contribution_commit(ehr_id, body, negotiate::prefers_representation(h))
                .await?;
            // 201_CONTRIBUTION: ETag(contribution_uid) + Location; body per Prefer.
            Ok(negotiate::write_json(
                h,
                &base,
                created,
                created,
                Some("contribution"),
                &resp,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let cid = parse_uuid(&p.contribution_uid, "contribution id")?;
            // `Prefer: resolve_refs` (Requests_and_responses §Representation
            // details negotiation): versions as full ORIGINAL_VERSIONs.
            let body = if negotiate::prefers_resolve_refs(h) {
                state
                    .backend()
                    .get_contribution_resolved(ehr_id, cid)
                    .await?
            } else {
                state.backend().get_contribution(ehr_id, cid).await?
            };
            Ok(negotiate::respond(h, ok, &body))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
