// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The ITS-REST ehr API (Release-1.1.0, STABLE).
//!
//! Governing spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` and
//! the `ehr-*.openapi.yaml` OAS group generated into `openehr_its::rest`.
//!
//! [`dispatch`] routes an operation id to one of seven resource modules, one per
//! spec resource boundary: [`ehr_resource`], [`ehr_status`],
//! [`versioned_ehr_status`], [`composition`], [`versioned_composition`],
//! [`directory`] and [`contribution`].
//!
//! Each arm rebuilds the operation's `*Params`, decodes wire strings into the SM
//! catalog's native argument types through [`crate::overview::version_id`],
//! decodes any body (RM-typed bodies accept JSON or canonical XML), calls the
//! SM catalog methods on the platform service `S`, and rebuilds a
//! [`ServiceResponse`] from the result, which the `negotiate::*` helpers render
//! with the spec's `ETag`/`Location`/`Prefer` behaviour. The shared
//! write, read, committal and item-tag helpers below back all seven modules.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the handler carries the canonical fragment the \
              negotiate seam produced once (stored-content serving / commit interior)"
)]

pub mod composition;
pub mod contribution;
pub mod directory;
pub mod dispatch;
pub mod ehr_resource;
pub mod ehr_status;
pub mod openapi_routes;
pub mod versioned_composition;
pub mod versioned_ehr_status;

// COMPOSITION + CONTRIBUTION negotiate the Simplified-Formats (FLAT/STRUCTURED)
// representations through the shared `crate::formats::dispatch` adapter, called
// by its full path (no module alias — every import names its defining module).

use axum::response::Response;
use http::{HeaderMap, HeaderName};
use serde_json::Value;
use uuid::Uuid;

