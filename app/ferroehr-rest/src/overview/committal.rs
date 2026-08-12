// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `openehr-version` / `openehr-audit-details` committal request headers.
//!
//! ITS-REST overview §"openehr-version and openehr-audit-details"
//! (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
//! lines 72–96) makes it a **MUST** that a service accept these custom request
//! headers on the direct-commit change-controlled writes (COMPOSITION /
//! `EHR_STATUS` / directory FOLDER `POST`/`PUT`/`DELETE` — EHR creation
//! included, since it commits the bootstrap `EHR_STATUS` + `EHR_ACCESS` in a
//! CONTRIBUTION, RM ehr `master04-ehr_package.adoc` §EHR Creation) and merge
//! whatever is provided with the server's default VERSION + `commit_audit`
//! attributes:
//!
//! > "None of these headers are mandatory, but whatever is provided it MUST be
//! >  merged with the default VERSION and VERSION.audit_details attributes on
//! >  commit runtime."
//!
//! **Release-1.1.0** moved each attribute path *into the header
//! value*. The current header names are lowercase and the value is a
//! comma-separated list of `attr_path.key="value"` pairs (worked example,
//! lines 85–91):
//!
//! ```http
//! openehr-version: lifecycle_state.code_string="532"
//! openehr-audit-details: change_type.code_string="251"
//! openehr-audit-details: description.value="An updated composition contribution description"
//! openehr-audit-details: committer.name="John Doe",committer.external_ref.id="…",committer.external_ref.namespace="demographic",committer.external_ref.type="PERSON"
//! openehr-audit-details: system_id="example.openehr.systemid"
//! ```
//!
//! A header MAY appear multiple times (`openehr-audit-details` does above) and
//! all occurrences are merged (`get_all`).
//!
//! The **deprecated Release-1.0.3 forms** carried the attribute path in the
//! header *name* — `openEHR-VERSION.lifecycle_state`,
//! `openEHR-AUDIT_DETAILS.change_type` / `.description` / `.committer` /
//! `.system_id` — with a bare `key="value"` list in the value. §"Deprecated
//! headers" keeps these "available for backward compatibility" (a MAY), so we
//! still accept them; the new value-carrying form **wins on conflict**.
//!
//! `system_id` (Release-1.1.0): a client MAY supply it here; when it is
//! absent "the server MUST set it to its own configured system identifier"
//! (line 94). The header layer only carries a client-supplied value into
//! `UpdateAudit::system_id`; the server default is asserted at the versioning
//! seam, not here.
//!
//! NOTE (wire, spec-silent): the per-attribute value grammar is given only
//! by example — the spec states no formal ABNF for the `attr.key="value"` list,
//! its quoting, or escaping. We parse a tolerant comma-separated list of
//! `path="value"` (or bare `path=value`) pairs, treating a quoted value as
//! opaque (so a `description` value may itself contain commas). A header that
//! does not yield the attribute it targets is ignored (the server default
//! stands), never an error — the spec only says "merge whatever is provided".
//!
//! NOTE (wire, spec-silent): the `committer` `external_ref.id` is wrapped
//! as a `HIER_OBJECT_ID` (the spec example is a UUID and gives no `OBJECT_ID`
//! subtype); if the assembled `PARTY_IDENTIFIED` fails to type, the committer is
//! left at the server default.

#![allow(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction); the carriers here are \
              cfg(test)-only, so #[expect] would be unfulfilled in the non-test build"
)]

use http::HeaderMap;
use indexmap::IndexMap;

use super::params::key_value_pairs;

use openehr_base::prelude::{HierObjectId, ObjectId, PartyRef};
use openehr_its::rest::generated::common::{UpdateAudit, UpdateAuditData, UpdateVersion};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::{DvCodedText, PartyIdentified, PartyIdentifiedData, PartyProxy};

use ferroehr::service::version_update::{
    Committal, audit_base_mut, change_type_coded, lifecycle_state_coded, plain_text,
    system_committer, unstated_code,
};

/// New-form (Release-1.1.0) header names — the attribute path lives in the
/// value.
const H_VERSION: &str = "openehr-version";
const H_AUDIT_DETAILS: &str = "openehr-audit-details";

