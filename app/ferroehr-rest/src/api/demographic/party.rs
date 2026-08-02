//! The per-kind party CRUD operations (`{kind}_{create,get,update,delete}`) —
//! `operations/person_create.yaml`, `person_get.yaml`, `person_update.yaml`,
//! `person_delete.yaml` (and the field-identical `agent_*`/`group_*`/
//! `organisation_*`/`role_*`). Party bodies use the **LOCATABLE** content
//! negotiation (`Accept_LOCATABLE`/`ContentType_LOCATABLE`): canonical JSON + XML.

use axum::response::Response;
use http::{HeaderMap, StatusCode};

use openehr_base::prelude::ObjectVersionId;
use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentDeleteParams, AgentGetParams, AgentUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{Agent, Group, Organisation, Person, Role};

use crate::api::RequestParts;
use crate::overview::error::{RestError, sm_api_error};
use crate::state::AppState;
use crate::{negotiate, params};
use ferroehr::service::demographic::types::PartyKind;
use ferroehr::service::response::ServiceResponse;
use ferroehr::service::status::CallStatusType;

/// The per-kind CRUD operations (`create`/`get`/`update`/`delete`).
pub(super) async fn run(
    state: AppState,
    kind: PartyKind,
    action: &str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let base = state.config().server.base_path.clone();
    let seg = kind.segment();

    // Demographic PARTY types are not templated → no Simplified-Formats
    // mapping; reject a simplified Content-Type/Accept uniformly.
    crate::formats::dispatch::guard_non_templated(h)?;

    match action {
        "create" => {
            // All per-kind `*CreateParams` are field-identical; reuse one.
            let _p = params::build::<AgentCreateParams>(&parts.path, q, h)?;
            let body = decode_party_body(kind, h, &parts.body)?;
            // person_create.yaml declares 201/400/422/404; a service NotFound
            // maps to 404, PreconditionViolation to 400, ContentInvalid to 422
            // (overview::error::sm_api_error) — `?` routes each to its status.
            let mut resp = state
                .backend()
                .party_create(
                    kind,
                    body,
                    crate::overview::committal::committal_audit(
                        h,
                        crate::api::ehr::committer_proxy(),
                    ),
                )
                .await?;
            // the incoming `openehr-item-tag` request header (person_create.yaml)
            // carries ITEM_TAGs to persist. The party must exist first
            // (`item_tag.target_vo_id` FK), so tags are persisted after the create
            // and the stored set is reflected on the response metadata seam.
            persist_request_tags(&state, kind, h, &mut resp).await?;
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
            Ok(read_party(kind, h, &resp))
        }
        "update" => {
            let p = params::build::<AgentUpdateParams>(&parts.path, q, h)?;
            let uid = p.uid_based_id.clone();
            let body = decode_party_body(kind, h, &parts.body)?;
            match state
                .backend()
                .party_update(
                    kind,
                    p.uid_based_id,
                    // The `W/"…"`/quoted ETag syntax is decoded here, at the
                    // adapter seam, so the service compares a bare
                    // OBJECT_VERSION_ID (overview §"`ETag` and Last-Modified").
                    super::if_match_token(&p.if_match),
                    body,
                    crate::overview::committal::committal_audit(
                        h,
                        crate::api::ehr::committer_proxy(),
                    ),
                )
                .await
            {
                Ok(mut resp) => {
                    // persist any `openehr-item-tag` request-header tags
                    // against the updated party and reflect the stored set on the
                    // response metadata (person_update.yaml).
                    persist_request_tags(&state, kind, h, &mut resp).await?;
                    let mut out = write_party(
                        kind,
                        h,
                        &base,
                        StatusCode::NO_CONTENT,
                        StatusCode::OK,
                        &resp,
                    );
                    super::set_item_tag_headers(&mut out, &resp);
                    Ok(out)
                }
                // person_update.yaml: `If-Match` mismatch → 412 + latest version_uid.
                Err(e) if super::is_precondition(&e) => {
                    let meta = state
                        .backend()
                        .demographic_latest_meta(kind, uid)
                        .await
                        .ok()
                        .flatten();
                    Ok(super::error_with_meta(sm_api_error(e), meta.as_ref()))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "delete" => run_delete(&state, kind, &parts).await,
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic party operation: {seg}_{other}"
        )))),
    }
}

