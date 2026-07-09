//! HTTP dispatch for the `demographic` API group.
//!
//! ITS-REST 1.0.3 defines **no** demographic wire contract (the SM demographic
//! service is abstract; the CNF demographic schedule — master10 — is all TBD;
//! CNF profiles list demographic as OPTIONS-profile only). The whole group is
//! served through the [`DemographicService`] seam by direct analogy with the
//! EHR group ([`super::ehr`]): the same status codes, `ETag`/`Location`,
//! `Prefer`, `If-Match`, and deleted-read→`204` behaviour — the only
//! differences being the `/demographic/{segment}/{uid}` `Location` shape and
//! the absence of an EHR scope (a demographic [`ResourceMeta`] carries an empty
//! `ehr_id`).
//!
//! The five per-kind operation families (`agent`/`group`/`organisation`/
//! `person`/`role`) are collapsed by mapping the operation-id prefix to a
//! [`PartyKind`] ([`parse_party_op`]); the generated per-kind `*Params` structs
//! are field-identical, so one representative struct is reused across kinds.

use axum::response::{IntoResponse, Response};
use http::{HeaderValue, StatusCode, header};

use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentGetParams, AgentTagsDeleteParams, AgentTagsGetParams,
    AgentTagsUpdateParams, AgentUpdateParams, ContributionGetParams, DemographicTagsGetParams,
    VersionedPartyGetParams, VersionedPartyVersionGetAtTimeParams,
    VersionedPartyVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Agent, Group, Organisation, PartyRelationship, Person, Role};

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use ehrbase_sm::Platform;
use ehrbase_sm::types::PartyKind;
use ehrbase_sm::types::{ResourceMeta, ServiceResponse};

use crate::state::AppState;
use crate::{negotiate, params};

pub(super) fn dispatch<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

/// Map an operation id to `(PartyKind, action)` where `action` is the suffix
/// after the kind prefix (e.g. `person_tags_update` → `(Person, "tags_update")`).
/// `None` for the kind-agnostic operations (`versioned_party_*`,
/// `contribution_*`, `demographic_tags_get`).
fn parse_party_op(op: &str) -> Option<(PartyKind, &str)> {
    const KINDS: &[(&str, PartyKind)] = &[
        ("agent", PartyKind::Agent),
        ("group", PartyKind::Group),
        ("organisation", PartyKind::Organisation),
        ("person", PartyKind::Person),
        ("role", PartyKind::Role),
    ];
    for (prefix, kind) in KINDS {
        if let Some(rest) = op.strip_prefix(prefix).and_then(|r| r.strip_prefix('_')) {
            return Some((*kind, rest));
        }
    }
    None
}

async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    // Our-own-design PARTY_RELATIONSHIP extension routes (no ITS-REST contract):
    // matched before the per-kind party ops (which never share this prefix).
    if op.starts_with("party_relationship") || op.starts_with("versioned_party_relationship") {
        return run_relationship(state, op, parts).await;
    }
    if let Some((kind, action)) = parse_party_op(op) {
        return run_party(state, kind, action, parts).await;
    }
    run_shared(state, op, parts).await
}

/// Whether an error is the optimistic-concurrency precondition failure
/// (`If-Match` mismatch → `412`).
fn is_precondition(e: &ApiError) -> bool {
    matches!(e, ApiError::PreconditionFailed(_))
}