/// Deprecated (Release-1.0.3) header names — the attribute path is the name
/// suffix, the value is a bare `key="value"` list. Kept accepted per
/// §"Deprecated headers" (a MAY).
const H_DEP_LIFECYCLE: &str = "openEHR-VERSION.lifecycle_state";
/// The BARE deprecated header name from the §"Deprecated headers" table —
/// distinct from the new name after lowercasing (`openehr-audit_details` vs
/// `openehr-audit-details`), so it needs its own lookup. Value grammar is the
/// attribute-path-in-value form, like the new header.
const H_DEP_AUDIT_DETAILS_BARE: &str = "openEHR-AUDIT_DETAILS";
const H_DEP_CHANGE_TYPE: &str = "openEHR-AUDIT_DETAILS.change_type";
const H_DEP_DESCRIPTION: &str = "openEHR-AUDIT_DETAILS.description";
const H_DEP_COMMITTER: &str = "openEHR-AUDIT_DETAILS.committer";
const H_DEP_SYSTEM_ID: &str = "openEHR-AUDIT_DETAILS.system_id";

/// The five attribute targets the committal headers may set.
const T_LIFECYCLE: &str = "lifecycle_state";
const T_CHANGE_TYPE: &str = "change_type";
const T_DESCRIPTION: &str = "description";
const T_COMMITTER: &str = "committer";
const T_SYSTEM_ID: &str = "system_id";

/// Merge any present committal headers (development-edition `openehr-version` /
/// `openehr-audit-details`, or the deprecated dotted-name forms) into a
/// synthesized [`UpdateVersion`] commit envelope, overriding the server defaults
/// set by the caller. Absent headers leave the defaults intact; the new form
/// wins over the deprecated form on conflict.
///
/// # Errors
/// [`ApiError::BadRequest`] when a header carries a malformed identifier (see
/// [`build_committer`]).
pub(crate) fn merge_committal_headers<T>(
    uv: &mut UpdateVersion<T>,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    apply_attrs(
        &mut uv.lifecycle_state,
        &mut uv.commit_audit,
        &collect_attrs(headers)?,
    )
}

/// Overlay already-collected committal attributes onto the two halves of a
/// commit envelope — the merge itself, split from the header collection so a
/// caller that needs to know whether ANY attribute was supplied parses the
/// headers exactly once. It touches nothing but the VERSION
/// `lifecycle_state` and the `UPDATE_AUDIT` attributes, which is why it is
/// independent of the envelope's content type.
fn apply_attrs(
    lifecycle_state: &mut DvCodedText,
    commit_audit: &mut UpdateAudit,
    attrs: &IndexMap<String, Vec<(String, String)>>,
) -> Result<(), ApiError> {
    let audit = audit_base_mut(commit_audit);
    if let Some(code) = attrs.get(T_LIFECYCLE).and_then(|p| pair(p, "code_string")) {
        *lifecycle_state = lifecycle_state_coded(&code);
    }
    if let Some(code) = attrs
        .get(T_CHANGE_TYPE)
        .and_then(|p| pair(p, "code_string"))
    {
        *audit.change_type = change_type_coded(&code);
    }
    if let Some(desc) = attrs.get(T_DESCRIPTION).and_then(|p| scalar(p)) {
        // The header grammar carries the attribute pre-flattened as the
        // `description.value` subkey — the `DV_TEXT.value` string both
        // `DV_TEXT` and `DV_CODED_TEXT` share — so a header-borne description
        // is always a plain `DV_TEXT`: `DV_CODED_TEXT.defining_code` has no
        // subkey to travel in (ITS-REST overview `Requests_and_responses.md`
        // §"openehr-version and openehr-audit-details", the worked example).
        // A coded description IS expressible where the whole
        // `UPDATE_AUDIT` travels in the body.
        *audit.description = Some(plain_text(&desc));
    }
    if let Some(pairs) = attrs.get(T_COMMITTER)
        && let Some(committer) = build_committer(pairs)?
    {
        *audit.committer = committer;
    }
    if let Some(system_id) = attrs.get(T_SYSTEM_ID).and_then(|p| scalar(p)) {
        *audit.system_id = Some(system_id);
    }
    Ok(())
}

