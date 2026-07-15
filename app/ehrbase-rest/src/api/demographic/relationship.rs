//! The `PARTY_RELATIONSHIP` wire — **our own extension** (G-5).
//!
//! No openEHR ITS-REST operation governs this: the vendored Demographic API
//! (`demographic.openapi.yaml`) defines **no** `party_relationship` /
//! `versioned_party_relationship` paths anywhere. These routes are our own
//! extension realizing SM `I_PARTY_RELATIONSHIP`
//! (`docs/specs/openehr/SM/.../i_party_relationship.adoc` — the *service*
//! basis, not a *wire* basis) and are **excluded from any
//! conformance-profile claim**. The envelope mirrors the party CRUD + versioned
//! reads with one fixed `party_relationship` / `versioned_party_relationship`
//! segment; because there is no wire contract, the generated party `*Params`
//! structs are reused by analogy.

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentGetParams, AgentUpdateParams, VersionedPartyGetParams,
    VersionedPartyVersionGetAtTimeParams, VersionedPartyVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::PartyRelationship;

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::{Platform, ServiceResponse};

/// The `PARTY_RELATIONSHIP` extension route table — **no ITS-REST contract**
/// (see the module docs); mounted alongside the generated demographic `ROUTES`
/// and served by the same group [`dispatch`](super::dispatch).
pub(crate) const RELATIONSHIP_ROUTES: &[(&str, &str, &str)] = &[
    (
        "POST",
        "/demographic/party_relationship",
        "party_relationship_create",
    ),
    (
        "GET",
        "/demographic/party_relationship/{uid_based_id}",
        "party_relationship_get",
    ),
    (
        "PUT",
        "/demographic/party_relationship/{uid_based_id}",
        "party_relationship_update",
    ),
    (
        "DELETE",
        "/demographic/party_relationship/{uid_based_id}",
        "party_relationship_delete",
    ),
    (
        "GET",
        "/demographic/versioned_party_relationship/{versioned_object_uid}",
        "versioned_party_relationship_get",
    ),
    (
        "GET",
        "/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history",
        "party_relationship_revision_history",
    ),
    (
        "GET",
        "/demographic/versioned_party_relationship/{versioned_object_uid}/version",
        "party_relationship_version_get_at_time",
    ),
    (
        "GET",
        "/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}",
        "party_relationship_version_get_by_id",
    ),
];

/// `PARTY_RELATIONSHIP` operations — same envelope/header rules as the party
/// routes, one fixed `party_relationship` segment (our own wire; no ITS-REST
/// operation governs it — see the module docs).
#[allow(clippy::too_many_lines)] // one arm per relationship op, like party::run
pub(super) async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    const SEG: &str = "party_relationship";
    const VSEG: &str = "versioned_party_relationship";
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().server.base_path.clone();

    match op {
        "party_relationship_create" => {
            let _p = params::build::<AgentCreateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<PartyRelationship>(h, &parts.body)?;
            let resp = state.backend().party_relationship_create(body).await?;
            Ok(write_relationship(
                h,
                &base,
                SEG,
                StatusCode::CREATED,
                StatusCode::CREATED,
                &resp,
            ))
        }
        "party_relationship_get" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_get(p.uid_based_id, p.version_at_time)
                .await?;
            if resp.is_empty() {
                return Ok(negotiate::empty(StatusCode::NO_CONTENT));
            }
            Ok(read_relationship(h, &base, SEG, &resp))
        }
        "party_relationship_update" => {
            let p = params::build::<AgentUpdateParams>(&parts.path, q, h)?;
            let uid = p.uid_based_id.clone();
            let body = negotiate::rm_value::<PartyRelationship>(h, &parts.body)?;
            match state
                .backend()
                .party_relationship_update(p.uid_based_id, p.if_match, body)
                .await
            {
                Ok(resp) => Ok(write_relationship(
                    h,
                    &base,
                    SEG,
                    StatusCode::NO_CONTENT,
                    StatusCode::OK,
                    &resp,
                )),
                Err(e) if super::is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .party_relationship_latest_meta(uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(super::error_with_headers(
                        sm_api_error(e),
                        &base,
                        SEG,
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "party_relationship_delete" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_delete(p.uid_based_id, super::if_match_of(h))
                .await?;
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            super::set_headers(&mut out, &base, SEG, resp.meta.as_ref());
            Ok(out)
        }
        "versioned_party_relationship_get" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_relationship_get(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        "party_relationship_revision_history" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_revision_history(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        "party_relationship_version_get_at_time" => {
            let p = params::build::<VersionedPartyVersionGetAtTimeParams>(&parts.path, q, h)?;
            let segment = format!("{VSEG}/{}/version", p.versioned_object_uid);
            let resp = state
                .backend()
                .party_relationship_version_get_at_time(p.versioned_object_uid, p.version_at_time)
                .await?;
            let mut out = negotiate::respond(h, StatusCode::OK, &resp.body);
            super::set_headers(&mut out, &base, &segment, resp.meta.as_ref());
            Ok(out)
        }
        "party_relationship_version_get_by_id" => {
            let p = params::build::<VersionedPartyVersionGetByIdParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_version_get_by_id(p.versioned_object_uid, p.version_uid)
                .await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic relationship operation: {other}"
        )))),
    }
}

/// A create/update relationship response honouring `Prefer` (canonical
/// JSON/XML via the concrete `PARTY_RELATIONSHIP` type) and setting the
/// demographic `ETag`/`Location`.
fn write_relationship(
    h: &http::HeaderMap,
    base: &str,
    segment: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    resp: &ServiceResponse,
) -> Response {
    let mut out = if negotiate::prefers_representation(h) {
        negotiate::respond_rm::<PartyRelationship>(h, repr_status, &resp.body, "party_relationship")
    } else {
        negotiate::empty(minimal_status)
    };
    super::set_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}

/// A `200 OK` read of a relationship, setting the demographic `ETag`/`Location`.
fn read_relationship(
    h: &http::HeaderMap,
    base: &str,
    segment: &str,
    resp: &ServiceResponse,
) -> Response {
    let mut out = negotiate::respond_rm::<PartyRelationship>(
        h,
        StatusCode::OK,
        &resp.body,
        "party_relationship",
    );
    super::set_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}
