//! The `CONTRIBUTION` resource.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (CONTRIBUTION) +
//! `specifications/operations/{contribution_create,contribution_get}.yaml`.

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::ehr::{ContributionCreateParams, ContributionGetParams};
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::negotiate::WireFormat;
use crate::overview::error::RestError;
use crate::overview::version_id::{parse_ehr_id, parse_uuid};
use crate::state::AppState;
use crate::{negotiate, params};

/// The representations a CONTRIBUTION endpoint negotiates. The envelope is
/// always canonical JSON (`contribution_create.yaml` / `contribution_get.yaml`
/// §Simplified Formats: "the CONTRIBUTION envelope itself remains canonical
/// JSON"); the Simplified types select the inner `versions[i].data` form. There
/// is no canonical-XML CONTRIBUTION wire shape, so XML is not offered.
const CONTRIBUTION_FORMATS: &[WireFormat] = &[
    WireFormat::CanonicalJson,
    WireFormat::Flat,
    WireFormat::Structured,
];

#[allow(clippy::too_many_lines)] // create + get, each with envelope + simplified branches
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
            // NOTE: a CONTRIBUTION commit is a wrapper DTO (a version set +
            // audit), not a single canonical RM value with a defined canonical-XML
            // shape — so it is accepted as JSON only.
            //
            // Committed as the *raw wire body* through the `ContributionAdapter`
            // seam, not the typed SM `commit_contribution`: the typed
            // `UpdateVersion` envelope cannot represent attestation-only (666) or
            // delete (523) members, or committer/system_id inheritance from the
            // CONTRIBUTION audit (see the trait's NOTE; RM common master06
            // §Committal m4).
            //
            // The envelope is canonical JSON; a Simplified `Content-Type`
            // rebuilds each `versions[i].data` COMPOSITION into canonical form
            // before commit (`contribution_create.yaml` §Simplified Formats).
            let body = match negotiate::content_type_format(h) {
                Some(WireFormat::CanonicalJson) => negotiate::json_value(h, &parts.body)?,
                Some(fmt @ (WireFormat::Flat | WireFormat::Structured)) => {
                    crate::formats::dispatch::contribution_from_simplified(
                        &state,
                        h,
                        &parts.body,
                        fmt,
                    )
                    .await?
                }
                _ => {
                    return Err(RestError(ApiError::UnsupportedMediaType(
                        "a CONTRIBUTION is committed as application/json, \
                         application/openehr.wt.flat+json, or \
                         application/openehr.wt.structured+json"
                            .to_owned(),
                    )));
                }
            };
            // Under `return=minimal` the response is headers-only (ETag +
            // Location), so the composite CONTRIBUTION body is not built and its
            // post-commit re-read is skipped; `return=representation` assembles
            // it (ITS-REST `Requests_and_responses` §Representation details).
            let want_repr = negotiate::prefers_representation(h);
            let resp = state
                .backend()
                .ehr_contribution_commit(ehr_id, body, want_repr)
                .await?;
            // 201_CONTRIBUTION: ETag(contribution_uid) + Location; body per
            // Prefer, in the negotiated inner form on a Simplified `Accept`.
            match negotiate::resolve_accept(h, CONTRIBUTION_FORMATS, WireFormat::CanonicalJson) {
                Some(fmt @ (WireFormat::Flat | WireFormat::Structured)) if want_repr => {
                    let mut out = crate::formats::dispatch::contribution_to_simplified(
                        &state, created, &resp.body, fmt,
                    )
                    .await?;
                    if let Some(meta) = &resp.meta {
                        negotiate::set_resource_headers(
                            &mut out,
                            &base,
                            Some("contribution"),
                            meta,
                        );
                    }
                    Ok(out)
                }
                _ => Ok(negotiate::write_json(
                    h,
                    &base,
                    created,
                    created,
                    Some("contribution"),
                    &resp,
                )),
            }
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
            // The envelope stays canonical JSON; a Simplified `Accept`
            // serializes each present `versions[i].data` COMPOSITION into the
            // requested inner form (`contribution_get.yaml` §Simplified Formats).
            // A non-COMPOSITION inner payload → 406; canonical JSON (or an
            // unfulfillable Accept, e.g. XML) is answered by `respond`.
            match negotiate::resolve_accept(h, CONTRIBUTION_FORMATS, WireFormat::CanonicalJson) {
                Some(fmt @ (WireFormat::Flat | WireFormat::Structured)) => {
                    crate::formats::dispatch::contribution_to_simplified(&state, ok, &body, fmt)
                        .await
                }
                _ => Ok(negotiate::respond(h, ok, &body)),
            }
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}