/// [`committal_commit`]'s audit half for a **DELETE** wire, which additionally REFUSES a
/// committal header that names a lifecycle state other than `523|deleted|`.
///
/// A `DELETE` on a change-controlled resource is the logical-deletion
/// procedure, and that procedure fixes the state: "create a new Version in the
/// normal way; delete its `_data_`; set the `_lifecycle_state_` value to the
/// code for `deleted`; commit in the normal way" (RM common
/// `master06-change_control_package.adoc` §Logical Deletion). A request that
/// says `openehr-version: lifecycle_state.code_string="532"` on a DELETE is
/// therefore asking for two contradictory things at once. The overview's merge
/// duty — "whatever is provided it MUST be merged with the default VERSION and
/// `VERSION.audit_details` attributes on commit runtime" (ITS-REST overview
/// `Requests_and_responses.md` §"openehr-version and openehr-audit-details")
/// — cannot be honoured for such a value, and silently discarding it is the
/// leniency this refusal removes: the client is told its instruction was not
/// followed rather than being led to believe it was. `400` is the shape class
/// the overview assigns to "syntactically invalid header, parameter or
/// content".
///
/// A DELETE with NO lifecycle attribute is unaffected — the header set is
/// optional ("None of these headers are mandatory") — as is one that states
/// the `523|deleted|` the operation already commits.
///
/// # Errors
/// [`ApiError::BadRequest`] when a header carries a malformed identifier, or a
/// lifecycle state other than `523|deleted|`.
pub(crate) fn committal_audit_for_delete(
    headers: &HeaderMap,
    committer: PartyProxy,
) -> Result<Option<UpdateAudit>, ApiError> {
    let Some(committal) = merged_committal(headers, Some(committer))? else {
        return Ok(None);
    };
    if let Some(code) = committal.lifecycle_state.as_deref()
        && code != DELETED_LIFECYCLE
    {
        return Err(ApiError::BadRequest(format!(
            "openehr-version lifecycle_state.code_string={code:?} contradicts DELETE — \
             a delete commits a {DELETED_LIFECYCLE}|deleted| version (RM common master06 \
             §Logical Deletion)"
        )));
    }
    Ok(Some(committal.audit))
}

/// The `version_lifecycle_state` code a logical deletion commits (RM common
/// master06 §Logical Deletion; openEHR terminology `version lifecycle state`).
const DELETED_LIFECYCLE: &str = "523";

/// The full committal metadata — merged `UPDATE_AUDIT` **and** the VERSION
/// `lifecycle_state` — of a request that commits a change-controlled resource
/// whose `UPDATE_VERSION` envelope never travels in the body: the bare EHR
/// creates (`POST /ehr`, `PUT /ehr/{ehr_id}`), whose only committal channel is
/// these headers (overview §"openehr-version and openehr-audit-details":
/// "services MUST accept `openehr-version` and `openehr-audit-details` custom
/// request headers" on the direct `PUT`/`POST`/`DELETE` commits, and
/// "whatever is provided it MUST be merged with the default VERSION and
/// `VERSION.audit_details` attributes on commit runtime").
///
/// `committer` is the server default the merge starts from — the request's
/// authenticated principal — so an unsupplied `committer` keeps it rather than
/// being clobbered. `None` when the request carried no committal header at
/// all, leaving the service on its own default attribution path.
///
/// # Errors
/// [`ApiError::BadRequest`] when a header carries a malformed identifier.
pub(crate) fn committal_commit(
    headers: &HeaderMap,
    committer: PartyProxy,
) -> Result<Option<Committal>, ApiError> {
    merged_committal(headers, Some(committer))
}

