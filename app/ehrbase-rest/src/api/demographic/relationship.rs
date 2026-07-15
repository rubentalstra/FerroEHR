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

use axum::extract::State;
use axum::response::Response;
use http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentGetParams, AgentUpdateParams, VersionedPartyGetParams,
    VersionedPartyVersionGetAtTimeParams, VersionedPartyVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::PartyRelationship;

use crate::api::{RequestParts, guarded_dispatch};
use crate::overview::error::{RestError, sm_api_error};
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::{Platform, ServiceResponse};

/// The `PARTY_RELATIONSHIP` extension routes as a native `utoipa-axum` router —
/// **no ITS-REST contract** (see the module docs), realizing SM
/// `I_PARTY_RELATIONSHIP` with our own wire. Group-relative paths (nested under
/// `base_path`); every operation runs through [`guarded_dispatch`] with the
/// demographic group [`dispatch`](super::dispatch), which routes relationship
/// ops back into [`run`] — so the wire behaviour is identical to the former
/// table-driven mount.
pub(crate) fn relationship_routes<S: Platform>() -> OpenApiRouter<AppState<S>> {
    OpenApiRouter::new().routes(routes!(
        party_relationship_create,
        party_relationship_get,
        party_relationship_update,
        party_relationship_delete,
        versioned_party_relationship_get,
        party_relationship_revision_history,
        party_relationship_version_get_at_time,
        party_relationship_version_get_by_id,
    ))
}

// ── Handlers (our own wire; no ITS-REST operation governs these) ──────────────
// Each snapshots the request and runs it through the demographic group
// dispatcher (`super::dispatch`), which routes relationship ops into [`run`].

/// Create a `PARTY_RELATIONSHIP` (RM canonical JSON body). 201 with the created
/// resource; ETag/Location headers.
#[utoipa::path(
    post, path = "/demographic/party_relationship", tag = "demographic-relationship",
    request_body(content = serde_json::Value, description = "An RM PARTY_RELATIONSHIP (canonical JSON)."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn party_relationship_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_create",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// Read a `PARTY_RELATIONSHIP` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    responses(
        (status = 200, description = "The relationship (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "party_relationship_get", parts, super::dispatch::<S>).await
}

/// Update a `PARTY_RELATIONSHIP` (If-Match required; RM canonical JSON body).
#[utoipa::path(
    put, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    request_body(content = serde_json::Value, description = "The updated RM PARTY_RELATIONSHIP (canonical JSON)."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn party_relationship_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_update",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// Delete a `PARTY_RELATIONSHIP` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn party_relationship_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_delete",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// Read the `VERSIONED_PARTY_RELATIONSHIP` container.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}", tag = "demographic-relationship",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses((status = 200, description = "The VERSIONED_PARTY_RELATIONSHIP (RM canonical JSON).", body = serde_json::Value))
)]
pub(crate) async fn versioned_party_relationship_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_relationship_get",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// The relationship's `REVISION_HISTORY`.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history", tag = "demographic-relationship",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses((status = 200, description = "The REVISION_HISTORY (RM canonical JSON).", body = serde_json::Value))
)]
pub(crate) async fn party_relationship_revision_history<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_revision_history",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// The relationship VERSION at a point in time (`?version_at_time=`).
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/version", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path, description = "The versioned-object uid."),
        ("version_at_time" = Option<String>, Query, description = "Optional ISO-8601 instant; latest when omitted.")
    ),
    responses((status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value))
)]
pub(crate) async fn party_relationship_version_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_version_get_at_time",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// A specific relationship VERSION by version uid.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path, description = "The versioned-object uid."),
        ("version_uid" = String, Path, description = "The OBJECT_VERSION_ID.")
    ),
    responses((status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value))
)]
pub(crate) async fn party_relationship_version_get_by_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_version_get_by_id",
        parts,
        super::dispatch::<S>,
    )
    .await
}

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
