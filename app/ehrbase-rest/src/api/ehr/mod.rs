//! The ITS-REST **ehr API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` + the
//! `ehr-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/ehr.md`.
//!
//! [`dispatch`] is the operation-id → resource-module router; the 33 EHR-group
//! operations are implemented in one module per spec resource boundary
//! (`docs/specs/openehr/ITS-REST/specifications/docs/ehr/`,
//! `specifications/operations/*.yaml`):
//!
//! - [`ehr`] — the `EHR` resource + EHR-level item tags
//! - [`ehr_status`] — the `EHR_STATUS` resource + its item tags
//! - [`versioned_ehr_status`] — the `VERSIONED_EHR_STATUS` container
//! - [`composition`] — the `COMPOSITION` resource + its item tags
//! - [`versioned_composition`] — the `VERSIONED_COMPOSITION` container
//! - [`directory`] — the `DIRECTORY` (FOLDER) resource
//! - [`contribution`] — the `CONTRIBUTION` resource
//!
//! Each arm rebuilds the operation's `*Params`, decodes wire strings into the
//! SM catalog's native argument types (`uuid::Uuid`,
//! [`ObjectVersionId`](openehr_base::prelude::ObjectVersionId),
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
pub mod ehr;
pub mod ehr_status;
pub mod versioned_composition;
pub mod versioned_ehr_status;

pub(crate) use dispatch::dispatch;

// COMPOSITION create/get/update negotiate the Simplified-Formats
// (FLAT/STRUCTURED) representations through the shared converter seam; the
// group-level alias lets the `composition` module's `super::flat::…` resolve to
// that converter module (the same pattern the `definition` group uses).
//
// TODO(w3e-integrate): the shared converters
// `crate::formats::dispatch::{composition_from_flat,composition_from_structured,
// composition_flat_response,composition_structured_response}` are currently
// `pub(super)` (visible only inside `crate::formats`); widen them to
// `pub(crate)` (or re-export at `crate::formats`) so these cross-group calls
// compile — the same reconciliation the `definition` example handler needs.
use crate::formats::dispatch as flat;

use axum::response::Response;
use http::{HeaderMap, HeaderName};
use serde_json::{Value, json};
use uuid::Uuid;

use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{PartyProxy, PartySelf};

use ehrbase_sm::Platform;
use ehrbase_sm::{ResourceMeta, ServiceResponse, UpdateAudit, UpdateVersion};

use crate::AuthMethod;
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
/// with an `ETag`/`Location` needs (the version id is the object's own `uid`).
fn resource_meta_from(ehr_id: &str, body: &Value) -> Option<ResourceMeta> {
    body.get("uid")
        .and_then(|u| u.get("value"))
        .and_then(Value::as_str)
        .map(|uid| ResourceMeta::new(ehr_id.to_owned(), uid.to_owned()))
}

/// Wrap a read body as a [`ServiceResponse`], attaching resource metadata drawn
/// from the body's own `uid` when present.
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
/// published by the auth middleware (system identity when none). The SM service
/// impl re-derives the committer from the same principal, so this rides in the
/// [`UpdateVersion`] envelope for completeness.
fn committer_proxy() -> PartyProxy {
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
        },
        signature: None,
    };
    crate::overview::committal::merge_committal_headers(&mut uv, headers);
    uv
}

/// Decompose an [`ObjectVersionId`] into the `(versioned-object uuid,
/// version_tree_id)` pair the SM `*_at_version` reads take. Branch version ids
/// are first-class (RM common master06 §Version tree; the former trunk-only
/// rejection F-06-09 is retired).
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
/// ITEM_TAG list, and "providing an empty value for this header will
/// effectively remove all ITEM_TAGs associated with the given target". The
/// header parse is [`crate::overview::params::parse_item_tag_header`]; the
/// entries are folded onto the existing vo-keyed
/// [`ItemTagAdapter`](ehrbase_sm::ItemTagAdapter) `target_tags_replace` seam
/// (the same seam the dedicated `*_tags_*` operations use).
///
/// Returns the present header name(s) + the stored list (for the optional
/// response echo) when either wrapper header was present, or `None` when
/// neither was — an absent header leaves the target's tags untouched. This
/// server supports ITEM_TAGs; a server that did not would ignore the headers
/// (spec: "these headers will also be unsupported").
pub(super) async fn apply_item_tag_headers<S: Platform>(
    state: &AppState<S>,
    ehr_id: Uuid,
    target_type: &str,
    version_uid: &str,
    headers: &HeaderMap,
) -> Result<Option<(Vec<&'static str>, Vec<Value>)>, RestError> {
    // `None` = header absent (leave tags intact); `Some(empty)` = empty header
    // value (clear all tags) — the parse helper draws the distinction.
    let object_tags = parse_item_tag_header(headers, H_ITEM_TAG);
    let version_tags = parse_item_tag_header(headers, H_VERSION_ITEM_TAG);
    if object_tags.is_none() && version_tags.is_none() {
        return Ok(None);
    }
    let mut present: Vec<&'static str> = Vec::new();
    if object_tags.is_some() {
        present.push(H_ITEM_TAG);
    }
    if version_tags.is_some() {
        present.push(H_VERSION_ITEM_TAG);
    }
    // PORT NOTE (wire): the storage seam (`ItemTagAdapter::target_tags_replace`)
    // is keyed by VERSIONED_OBJECT (the vo_id parsed from the version_uid).
    // openEHR distinguishes `openehr-item-tag` (VERSIONED_OBJECT target) from
    // `openehr-version-item-tag` (a specific VERSION target) via the tag's
    // `target_path`; with a single vo-keyed replace seam both wrappers fold into
    // one replace list here.
    let tags: Vec<Value> = object_tags
        .into_iter()
        .flatten()
        .chain(version_tags.into_iter().flatten())
        .map(|entry| entry_to_value(&entry))
        .collect();
    let stored = state
        .backend()
        .target_tags_replace(ehr_id, version_uid.to_owned(), target_type, tags)
        .await?;
    Ok(Some((present, stored)))
}

/// One parsed ITEM_TAG header entry → the ITEM_TAG JSON the storage seam takes.
/// An empty header value carries no `value` (RM ITEM_TAG `Inv_value_valid`:
/// "value /= Void implies not value.is_empty" — value is optional but, if set,
/// non-empty).
fn entry_to_value(entry: &ItemTagHeaderEntry) -> Value {
    let mut t = json!({ "key": entry.key });
    if !entry.value.is_empty() {
        t["value"] = json!(entry.value);
    }
    if let Some(path) = &entry.target_path {
        t["target_path"] = json!(path);
    }
    t
}

/// Echo the stored ITEM_TAG list onto a create/update response under the
/// wrapper header name(s) the request used — MAY-level confirmation
/// (`Requests_and_responses.md §…§Usage in Responses`: "Servers MAY include the
/// `openehr-item-tag` … header in responses to confirm the actual list of
/// ITEM_TAGs stored on the server side"). Rendered via
/// [`crate::overview::params::emit_item_tag_header`]; an empty list confirms a
/// clear.
pub(super) fn echo_item_tags(resp: &mut Response, names: &[&'static str], tags: &[Value]) {
    let entries: Vec<ItemTagHeaderEntry> = tags.iter().filter_map(value_to_entry).collect();
    let value = emit_item_tag_header(&entries);
    for &name in names {
        resp.headers_mut()
            .insert(HeaderName::from_static(name), value.clone());
    }
}

/// A stored ITEM_TAG JSON value → a header entry (for the response echo).
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