/// One header parse, both halves of the merge. `committer` seeds the audit's
/// server default; `None` leaves this server's own system identity as the
/// committer.
fn merged_committal(
    headers: &HeaderMap,
    committer: Option<PartyProxy>,
) -> Result<Option<Committal>, ApiError> {
    let attrs = collect_attrs(headers)?;
    if attrs.is_empty() {
        return Ok(None);
    }
    // A header-derived envelope states NO code until a header supplies one:
    // the two coded members start unstated so only a header-carried value
    // survives the merge — the service merges a NON-empty code verbatim
    // (after group + operation validation) and falls back to the operation's
    // default on empty (`versioning::audit`, `versioning::lifecycle`;
    // overview §"openehr-version and openehr-audit-details": "whatever is
    // provided it MUST be merged").
    let mut lifecycle_state = unstated_code();
    let mut commit_audit = UpdateAudit::UpdateAudit(UpdateAuditData {
        _type: None,
        system_id: None,
        change_type: unstated_code(),
        description: None,
        committer: committer.unwrap_or_else(system_committer),
    });
    apply_attrs(&mut lifecycle_state, &mut commit_audit, &attrs)?;
    Ok(Some(Committal {
        audit: commit_audit,
        lifecycle_state: Some(lifecycle_state.defining_code.code_string).filter(|c| !c.is_empty()),
    }))
}

/// Collect all committal-header attributes into `target → [(subkey, value)]`,
/// deprecated forms first and the development-edition forms last so the new form
/// wins on conflict.
///
/// # Errors
/// [`ApiError::BadRequest`] when a committal header carries an undecodable
/// value ([`header_values`]).
fn collect_attrs(headers: &HeaderMap) -> Result<IndexMap<String, Vec<(String, String)>>, ApiError> {
    // Deprecated forms: the attribute target is the header NAME suffix; the value
    // is a bare `key="value"` list (subkeys carry no `target.` prefix).
    let mut attrs: IndexMap<String, Vec<(String, String)>> = IndexMap::new();
    for (name, target) in [
        (H_DEP_LIFECYCLE, T_LIFECYCLE),
        (H_DEP_CHANGE_TYPE, T_CHANGE_TYPE),
        (H_DEP_DESCRIPTION, T_DESCRIPTION),
        (H_DEP_COMMITTER, T_COMMITTER),
        (H_DEP_SYSTEM_ID, T_SYSTEM_ID),
    ] {
        for raw in header_values(headers, name)? {
            attrs.insert(target.to_owned(), key_value_pairs(&raw));
        }
    }

    // The BARE deprecated name from the §"Deprecated headers" table:
    // `openEHR-AUDIT_DETAILS` (which lowercases to `openehr-audit_details`,
    // a different name than the Release-1.1.0 `openehr-audit-details`). The
    // table keeps it "available for backward compatibility"; it carries the
    // same attribute-path-in-value grammar as the new form. Parsed between
    // the dotted-suffix forms and the new form so precedence is
    // dotted < bare-deprecated < new. (`openEHR-VERSION` needs no entry —
    // it lowercases to the new `openehr-version` name and is read there.)
    let mut bare_dep: IndexMap<String, Vec<(String, String)>> = IndexMap::new();
    for raw in header_values(headers, H_DEP_AUDIT_DETAILS_BARE)? {
        collect_path_pairs(&raw, &mut bare_dep);
    }
    for (target, pairs) in bare_dep {
        attrs.insert(target, pairs);
    }

    // Development-edition forms: the attribute path is the leading segment of
    // each pair's key, inside the value. `openehr-version` carries VERSION
    // attributes (lifecycle_state); `openehr-audit-details` carries AUDIT_DETAILS
    // attributes (change_type/description/committer/system_id). Collected into a
    // separate map so a dev-form target REPLACES the deprecated one entirely (the
    // new form wins), rather than being appended to it.
    let mut dev: IndexMap<String, Vec<(String, String)>> = IndexMap::new();
    for name in [H_VERSION, H_AUDIT_DETAILS] {
        for raw in header_values(headers, name)? {
            collect_path_pairs(&raw, &mut dev);
        }
    }
    for (target, pairs) in dev {
        attrs.insert(target, pairs);
    }

    Ok(attrs)
}