use openehr_base::prelude::ObjectVersionId;
use openehr_its::rest::generated::common::{
    UpdateAudit, UpdateAuditData, UpdateItemTag, UpdateVersion,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{
    DvIdentifier, ItemTag, PartyIdentified, PartyIdentifiedData, PartyProxy,
};

use ferroehr::ids::EhrId;
use ferroehr::service::response::{ResourceMeta, ServiceResponse};
use ferroehr::service::version_update::{change_type_coded, lifecycle_state_coded, plain_text};

use crate::extensions::access::authn::AuthMethod;
use crate::overview::error::RestError;
use crate::overview::params::{
    H_ITEM_TAG, H_VERSION_ITEM_TAG, ItemTagHeaderEntry, emit_item_tag_header,
    item_tag_to_header_entry, parse_item_tag_header, query_param, validate_item_tag_entries,
};
use crate::state::AppState;

/// openEHR *audit change type* codes (the `openehr` terminology group).
pub(super) const CHANGE_CREATION: &str = "249";
pub(super) const CHANGE_MODIFICATION: &str = "251";
/// `532|complete|` — the lifecycle state stamped on synthesized version updates.
pub(super) const LIFECYCLE_COMPLETE: &str = "532";

/// The `uid`/`value` of a returned RM object → the resource metadata a read
/// with an `ETag`/`Last-Modified` needs (the version id is the object's own
/// `uid`).
///
/// When the served body is a VERSION envelope, its `commit_audit.time_committed`
/// is also read as the `Last-Modified` instant: "For openEHR resources, this
/// value should be derived from `VERSION.commit_audit.time_committed.value`"
/// (`Requests_and_responses.md` §"`ETag` and Last-Modified").
///
/// A bare RM body carries no commit audit, so those routes take their metadata
/// from the service layer, which reads the commit instant off the version row.
fn resource_meta_from(ehr_id: &str, body: &Value) -> Option<ResourceMeta> {
    let uid = body
        .pointer("/uid/value")
        .or_else(|| body.pointer("/item/uid/value"))
        .and_then(Value::as_str)?;
    let meta = ResourceMeta::new(ehr_id.to_owned(), uid.to_owned());
    Some(match commit_instant(body) {
        Some(at) => meta.with_last_modified(at),
        None => meta,
    })
}

/// The commit instant of a served VERSION envelope —
/// `VERSION.commit_audit.time_committed.value` (RM common master04
/// §Audit Details; the `Last-Modified` source named by ITS-REST overview
/// §"`ETag` and Last-Modified"). `None` for a body that is not a VERSION.
fn commit_instant(body: &Value) -> Option<jiff::Timestamp> {
    body.get("commit_audit")
        .and_then(|a| a.get("time_committed"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)?
        .parse::<jiff::Timestamp>()
        .ok()
}

/// The canonical-XML root tag for a served VERSION envelope: the document
/// element the published ITS-XML schemas declare for the resource.
///
/// `Resources.md` §"XML Format" requires responses to "conform to the
/// [published XSDs]", and the schemas publish exactly one document element for a
/// VERSION, spelled identically in both lineages. `VERSION` is abstract there,
/// so the concrete subtype is named by `xsi:type` on that same root, which is
/// how an `IMPORTED_VERSION` stays distinguishable from an `ORIGINAL_VERSION`
/// (RM common master06 §Version and its Subtypes).
pub(super) const VERSION_ROOT_TAG: &str = "version";

/// Re-inlines externalized `DV_MULTIMEDIA` content when the caller asked for it
/// with `?expand_multimedia=true`, verifying each blob's integrity.
///
/// Every read that can return externalized content routes through here, because
/// externalization is applied on the way in by the generic versioning path, so a
/// read that could not restore it would leave clinical content in the object
/// store with no API that returns it.
///
/// NOTE: no openEHR spec governs this — our own design/extension, so the
/// parameter is read off the raw query string rather than a generated params
/// struct.
///
/// # Errors
/// Propagates the service's failure when a referenced blob cannot be fetched or
/// fails its integrity check — never a silent fall back to the stored form.
pub(super) async fn expand_multimedia_if_requested(
    state: &AppState,
    query: Option<&str>,
    body: Value,
) -> Result<Value, RestError> {
    if query_param(query, "expand_multimedia").as_deref() == Some("true") {
        return Ok(state.backend().expand_multimedia(body).await?);
    }
    Ok(body)
}

/// Wrap a read body as a [`ServiceResponse`], attaching resource metadata drawn
/// from the body's own `uid` (and, for a VERSION envelope, its commit instant)
/// when present.
pub(super) fn read_resp(ehr_id: &str, body: Value) -> ServiceResponse {
    match resource_meta_from(ehr_id, &body) {
        Some(m) => ServiceResponse::new(body, m),
        None => ServiceResponse::plain(body),
    }
}

/// The committer `PARTY_PROXY` for a write, from the authenticated principal
/// published by the auth middleware. With no authenticated principal the write
/// is attributed to this CDR's own system identity
/// ([`ferroehr::service::SYSTEM_COMMITTER_NAME`]) — the same constant the
/// platform library uses, so both layers name the system committer identically.
/// The SM service impl re-derives the committer from the same principal, so
/// this rides in the [`UpdateVersion`] envelope for completeness.
pub(crate) fn committer_proxy() -> PartyProxy {
    let party = match crate::extensions::access::authn::current_principal() {
        Some(principal) => {
            let id_type = match principal.method {
                AuthMethod::Basic => "basic",
                AuthMethod::Bearer => "oauth2",
            };
            PartyIdentifiedData {
                external_ref: None,
                name: Some(principal.subject.clone()),
                identifiers: Some(openehr_base::containers::NonEmptyVec::of(DvIdentifier {
                    issuer: Some("ferroehr".to_owned()),
                    assigner: None,
                    id: principal.subject,
                    r#type: Some(id_type.to_owned()),
                })),
            }
        }
        None => PartyIdentifiedData {
            external_ref: None,
            name: Some(ferroehr::service::SYSTEM_COMMITTER_NAME.to_owned()),
            identifiers: None,
        },
    };
    PartyProxy::PartyIdentified(PartyIdentified::PartyIdentified(party))
}

/// Synthesizes the SM `UPDATE_VERSION` commit envelope for a bare-RM-body write
/// route: the RM object is the `data`, the `If-Match` is the
/// `preceding_version_uid`, and the audit carries the change type and committer.
///
/// The server defaults are then merged with any committal request headers the
/// client supplied, the ITS-REST MUST of overview §"openehr-version and
/// openehr-audit-details" (`crate::overview::committal`).
///
/// # Errors
/// [`ApiError::BadRequest`] when a committal header carries a malformed
/// identifier (`crate::overview::committal::build_committer`).
pub(super) fn mk_update_version<T>(
    headers: &HeaderMap,
    data: T,
    change_code: &str,
    description: &str,
    preceding: Option<ObjectVersionId>,
) -> Result<UpdateVersion<T>, ApiError> {
    let mut uv = UpdateVersion {
        preceding_version_uid: preceding,
        signature: None,
        lifecycle_state: lifecycle_state_coded(LIFECYCLE_COMPLETE),
        attestations: None,
        data,
        commit_audit: UpdateAudit::UpdateAudit(UpdateAuditData {
            _type: None,
            system_id: None,
            change_type: change_type_coded(change_code),
            description: Some(plain_text(description)),
            committer: committer_proxy(),
        }),
    };
    crate::overview::committal::merge_committal_headers(&mut uv, headers)?;
    Ok(uv)
}

/// Decomposes an [`ObjectVersionId`] into the `(versioned-object uuid,
/// version_tree_id)` pair the SM `*_at_version` reads take.
///
/// Branch version ids are first-class (RM common master06 §The 'Virtual Version
/// Tree').
/// Verify a version-addressed read served the VERSION the path named: the
/// addressed `version_uid` must equal the served body's `uid.value` — the
/// stored full `object_id :: creating_system_id :: version_tree_id` identity
/// (ITS-REST overview `Resources.md` §Identifier types) — compared
/// case-insensitively (BASE master05 §"Composite Identifiers and Case").
/// A tree-only fetch would satisfy a fabricated `creating_system_id`; that
/// names no VERSION in this repository → 404. A body without a served uid
/// (nothing to verify against) passes.
pub(super) fn ensure_served_version(addressed: &str, body: &Value) -> Result<(), RestError> {
    match body
        .get("uid")
        .and_then(|u| u.get("value"))
        .and_then(Value::as_str)
    {
        Some(served) if !served.eq_ignore_ascii_case(addressed) => Err(RestError(
            ApiError::NotFound(format!("version {addressed}")),
        )),
        _ => Ok(()),
    }
}

pub(super) fn version_components(ovid: &ObjectVersionId) -> Result<(Uuid, String), ApiError> {
    let vo = crate::overview::version_id::object_id_uuid(ovid).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "OBJECT_VERSION_ID object_id is not a UUID: {}",
            ovid.value()
        ))
    })?;
    Ok((vo, ovid.version_tree_id().value().to_owned()))
}

