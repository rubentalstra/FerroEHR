//! The `COMPOSITION` resource + its item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (COMPOSITION) +
//! `specifications/operations/{composition_create,composition_get,
//! composition_update,composition_delete,composition_tags_get,
//! composition_tags_update,composition_tags_delete}.yaml`.
//!
//! The FLAT/STRUCTURED converters live in `crate::formats::dispatch` (the
//! Simplified-Formats wire adapter) and are called by their full path.

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use ehrbase::ids::{EhrId, VoId};
use ehrbase::service::response::{ResourceMeta, ServiceResponse};
use ehrbase::versioning::change::Committed;

use crate::api::RequestParts;
use crate::negotiate::{AppliedPreference, WireFormat};
use crate::overview::error::RestError;
use crate::overview::version_id::{
    object_id_uuid, parse_ehr_id, parse_uid_based_id, parse_version_uid, require_if_match,
};
use crate::state::AppState;
use crate::{negotiate, params};

/// The representations a COMPOSITION endpoint negotiates: canonical JSON/XML
/// plus the two Simplified data-instance forms (`Accept_LOCATABLE` /
/// `ContentType_LOCATABLE`). The mapping tables (`simplified_formats/master05`)
/// govern COMPOSITION content, so all four are supported here.
const COMPOSITION_FORMATS: &[WireFormat] = &[
    WireFormat::CanonicalJson,
    WireFormat::CanonicalXml,
    WireFormat::Flat,
    WireFormat::Structured,
];

/// Decode a COMPOSITION request body per its `Content-Type`: canonical JSON/XML
/// through the RM decoder, FLAT/STRUCTURED through the Simplified-Formats
/// adapter. A `Content-Type` outside `ContentType_LOCATABLE` (e.g. the
/// Web Template media type, a deprecated `…schema+json`, or an unknown type) is
/// `415` (Resources.md §Simplified Formats MUST).
async fn decode_composition_body(
    state: &AppState,
    h: &http::HeaderMap,
    body: &bytes::Bytes,
) -> Result<Value, RestError> {
    match negotiate::content_type_format(h) {
        Some(WireFormat::CanonicalJson | WireFormat::CanonicalXml) => {
            Ok(negotiate::rm_value::<Composition>(h, body)?)
        }
        Some(WireFormat::Flat) => {
            crate::formats::dispatch::composition_from_flat(state, h, body).await
        }
        Some(WireFormat::Structured) => {
            crate::formats::dispatch::composition_from_structured(state, h, body).await
        }
        _ => Err(RestError(ApiError::UnsupportedMediaType(
            "a COMPOSITION is committed as application/json, application/xml, \
             application/openehr.wt.flat+json, or application/openehr.wt.structured+json"
                .to_owned(),
        ))),
    }
}