/// Parse an attribute-path-in-value header (`change_type.code_string="251"`)
/// into `target → [(subkey, value)]` entries of `map`.
fn collect_path_pairs(raw: &str, map: &mut IndexMap<String, Vec<(String, String)>>) {
    for (full_key, value) in key_value_pairs(raw) {
        let (target, subkey) = match full_key.split_once('.') {
            Some((t, k)) => (t.to_owned(), k.to_owned()),
            // No dot ⇒ the whole key is a scalar target (e.g. `system_id`).
            None => (full_key.clone(), String::new()),
        };
        map.entry(target).or_default().push((subkey, value));
    }
}

/// Every value of a (possibly repeated) request header.
///
/// A value that is not decodable as text is a client defect the caller must
/// hear about: dropping it silently would commit a version whose audit
/// attributes differ from the ones the client sent, with nothing on the wire
/// saying so. (No openEHR spec governs undecodable header bytes — our own
/// design: refuse rather than guess.)
///
/// # Errors
/// [`ApiError::BadRequest`] naming the header whose value is not decodable.
fn header_values(headers: &HeaderMap, name: &str) -> Result<Vec<String>, ApiError> {
    headers
        .get_all(name)
        .iter()
        .map(|v| {
            v.to_str().map(str::to_owned).map_err(|e| {
                tracing::debug!(header = name, error = %e, "undecodable header value → 400");
                ApiError::BadRequest(format!(
                    "header {name} carries a value that is not decodable as text"
                ))
            })
        })
        .collect()
}

/// Build a `PARTY_IDENTIFIED` committer from the parsed `committer` header pairs,
/// or `None` when the value carries nothing usable (→ keep the server default).
///
/// # Errors
/// [`ApiError::BadRequest`] when `external_ref.id` is not a well-formed
/// `HIER_OBJECT_ID` (BASE `master05-identification_package.adoc` §Syntaxes).
fn build_committer(pairs: &[(String, String)]) -> Result<Option<PartyProxy>, ApiError> {
    let name = pair(pairs, "name");
    let ext_id = pair(pairs, "external_ref.id");
    if name.is_none() && ext_id.is_none() {
        return Ok(None);
    }
    // `external_ref.id` is client-supplied text, and `PARTY_REF.id` is an
    // `OBJECT_ID` — here a `HIER_OBJECT_ID`, whose lexical form BASE
    // `base_types/master05-identification_package.adoc` §Syntaxes defines. It
    // goes through the validating construction door, so a malformed value is
    // refused at the header rather than stamped into an AUDIT_DETAILS.
    let external_ref = match ext_id {
        Some(id) => Some(PartyRef {
            namespace: pair(pairs, "external_ref.namespace")
                .unwrap_or_else(|| "demographic".to_owned()),
            r#type: pair(pairs, "external_ref.type").unwrap_or_else(|| "PERSON".to_owned()),
            id: ObjectId::HierObjectId(HierObjectId::new(&id).map_err(|e| {
                ApiError::BadRequest(format!(
                    "openehr-audit-details committer external_ref.id {id:?} is not a \
                     well-formed HIER_OBJECT_ID: {e}"
                ))
            })?),
        }),
        None => None,
    };
    Ok(Some(PartyProxy::PartyIdentified(
        PartyIdentified::PartyIdentified(PartyIdentifiedData {
            external_ref,
            name,
            identifiers: None,
        }),
    )))
}

