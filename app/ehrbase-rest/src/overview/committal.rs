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

use http::HeaderMap;
use indexmap::IndexMap;
use serde_json::json;

use super::params::key_value_pairs;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::version_update::UpdateVersion;

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

/// An `openehr`-terminology coded value from a numeric `code_string`.
fn openehr_code(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// Merge any present committal headers (development-edition `openehr-version` /
/// `openehr-audit-details`, or the deprecated dotted-name forms) into a
/// synthesized [`UpdateVersion`] commit envelope, overriding the server defaults
/// set by the caller. Absent headers leave the defaults intact; the new form
/// wins over the deprecated form on conflict.
pub(crate) fn merge_committal_headers(uv: &mut UpdateVersion, headers: &HeaderMap) {
    apply_attrs(uv, &collect_attrs(headers));
}

/// Overlay already-collected committal attributes onto a commit envelope —
/// the merge itself, split from the header collection so a caller that needs
/// to know whether ANY attribute was supplied parses the headers exactly once.
fn apply_attrs(uv: &mut UpdateVersion, attrs: &IndexMap<String, Vec<(String, String)>>) {
    if let Some(code) = attrs.get(T_LIFECYCLE).and_then(|p| pair(p, "code_string")) {
        uv.lifecycle_state = openehr_code(&code);
    }
    if let Some(code) = attrs
        .get(T_CHANGE_TYPE)
        .and_then(|p| pair(p, "code_string"))
    {
        uv.audit.change_type = openehr_code(&code);
    }
    if let Some(desc) = attrs.get(T_DESCRIPTION).and_then(|p| scalar(p)) {
        uv.audit.description = Some(desc);
    }
    if let Some(pairs) = attrs.get(T_COMMITTER)
        && let Some(committer) = build_committer(pairs)
    {
        uv.audit.committer = committer;
    }
    if let Some(system_id) = attrs.get(T_SYSTEM_ID).and_then(|p| scalar(p)) {
        uv.audit.system_id = Some(system_id);
    }
}

/// The audit attributes of the committal headers, when the request carried
/// any — the demographic AND the EHR-group delete wires thread these into
/// their commits (the ITS-REST overview merge requirement applies to every
/// commit surface: §"openehr-version and openehr-audit-details" requires the
/// headers accepted on `PUT`, `POST` **and** `DELETE`). `None` when no
/// committal header is present, so a plain request keeps the server-default
/// attribution path.
/// Seeded with the request's authenticated committer (the same seed
/// `committal_commit` takes), so a header set that supplies only
/// `description` keeps the PRINCIPAL as committer — the merge only replaces
/// "whatever is provided" (overview §"openehr-version and
/// openehr-audit-details"); the `UpdateVersion::direct` system placeholder
/// is never a default the client chose.
pub(crate) fn committal_audit(
    headers: &HeaderMap,
    committer: PartyProxy,
) -> Option<ehrbase::service::version_update::UpdateAudit> {
    merged_committal(headers, Some(committer)).map(|c| c.audit)
}

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
pub(crate) fn committal_commit(
    headers: &HeaderMap,
    committer: PartyProxy,
) -> Option<ehrbase::service::version_update::Committal> {
    merged_committal(headers, Some(committer))
}

/// One header parse, both halves of the merge. `committer` seeds the audit's
/// server default; `None` keeps [`UpdateVersion::direct`]'s system
/// placeholder (the historical [`committal_audit`] behaviour).
fn merged_committal(
    headers: &HeaderMap,
    committer: Option<PartyProxy>,
) -> Option<ehrbase::service::version_update::Committal> {
    let attrs = collect_attrs(headers);
    if attrs.is_empty() {
        return None;
    }
    let mut uv = UpdateVersion::direct(serde_json::Value::Null);
    if let Some(committer) = committer {
        uv.audit.committer = committer;
    }
    // The direct() placeholder change type (`249|creation|`) and lifecycle
    // state (`532|complete|`) must not read as client-supplied values: blank
    // them so only a header-carried code survives the merge — the service
    // merges a NON-empty code verbatim (after group + operation validation)
    // and falls back to the operation's default on empty
    // (`versioning::audit`, `versioning::lifecycle`; overview
    // §"openehr-version and openehr-audit-details": "whatever is provided it
    // MUST be merged").
    uv.audit.change_type.code_string = String::new();
    uv.lifecycle_state.code_string = String::new();
    apply_attrs(&mut uv, &attrs);
    let lifecycle_state = Some(uv.lifecycle_state.code_string).filter(|c| !c.is_empty());
    Some(ehrbase::service::version_update::Committal {
        audit: uv.audit,
        lifecycle_state,
    })
}

/// Collect all committal-header attributes into `target → [(subkey, value)]`,
/// deprecated forms first and the development-edition forms last so the new form
/// wins on conflict.
fn collect_attrs(headers: &HeaderMap) -> IndexMap<String, Vec<(String, String)>> {
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
        for raw in header_values(headers, name) {
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
    for raw in header_values(headers, H_DEP_AUDIT_DETAILS_BARE) {
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
        for raw in header_values(headers, name) {
            collect_path_pairs(&raw, &mut dev);
        }
    }
    for (target, pairs) in dev {
        attrs.insert(target, pairs);
    }

    attrs
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

/// All decodable values of a (possibly repeated) request header.
fn header_values(headers: &HeaderMap, name: &str) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_owned))
        .collect()
}

