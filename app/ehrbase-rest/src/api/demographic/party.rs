//! The per-kind party CRUD operations (`{kind}_{create,get,update,delete}`) —
//! `operations/person_create.yaml`, `person_get.yaml`, `person_update.yaml`,
//! `person_delete.yaml` (and the field-identical `agent_*`/`group_*`/
//! `organisation_*`/`role_*`). Party bodies use the **LOCATABLE** content
//! negotiation (`Accept_LOCATABLE`/`ContentType_LOCATABLE`): canonical JSON + XML.

use axum::response::Response;
use http::{HeaderMap, StatusCode};

use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentDeleteParams, AgentGetParams, AgentUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Agent, Group, Organisation, Person, Role};

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::{CallStatusType, PartyKind, Platform, ServiceResponse};

/// The per-kind CRUD operations (`create`/`get`/`update`/`delete`).
pub(super) async fn run<S: Platform>(
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
            // G-3: the incoming `openehr-item-tag` / `openehr-version-item-tag`
            // request headers (person_create.yaml) carry ITEM_TAGs to persist.
            // TODO(w3e-integrate): hand these to the service — `party_create`
            // currently takes no tags; the central parser lands in the overview
            // worker's `crate::overview::committal`.
            let _tags = crate::overview::params::parse_item_tag_header(h, crate::overview::params::H_ITEM_TAG);
            let body = decode_party_body(kind, h, &parts.body)?;
            // person_create.yaml declares 201/400/422/404; a service NotFound
            // maps to 404, PreconditionViolation to 400, ContentInvalid to 422
            // (overview::error::sm_api_error) — `?` routes each to its status.
            let resp = state.backend().party_create(kind, body).await?;
            // 201 + ETag/Location; body per Prefer; + item-tag response headers.
            let mut out = write_party(
                kind,
                h,
                &base,
                StatusCode::CREATED,
                StatusCode::CREATED,
                &resp,
            );
            super::set_item_tag_headers(&mut out, &resp);
            Ok(out)
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
                // person_update.yaml: `If-Match` mismatch → 412 + latest version_uid.
                Err(e) if super::is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .demographic_latest_meta(kind, uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(super::error_with_headers(
                        sm_api_error(e),
                        &base,
                        seg,
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "delete" => run_delete(&state, kind, &base, seg, &parts).await,
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic party operation: {seg}_{other}"
        )))),
    }
}

/// `DELETE /demographic/{kind}/{uid_based_id}` — logical delete → `204` +
/// `ETag`/`Location` of the deleted version.
///
/// G-2: `person_delete.yaml` places the `preceding_version_uid` to delete in the
/// **path** (`uid_based_id_as_version_uid` — an `OBJECT_VERSION_ID`), not in an
/// `If-Match` header. Responses: `204_version_deleted`, `400_already_deleted`,
/// `404`, `409_PERSON_with_uid_based_id` (supplied uid doesn't match the latest
/// version; returns the latest `version_uid` in `ETag`).
async fn run_delete<S: Platform>(
    state: &AppState<S>,
    kind: PartyKind,
    base: &str,
    seg: &str,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    // The generated `*DeleteParams` carries only `uid_based_id` (the preceding
    // version_uid). All per-kind delete params are field-identical; reuse one.
    let p = params::build::<AgentDeleteParams>(&parts.path, parts.query.as_deref(), h)?;
    let preceding = p.uid_based_id.clone();
    // PORT NOTE (wire, compatibility): `If-Match` is retained only as a fallback
    // source of the preceding version for older clients — `person_delete.yaml`
    // declares no `If-Match` and takes the preceding version from the path.
    // TODO(w3e-integrate): `party_delete` should take the preceding version_uid
    // positionally (design its-rest/demographic.md §2.2), and the service should
    // signal a stale uid via `version_mismatch` (→ 409) and an already-deleted
    // target via `precondition_violation` (→ 400_already_deleted). Passing the
    // path uid; `If-Match` kept as the compatibility fallback.
    match state
        .backend()
        .party_delete(kind, preceding.clone(), super::if_match_of(h))
        .await
    {
        Ok(resp) => {
            // 204_version_deleted: ETag + Location of the deleted version.
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            super::set_headers(&mut out, base, seg, resp.meta.as_ref());
            Ok(out)
        }
        // 409_PERSON_with_uid_based_id: supplied uid_based_id ≠ latest version.
        Err(e) if e.status == CallStatusType::VersionMismatch => {
            let meta = state
                .backend()
                .demographic_latest_meta(kind, preceding)
                .await
                .ok()
                .flatten();
            Ok(super::error_with_headers(
                ApiError::Conflict(e.message),
                base,
                seg,
                meta.as_ref(),
            ))
        }
        // 400_already_deleted (precondition_violation → 400) and 404 (not-found
        // family) map through overview::error::sm_api_error.
        Err(e) => Err(RestError::from(e)),
    }
}

/// Decode a party request body (canonical JSON or XML) into the canonical JSON
/// `Value` the seam expects, re-typing XML through the concrete `openehr-rm`
/// party type for the routed kind.
fn decode_party_body(
    kind: PartyKind,
    h: &HeaderMap,
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
    h: &HeaderMap,
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
    h: &HeaderMap,
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
    super::set_headers(&mut out, base, kind.segment(), resp.meta.as_ref());
    out
}

/// A `200 OK` read of a party, setting the demographic `ETag`/`Location` and the
/// ITEM_TAG response headers (person_get.yaml).
fn read_party(kind: PartyKind, h: &HeaderMap, base: &str, resp: &ServiceResponse) -> Response {
    let mut out = respond_party(kind, h, StatusCode::OK, &resp.body);
    super::set_headers(&mut out, base, kind.segment(), resp.meta.as_ref());
    super::set_item_tag_headers(&mut out, resp);
    out
}