/// Apply the `openehr-item-tag` / `openehr-version-item-tag` write-wrapper
/// request headers on a change-controlled write
/// (`Requests_and_responses.md §openehr-item-tag and openehr-version-item-tag
/// §Usage in Requests`): the provided tag list **replaces** the target's
/// `ITEM_TAG` list, and "providing an empty value for this header will
/// effectively remove all `ITEM_TAGs` associated with the given target".
///
/// This is the WRITE half only: the headers were already parsed and
/// invariant-checked by [`pending_item_tags`], BEFORE the content commit, so
/// by the time this runs the only failure left is a storage one. The write
/// stays after the commit because the tags target the version the commit
/// mints, and a tag must not re-version the content it annotates. The
/// entries are folded onto the existing vo-keyed
/// `FerroEhrService::target_tags_replace` seam (the same seam the dedicated
/// `*_tags_*` operations use).
///
/// Returns the stored list **per header**, because the two headers address
/// distinct targets ([`StoredItemTags`]); a header the request did not carry
/// leaves its target's tags untouched and echoes nothing. This server supports
/// `ITEM_TAGs`; a server that did not would ignore the headers (spec: "these
/// headers will also be unsupported").
pub(super) async fn apply_item_tag_headers(
    state: &AppState,
    ehr_id: EhrId,
    target_type: &str,
    version_uid: &str,
    pending: PendingItemTags,
) -> Result<StoredItemTags, RestError> {
    let PendingItemTags {
        object: object_tags,
        version: version_tags,
    } = pending;
    // The two wrappers address DISTINCT collections
    // (Requests_and_responses.md §"openehr-item-tag and
    // openehr-version-item-tag"): `openehr-item-tag` replaces the
    // VERSIONED_OBJECT container's tags, `openehr-version-item-tag` the
    // just-committed VERSION's own. An absent header leaves its collection
    // untouched and the two lists are never merged. The container id is the
    // `object_id` of the committed OBJECT_VERSION_ID, read through the BASE
    // accessor (`base_types` §Functions `object_id`), never a local `::` split.
    // NOTE: the just-committed version uid is server-minted, so a parse failure
    // here is a server fault, never a client one.
    let container_uid = ObjectVersionId::new(version_uid)
        .map_err(|e| {
            crate::overview::error::internal_fault(
                "read the committed version uid",
                &format!("{version_uid:?}: {e}"),
            )
        })?
        .object_id()
        .value()
        .into_owned();
    let mut stored = StoredItemTags::default();
    if let Some(tags) = object_tags {
        stored.object = Some(
            state
                .backend()
                .target_tags_replace(ehr_id, container_uid, target_type, tags)
                .await?,
        );
    }
    if let Some(tags) = version_tags {
        stored.version = Some(
            state
                .backend()
                .target_tags_replace(ehr_id, version_uid.to_owned(), target_type, tags)
                .await?,
        );
    }
    Ok(stored)
}

/// The `ITEM_TAG` lists a request's wrapper headers ask for, parsed and
/// invariant-checked **before** any content is committed.
///
/// `None` per field = the request carried no such header, so that collection is
/// left untouched; `Some(list)` = the list to write (empty = the release's
/// clear-all form).
#[derive(Debug, Default)]
pub(super) struct PendingItemTags {
    /// What `openehr-item-tag` asks to store on the `VERSIONED_OBJECT`.
    object: Option<Vec<UpdateItemTag>>,
    /// What `openehr-version-item-tag` asks to store on the committed VERSION.
    version: Option<Vec<UpdateItemTag>>,
}