/// The value of the first `key` in a parsed pair list.
fn pair(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// The scalar value of a single-valued attribute: the `value` subkey (the
/// `description.value` / deprecated `value="…"` form) or, failing that, a bare
/// scalar subkey (the development-edition `system_id="…"` form).
fn scalar(pairs: &[(String, String)]) -> Option<String> {
    pair(pairs, "value").or_else(|| pair(pairs, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroehr::service::version_update::{AuditBase, audit_base, text_value};
    use http::{HeaderValue, header};
    use openehr_its::rest::generated::common::UpdateAudit;

    fn base_uv() -> UpdateVersion<serde_json::Value> {
        UpdateVersion {
            preceding_version_uid: None,
            signature: None,
            lifecycle_state: lifecycle_state_coded("532"),
            attestations: None,
            data: serde_json::json!({ "_type": "COMPOSITION" }),
            commit_audit: UpdateAudit::UpdateAudit(UpdateAuditData {
                _type: None,
                system_id: None,
                change_type: change_type_coded("249"),
                description: Some(plain_text("default")),
                committer: party("default"),
            }),
        }
    }

    /// The merged `UPDATE_AUDIT` attributes of an envelope, read through the
    /// shared base accessor (both concrete forms carry them).
    fn audit_of(uv: &UpdateVersion<serde_json::Value>) -> AuditBase<'_> {
        audit_base(&uv.commit_audit)
    }

    /// The audit half of the one merge ([`committal_commit`]) — the shape the
    /// audit-only assertions below are about.
    fn committal_audit_half(
        headers: &HeaderMap,
        committer: PartyProxy,
    ) -> Result<Option<UpdateAudit>, ApiError> {
        Ok(committal_commit(headers, committer)?.map(|c| c.audit))
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn parses_single_code_string_pair() {
        let pairs = key_value_pairs("code_string=\"532\"");
        assert_eq!(pairs, vec![("code_string".to_owned(), "532".to_owned())]);
    }

    #[test]
    fn parses_bare_value() {
        let pairs = key_value_pairs("code_string=532");
        assert_eq!(pair(&pairs, "code_string").as_deref(), Some("532"));
    }

    #[test]
    fn quoted_value_may_contain_commas() {
        let pairs = key_value_pairs("value=\"an updated, comma-bearing description\"");
        assert_eq!(
            pair(&pairs, "value").as_deref(),
            Some("an updated, comma-bearing description")
        );
    }

    // ── the BARE deprecated name (§"Deprecated headers" table) ──────────────

    /// The table row `openEHR-AUDIT_DETAILS` "remain[s] available for
    /// backward compatibility": the bare deprecated spelling carries the same
    /// attribute-path-in-value grammar and merges like the new name.
    #[test]
    fn bare_deprecated_audit_details_name_is_accepted() {
        let mut uv = base_uv();
        let h = headers(&[(
            H_DEP_AUDIT_DETAILS_BARE,
            "change_type.code_string=\"250\",description.value=\"legacy client\"",
        )]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(audit_of(&uv).change_type.defining_code.code_string, "250");
        assert_eq!(
            audit_of(&uv).description.map(text_value),
            Some("legacy client")
        );
    }

    /// Precedence: dotted 1.0.3 forms < bare deprecated name < the
    /// Release-1.1.0 name (the new form wins on conflict).
    #[test]
    fn new_form_wins_over_bare_deprecated() {
        let mut uv = base_uv();
        let h = headers(&[
            (H_DEP_AUDIT_DETAILS_BARE, "description.value=\"old\""),
            (H_AUDIT_DETAILS, "description.value=\"new\""),
        ]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(audit_of(&uv).description.map(text_value), Some("new"));

        let mut uv = base_uv();
        let h = headers(&[
            (H_DEP_CHANGE_TYPE, "value=\"252\""),
            (H_DEP_AUDIT_DETAILS_BARE, "change_type.code_string=\"250\""),
        ]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(
            audit_of(&uv).change_type.defining_code.code_string,
            "250",
            "bare deprecated wins over the dotted 1.0.3 form"
        );
    }

    /// The merged audit must not leak the `direct()` placeholder change type
    /// (`249|creation|`) as a client-supplied value: a header set without a
    /// `change_type` yields an EMPTY code, which the service resolves to the
    /// operation's default (`versioning::audit::merged_change_type`).
    #[test]
    fn merged_audit_blanks_the_placeholder_change_type() {
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let audit = committal_audit_half(
            &h,
            openehr_its::json::from_canonical_value(
                &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" }),
            )
            .expect("committer"),
        )
        .expect("well-formed committal headers")
        .expect("headers present");
        assert_eq!(audit_base(&audit).change_type.defining_code.code_string, "");
        assert_eq!(audit_base(&audit).description.map(text_value), Some("why"));

        let h = headers(&[(H_AUDIT_DETAILS, "change_type.code_string=\"250\"")]);
        let audit = committal_audit_half(
            &h,
            openehr_its::json::from_canonical_value(
                &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" }),
            )
            .expect("committer"),
        )
        .expect("well-formed committal headers")
        .expect("headers present");
        assert_eq!(
            audit_base(&audit).change_type.defining_code.code_string,
            "250"
        );

        assert!(
            committal_audit_half(
                &HeaderMap::new(),
                openehr_its::json::from_canonical_value(
                    &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" })
                )
                .expect("committer")
            )
            .expect("well-formed committal headers")
            .is_none()
        );
    }

    // ── development-edition form (the worked example, lines 85–91) ───────────

    #[test]
    fn merges_dev_edition_form() {
        let mut uv = base_uv();
        let h = headers(&[
            (H_VERSION, "lifecycle_state.code_string=\"523\""),
            (H_AUDIT_DETAILS, "change_type.code_string=\"251\""),
            (
                H_AUDIT_DETAILS,
                "description.value=\"An updated composition contribution description\"",
            ),
            (
                H_AUDIT_DETAILS,
                "committer.name=\"John Doe\",committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",committer.external_ref.namespace=\"demographic\",committer.external_ref.type=\"PERSON\"",
            ),
            (H_AUDIT_DETAILS, "system_id=\"example.openehr.systemid\""),
        ]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(uv.lifecycle_state.defining_code.code_string, "523");
        assert_eq!(audit_of(&uv).change_type.defining_code.code_string, "251");
        assert_eq!(
            audit_of(&uv).description.map(text_value),
            Some("An updated composition contribution description")
        );
        assert_eq!(audit_of(&uv).system_id, Some("example.openehr.systemid"));
        let committer = openehr_its::json::to_canonical_value(&audit_of(&uv).committer);
        assert_eq!(committer["_type"], "PARTY_IDENTIFIED");
        assert_eq!(committer["name"], "John Doe");
        assert_eq!(committer["external_ref"]["namespace"], "demographic");
        assert_eq!(committer["external_ref"]["type"], "PERSON");
        assert_eq!(
            committer["external_ref"]["id"]["value"],
            "BC8132EA-8F4A-11E7-BB31-BE2E44B06B34"
        );
    }

    #[test]
    fn dev_edition_committer_is_party_identified() {
        let mut uv = base_uv();
        let h = headers(&[(
            H_AUDIT_DETAILS,
            "committer.name=\"Jane\",committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\"",
        )]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert!(
            matches!(audit_of(&uv).committer, PartyProxy::PartyIdentified(_)),
            "expected PARTY_IDENTIFIED, got {:?}",
            audit_of(&uv).committer
        );
    }

    // ── deprecated Release-1.0.3 form still accepted (§Deprecated headers) ───

    #[test]
    fn merges_deprecated_dotted_name_form() {
        let mut uv = base_uv();
        let h = headers(&[
            (H_DEP_LIFECYCLE, "code_string=\"523\""),
            (H_DEP_CHANGE_TYPE, "code_string=\"251\""),
            (H_DEP_DESCRIPTION, "value=\"An updated composition\""),
            (H_DEP_SYSTEM_ID, "value=\"legacy.systemid\""),
        ]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(uv.lifecycle_state.defining_code.code_string, "523");
        assert_eq!(audit_of(&uv).change_type.defining_code.code_string, "251");
        assert_eq!(
            audit_of(&uv).description.map(text_value),
            Some("An updated composition")
        );
        assert_eq!(audit_of(&uv).system_id, Some("legacy.systemid"));
    }

    #[test]
    fn new_form_wins_on_conflict() {
        let mut uv = base_uv();
        let h = headers(&[
            // Deprecated says 249, new form says 251 → new form wins.
            (H_DEP_CHANGE_TYPE, "code_string=\"249\""),
            (H_AUDIT_DETAILS, "change_type.code_string=\"251\""),
        ]);
        merge_committal_headers(&mut uv, &h).expect("well-formed committal headers");
        assert_eq!(audit_of(&uv).change_type.defining_code.code_string, "251");
    }

    /// A header set supplying only `description` keeps the SEEDED principal
    /// as committer — the merge replaces "whatever is provided", never the
    /// default (overview §"openehr-version and openehr-audit-details").
    #[test]
    fn merged_audit_keeps_the_seeded_principal_committer() {
        let seed = openehr_its::json::from_canonical_value(&serde_json::json!({
            "_type": "PARTY_IDENTIFIED", "name": "dr-alice"
        }))
        .expect("committer");
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let audit = committal_audit_half(&h, seed)
            .expect("well-formed committal headers")
            .expect("headers present");
        let committer = openehr_its::json::to_canonical_value(audit_base(&audit).committer);
        assert_eq!(
            committer["name"], "dr-alice",
            "principal survives a partial header set"
        );

        // A header-supplied committer still wins over the seed.
        let seed = openehr_its::json::from_canonical_value(&serde_json::json!({
            "_type": "PARTY_IDENTIFIED", "name": "dr-alice"
        }))
        .expect("committer");
        let h = headers(&[(H_AUDIT_DETAILS, "committer.name=\"Locum\"")]);
        let audit = committal_audit_half(&h, seed)
            .expect("well-formed committal headers")
            .expect("headers present");
        let committer = openehr_its::json::to_canonical_value(audit_base(&audit).committer);
        assert_eq!(committer["name"], "Locum");
    }

    /// `committal_commit` carries BOTH halves of the merge from ONE parse: the
    /// `UPDATE_AUDIT` attributes and the VERSION `lifecycle_state` the
    /// `openehr-version` header supplied (the worked example's
    /// `lifecycle_state.code_string` line).
    #[test]
    fn committal_commit_carries_audit_and_lifecycle() {
        let h = headers(&[
            (H_VERSION, "lifecycle_state.code_string=\"553\""),
            (H_AUDIT_DETAILS, "description.value=\"why\""),
        ]);
        let committal = committal_commit(&h, party("principal"))
            .expect("well-formed committal headers")
            .expect("headers present");
        assert_eq!(committal.lifecycle_state.as_deref(), Some("553"));
        assert_eq!(
            audit_base(&committal.audit).description.map(text_value),
            Some("why")
        );
        // No client `change_type` ⇒ empty, so the service applies the
        // operation's default.
        assert_eq!(
            audit_base(&committal.audit)
                .change_type
                .defining_code
                .code_string,
            ""
        );
    }

    /// An unsupplied attribute keeps the SERVER default: the seeded committer
    /// survives a header set that names no `committer`, and an absent
    /// `openehr-version` leaves `lifecycle_state` unset (the service default
    /// `532|complete|` stands). "Whatever is provided it MUST be merged" —
    /// nothing more.
    #[test]
    fn committal_commit_keeps_unsupplied_defaults() {
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let committal = committal_commit(&h, party("principal"))
            .expect("well-formed committal headers")
            .expect("headers present");
        assert_eq!(committal.lifecycle_state, None);
        let committer =
            openehr_its::json::to_canonical_value(audit_base(&committal.audit).committer);
        assert_eq!(committer["name"], "principal");

        // A supplied committer overrides the seed.
        let h = headers(&[(H_AUDIT_DETAILS, "committer.name=\"Dr Chart\"")]);
        let committal = committal_commit(&h, party("principal"))
            .expect("well-formed committal headers")
            .expect("headers present");
        let committer =
            openehr_its::json::to_canonical_value(audit_base(&committal.audit).committer);
        assert_eq!(committer["name"], "Dr Chart");

        assert!(
            committal_commit(&HeaderMap::new(), party("principal"))
                .expect("well-formed committal headers")
                .is_none()
        );
    }

    fn party(name: &str) -> PartyProxy {
        openehr_its::json::from_canonical_value(
            &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": name }),
        )
        .unwrap()
    }

    #[test]
    fn absent_headers_leave_defaults() {
        let mut uv = base_uv();
        merge_committal_headers(&mut uv, &HeaderMap::new())
            .expect("an empty header map is well-formed");
        assert_eq!(uv.lifecycle_state.defining_code.code_string, "532");
        assert_eq!(audit_of(&uv).change_type.defining_code.code_string, "249");
        assert_eq!(audit_of(&uv).description.map(text_value), Some("default"));
        assert_eq!(audit_of(&uv).system_id, None);
    }
}
