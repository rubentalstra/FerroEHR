//! HTTP dispatch for the `ehr` API group.
//!
//! Each arm rebuilds the operation's `*Params`, decodes wire strings into the
//! SM catalog's native argument types (`uuid::Uuid`,
//! [`ObjectVersionId`](openehr_base::prelude::ObjectVersionId),
//! [`UpdateVersion`](ehrbase_sm::types::UpdateVersion)) via
//! [`crate::version_id`], decodes any body (RM-typed bodies accept JSON or
//! canonical XML), calls the EHR-core SM catalog methods on the platform service
//! `S`, and rebuilds a [`ServiceResponse`] (RM payload + typed [`ResourceMeta`])
//! from the native result — from which the `negotiate::*` helpers render the
//! spec's `ETag`/`Location`/`Prefer` behaviour (ADR-011).
//!
//! The SM `create`/`update` calls return only the new `version_uid` (the literal
//! SM shape); a `Prefer: return=representation` response therefore re-reads the
//! resource through the matching `get_*` call. Header policy is per operation,
//! per the ITS-REST 1.0.3 response definitions: writes honour `Prefer` and set
//! `ETag`/`Location`; the `COMPOSITION`/`EHR_STATUS` reads set them too;
//! header-free reads (VERSION wrappers, revision histories, EHR/FOLDER
//! retrieval, item tags, CONTRIBUTION retrieval) render the body alone; on a
//! `409`/`412` the write arms decorate the error with the current `version_uid`.

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrCreateParams, EhrCreateWithIdParams,
    EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{
    Composition, Ehr, EhrStatus, Folder, OriginalVersion, PartyProxy, PartySelf, RevisionHistory,
    VersionedObjectData,
};

// The EHR-core service methods called via method syntax on `&S` are brought
// into scope by the `S: Platform` bound (their traits are Platform supertraits);
// only the contribution trait is named explicitly (its call names collide with
// other groups, so a trait-path call disambiguates).
use ehrbase_sm::services::EhrContributionService;
use ehrbase_sm::types::{ResourceMeta, ServiceResponse, UpdateAudit, UpdateVersion};
use ehrbase_sm::{CallStatusType, Platform};

use super::{BoxResponse, RequestParts};
use crate::error::{RestError, sm_api_error};
use crate::state::AppState;
use crate::version_id::{
    object_id_uuid, parse_ehr_id, parse_uid_based_id, parse_uuid, parse_version_uid,
    require_if_match,
};
use crate::{AuthMethod, negotiate, params};

/// openEHR *audit change type* codes (the `openehr` terminology group).
const CHANGE_CREATION: &str = "249";
const CHANGE_MODIFICATION: &str = "251";
/// `532|complete|` — the lifecycle state stamped on synthesized version updates.
const LIFECYCLE_COMPLETE: &str = "532";

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

/// The `uid`/`value` of a returned RM object → the resource metadata a read
/// with an `ETag`/`Location` needs (the version id is the object's own `uid`).
fn resource_meta_from(ehr_id: &str, body: &Value) -> Option<ResourceMeta> {
    body.get("uid")
        .and_then(|u| u.get("value"))
        .and_then(Value::as_str)
        .map(|uid| ResourceMeta::new(ehr_id.to_owned(), uid.to_owned()))
}

/// Wrap a read body as a [`ServiceResponse`], attaching resource metadata drawn
/// from the body's own `uid` when present.
fn read_resp(ehr_id: &str, body: Value) -> ServiceResponse {
    match resource_meta_from(ehr_id, &body) {
        Some(m) => ServiceResponse::new(body, m),
        None => ServiceResponse::plain(body),
    }
}

/// An `openehr` terminology code (the audit change type / lifecycle state).
fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// The committer `PARTY_PROXY` for a write, from the authenticated principal
/// published by the auth middleware (system identity when none). The SM service
/// impl re-derives the committer from the same principal, so this rides in the
/// [`UpdateVersion`] envelope for completeness.
fn committer_proxy() -> PartyProxy {
    let value = match crate::access::authn::current_principal() {
        Some(principal) => {
            let id_type = match principal.method {
                AuthMethod::Basic => "basic",
                AuthMethod::Bearer => "oauth2",
            };
            json!({
                "_type": "PARTY_IDENTIFIED",
                "name": principal.subject.clone(),
                "identifiers": [{
                    "_type": "DV_IDENTIFIER",
                    "id": principal.subject,
                    "issuer": "ehrbase-rs",
                    "type": id_type
                }]
            })
        }
        None => json!({ "_type": "PARTY_IDENTIFIED", "name": "ehrbase-rs.local" }),
    };
    serde_json::from_value(value).unwrap_or(PartyProxy::PartySelf(PartySelf { external_ref: None }))
}