/// The per-kind CRUD + tag operations (`{kind}_{action}`).
async fn run_party<S: Platform>(
    state: AppState<S>,
    kind: PartyKind,
    action: &str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().base_path.clone();
    let seg = kind.segment();

    match action {
        "create" => {
            // All per-kind `*CreateParams` are field-identical; reuse one.
            let _p = params::build::<AgentCreateParams>(&parts.path, q, h)?;
            let body = decode_party_body(kind, h, &parts.body)?;
            let resp = state.backend().party_create(kind, body).await?;
            // 201 + ETag/Location; body per Prefer.
            Ok(write_party(
                kind,
                h,
                &base,
                StatusCode::CREATED,
                StatusCode::CREATED,
                &resp,
            ))
        }
        "get" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_get(kind, p.uid_based_id, p.version_at_time)
                .await?;
            // A deleted current version → Null body → 204 (like composition_get).
            if resp.is_empty() {
                return Ok(negotiate::empty(StatusCode::NO_CONTENT));
            }
            Ok(read_party(kind, h, &base, &resp))
        }
        "update" => {
            let p = params::build::<AgentUpdateParams>(&parts.path, q, h)?;
            let uid = p.uid_based_id.clone();
            let body = decode_party_body(kind, h, &parts.body)?;
            match state
                .backend()
                .party_update(kind, p.uid_based_id, p.if_match, body)
                .await
            {
                Ok(resp) => Ok(write_party(
                    kind,
                    h,
                    &base,
                    StatusCode::NO_CONTENT,
                    StatusCode::OK,
                    &resp,
                )),
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .demographic_latest_meta(kind, uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(error_with_headers(e, &base, seg, meta.as_ref()))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "delete" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state.backend().party_delete(kind, p.uid_based_id).await?;
            // 204 + ETag/Location of the deleted version.
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            set_headers(&mut out, &base, seg, resp.meta.as_ref());
            Ok(out)
        }
        "tags_get" => {
            let p = params::build::<AgentTagsGetParams>(&parts.path, q, h)?;
            let resp = state.backend().party_tags_get(kind, p.uid_based_id).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        "tags_update" => {
            let p = params::build::<AgentTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let resp = state
                .backend()
                .party_tags_update(kind, p.uid_based_id, body)
                .await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        "tags_delete" => {
            let p = params::build::<AgentTagsDeleteParams>(&parts.path, q, h)?;
            state
                .backend()
                .party_tags_delete(kind, p.uid_based_id, p.key)
                .await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic party operation: {seg}_{other}"
        )))),
    }
}

/// The kind-agnostic operations: `versioned_party_*`, `contribution_*`,
/// `demographic_tags_get`.
async fn run_shared<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let base = state.config().base_path.clone();

    match op {
        "versioned_party_get" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_get(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_party_revision_history" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_revision_history(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_party_version_get_at_time" => {
            let p = params::build::<VersionedPartyVersionGetAtTimeParams>(&parts.path, q, h)?;
            // 200_VERSION_at_time analogue: ETag(version_uid) + Location of the
            // VERSION resource (…/versioned_party/{uid}/version/{version_uid}).
            let segment = format!("versioned_party/{}/version", p.versioned_object_uid);
            let resp = state
                .backend()
                .versioned_party_version_get_at_time(p.versioned_object_uid, p.version_at_time)
                .await?;
            let mut out = negotiate::respond(h, ok, &resp.body);
            set_headers(&mut out, &base, &segment, resp.meta.as_ref());
            Ok(out)
        }
        "versioned_party_version_get_by_id" => {
            let p = params::build::<VersionedPartyVersionGetByIdParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_version_get_by_id(p.versioned_object_uid, p.version_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "contribution_create" => {
            // A CONTRIBUTION commit is a wrapper DTO, JSON only (like the EHR group).
            let body = negotiate::json_value(h, &parts.body)?;
            let resp = state
                .backend()
                .demographic_contribution_create(body)
                .await?;
            // 201 + ETag(contribution_uid)/Location; body per Prefer.
            Ok(write_shared(
                h,
                &base,
                "contribution",
                StatusCode::CREATED,
                StatusCode::CREATED,
                &resp,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .demographic_contribution_get(p.contribution_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "demographic_tags_get" => {
            let p = params::build::<DemographicTagsGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .demographic_tags_get(p.tag_key, p.tag_value, p.tag_target_path)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic operation: {other}"
        )))),
    }
}

/// The `PARTY_RELATIONSHIP` extension routes — our own design (no ITS-REST
/// contract; PORT NOTE), mounted alongside the generated demographic `ROUTES`
/// and served by the same [`dispatch`]. Mirrors the party CRUD + versioned
/// reads, one segment (`party_relationship` / `versioned_party_relationship`).
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
/// routes, one fixed `party_relationship` segment (SM `I_PARTY_RELATIONSHIP` /
/// `i_demographic_service.adoc create_party_relationship`; our own wire design).
#[allow(clippy::too_many_lines)] // one arm per relationship op, like `run_party`/`run_shared`
async fn run_relationship<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    const SEG: &str = "party_relationship";
    const VSEG: &str = "versioned_party_relationship";
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().base_path.clone();

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
                Err(e) if is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .party_relationship_latest_meta(uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(error_with_headers(e, &base, SEG, meta.as_ref()))
                }
                Err(e) => Err(RestError(e)),
            }
        }
        "party_relationship_delete" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .party_relationship_delete(p.uid_based_id)
                .await?;
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            set_headers(&mut out, &base, SEG, resp.meta.as_ref());
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
            set_headers(&mut out, &base, &segment, resp.meta.as_ref());
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
    set_headers(&mut out, base, segment, resp.meta.as_ref());
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
    set_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}