#[allow(clippy::too_many_lines)] // one arm per COMPOSITION operation; a flat match is clearest
pub(super) async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let created = StatusCode::CREATED;
    let no_content = StatusCode::NO_CONTENT;
    let base = state.config().server.base_path.clone();

    match op {
        "composition_create" => {
            let p = params::build::<CompositionCreateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // A FLAT/STRUCTURED (wt.flat/structured+json) body is rebuilt into a
            // canonical composition; canonical JSON/XML pass through the RM
            // decoder; any other Content-Type is 415.
            let body = decode_composition_body(&state, h, &parts.body).await?;
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_CREATION,
                "COMPOSITION creation",
                None,
            );
            let committed = state
                .backend()
                .create_composition(ehr_id, uv)
                .await
                .map_err(|e| RestError::from(ApiError::from(e)))?;
            let uid = committed.version_uid();
            // apply the openehr-item-tag / openehr-version-item-tag
            // write-wrapper headers to the committed COMPOSITION
            // (Requests_and_responses.md §…§Usage in Requests).
            let stored_tags =
                super::apply_item_tag_headers(&state, ehr_id, "COMPOSITION", &uid, h).await?;
            let meta = commit_meta(ehr_id, uid, &committed);
            let mut resp =
                composition_write_response(&state, h, &base, ehr_id, meta, created, created)
                    .await?;
            super::echo_item_tags(&mut resp, &stored_tags);
            Ok(resp)
        }
        "composition_get" => {
            let p = params::build::<CompositionGetParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            let uid = parse_uid_based_id(&p.uid_based_id)?;
            // The service returns the read PLUS its version metadata (uid +
            // commit instant): the served body is a BARE COMPOSITION, which
            // carries no `commit_audit`, so the `Last-Modified` instant the
            // spec derives from `VERSION.commit_audit.time_committed.value`
            // (Requests_and_responses.md §"ETag and Last-Modified") can only
            // come from the version row the service already read.
            let read = if let Some(ovid) = uid.version {
                state
                    .backend()
                    .composition_at_version_response(ehr_id, ovid)
                    .await?
            } else if p.version_at_time.is_some() {
                state
                    .backend()
                    .composition_at_time_response(ehr_id, uid.vo_id, p.version_at_time)
                    .await?
            } else {
                state
                    .backend()
                    .composition_latest_response(ehr_id, uid.vo_id)
                    .await?
            };
            let ServiceResponse { mut body, meta } = read;
            // A deleted version resolves to a null body → 204 No Content
            // (composition_get.yaml `204_because_deleted*`).
            if body.is_null() {
                return Ok(negotiate::empty(no_content));
            }
            // `?expand_multimedia=true`: transparently re-inline any
            // externalized DV_MULTIMEDIA blobs, verifying integrity. A no-op
            // when externalization is off or the body has no external media.
            // Not an openEHR spec parameter, so read off the raw query string
            // (the `template_id` precedent), never a generated params struct.
            if params::query_param(q, "expand_multimedia").as_deref() == Some("true") {
                body = state.backend().expand_multimedia(body).await?;
            }
            // Negotiate the representation across `Accept_LOCATABLE`: FLAT /
            // STRUCTURED via the adapter, else canonical JSON/XML through
            // `read_rm` (which answers 406 for an unfulfillable Accept).
            // The version-identity headers are representation-independent —
            // "the `ETag` value is independent of its resource serialization
            // format (JSON/XML)" (§"ETag and Last-Modified") — so the
            // simplified representations carry them too.
            match negotiate::resolve_accept(h, COMPOSITION_FORMATS, WireFormat::CanonicalJson) {
                Some(WireFormat::Flat) => {
                    let mut out =
                        crate::formats::dispatch::composition_flat_response(&state, ok, &body)
                            .await?;
                    if let Some(meta) = &meta {
                        negotiate::set_versioning_headers(&mut out, meta);
                    }
                    return Ok(out);
                }
                Some(WireFormat::Structured) => {
                    let mut out = crate::formats::dispatch::composition_structured_response(
                        &state, ok, &body,
                    )
                    .await?;
                    if let Some(meta) = &meta {
                        negotiate::set_versioning_headers(&mut out, meta);
                    }
                    return Ok(out);
                }
                _ => {}
            }
            // 200_COMPOSITION_retrieved: ETag(version_uid) + Last-Modified
            // (§"ETag and Last-Modified": both SHOULD accompany a resource
            // with a unique state identifier).
            let resp = ServiceResponse { body, meta };
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
            let body = decode_composition_body(&state, h, &parts.body).await?;
            // A body-supplied COMPOSITION.uid must identify the same
            // versioned object as the path `uid_based_id` — never a silent
            // write to the path's object. NOTE: no released ITS-REST sentence
            // assigns this rejection (the "must match" rule appears only in
            // the stalled OAS operation description); the body is well-formed,
            // so the fitting released row is 422 ("well-formed but was unable
            // to be followed due to semantic errors",
            // Requests_and_responses.md §HTTP status codes) — our register-
            // documented handling.
            if let Some(body_uid) = body
                .get("uid")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str)
            {
                let body_vo = body_uid.split("::").next().unwrap_or(body_uid);
                if body_vo.parse::<Uuid>() != Ok(uid.vo_id.0) {
                    return Err(ApiError::Unprocessable(format!(
                        "the body COMPOSITION.uid {body_uid:?} does not identify the \
                         versioned object addressed by the request path ({})",
                        uid.vo_id
                    ))
                    .into());
                }
            }
            let uv = super::mk_update_version(
                h,
                body,
                super::CHANGE_MODIFICATION,
                "COMPOSITION update",
                Some(require_if_match(&p.if_match)?),
            );
            match state
                .backend()
                .update_composition(ehr_id, uid.vo_id, uv)
                .await
            {
                Ok(committed) => {
                    let new_uid = committed.version_uid();
                    // apply item-tag write-wrapper headers to the new version.
                    let stored_tags =
                        super::apply_item_tag_headers(&state, ehr_id, "COMPOSITION", &new_uid, h)
                            .await?;
                    let meta = commit_meta(ehr_id, new_uid, &committed);
                    let mut resp =
                        composition_write_response(&state, h, &base, ehr_id, meta, ok, ok).await?;
                    super::echo_item_tags(&mut resp, &stored_tags);
                    Ok(resp)
                }
                Err(e @ ehrbase::service::error::ServiceError::VersionConflict(_)) => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, uid.vo_id)
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        ApiError::from(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(ApiError::from(e))),
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
            // A DELETE commits a `523|deleted|` version, so the committal
            // request headers are accepted and merged here too (overview
            // §"openehr-version and openehr-audit-details": PUT, POST and
            // DELETE).
            let update_audit =
                crate::overview::committal::committal_audit(h, super::committer_proxy());
            match state
                .backend()
                .delete_composition(ehr_id, &ovid, update_audit.as_ref())
                .await
            {
                Ok(committed) => {
                    // 204_COMPOSITION_deleted: the deleted version's ETag +
                    // Last-Modified — a logical delete commits a
                    // `523|deleted|` VERSION, so its commit instant is the
                    // resource's last modification (§"ETag and
                    // Last-Modified"; RM common master06 §Logical Deletion).
                    let uid = committed.version_uid();
                    let resp = ServiceResponse::deleted(commit_meta(ehr_id, uid, &committed));
                    Ok(negotiate::deleted_with_headers(
                        &base,
                        Some("composition"),
                        &resp,
                    ))
                }
                // 409_COMPOSITION_with_uid_based_id (stale / not-modifiable) →
                // decorated with the latest version_uid.
                Err(e @ ehrbase::service::error::ServiceError::Conflict(_)) => {
                    let meta = state
                        .backend()
                        .composition_latest_meta(ehr_id, VoId(vo_id))
                        .await
                        .ok()
                        .flatten();
                    Ok(negotiate::error_with_meta(
                        ApiError::from(e),
                        &base,
                        Some("composition"),
                        meta.as_ref(),
                    ))
                }
                Err(e) => Err(RestError::from(ApiError::from(e))),
            }
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
            // composition_tags_update.yaml — 200 (the stored ITEM_TAG list)
            // on `Prefer: return=representation`; 204 (`204_updated.yaml`)
            // when `Prefer` is missing or `return=minimal` (the default —
            // overview §Prefer), with `Preference-Applied` declaring which.
            Ok(negotiate::write_collection(
                h,
                no_content,
                ok,
                &Value::Array(tags),
            ))
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
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ehr operation: {other}"
        )))),
    }
}