/// Synthesize the SM `UPDATE_VERSION` commit envelope for a bare-RM-body write
/// route (`POST`/`PUT` of a `COMPOSITION/EHR_STATUS/FOLDER)`: the RM object is the
/// `data`, the `If-Match` is the `preceding_version_uid`, and the audit carries
/// the change type + committer.
///
/// The server defaults (lifecycle `532|complete|`, the verb-derived change type,
/// the authenticated committer) are then **merged** with any
/// `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal request headers the
/// client supplied — the ITS-REST MUST (overview §"openEHR-VERSION and
/// openEHR-AUDIT_DETAILS"; `crate::committal`).
fn mk_update_version(
    headers: &HeaderMap,
    data: Value,
    change_code: &str,
    description: &str,
    preceding: Option<ObjectVersionId>,
) -> UpdateVersion {
    let mut uv = UpdateVersion {
        preceding_version_uid: preceding,
        lifecycle_state: term(LIFECYCLE_COMPLETE),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: Some(description.to_owned()),
            committer: committer_proxy(),
        },
        signature: None,
    };
    crate::committal::merge_committal_headers(&mut uv, headers);
    uv
}

/// Decompose an [`ObjectVersionId`] into the `(versioned-object uuid,
/// version_tree_id)` pair the SM `*_at_version` reads take. Branch version ids
/// are first-class (RM common master06 §Version tree; the former trunk-only
/// rejection F-06-09 is retired).
fn version_components(ovid: &ObjectVersionId) -> Result<(Uuid, String), ApiError> {
    let vo = object_id_uuid(ovid).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "OBJECT_VERSION_ID object_id is not a UUID: {}",
            ovid.value
        ))
    })?;
    Ok((vo, ovid.version_tree_id().value.clone()))
}

