//! The ITS-REST **ehr API** (Release-1.1.0, STABLE) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` + the
//! `ehr-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Governing spec: `docs/specs/openehr/ITS-REST/specifications/docs/ehr/`.
//!
//! [`dispatch`] is the operation-id → resource-module router; the 33 EHR-group
//! operations are implemented in one module per spec resource boundary
//! (`docs/specs/openehr/ITS-REST/specifications/docs/ehr/`,
//! `specifications/operations/*.yaml`):
//!
//! - [`ehr_resource`] — the `EHR` resource + EHR-level item tags
//! - [`ehr_status`] — the `EHR_STATUS` resource + its item tags
//! - [`versioned_ehr_status`] — the `VERSIONED_EHR_STATUS` container
//! - [`composition`] — the `COMPOSITION` resource + its item tags
//! - [`versioned_composition`] — the `VERSIONED_COMPOSITION` container
//! - [`directory`] — the `DIRECTORY` (FOLDER) resource
//! - [`contribution`] — the `CONTRIBUTION` resource
//!
//! Each arm rebuilds the operation's `*Params`, decodes wire strings into the
//! SM catalog's native argument types (`uuid::Uuid`,
//! [`openehr_base::prelude::ObjectVersionId`],
//! [`UpdateVersion`]) via [`crate::overview::version_id`], decodes any body
//! (RM-typed bodies accept JSON or canonical XML), calls the EHR-core SM catalog
//! methods on the platform service `S`, and rebuilds a [`ServiceResponse`] (RM
//! payload + typed [`ResourceMeta`]) from the native result — from which the
//! `negotiate::*` helpers render the spec's `ETag`/`Location`/`Prefer`
//! behaviour. The shared write/read/committal/item-tag helpers below back all
//! seven resource modules.

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
use serde_json::{Value, json};
use uuid::Uuid;

use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{PartyProxy, PartySelf};

use ferroehr::ids::EhrId;
use ferroehr::service::response::{ResourceMeta, ServiceResponse};
use ferroehr::service::version_update::{UpdateAudit, UpdateVersion};

use crate::extensions::access::authn::AuthMethod;
use crate::overview::error::RestError;
use crate::overview::params::{
    H_ITEM_TAG, H_VERSION_ITEM_TAG, ItemTagHeaderEntry, emit_item_tag_header, parse_item_tag_header,
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
/// When the served body is a VERSION envelope (an `ORIGINAL_VERSION`), its
/// `commit_audit.time_committed` is also read as the `Last-Modified` instant:
/// ITS-REST overview `Requests_and_responses.md` §"`ETag` and Last-Modified"
/// — "For openEHR resources, this value should be derived from
/// `VERSION.commit_audit.time_committed.value`" — and both headers "SHOULD be
/// included in responses for VERSION, `VERSIONED_OBJECT`, or other resources
/// that have versioning or unique state identifiers".
///
/// A bare RM body (a COMPOSITION / `EHR_STATUS`, or a `VERSIONED_OBJECT`
/// container) carries no commit audit; those routes take their metadata from
/// the service layer instead, which reads the commit instant off the version
/// row.
fn resource_meta_from(ehr_id: &str, body: &Value) -> Option<ResourceMeta> {
    let uid = body
        .get("uid")
        .and_then(|u| u.get("value"))
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

/// Wrap a read body as a [`ServiceResponse`], attaching resource metadata drawn
/// from the body's own `uid` (and, for a VERSION envelope, its commit instant)
/// when present.
pub(super) fn read_resp(ehr_id: &str, body: Value) -> ServiceResponse {
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
/// published by the auth middleware. With no authenticated principal the write
/// is attributed to this CDR's own system identity
/// ([`ferroehr::service::SYSTEM_COMMITTER_NAME`]) — the same constant the
/// platform library uses, so both layers name the system committer identically.
/// The SM service impl re-derives the committer from the same principal, so
/// this rides in the [`UpdateVersion`] envelope for completeness.
pub(crate) fn committer_proxy() -> PartyProxy {
    let value = match crate::extensions::access::authn::current_principal() {
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
                    "issuer": "ferroehr",
                    "type": id_type
                }]
            })
        }
        None => json!({
            "_type": "PARTY_IDENTIFIED",
            "name": ferroehr::service::SYSTEM_COMMITTER_NAME
        }),
    };
    openehr_its::json::from_canonical_value(&value)
        .unwrap_or(PartyProxy::PartySelf(PartySelf { external_ref: None }))
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
/// openEHR-AUDIT_DETAILS"; `crate::overview::committal`).
pub(super) fn mk_update_version(
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
            system_id: None,
        },
        signature: None,
    };
    crate::overview::committal::merge_committal_headers(&mut uv, headers);
    uv
}

/// Decompose an [`ObjectVersionId`] into the `(versioned-object uuid,
/// version_tree_id)` pair the SM `*_at_version` reads take. Branch version ids
/// are first-class (RM common master06 §Version tree; the former trunk-only
/// rejection is retired).
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
            ovid.value
        ))
    })?;
    Ok((vo, ovid.version_tree_id().value.clone()))
}