/// The committed COMPOSITION version's [`ResourceMeta`]: the new
/// `OBJECT_VERSION_ID` (the `ETag`/`Location` value) plus the server commit
/// instant the commit result already carries — the `Last-Modified` value
/// ITS-REST derives from `VERSION.commit_audit.time_committed.value`
/// (`Requests_and_responses.md` §"`ETag` and Last-Modified"). Taken from the
/// commit result, never re-read.
fn commit_meta(ehr_id: EhrId, uid: String, committed: &Committed) -> ResourceMeta {
    ResourceMeta::new(ehr_id.to_string(), uid).with_last_modified(committed.time_committed)
}

/// Render a COMPOSITION create/update response: FLAT/STRUCTURED interop bodies
/// when requested (always the representation), else the canonical
/// `ETag`/`Last-Modified`/`Location` + `Prefer` write response.
async fn composition_write_response(
    state: &AppState,
    h: &http::HeaderMap,
    base: &str,
    ehr_id: EhrId,
    meta: ResourceMeta,
    minimal: StatusCode,
    repr: StatusCode,
) -> Result<Response, RestError> {
    // A FLAT/STRUCTURED Accept returns the simplified representation (which
    // needs the stored body regardless of `Prefer`); canonical JSON/XML (or an
    // unfulfillable Accept) fall through to the `Prefer`-aware canonical write.
    if let Some(fmt @ (WireFormat::Flat | WireFormat::Structured)) =
        negotiate::resolve_accept(h, COMPOSITION_FORMATS, WireFormat::CanonicalJson)
    {
        let ovid = parse_version_uid(&meta.uid)?;
        let body = state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?;
        let mut out = if fmt == WireFormat::Structured {
            crate::formats::dispatch::composition_structured_response(state, repr, &body).await?
        } else {
            crate::formats::dispatch::composition_flat_response(state, repr, &body).await?
        };
        // The committed-resource version-id headers are representation-
        // independent: a FLAT/STRUCTURED commit carries the same `ETag`
        // (new version uid) + `Location` as the canonical path. RFC 7231
        // §6.3.2 requires `Location` on a `201` regardless of body form, and
        // the ITS-REST response definitions declare both headers on every
        // COMPOSITION commit (`docs/specs/openehr/ITS-REST/specifications/
        // responses/201_COMPOSITION.yaml` + `200_COMPOSITION_updated.yaml`
        // — `ETag`/`Location` unconditional;
        // `specifications/docs/overview/Requests_and_responses.md` §Prefer).
        // Route through the same header helper the canonical write uses, as
        // the CONTRIBUTION simplified-commit path already does.
        negotiate::set_resource_headers(&mut out, base, Some("composition"), &meta);
        // NOTE: a Simplified-Formats commit always answers with the committed
        // COMPOSITION in the negotiated FLAT/STRUCTURED form — the `Accept`
        // decides the body here, not `Prefer` — so the preference this
        // response applies is `return=representation` whatever the client
        // asked for. `Preference-Applied` states what the response DID
        // (`Requests_and_responses.md` §Representation details negotiation),
        // so it is declared through the same seam as every other write path.
        negotiate::set_preference_applied(&mut out, AppliedPreference::Representation);
        return Ok(out);
    }
    let body = if negotiate::prefers_representation(h) {
        let ovid = parse_version_uid(&meta.uid)?;
        state
            .backend()
            .get_composition_at_version(ehr_id, ovid)
            .await?
    } else {
        Value::Null
    };
    let resp = ServiceResponse::new(body, meta);
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
