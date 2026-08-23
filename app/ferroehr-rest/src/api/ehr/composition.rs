// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `COMPOSITION` resource + its item tags.
//!
//! Spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` (COMPOSITION) +
//! `specifications/operations/{composition_create,composition_get,
//! composition_update,composition_delete,composition_tags_get,
//! composition_tags_update,composition_tags_delete}.yaml`.
//!
//! The FLAT/STRUCTURED converters live in `crate::formats::dispatch` (the
//! Simplified-Formats wire adapter) and are called by their full path.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the handler carries the canonical fragment the \
              negotiate seam produced once (stored-content serving / commit interior)"
)]

use axum::response::Response;
use http::StatusCode;
use serde_json::Value;

use openehr_base::prelude::UidBasedId;

use openehr_its::rest::generated::common::UpdateItemTag;
use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use ferroehr::ids::{EhrId, VoId};
use ferroehr::service::response::{ResourceMeta, ServiceResponse};
use ferroehr::versioning::change::Committed;

use ferroehr::versioning::object_version_id::parse_uid_based_id;

use crate::api::RequestParts;
use crate::negotiate::{AppliedPreference, WireFormat};
use crate::overview::error::RestError;
use crate::overview::version_id::{
    object_id_uuid, parse_ehr_id, parse_version_uid, require_if_match,
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
) -> Result<Composition, RestError> {
    match negotiate::content_type_format(h) {
        Some(WireFormat::CanonicalJson | WireFormat::CanonicalXml) => {
            Ok(negotiate::rm_value::<Composition>(h, body)?)
        }
        // The Simplified-Formats adapters build a canonical COMPOSITION
        // fragment (their algorithms are defined over the wire form, not over
        // the RM types — `simplified_formats/master04`), so the converted
        // result re-enters the typed seam through the same strict reader every
        // canonical-JSON body goes through: one decode, one refusal class.
        Some(WireFormat::Flat) => typed_composition(
            &crate::formats::dispatch::composition_from_flat(state, h, body).await?,
        ),
        Some(WireFormat::Structured) => typed_composition(
            &crate::formats::dispatch::composition_from_structured(state, h, body).await?,
        ),
        _ => Err(RestError(ApiError::UnsupportedMediaType(
            "a COMPOSITION is committed as application/json, application/xml, \
             application/openehr.wt.flat+json, or application/openehr.wt.structured+json"
                .to_owned(),
        ))),
    }
}