/// Decode a party request body (canonical JSON or XML) into the canonical JSON
/// `Value` the seam expects, re-typing XML through the concrete `openehr-rm`
/// party type for the routed kind.
fn decode_party_body(
    kind: PartyKind,
    h: &http::HeaderMap,
    body: &bytes::Bytes,
) -> Result<serde_json::Value, ApiError> {
    match kind {
        PartyKind::Agent => negotiate::rm_value::<Agent>(h, body),
        PartyKind::Group => negotiate::rm_value::<Group>(h, body),
        PartyKind::Organisation => negotiate::rm_value::<Organisation>(h, body),
        PartyKind::Person => negotiate::rm_value::<Person>(h, body),
        PartyKind::Role => negotiate::rm_value::<Role>(h, body),
    }
}

/// Render a party body as JSON or canonical XML (monomorphized per kind).
fn respond_party(
    kind: PartyKind,
    h: &http::HeaderMap,
    status: StatusCode,
    body: &serde_json::Value,
) -> Response {
    match kind {
        PartyKind::Agent => negotiate::respond_rm::<Agent>(h, status, body, "agent"),
        PartyKind::Group => negotiate::respond_rm::<Group>(h, status, body, "group"),
        PartyKind::Organisation => {
            negotiate::respond_rm::<Organisation>(h, status, body, "organisation")
        }
        PartyKind::Person => negotiate::respond_rm::<Person>(h, status, body, "person"),
        PartyKind::Role => negotiate::respond_rm::<Role>(h, status, body, "role"),
    }
}

/// A create/update response honouring `Prefer` and setting the demographic
/// `ETag`/`Location`.
fn write_party(
    kind: PartyKind,
    h: &http::HeaderMap,
    base: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    resp: &ServiceResponse,
) -> Response {
    let mut out = if negotiate::prefers_representation(h) {
        respond_party(kind, h, repr_status, &resp.body)
    } else {
        negotiate::empty(minimal_status)
    };
    set_headers(&mut out, base, kind.segment(), resp.meta.as_ref());
    out
}

/// A `200 OK` read of a party, setting the demographic `ETag`/`Location`.
fn read_party(
    kind: PartyKind,
    h: &http::HeaderMap,
    base: &str,
    resp: &ServiceResponse,
) -> Response {
    let mut out = respond_party(kind, h, StatusCode::OK, &resp.body);
    set_headers(&mut out, base, kind.segment(), resp.meta.as_ref());
    out
}

/// A create/update response for a JSON-only payload (CONTRIBUTION), honouring
/// `Prefer` and setting the demographic `ETag`/`Location`.
fn write_shared(
    h: &http::HeaderMap,
    base: &str,
    segment: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    resp: &ServiceResponse,
) -> Response {
    let mut out = if negotiate::prefers_representation(h) {
        negotiate::respond(h, repr_status, &resp.body)
    } else {
        negotiate::empty(minimal_status)
    };
    set_headers(&mut out, base, segment, resp.meta.as_ref());
    out
}

/// Set `ETag` (the resource uid, double-quoted) and a `/demographic/{segment}/
/// {uid}` `Location` from a demographic [`ResourceMeta`] (whose `ehr_id` is
/// empty — parties are not EHR-scoped).
fn set_headers(resp: &mut Response, base: &str, segment: &str, meta: Option<&ResourceMeta>) {
    let Some(meta) = meta else { return };
    if let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", meta.uid)) {
        resp.headers_mut().insert(header::ETAG, etag);
    }
    let location = format!("{base}/demographic/{segment}/{}", meta.uid);
    if let Ok(loc) = HeaderValue::from_str(&location) {
        resp.headers_mut().insert(header::LOCATION, loc);
    }
    resp.extensions_mut().insert(crate::audit::AuditObject {
        ehr_id: None,
        uid: Some(meta.uid.clone()),
    });
}

/// Render an error, additionally setting the latest-version `ETag`/`Location`
/// the analogous `412`/`409` requires.
fn error_with_headers(
    error: ApiError,
    base: &str,
    segment: &str,
    meta: Option<&ResourceMeta>,
) -> Response {
    let mut out = RestError(error).into_response();
    set_headers(&mut out, base, segment, meta);
    out
}