/// Parses and validates both wrapper headers before the content write.
///
/// The release gives the wrapper headers no atomicity semantics, so the ordering
/// is ours: a header defect the server can detect without touching storage
/// rejects the whole request while nothing has been committed, because failing
/// after the commit would answer 4xx for a request whose VERSION is already
/// durable and whose response carries no `ETag`/`Location`.
///
/// The tag write itself stays after the commit and must: the tags target the
/// version the commit mints, and RM common `master07-tags.adoc` forbids a tag
/// from participating in the content's change control — "they do not cause
/// re-versioning of the content".
///
/// # Errors
/// [`ApiError::BadRequest`] for a malformed header entry;
/// [`ApiError::Unprocessable`] for an entry that breaks an RM `ITEM_TAG`
/// invariant.
pub(super) fn pending_item_tags(headers: &HeaderMap) -> Result<PendingItemTags, RestError> {
    // `None` = header absent (leave tags intact); `Some(empty)` = empty header
    // value (clear all tags) — the parse helper draws the distinction.
    Ok(PendingItemTags {
        object: validated_entries(parse_item_tag_header(headers, H_ITEM_TAG)?, H_ITEM_TAG)?,
        version: validated_entries(
            parse_item_tag_header(headers, H_VERSION_ITEM_TAG)?,
            H_VERSION_ITEM_TAG,
        )?,
    })
}

/// Turn one header's parsed entries into the EHR group's write DTOs, refusing
/// any entry the RM `ITEM_TAG` invariants reject
/// ([`crate::overview::params::validate_item_tag_entries`] — the one judgement
/// both tag families share).
fn validated_entries(
    entries: Option<Vec<ItemTagHeaderEntry>>,
    name: &str,
) -> Result<Option<Vec<UpdateItemTag>>, RestError> {
    let Some(entries) = entries else {
        return Ok(None);
    };
    validate_item_tag_entries(&entries, name)?;
    Ok(Some(
        entries
            .into_iter()
            .map(|entry| UpdateItemTag {
                key: entry.key,
                value: entry.value,
                target_path: entry.target_path,
            })
            .collect(),
    ))
}

/// The `ITEM_TAG` collections an item-tag write-wrapper request stored, kept
/// one per header because `openehr-item-tag` and `openehr-version-item-tag`
/// "apply to" different targets — a `VERSIONED_OBJECT` and a specific VERSION
/// within it (`Requests_and_responses.md` §"openehr-item-tag and
/// openehr-version-item-tag"). `None` = the request carried no such header, so
/// that target was not written and nothing is echoed for it; `Some(list)` =
/// the list now stored on that target (empty confirms a clear).
#[derive(Debug, Default)]
pub(super) struct StoredItemTags {
    /// The `VERSIONED_OBJECT` container's stored tags (`openehr-item-tag`).
    object: Option<Vec<ItemTag>>,
    /// The committed VERSION's own stored tags (`openehr-version-item-tag`).
    version: Option<Vec<ItemTag>>,
}

/// Echoes the stored `ITEM_TAG` lists onto a create/update response: "Servers
/// MAY include the `openehr-item-tag` or `openehr-version-item-tag` header in
/// responses to confirm the actual list of `ITEM_TAGs` stored on the server
/// side" (`Requests_and_responses.md` §Usage in Responses).
///
/// Each header carries its own target's collection and nothing else, so a
/// response never repeats one target's tags under the other target's name; a
/// header the request did not carry is not echoed at all, and an empty list
/// confirms a clear.
///
/// A list that cannot be rendered as an HTTP field value — a tag carrying a
/// control character, which nothing in the RM bars from a key — omits the header
/// entirely rather than emitting an empty one: an empty `openehr-item-tag` is
/// the release's "remove all `ITEM_TAGs`" instruction (§Usage in Requests), so
/// echoing one would tell a mirroring client to wipe the collection this
/// response just confirmed. The echo is a MAY, so declining is always available.
pub(super) fn echo_item_tags(resp: &mut Response, stored: &StoredItemTags) {
    for (name, tags) in [
        (H_ITEM_TAG, stored.object.as_deref()),
        (H_VERSION_ITEM_TAG, stored.version.as_deref()),
    ] {
        let Some(tags) = tags else {
            continue;
        };
        let entries: Vec<ItemTagHeaderEntry> = tags.iter().map(item_tag_to_header_entry).collect();
        if let Some(value) = emit_item_tag_header(&entries) {
            resp.headers_mut()
                .insert(HeaderName::from_static(name), value);
        }
    }
}