#[allow(clippy::too_many_lines)] // one arm per operation; a flat match is clearest
async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let created = StatusCode::CREATED;
    let no_content = StatusCode::NO_CONTENT;
    // The configured base path, for building `Location` URLs.
    let base = state.config().base_path.clone();

    match op {
        // ── EHR ──────────────────────────────────────────────────────────────
        "ehr_get_by_subject" => {
            let p = params::build::<EhrGetBySubjectParams>(&parts.path, q, h)?;
            let body = state
                .backend()
                .ehr_object_for_subject(&p.subject_id, &p.subject_namespace)
                .await?;
            // 200_EHR: no ETag/Location declared for EHR retrieval.
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &body, "ehr"))
        }
        "ehr_create" => {
            let _p = params::build::<EhrCreateParams>(&parts.path, q, h)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let ehr_id = state.backend().create_ehr(status).await?;
            ehr_write_response(&state, h, &base, ehr_id).await
        }
        "ehr_create_with_id" => {
            let p = params::build::<EhrCreateWithIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let status = negotiate::optional_rm_value::<EhrStatus>(h, &parts.body)?;
            let ehr_id = state.backend().create_ehr_with_id(ehr_id, status).await?;
            ehr_write_response(&state, h, &base, ehr_id).await
        }
        "ehr_get_by_id" => {
            let p = params::build::<EhrGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().ehr_object(ehr_id).await?;
            Ok(negotiate::respond_rm::<Ehr>(h, ok, &body, "ehr"))
        }
        // ── EHR_STATUS ───────────────────────────────────────────────────────
        "ehr_status_get_by_version_id" => {
            let p = params::build::<EhrStatusGetByVersionIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let (vo_id, version) = version_components(&parse_version_uid(&p.version_uid)?)?;
            // F-01-03: the bare EHR_STATUS at that version (not ORIGINAL_VERSION);
            // 200_EHR_STATUS_retrieved: ETag(version_uid) + Location.
            let body = state
                .backend()
                .get_ehr_status_at_version(ehr_id, vo_id, &version)
                .await?;
            let resp = ServiceResponse::new(body, ResourceMeta::new(p.ehr_id, p.version_uid));
            Ok(negotiate::read_rm::<EhrStatus>(
                h,
                &base,
                Some("ehr_status"),
                &resp,
                "ehr_status",
            ))
        }
        "ehr_status_get_at_time" => {
            let p = params::build::<EhrStatusGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .get_ehr_status_at_time(ehr_id, p.version_at_time)
                .await?;
            let resp = read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<EhrStatus>(
                h,
                &base,
                Some("ehr_status"),
                &resp,
                "ehr_status",
            ))
        }
        "ehr_status_update" => {
            let p = params::build::<EhrStatusUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<EhrStatus>(h, &parts.body)?;
            let uv = mk_update_version(
                h,
                body,
                CHANGE_MODIFICATION,
                "EHR_STATUS update",
                Some(require_if_match(&p.if_match)?),
            );
            // 204_EHR_STATUS (default minimal) / 200_EHR_STATUS_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().replace_ehr_status(ehr_id, uv).await {
                Ok(uid) => {
                    let repr = if negotiate::prefers_representation(h) {
                        state.backend().get_ehr_status(ehr_id).await?
                    } else {
                        Value::Null
                    };
                    let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
                    Ok(negotiate::write_rm::<EhrStatus>(
                        h,
                        &base,
                        no_content,
                        ok,
                        Some("ehr_status"),
                        &resp,
                        "ehr_status",
                    ))
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .ehr_status_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("ehr_status"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "versioned_ehr_status_get" => {
            let p = params::build::<VersionedEhrStatusGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().get_versioned_ehr_status(ehr_id).await?;
            // VERSIONED_OBJECT container — canonical JSON or XML (F-05-06:
            // ITS-XML `Version.xsd`/`Common.xsd` define the shape; the generated
            // `ToXml` for `VersionedObjectData` serves it).
            Ok(negotiate::respond_rm::<VersionedObjectData>(
                h,
                ok,
                &body,
                "versioned_ehr_status",
            ))
        }
        "versioned_ehr_status_revision_history" => {
            let p = params::build::<VersionedEhrStatusRevisionHistoryParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state.backend().ehr_status_revision_history(ehr_id).await?;
            Ok(negotiate::respond_rm::<RevisionHistory>(
                h,
                ok,
                &body,
                "revision_history",
            ))
        }
        "versioned_ehr_status_version_get_at_time" => {
            let p = params::build::<VersionedEhrStatusVersionGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .ehr_status_version_at_time(ehr_id, p.version_at_time)
                .await?;
            // 200_VERSION_at_time: ETag(version_uid) + Location of the VERSION;
            // body is an ORIGINAL_VERSION<EHR_STATUS> (JSON or canonical XML).
            let resp = read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<OriginalVersion<EhrStatus>>(
                h,
                &base,
                Some("versioned_ehr_status/version"),
                &resp,
                "original_version",
            ))
        }
        "versioned_ehr_status_version_get_by_id" => {
            let p = params::build::<VersionedEhrStatusVersionGetByIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let (vo_id, version) = version_components(&parse_version_uid(&p.version_uid)?)?;
            let body = state
                .backend()
                .ehr_status_original_version(ehr_id, vo_id, &version)
                .await?;
            Ok(negotiate::respond_rm::<OriginalVersion<EhrStatus>>(
                h,
                ok,
                &body,
                "original_version",
            ))
        }
        // ── COMPOSITION ──────────────────────────────────────────────────────
        "composition_create" => {
            let p = params::build::<CompositionCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // A FLAT/STRUCTURED (wt.flat/structured+json) body is rebuilt into a
            // canonical composition.
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            let uv = mk_update_version(h, body, CHANGE_CREATION, "COMPOSITION creation", None);
            let uid = state.backend().create_composition(ehr_id, uv).await?;
            composition_write_response(&state, h, &base, ehr_id, uid, created, created).await
        }
        "composition_get" => {
            let p = params::build::<CompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let uid = parse_uid_based_id(&p.uid_based_id)?;
            let body = if let Some(ovid) = uid.version {
                state
                    .backend()
                    .get_composition_at_version(ehr_id, ovid)
                    .await?
            } else if p.version_at_time.is_some() {
                state
                    .backend()
                    .get_composition_at_time(ehr_id, uid.vo_id, p.version_at_time)
                    .await?
            } else {
                state
                    .backend()
                    .get_composition_latest(ehr_id, uid.vo_id)
                    .await?
            };
            // A deleted version resolves to a null body → 204 No Content
            // (composition_get.yaml 204_because_deleted*; F-02-01).
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            // `?expand_multimedia=true` (ADR-017): transparently re-inline any
            // externalized DV_MULTIMEDIA blobs, verifying integrity. A no-op
            // when externalization is off or the body has no external media.
            // Not an openEHR spec parameter, so read off the raw query string
            // (the `template_id` precedent), never a generated params struct.
            let body = if params::query_param(q, "expand_multimedia").as_deref() == Some("true") {
                state.backend().expand_multimedia(body).await?
            } else {
                body
            };
            if negotiate::wants_flat(h) {
                return super::flat::composition_flat_response(&state, ok, &body).await;
            }
            if negotiate::wants_structured(h) {
                return super::flat::composition_structured_response(&state, ok, &body).await;
            }
            // 200_COMPOSITION_retrieved: ETag(version_uid) + Location.
            let resp = read_resp(&p.ehr_id, body);
            Ok(negotiate::read_rm::<Composition>(
                h,
                &base,
                Some("composition"),
                &resp,
                "composition",
            ))
        }
        "composition_update" => {
            let p = params::build::<CompositionUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let uid = parse_uid_based_id(&p.uid_based_id)?;
            let body = if negotiate::is_flat_body(h) {
                super::flat::composition_from_flat(&state, q, h, &parts.body).await?
            } else if negotiate::is_structured_body(h) {
                super::flat::composition_from_structured(&state, q, h, &parts.body).await?
            } else {
                negotiate::rm_value::<Composition>(h, &parts.body)?
            };
            // A body-supplied COMPOSITION.uid must identify the same
            // versioned object as the path `uid_based_id` (ITS-REST
            // `composition_update`: "the uid, if present, must match") —
            // a mismatched body uid is a 400, never a silent write to the
            // path's object.
            if let Some(body_uid) = body
                .get("uid")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str)
            {
                let body_vo = body_uid.split("::").next().unwrap_or(body_uid);
                if body_vo.parse::<uuid::Uuid>() != Ok(uid.vo_id) {
                    return Err(ApiError::BadRequest(format!(
                        "the body COMPOSITION.uid {body_uid:?} does not identify the \
                         versioned object addressed by the request path ({})",
                        uid.vo_id
                    ))
                    .into());
                }
            }
            let uv = mk_update_version(
                h,
                body,
                CHANGE_MODIFICATION,
                "COMPOSITION update",
                Some(require_if_match(&p.if_match)?),
            );
            match state
                .backend()
                .update_composition(ehr_id, uid.vo_id, uv)
                .await
            {
                Ok(new_uid) => {
                    composition_write_response(&state, h, &base, ehr_id, new_uid, ok, ok).await
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, uid.vo_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "composition_delete" => {
            let p = params::build::<CompositionDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // composition_delete.yaml: the uid_based_id MUST be an OBJECT_VERSION_ID
            // (the preceding_version_uid to delete); a bare HIER_OBJECT_ID → 400.
            let ovid = parse_version_uid(&p.uid_based_id)?;
            let vo_id = object_id_uuid(&ovid).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "OBJECT_VERSION_ID object_id is not a UUID: {}",
                    p.uid_based_id
                ))
            })?;
            match state.backend().delete_composition(ehr_id, ovid).await {
                Ok(uid) => {
                    // 204_COMPOSITION_deleted: ETag + Location of the deleted version.
                    let resp = ServiceResponse::deleted(ResourceMeta::new(p.ehr_id, uid));
                    Ok(negotiate::deleted_with_headers(
                        &base,
                        Some("composition"),
                        &resp,
                    ))
                }
                // 409_COMPOSITION_with_uid_based_id (stale) → latest version_uid.
                Err(e) if e.status == CallStatusType::CompositionAlreadyExists => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, vo_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "versioned_composition_get" => {
            let p = params::build::<VersionedCompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .get_versioned_composition(ehr_id, vo_id)
                .await?;
            // VERSIONED_OBJECT container — canonical JSON or XML (F-05-06).
            Ok(negotiate::respond_rm::<VersionedObjectData>(
                h,
                ok,
                &body,
                "versioned_composition",
            ))
        }
        "versioned_composition_revision_history" => {
            let p = params::build::<VersionedCompositionRevisionHistoryParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let vo_id = parse_uuid(&p.versioned_object_uid, "versioned_object_uid")?;
            let body = state
                .backend()
                .composition_revision_history(ehr_id, vo_id)
                .await?;
            Ok(negotiate::respond_rm::<RevisionHistory>(
                h,
                ok,
                &body,
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
                .composition_version_at_time(ehr_id, vo_id, p.version_at_time)
                .await?;
            let resp = read_resp(&p.ehr_id, body);
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML (F-05-06).
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
            // ORIGINAL_VERSION<COMPOSITION> — JSON or canonical XML; carries the
            // version `<signature>` (ECC-SIG-001, version-signing.md §4.4).
            Ok(negotiate::respond_rm::<OriginalVersion<Composition>>(
                h,
                ok,
                &body,
                "original_version",
            ))
        }
        // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────
        "directory_get_at_time" => {
            let p = params::build::<DirectoryGetAtTimeParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = state
                .backend()
                .get_directory_at_time(ehr_id, p.version_at_time, p.path)
                .await?;
            // Deleted directory → 204 (directory_get_at_time.yaml 204_because_deleted_at_time).
            // 200_FOLDER_retrieved declares no ETag/Location.
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &body, "folder"))
        }
        "directory_update" => {
            let p = params::build::<DirectoryUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            let uv = mk_update_version(
                h,
                body,
                CHANGE_MODIFICATION,
                "DIRECTORY update",
                Some(require_if_match(&p.if_match)?),
            );
            // 204_directory_updated (default) / 200_directory_updated
            // (representation); ETag + Location on both. 412 → latest version_uid.
            match state.backend().update_directory(ehr_id, uv).await {
                Ok(uid) => {
                    let repr = if negotiate::prefers_representation(h) {
                        state
                            .backend()
                            .get_directory_at_time(ehr_id, None, None)
                            .await?
                    } else {
                        Value::Null
                    };
                    let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
                    Ok(negotiate::write_rm::<Folder>(
                        h,
                        &base,
                        no_content,
                        ok,
                        Some("directory"),
                        &resp,
                        "folder",
                    ))
                }
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "directory_create" => {
            let p = params::build::<DirectoryCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::rm_value::<Folder>(h, &parts.body)?;
            let uv = mk_update_version(h, body, CHANGE_CREATION, "DIRECTORY creation", None);
            let uid = state.backend().create_directory(ehr_id, uv).await?;
            let repr = if negotiate::prefers_representation(h) {
                state
                    .backend()
                    .get_directory_at_time(ehr_id, None, None)
                    .await?
            } else {
                Value::Null
            };
            let resp = ServiceResponse::new(repr, ResourceMeta::new(p.ehr_id, uid));
            // 201_directory: ETag + Location; body only on return=representation.
            Ok(negotiate::write_rm::<Folder>(
                h,
                &base,
                created,
                created,
                Some("directory"),
                &resp,
                "folder",
            ))
        }
        "directory_delete" => {
            let p = params::build::<DirectoryDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // 204_because_deleted declares no headers; 412_directory → latest version_uid.
            match state
                .backend()
                .delete_directory(ehr_id, Some(require_if_match(&p.if_match)?))
                .await
            {
                Ok(()) => Ok(negotiate::empty(no_content)),
                Err(e) if e.status == CallStatusType::VersionMismatch => {
                    let meta = state
                        .backend()
                        .directory_latest_meta(ehr_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        sm_api_error(e),
                        &base,
                        Some("directory"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(e)),
            }
        }
        "directory_get_by_version_id" => {
            let p = params::build::<DirectoryGetByVersionIdParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let ovid = parse_version_uid(&p.version_uid)?;
            let body = state
                .backend()
                .get_directory_at_version(ehr_id, ovid)
                .await?;
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            Ok(negotiate::respond_rm::<Folder>(h, ok, &body, "folder"))
        }
        // ── CONTRIBUTION ─────────────────────────────────────────────────────
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
            let resp = state
                .backend()
                .ehr_contribution_commit(ehr_id, body)
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
                EhrContributionService::get_contribution_resolved(state.backend(), ehr_id, cid)
                    .await?
            } else {
                EhrContributionService::get_contribution(state.backend(), ehr_id, cid).await?
            };
            Ok(negotiate::respond(h, ok, &body))
        }
        // ── item tags ────────────────────────────────────────────────────────
        "ehr_tags_get" => {
            let p = params::build::<EhrTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .ehr_tags_get(ehr_id, p.tag_key, p.tag_value, p.tag_target_path)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "composition_tags_get" => {
            let p = params::build::<CompositionTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .target_tags_get(ehr_id, p.uid_based_id)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "composition_tags_update" => {
            let p = params::build::<CompositionTagsUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let tags = state
                .backend()
                .target_tags_replace(ehr_id, p.uid_based_id, "COMPOSITION", body)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "composition_tags_delete" => {
            let p = params::build::<CompositionTagsDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            state
                .backend()
                .target_tag_delete(ehr_id, p.uid_based_id, p.key)
                .await?;
            Ok(negotiate::empty(no_content))
        }
        "ehr_status_tags_get" => {
            let p = params::build::<EhrStatusTagsGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let tags = state
                .backend()
                .target_tags_get(ehr_id, p.uid_based_id)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "ehr_status_tags_update" => {
            let p = params::build::<EhrStatusTagsUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let tags = state
                .backend()
                .target_tags_replace(ehr_id, p.uid_based_id, "EHR_STATUS", body)
                .await?;
            Ok(negotiate::respond(h, ok, &Value::Array(tags)))
        }
        "ehr_status_tags_delete" => {
            let p = params::build::<EhrStatusTagsDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            state
                .backend()
                .target_tag_delete(ehr_id, p.uid_based_id, p.key)
                .await?;
            Ok(negotiate::empty(no_content))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted ehr operation: {other}"),
        ))),
    }
}

/// Render an EHR create response (`201_EHR)`: `ETag(ehr_id)` + `Location`, with the
/// RM `EHR` body only on `Prefer: return=representation`.
async fn ehr_write_response<S: Platform>(
    state: &AppState<S>,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: Uuid,
) -> Result<Response, RestError> {
    let ehr_id_str = ehr_id.to_string();
    let body = if negotiate::prefers_representation(h) {
        state.backend().ehr_object(ehr_id).await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, ResourceMeta::new(ehr_id_str.clone(), ehr_id_str));
    Ok(negotiate::write_rm::<Ehr>(
        h,
        base,
        StatusCode::CREATED,
        StatusCode::CREATED,
        None,
        &resp,
        "ehr",
    ))
}

/// Render a COMPOSITION create/update response: FLAT/STRUCTURED interop bodies
/// when requested (always the representation), else the canonical
/// `ETag`/`Location` + `Prefer` write response.
async fn composition_write_response<S: Platform>(
    state: &AppState<S>,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: Uuid,
    uid: String,
    minimal: StatusCode,
    repr: StatusCode,
) -> Result<Response, RestError> {
    let ehr_id_str = ehr_id.to_string();
    // FLAT/STRUCTURED Accept returns the Better representation (interop format),
    // which needs the stored body regardless of Prefer.
    if negotiate::wants_flat(h) || negotiate::wants_structured(h) {
        let ovid = parse_version_uid(&uid)?;
        let body = state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?;
        if negotiate::wants_flat(h) {
            return super::flat::composition_flat_response(state, repr, &body).await;
        }
        return super::flat::composition_structured_response(state, repr, &body).await;
    }
    let body = if negotiate::prefers_representation(h) {
        let ovid = parse_version_uid(&uid)?;
        state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, ResourceMeta::new(ehr_id_str, uid));
    Ok(negotiate::write_rm::<Composition>(
        h,
        base,
        minimal,
        repr,
        Some("composition"),
        &resp,
        "composition",
    ))
}