/// Build a `PARTY_IDENTIFIED` committer from the parsed `committer` header pairs,
/// or `None` when the value carries nothing usable (→ keep the server default).
fn build_committer(pairs: &[(String, String)]) -> Option<PartyProxy> {
    let name = pair(pairs, "name");
    let ext_id = pair(pairs, "external_ref.id");
    if name.is_none() && ext_id.is_none() {
        return None;
    }
    let mut party = serde_json::Map::new();
    party.insert("_type".to_owned(), json!("PARTY_IDENTIFIED"));
    if let Some(name) = name {
        party.insert("name".to_owned(), json!(name));
    }
    if let Some(id) = ext_id {
        party.insert(
            "external_ref".to_owned(),
            json!({
                "_type": "PARTY_REF",
                "namespace": pair(pairs, "external_ref.namespace").unwrap_or_else(|| "demographic".to_owned()),
                "type": pair(pairs, "external_ref.type").unwrap_or_else(|| "PERSON".to_owned()),
                "id": { "_type": "HIER_OBJECT_ID", "value": id },
            }),
        );
    }
    openehr_its::json::from_canonical_value(&serde_json::Value::Object(party)).ok()
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
    use ehrbase::service::version_update::UpdateAudit;
    use http::{HeaderValue, header};

    fn base_uv() -> UpdateVersion {
        UpdateVersion {
            preceding_version_uid: None,
            lifecycle_state: openehr_code("532"),
            attestations: None,
            data: serde_json::json!({ "_type": "COMPOSITION" }),
            audit: UpdateAudit {
                change_type: openehr_code("249"),
                description: Some("default".to_owned()),
                committer: openehr_its::json::from_canonical_value(
                    &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "default" }),
                )
                .unwrap(),
                system_id: None,
            },
            signature: None,
        }
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
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.audit.change_type.code_string, "250");
        assert_eq!(uv.audit.description.as_deref(), Some("legacy client"));
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
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.audit.description.as_deref(), Some("new"));

        let mut uv = base_uv();
        let h = headers(&[
            (H_DEP_CHANGE_TYPE, "value=\"252\""),
            (H_DEP_AUDIT_DETAILS_BARE, "change_type.code_string=\"250\""),
        ]);
        merge_committal_headers(&mut uv, &h);
        assert_eq!(
            uv.audit.change_type.code_string, "250",
            "bare deprecated wins over the dotted 1.0.3 form"
        );
    }

    /// `committal_audit` must not leak the `direct()` placeholder change type
    /// (`249|creation|`) as a client-supplied value: a header set without a
    /// `change_type` yields an EMPTY code, which the service resolves to the
    /// operation's default (`versioning::audit::merged_change_type`).
    #[test]
    fn committal_audit_blanks_the_placeholder_change_type() {
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let audit = committal_audit(
            &h,
            openehr_its::json::from_canonical_value(
                &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" }),
            )
            .expect("committer"),
        )
        .expect("headers present");
        assert_eq!(audit.change_type.code_string, "");
        assert_eq!(audit.description.as_deref(), Some("why"));

        let h = headers(&[(H_AUDIT_DETAILS, "change_type.code_string=\"250\"")]);
        let audit = committal_audit(
            &h,
            openehr_its::json::from_canonical_value(
                &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" }),
            )
            .expect("committer"),
        )
        .expect("headers present");
        assert_eq!(audit.change_type.code_string, "250");

        assert!(
            committal_audit(
                &HeaderMap::new(),
                openehr_its::json::from_canonical_value(
                    &serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "principal" })
                )
                .expect("committer")
            )
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
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.lifecycle_state.code_string, "523");
        assert_eq!(uv.audit.change_type.code_string, "251");
        assert_eq!(
            uv.audit.description.as_deref(),
            Some("An updated composition contribution description")
        );
        assert_eq!(
            uv.audit.system_id.as_deref(),
            Some("example.openehr.systemid")
        );
        let committer = openehr_its::json::to_canonical_value(&uv.audit.committer);
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
        merge_committal_headers(&mut uv, &h);
        assert!(
            matches!(uv.audit.committer, PartyProxy::PartyIdentified(_)),
            "expected PARTY_IDENTIFIED, got {:?}",
            uv.audit.committer
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
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.lifecycle_state.code_string, "523");
        assert_eq!(uv.audit.change_type.code_string, "251");
        assert_eq!(
            uv.audit.description.as_deref(),
            Some("An updated composition")
        );
        assert_eq!(uv.audit.system_id.as_deref(), Some("legacy.systemid"));
    }

    #[test]
    fn new_form_wins_on_conflict() {
        let mut uv = base_uv();
        let h = headers(&[
            // Deprecated says 249, new form says 251 → new form wins.
            (H_DEP_CHANGE_TYPE, "code_string=\"249\""),
            (H_AUDIT_DETAILS, "change_type.code_string=\"251\""),
        ]);
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.audit.change_type.code_string, "251");
    }

    /// A header set supplying only `description` keeps the SEEDED principal
    /// as committer — the merge replaces "whatever is provided", never the
    /// default (overview §"openehr-version and openehr-audit-details").
    #[test]
    fn committal_audit_keeps_the_seeded_principal_committer() {
        let seed = openehr_its::json::from_canonical_value(&serde_json::json!({
            "_type": "PARTY_IDENTIFIED", "name": "dr-alice"
        }))
        .expect("committer");
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let audit = committal_audit(&h, seed).expect("headers present");
        let committer = openehr_its::json::to_canonical_value(&audit.committer);
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
        let audit = committal_audit(&h, seed).expect("headers present");
        let committer = openehr_its::json::to_canonical_value(&audit.committer);
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
        let committal = committal_commit(&h, party("principal")).expect("headers present");
        assert_eq!(committal.lifecycle_state.as_deref(), Some("553"));
        assert_eq!(committal.audit.description.as_deref(), Some("why"));
        // No client `change_type` ⇒ empty, so the service applies the
        // operation's default.
        assert_eq!(committal.audit.change_type.code_string, "");
    }

    /// An unsupplied attribute keeps the SERVER default: the seeded committer
    /// survives a header set that names no `committer`, and an absent
    /// `openehr-version` leaves `lifecycle_state` unset (the service default
    /// `532|complete|` stands). "Whatever is provided it MUST be merged" —
    /// nothing more.
    #[test]
    fn committal_commit_keeps_unsupplied_defaults() {
        let h = headers(&[(H_AUDIT_DETAILS, "description.value=\"why\"")]);
        let committal = committal_commit(&h, party("principal")).expect("headers present");
        assert_eq!(committal.lifecycle_state, None);
        let committer = openehr_its::json::to_canonical_value(&committal.audit.committer);
        assert_eq!(committer["name"], "principal");

        // A supplied committer overrides the seed.
        let h = headers(&[(H_AUDIT_DETAILS, "committer.name=\"Dr Chart\"")]);
        let committal = committal_commit(&h, party("principal")).expect("headers present");
        let committer = openehr_its::json::to_canonical_value(&committal.audit.committer);
        assert_eq!(committer["name"], "Dr Chart");

        assert!(committal_commit(&HeaderMap::new(), party("principal")).is_none());
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
        merge_committal_headers(&mut uv, &HeaderMap::new());
        assert_eq!(uv.lifecycle_state.code_string, "532");
        assert_eq!(uv.audit.change_type.code_string, "249");
        assert_eq!(uv.audit.description.as_deref(), Some("default"));
        assert_eq!(uv.audit.system_id, None);
    }
}
