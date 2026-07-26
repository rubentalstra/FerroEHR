//! The `PARTY_RELATIONSHIP` wire — **our own extension**.
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
use ehrbase::service::response::ServiceResponse;

/// The `PARTY_RELATIONSHIP` extension routes as a native `utoipa-axum` router —
/// **no ITS-REST contract** (see the module docs), realizing SM
/// `I_PARTY_RELATIONSHIP` with our own wire. Group-relative paths (nested under
/// `base_path`); every operation runs through [`guarded_dispatch`] with the
/// demographic group [`dispatch`](super::dispatch::dispatch), which routes relationship
/// ops back into [`run`] — so the wire behaviour is identical to the former
/// table-driven mount.
pub(crate) fn relationship_routes() -> OpenApiRouter<AppState> {
    // One `routes!` per PATH (the macro composes one method-router — handlers
    // in a single call must share the path; mixing paths panics at build with
    // "Overlapping method route").
    OpenApiRouter::new()
        .routes(routes!(party_relationship_create))
        .routes(routes!(
            party_relationship_get,
            party_relationship_update,
            party_relationship_delete
        ))
        .routes(routes!(versioned_party_relationship_get))
        .routes(routes!(party_relationship_revision_history))
        .routes(routes!(party_relationship_version_get_at_time))
        .routes(routes!(party_relationship_version_get_by_id))
}

// ── Handlers (our own wire; no ITS-REST operation governs these) ──────────────
// Each snapshots the request and runs it through the demographic group
// dispatcher (`super::dispatch::dispatch`), which routes relationship ops into [`run`].

/// Create a `PARTY_RELATIONSHIP`
/// (`POST /demographic/party_relationship`) — our own extension (no ITS-REST
/// operation governs it; see the module docs).
#[utoipa::path(
    post, path = "/demographic/party_relationship", tag = "demographic-relationship",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created relationship), or \
                        `return=identifier` (`{uid}` only)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    request_body(content = serde_json::Value,
                 description = "An RM PARTY_RELATIONSHIP (canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`.",
         body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted relationship.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (relationships are not \
                                      templated).", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (relationships are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The relationship fails RM/semantic \
                                      validation.", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_create",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a `PARTY_RELATIONSHIP` by uid-based id
/// (`GET /demographic/party_relationship/{uid_based_id}`) — our own extension.
#[utoipa::path(
    get, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (`version_uid`) or a \
                        HIER_OBJECT_ID (`versioned_object_uid`) for the latest \
                        / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The relationship (RM canonical JSON/XML); \
                                      `ETag` carries the version uid (weak `W/` \
                                      form).", body = serde_json::Value),
        (status = 204, description = "The relationship version at the requested \
                                      time is deleted."),
        (status = 404, description = "Unknown relationship, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (relationships are not \
                                      templated).", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Update a `PARTY_RELATIONSHIP`
/// (`PUT /demographic/party_relationship/{uid_based_id}`) — our own extension.
#[utoipa::path(
    put, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the \
                        relationship to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation`. `return=identifier` is treated \
                        as `minimal` here (our extension)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    request_body(content = serde_json::Value,
                 description = "The updated RM PARTY_RELATIONSHIP (canonical \
                                JSON or XML)."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation`); \
                                      `ETag`/`Location` carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown relationship.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (relationships are not \
                                      templated).", body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (relationships are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The relationship fails RM/semantic \
                                      validation.", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_update",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Delete a `PARTY_RELATIONSHIP`
/// (`DELETE /demographic/party_relationship/{uid_based_id}`) — our own
/// extension.
#[utoipa::path(
    delete, path = "/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("If-Match" = Option<String>, Header,
         description = "The latest `version_uid`, double-quoted (weak `W/` form \
                        also accepted); an alternative source of the preceding \
                        version to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the relationship is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown relationship.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (relationships are not \
                                      templated).", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (relationships are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve the `VERSIONED_PARTY_RELATIONSHIP` container
/// (`GET /demographic/versioned_party_relationship/{versioned_object_uid}`) —
/// our own extension.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY_RELATIONSHIP uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The VERSIONED_PARTY_RELATIONSHIP (RM \
                                      canonical JSON/XML).", body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY_RELATIONSHIP.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_relationship_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_relationship_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve the relationship's `REVISION_HISTORY`
/// (`GET /demographic/versioned_party_relationship/{versioned_object_uid}/revision_history`)
/// — our own extension.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY_RELATIONSHIP uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The REVISION_HISTORY (RM canonical \
                                      JSON/XML).", body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY_RELATIONSHIP.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_revision_history(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_revision_history",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve the relationship VERSION at a point in time
/// (`GET /demographic/versioned_party_relationship/{versioned_object_uid}/version`)
/// — our own extension.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/version", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY_RELATIONSHIP uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`)."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; selects the VERSION extant \
                        at that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON/XML); \
                                      `ETag` carries the version uid (weak `W/` \
                                      form), `Location` the version URL.",
         body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY_RELATIONSHIP, or \
                                      no version at the requested \
                                      `version_at_time`.", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_version_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_version_get_at_time",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a specific relationship VERSION by version uid