/// Apply the `openehr-item-tag` / `openehr-version-item-tag` write-wrapper
/// request headers on a change-controlled write
/// (`Requests_and_responses.md §openehr-item-tag and openehr-version-item-tag
/// §Usage in Requests`): the provided tag list **replaces** the target's
/// `ITEM_TAG` list, and "providing an empty value for this header will
/// effectively remove all `ITEM_TAGs` associated with the given target". The
/// header parse is [`crate::overview::params::parse_item_tag_header`]; the
/// entries are folded onto the existing vo-keyed
/// [`ItemTagAdapter`](ferroehr::service::ItemTagAdapter) `target_tags_replace` seam
/// (the same seam the dedicated `*_tags_*` operations use).
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
    headers: &HeaderMap,
) -> Result<StoredItemTags, RestError> {
    // `None` = header absent (leave tags intact); `Some(empty)` = empty header
    // value (clear all tags) — the parse helper draws the distinction.
    let object_tags = parse_item_tag_header(headers, H_ITEM_TAG);
    let version_tags = parse_item_tag_header(headers, H_VERSION_ITEM_TAG);
    // The two wrappers address DISTINCT collections (Requests_and_responses.md
    // §"openehr-item-tag and openehr-version-item-tag": "`openehr-item-tag`
    // applies to *VERSIONED_OBJECT* targets" while "`openehr-version-item-tag`
    // applies to a specific target *VERSION* within a VERSIONED_OBJECT"):
    // `openehr-item-tag` replaces the VERSIONED_OBJECT container's tags
    // (addressed by the bare object id), and `openehr-version-item-tag`
    // replaces the just-committed VERSION's own tags (addressed by the full
    // version_uid). An absent header leaves its collection untouched, and each
    // stored collection stays separate all the way to its own response header
    // — the echo confirms "the actual list of ITEM_TAGs stored" for the target
    // the header names, so the two lists are never merged.
    let container_uid = version_uid
        .split_once("::")
        .map_or(version_uid, |(object_id, _)| object_id);
    let mut stored = StoredItemTags::default();
    if let Some(entries) = object_tags {
        let tags = entries.iter().map(entry_to_value).collect();
        stored.object = Some(
            state
                .backend()
                .target_tags_replace(ehr_id, container_uid.to_owned(), target_type, tags)
                .await?,
        );
    }
    if let Some(entries) = version_tags {
        let tags = entries.iter().map(entry_to_value).collect();
        stored.version = Some(
            state
                .backend()
                .target_tags_replace(ehr_id, version_uid.to_owned(), target_type, tags)
                .await?,
        );
    }
    Ok(stored)
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
    object: Option<Vec<Value>>,
    /// The committed VERSION's own stored tags (`openehr-version-item-tag`).
    version: Option<Vec<Value>>,
}

/// One parsed `ITEM_TAG` header entry → the `ITEM_TAG` JSON the storage seam takes.
/// An empty header value carries no `value` (RM `ITEM_TAG` `Inv_value_valid`:
/// "value /= Void implies not `value.is_empty`" — value is optional but, if set,
/// non-empty).
fn entry_to_value(entry: &ItemTagHeaderEntry) -> Value {
    let mut t = serde_json::Map::new();
    t.insert("key".to_owned(), json!(entry.key));
    if !entry.value.is_empty() {
        t.insert("value".to_owned(), json!(entry.value));
    }
    if let Some(path) = &entry.target_path {
        t.insert("target_path".to_owned(), json!(path));
    }
    Value::Object(t)
}

/// Echo the stored `ITEM_TAG` lists onto a create/update response — MAY-level
/// confirmation (`Requests_and_responses.md §…§Usage in Responses`: "Servers
/// MAY include the `openehr-item-tag` or `openehr-version-item-tag` header in
/// responses to confirm the actual list of `ITEM_TAGs` stored on the server
/// side").
///
/// **Each header carries its own target's collection and nothing else**: the
/// confirmed list is "the actual list … stored" for the target that header
/// applies to (§"openehr-item-tag and openehr-version-item-tag" — the
/// `VERSIONED_OBJECT` for `openehr-item-tag`, the VERSION for
/// `openehr-version-item-tag`), so a response never repeats one target's tags
/// under the other target's name. A header the request did not carry is not
/// echoed at all. Rendered via
/// [`crate::overview::params::emit_item_tag_header`]; an empty list confirms a
/// clear.
pub(super) fn echo_item_tags(resp: &mut Response, stored: &StoredItemTags) {
    for (name, tags) in [
        (H_ITEM_TAG, stored.object.as_deref()),
        (H_VERSION_ITEM_TAG, stored.version.as_deref()),
    ] {
        let Some(tags) = tags else {
            continue;
        };
        let entries: Vec<ItemTagHeaderEntry> = tags.iter().filter_map(value_to_entry).collect();
        resp.headers_mut().insert(
            HeaderName::from_static(name),
            emit_item_tag_header(&entries),
        );
    }
}

/// A stored `ITEM_TAG` JSON value → a header entry (for the response echo).
fn value_to_entry(v: &Value) -> Option<ItemTagHeaderEntry> {
    let key = v.get("key").and_then(Value::as_str)?.to_owned();
    Some(ItemTagHeaderEntry {
        key,
        value: v
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        target_path: v
            .get("target_path")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}