/// Re-type a converted Simplified-Formats body as the RM `COMPOSITION` the
/// commit seam takes.
///
/// The document was already readable AS FLAT/STRUCTURED (conversion
/// succeeded), so a refusal here is template-mediated content that does not
/// form a valid resource — the `422` row, never the `400` parse class.
fn typed_composition(value: &Value) -> Result<Composition, RestError> {
    // NOTE: a post-conversion strict-reader refusal is the template-mediated
    // 422 class (master04 §Validation; `responses/422.yaml`).
    openehr_its::json::from_canonical_value(value).map_err(|e| {
        let detail = e.to_string();
        let hint = if detail.contains("invalid type") {
            " (value does not match the expected data type for the field)"
        } else {
            ""
        };
        RestError(ApiError::Unprocessable(format!(
            "the simplified-format body did not convert to a valid COMPOSITION: {detail}{hint}"
        )))
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one arm per COMPOSITION operation: a flat match keeps every \
              operation's wire behaviour readable in one place"
)]
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
            )?;
            // The item-tag wrapper headers are parsed + invariant-checked
            // BEFORE the commit, so a defective tag refuses the request while
            // nothing is durable (the WRITE stays post-commit — see
            // `pending_item_tags`).
            let pending_tags = super::pending_item_tags(h)?;
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
                super::apply_item_tag_headers(&state, ehr_id, "COMPOSITION", &uid, pending_tags)
                    .await?;
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
            body = super::expand_multimedia_if_requested(&state, q, body).await?;
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
            // write to the path's object.
            // NOTE: the rule is OAS-grounded (docs text silent) with no
            // assigned status; the fitting released row is 422
            // (Requests_and_responses.md §HTTP status codes) — adjudicated.
            if let Some(body_uid) = body.uid.as_ref() {
                // The versioned object a body `uid` names is its
                // OBJECT_VERSION_ID `object_id` (BASE `base_types` §Functions
                // `object_id`). A non-UUID `object_id` cannot name the
                // addressed object and a HIER_OBJECT_ID names no VERSION —
                // both fail the comparison; a malformed identifier never gets
                // here (the validating doors refuse it at parse, `400`).
                let body_vo = match body_uid {
                    UidBasedId::ObjectVersionId(ovid) => object_id_uuid(ovid),
                    UidBasedId::HierObjectId(_) => None,
                };
                if body_vo != Some(uid.vo_id.0) {
                    return Err(ApiError::Unprocessable(format!(
                        "the body COMPOSITION.uid {:?} does not identify the \
                         versioned object addressed by the request path ({})",
                        body_uid.value(),
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
            )?;
            // Judge the wrapper-header tags before the commit (see
            // `pending_item_tags`); the write itself stays post-commit.
            let pending_tags = super::pending_item_tags(h)?;
            match state
                .backend()
                .update_composition(ehr_id, uid.vo_id, uv)
                .await
            {
                Ok(committed) => {
                    let new_uid = committed.version_uid();
                    // apply item-tag write-wrapper headers to the new version.
                    let stored_tags = super::apply_item_tag_headers(
                        &state,
                        ehr_id,
                        "COMPOSITION",
                        &new_uid,
                        pending_tags,
                    )
                    .await?;
                    let meta = commit_meta(ehr_id, new_uid, &committed);
                    // The minimal-preference update is 204 — the docs text's
                    // §"Prefer minimal…" ("If no response body is returned,
                    // the service SHOULD use `204 No Content`") over the
                    // released 204_version_updated.yaml ("returned when the
                    // update operation was successful and the `Prefer` header
                    // is missing or is set to `return=minimal`"); a bodyless
                    // 200 matches neither declared response. Create stays
                    // 201-only (composition_create.yaml declares no 204).
                    let mut resp =
                        composition_write_response(&state, h, &base, ehr_id, meta, no_content, ok)
                            .await?;
                    super::echo_item_tags(&mut resp, &stored_tags);
                    Ok(resp)
                }
                Err(e @ ferroehr::service::error::ServiceError::VersionConflict(_)) => {
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
            // The operation declares no `If-Match` parameter, but a RECEIVED
            // one is honoured (overview §"If-Match and accidental
            // overwrites": condition false → the method MUST NOT be
            // performed). The service evaluates it AFTER its own 404/400/409
            // pre-checks, per the RFC 9110 §13.2.1 precedence rule (ignore
            // preconditions when the unconditioned answer is not 2xx/412);
            // a malformed value is a 400 like every required-If-Match route.
            let volunteered_if_match = match h.get("if-match").and_then(|v| v.to_str().ok()) {
                Some(raw) => Some(require_if_match(raw)?),
                None => None,
            };
            // A DELETE commits a `523|deleted|` version, so the committal
            // request headers are accepted and merged here too (overview
            // §"openehr-version and openehr-audit-details": PUT, POST and
            // DELETE).
            let update_audit = crate::overview::committal::committal_audit_for_delete(
                h,
                super::committer_proxy(),
            )?;
            match state
                .backend()
                .delete_composition(
                    ehr_id,
                    &ovid,
                    volunteered_if_match.as_ref(),
                    update_audit.as_ref(),
                )
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
                // 409_COMPOSITION_with_uid_based_id (stale / not-modifiable)
                // and the volunteered-If-Match 412 → both decorated with the
                // latest version_uid (overview §"If-Match and accidental
                // overwrites": the 412 "SHOULD return also latest
                // `version_uid` in the `ETag` response headers").
                Err(
                    e @ (ferroehr::service::error::ServiceError::Conflict(_)
                    | ferroehr::service::error::ServiceError::VersionConflict(_)),
                ) => {
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
                .target_tags_get(ehr_id, p.uid_based_id, "COMPOSITION")
                .await?;
            Ok(negotiate::respond(
                h,
                ok,
                &openehr_its::json::to_canonical_value(&tags),
            ))
        }
        "composition_tags_update" => {
            let p = params::build::<CompositionTagsUpdateParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            // Strict against `schemas/common/UpdateItemTag.yaml`
            // (`additionalProperties: false`, `key` required): an undeclared
            // member or a non-string `value`/`target_path` is a 400 naming the
            // member, never a silent drop.
            let body = negotiate::typed_json_vec::<UpdateItemTag>(h, &parts.body)?;
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
                &openehr_its::json::to_canonical_value(&tags),
            ))
        }
        "composition_tags_delete" => {
            let p = params::build::<CompositionTagsDeleteParams>(&parts.path, q, h)?;
            let ehr_id = parse_ehr_id(&p.ehr_id)?;
            state
                .backend()
                .target_tag_delete(ehr_id, p.uid_based_id, "COMPOSITION", p.key)
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
        // Version-id headers are representation-independent: RFC 7231
        // §6.3.2 requires `Location` on a `201` regardless of body form, and
        // the ITS-REST response definitions declare `ETag` + `Location` on
        // every COMPOSITION commit (`responses/201_COMPOSITION.yaml`,
        // `200_COMPOSITION_updated.yaml`; overview `Requests_and_responses.md`
        // §Prefer) — routed through the canonical write path's header helper.
        negotiate::set_resource_headers(&mut out, base, Some("composition"), &meta);
        // NOTE: a Simplified-Formats commit always returns the committed body
        // in the negotiated form (`Accept` decides, not `Prefer`), so the
        // applied preference is representation (§Representation details negotiation).
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