/// (`GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}`)
/// — our own extension.
#[utoipa::path(
    get, path = "/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY_RELATIONSHIP uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`)."),
        ("version_uid" = String, Path,
         description = "The VERSION identifier (OBJECT_VERSION_ID / \
                        `version_uid`).")
    ),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY_RELATIONSHIP or \
                                      version.", body = serde_json::Value)
    )
)]
pub(crate) async fn party_relationship_version_get_by_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "party_relationship_version_get_by_id",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// `PARTY_RELATIONSHIP` operations — same envelope/header rules as the party
/// routes, one fixed `party_relationship` segment (our own wire; no ITS-REST
/// operation governs it — see the module docs).
#[allow(clippy::too_many_lines)] // one arm per relationship op, like party::run
pub(super) async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    const SEG: &str = "party_relationship";
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().server.base_path.clone();

    // PARTY_RELATIONSHIP is not templated → no Simplified-Formats mapping;
    // reject a simplified Content-Type/Accept uniformly.
    crate::formats::dispatch::guard_non_templated(h)?;

    match op {
        "party_relationship_create" => {
            let _p = params::build::<AgentCreateParams>(&parts.path, q, h)?;
            let body = negotiate::rm_value::<PartyRelationship>(h, &parts.body)?;
            let resp = state
                .backend()
                .party_relationship_create(body, crate::overview::committal::committal_audit(h))
                .await?;
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
            Ok(read_relationship(h, &resp))
        }
        "party_relationship_update" => {
            let p = params::build::<AgentUpdateParams>(&parts.path, q, h)?;
            let uid = p.uid_based_id.clone();
            let body = negotiate::rm_value::<PartyRelationship>(h, &parts.body)?;
            match state
                .backend()
                .party_relationship_update(
                    p.uid_based_id,
                    // Decode the `W/"…"`/quoted ETag syntax at the adapter seam
                    // so the service compares a bare OBJECT_VERSION_ID.
                    super::if_match_token(&p.if_match),
                    body,
                    crate::overview::committal::committal_audit(h),
                )
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
                    Ok(super::error_with_meta(sm_api_error(e), meta.as_ref()))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "party_relationship_delete" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_delete(
                    p.uid_based_id,
                    super::if_match_of(h),
                    crate::overview::committal::committal_audit(h),
                )
                .await?;
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            super::set_versioning_headers(&mut out, resp.meta.as_ref());
            Ok(out)
        }
        // The versioned reads carry the weak `ETag` (+ `Last-Modified` where the
        // served body exposes a commit audit) the overview §"`ETag` and
        // Last-Modified" asks of VERSION / VERSIONED_OBJECT responses, and no
        // `Location` (§Location — creation/redirect only).
        "versioned_party_relationship_get" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let vo = p.versioned_object_uid.clone();
            let resp = state
                .backend()
                .versioned_party_relationship_get(p.versioned_object_uid)
                .await?;
            Ok(super::read_versioned(h, &vo, &resp.body))
        }
        "party_relationship_revision_history" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let vo = p.versioned_object_uid.clone();
            let resp = state
                .backend()
                .party_relationship_revision_history(p.versioned_object_uid)
                .await?;
            Ok(super::read_versioned(h, &vo, &resp.body))
        }
        "party_relationship_version_get_at_time" => {
            let p = params::build::<VersionedPartyVersionGetAtTimeParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_version_get_at_time(p.versioned_object_uid, p.version_at_time)
                .await?;
            let mut out = negotiate::respond(h, StatusCode::OK, &resp.body);
            super::set_versioning_headers(&mut out, resp.meta.as_ref());
            Ok(out)
        }
        "party_relationship_version_get_by_id" => {
            let p = params::build::<VersionedPartyVersionGetByIdParams>(&parts.path, q, h)?;
            let vo = p.versioned_object_uid.clone();
            let resp = state
                .backend()
                .party_relationship_version_get_by_id(p.versioned_object_uid, p.version_uid)
                .await?;
            Ok(super::read_versioned(h, &vo, &resp.body))
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
    // The full `Prefer` triad (overview §Prefer): representation → the RM
    // body; identifier → `{uid}` only; minimal (default) → empty.
    let uid = resp.meta.as_ref().map(|m| m.uid.clone());
    let mut out = if negotiate::prefers_representation(h) {
        negotiate::respond_rm::<PartyRelationship>(h, repr_status, &resp.body, "party_relationship")
    } else if let (true, Some(uid)) = (negotiate::prefers_identifier(h), uid.as_deref()) {
        negotiate::identifier_response(h, repr_status, uid)
    } else {
        negotiate::empty(minimal_status)
    };
    super::set_write_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}

/// A `200 OK` read of a relationship, setting the demographic
/// `ETag`/`Last-Modified`. No `Location` — overview §Location: the header "MUST
/// NOT be used to indicate an alternate representation of an existing resource
/// (e.g. via `GET` method)".
fn read_relationship(h: &http::HeaderMap, resp: &ServiceResponse) -> Response {
    let mut out = negotiate::respond_rm::<PartyRelationship>(
        h,
        StatusCode::OK,
        &resp.body,
        "party_relationship",
    );
    super::set_versioning_headers(&mut out, resp.meta.as_ref());
    out
}
