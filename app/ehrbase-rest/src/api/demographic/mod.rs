//! HTTP dispatch for the ITS-REST **demographic API** (development edition).
//!
//! A machine-readable openEHR wire contract for demographics now exists: the
//! ITS-REST **Demographic API** (`x-status: DEVELOPMENT`), vendored at
//! `docs/specs/openehr/ITS-REST/specifications/demographic.openapi.yaml` and its
//! per-operation contracts under `.../operations/` (`person_create.yaml`,
//! `person_get.yaml`, `person_update.yaml`, `person_delete.yaml` and the
//! identical `agent_*`/`group_*`/`organisation_*`/`role_*`; `versioned_party_*`;
//! `demographic_contribution_*`; `demographic_tags_get` + the per-party
//! `*_tags_*`). The route table + `*Params` structs are generated from that OAS
//! into `openehr_its::rest::generated::demographic`; this module **implements
//! that contract** over the `ehrbase-sm` native API — it is a spec-defined
//! (development-maturity) wire, not an "extension by analogy with the EHR
//! group". Where the EHR-group response envelope (`ETag`/`Location`/`Prefer`/
//! `If-Match`) coincides with the demographic operation YAMLs it is kept, but
//! justified from the demographic contract, not by analogy.
//!
//! The **only** genuinely spec-absent surface is `PARTY_RELATIONSHIP` (see
//! [`relationship`]): the vendored Demographic API defines no
//! `party_relationship` paths — those routes are our own extension realizing SM
//! `I_PARTY_RELATIONSHIP` and are excluded from conformance-profile claims.
//!
//! The five per-kind operation families are collapsed by mapping the
//! operation-id prefix to a [`PartyKind`] ([`parse_party_op`]); the generated
//! per-kind `*Params` structs are field-identical, so one representative struct
//! is reused across kinds. File layout mirrors the spec resources:
//! [`party`] (`{kind}` CRUD), [`tags`] (`ITEM_TAG` sub-resources +
//! `demographic_tags_get`), [`versioned_party`], [`contribution`],
//! [`relationship`] (extension), and [`dispatch`] as the operation-id match.

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, HeaderValue, header};

use openehr_its::rest::runtime::ApiError;

use crate::overview::error::RestError;
use crate::overview::params::ItemTagHeaderEntry;
use ehrbase::service::demographic::types::PartyKind;
use ehrbase::service::response::{ResourceMeta, ServiceResponse};
use ehrbase::service::status::{CallStatusType, SmError};

mod contribution;
pub(crate) mod dispatch;
pub(crate) mod openapi_routes;
mod party;
pub(crate) mod relationship;
mod tags;
mod versioned_party;

// The two symbols `app/ehrbase-rest/src/api/mod.rs` mounts: the group
// dispatcher (`demographic::dispatch`) and the native `utoipa-axum` router for
// the own-design PARTY_RELATIONSHIP extension (`demographic::relationship_routes`,
// which single-sources those routes + their OpenAPI paths and dispatches back
// through the group dispatcher).

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

/// Whether an error is the optimistic-concurrency precondition failure
/// (`If-Match` mismatch → `412` on update; a stale `uid_based_id` → `409` on
/// delete, per `person_delete.yaml` `409_PERSON_with_uid_based_id`).
fn is_precondition(e: &SmError) -> bool {
    e.status == CallStatusType::VersionMismatch
}

/// The `If-Match` header value (an `OBJECT_VERSION_ID`), if present and
/// well-formed.
fn if_match_of(h: &HeaderMap) -> Option<String> {
    h.get("if-match")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Set `ETag` (the resource uid, weak form `W/"…"` — the ITS-REST overview
/// §"`ETag` and Last-Modified" makes resource-identifier `ETag`s weak-type; the
/// bare quoted form is deprecated) and a `/demographic/{segment}/{uid}`
/// `Location` from a demographic [`ResourceMeta`] (whose `ehr_id` is empty —
/// parties are not EHR-scoped). Location shape: `headers/Location_PERSON.yaml`.
fn set_headers(resp: &mut Response, base: &str, segment: &str, meta: Option<&ResourceMeta>) {
    let Some(meta) = meta else { return };
    if let Ok(etag) = HeaderValue::from_str(&format!("W/\"{}\"", meta.uid)) {
        resp.headers_mut().insert(header::ETAG, etag);
    }
    // `Last-Modified` from the version's commit time — overview §"ETag and
    // Last-Modified": both SHOULD accompany versioned resources.
    if let Some(at) = meta.last_modified
        && let Ok(lm) = HeaderValue::from_str(&crate::overview::negotiate::http_date(at))
    {
        resp.headers_mut().insert(header::LAST_MODIFIED, lm);
    }
    let location = format!("{base}/demographic/{segment}/{}", meta.uid);
    if let Ok(loc) = HeaderValue::from_str(&location) {
        resp.headers_mut().insert(header::LOCATION, loc);
    }
    resp.extensions_mut()
        .insert(crate::system_log::middleware::AuditObject {
            ehr_id: None,
            uid: Some(meta.uid.clone()),
        });
}

/// Emit the `openehr-item-tag` / `openehr-version-item-tag` **response** headers
/// mandated by `responses/201_PERSON.yaml` (create) and `person_get.yaml` (get)
/// when a party carries `ITEM_TAGs` — the server-set tags ride the response
/// metadata seam ([`ResourceMeta::item_tags`], a canonical `ITEM_TAG` list) and
/// are rendered through [`crate::overview::params::emit_item_tag_header`]
/// (`headers/openehr-item-tag.yaml`, `headers/openehr-version-item-tag.yaml`).
///
/// Demographic `ITEM_TAGs` are stored against the `VERSIONED_OBJECT`
/// (`item_tag.target_vo_id`, no version anchor), so the full set is emitted for
/// both headers: `openehr-item-tag` (all tags on the `VERSIONED_OBJECT`) and
/// `openehr-version-item-tag` (all tags on the current VERSION) coincide here.
fn set_item_tag_headers(resp_out: &mut Response, resp: &ServiceResponse) {
    let Some(meta) = resp.meta.as_ref() else {
        return;
    };
    let Some(serde_json::Value::Array(tags)) = meta.item_tags.as_ref() else {
        return;
    };
    let entries: Vec<ItemTagHeaderEntry> = tags
        .iter()
        .filter_map(crate::overview::params::item_tag_to_header_entry)
        .collect();
    if entries.is_empty() {
        return;
    }
    let value = crate::overview::params::emit_item_tag_header(&entries);
    resp_out
        .headers_mut()
        .insert(crate::overview::params::H_ITEM_TAG, value.clone());
    resp_out
        .headers_mut()
        .insert(crate::overview::params::H_VERSION_ITEM_TAG, value);
}

/// Render an error, additionally setting the latest-version `ETag`/`Location`
/// the `412` (update) / `409` (delete) responses require.
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