/// Apply the `openehr-item-tag` / `openehr-version-item-tag` write-wrapper
/// request headers to the just-written party, mirroring the EHR side
/// (`Requests_and_responses.md` §"openehr-item-tag and openehr-version-item-tag"
/// §Usage in Requests): the two headers address DISTINCT collections —
/// `openehr-item-tag` replaces the `VERSIONED_OBJECT` container's tags
/// (addressed by the bare object id) and `openehr-version-item-tag` the
/// just-committed VERSION's own tags (addressed by the full `version_uid`).
/// A present-but-empty header clears its collection; an absent header leaves
/// its collection untouched and echoes nothing. The stored sets ride the
/// response metadata per header so the echo confirms exactly what each target
/// now holds. Both headers are accepted on create AND update — the released
/// update declares `openehr-version-item-tag` and its own prose says
/// "`openehr-item-tag` or `openehr-version-item-tag`" (register-documented).
async fn persist_request_tags(
    state: &AppState,
    kind: PartyKind,
    h: &HeaderMap,
    resp: &mut ServiceResponse,
) -> Result<(), RestError> {
    let object_entries = params::parse_item_tag_header(h, params::H_ITEM_TAG);
    let version_entries = params::parse_item_tag_header(h, params::H_VERSION_ITEM_TAG);
    if object_entries.is_none() && version_entries.is_none() {
        return Ok(());
    }
    let Some(version_uid) = resp.meta.as_ref().map(|m| m.uid.clone()) else {
        return Ok(());
    };
    // The VERSIONED_OBJECT the `openehr-item-tag` header addresses is the
    // `object_id` of the committed version's OBJECT_VERSION_ID, read through
    // the BASE accessor (`base_types` §Functions `object_id`;
    // `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.object_version_id.adoc`)
    // rather than a local `::` split.
    let container_uid = ObjectVersionId {
        value: version_uid.clone(),
    }
    .object_id()
    .value()
    .into_owned();
    if let Some(entries) = object_entries {
        let tags = params::item_tags_from_header_entries(&entries);
        let stored = state
            .backend()
            .party_tags_update(kind, container_uid, tags)
            .await?;
        if let Some(meta) = resp.meta.as_mut() {
            meta.item_tags = Some(stored.body);
        }
    }
    if let Some(entries) = version_entries {
        let tags = params::item_tags_from_header_entries(&entries);
        let stored = state
            .backend()
            .party_tags_update(kind, version_uid, tags)
            .await?;
        if let Some(meta) = resp.meta.as_mut() {
            meta.version_item_tags = Some(stored.body);
        }
    }
    Ok(())
}

/// `DELETE /demographic/{kind}/{uid_based_id}` — logical delete → `204` + the
/// deleted version's `ETag` (no `Location`: overview §"Deprecated headers"
/// deprecates `Location` on `DELETE` responses, §Location scopes it to
/// creation/redirect).
///
/// `person_delete.yaml` places the `preceding_version_uid` to delete in the
/// **path** (`uid_based_id_as_version_uid` — an `OBJECT_VERSION_ID`), not in an
/// `If-Match` header. Responses: `204_version_deleted`, `400_already_deleted`,
/// `404`, `409_PERSON_with_uid_based_id` (supplied uid doesn't match the latest
/// version; returns the latest `version_uid` in `ETag`).
async fn run_delete(
    state: &AppState,
    kind: PartyKind,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    // The generated `*DeleteParams` carries only `uid_based_id` (the preceding
    // version_uid). All per-kind delete params are field-identical; reuse one.
    let p = params::build::<AgentDeleteParams>(&parts.path, parts.query.as_deref(), h)?;
    let preceding = p.uid_based_id.clone();
    // NOTE (wire, compatibility): the delete operation takes the preceding
    // version from the path `uid_based_id` (an `OBJECT_VERSION_ID`), which is
    // passed positionally to `party_delete` below — ITS-REST overview
    // §"If-Match and accidental overwrites" requires `If-Match` only "when the
    // `preceding_version_uid` is not part of the endpoint path segment", and
    // here it is. `If-Match` is therefore accepted, never required, as an
    // alternative source of the preceding version. The service
    // signals a stale uid via `version_mismatch` (→ 409, handled below with the
    // latest `version_uid` echoed in `ETag`) and an already-deleted target via
    // `precondition_violation` (→ 400_already_deleted).
    match state
        .backend()
        .party_delete(
            kind,
            preceding.clone(),
            super::if_match_of(h),
            crate::overview::committal::committal_audit(h, crate::api::ehr::committer_proxy()),
        )
        .await
    {
        Ok(resp) => {
            // 204_version_deleted: the deleted version's ETag, no Location.
            let mut out = negotiate::empty(StatusCode::NO_CONTENT);
            super::set_versioning_headers(&mut out, resp.meta.as_ref());
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
            Ok(super::error_with_meta(
                ApiError::Conflict(e.message),
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
/// `ETag`/`Last-Modified` + the `Location` of the version this write committed
/// (overview §Location — creation/redirect only; §"Prefer minimal…" — "the
/// newly created or updated resource").
///
/// The body + `Preference-Applied` go through the shared
/// [`negotiate::write_negotiated`] seam, so a demographic write honours the
/// full `Prefer` triad (representation / identifier / minimal) and declares
/// the preference it applied exactly like every other write route.
fn write_party(
    kind: PartyKind,
    h: &HeaderMap,
    base: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    resp: &ServiceResponse,
) -> Response {
    let mut out = negotiate::write_negotiated(
        h,
        minimal_status,
        repr_status,
        resp.meta.as_ref().map(|m| m.uid.as_str()),
        |status| respond_party(kind, h, status, &resp.body),
    );
    super::set_write_headers(&mut out, base, kind.segment(), resp.meta.as_ref());
    out
}

/// A `200 OK` read of a party, setting the demographic `ETag`/`Last-Modified`
/// and the `ITEM_TAG` response headers (`person_get.yaml`). No `Location` —
/// overview §Location: the header "MUST NOT be used to indicate an alternate
/// representation of an existing resource (e.g. via `GET` method)".
fn read_party(kind: PartyKind, h: &HeaderMap, resp: &ServiceResponse) -> Response {
    let mut out = respond_party(kind, h, StatusCode::OK, &resp.body);
    super::set_versioning_headers(&mut out, resp.meta.as_ref());
    super::set_item_tag_headers(&mut out, resp);
    out
}
