//! `openehr-version` / `openehr-audit-details` committal request headers.
//!
//! ITS-REST overview §"openehr-version and openehr-audit-details"
//! (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
//! lines 72–96) makes it a **MUST** that a service accept these custom request
//! headers on the direct-commit change-controlled writes (COMPOSITION /
//! `EHR_STATUS` / directory FOLDER `POST`/`PUT`/`DELETE`) and merge whatever is
//! provided with the server's default VERSION + `commit_audit` attributes:
//!
//! > "None of these headers are mandatory, but whatever is provided it MUST be
//! >  merged with the default VERSION and VERSION.audit_details attributes on
//! >  commit runtime."
//!
//! The **development edition** moved each attribute path *into the header
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
//! `system_id` (development edition): a client MAY supply it here; when it is
//! absent "the server MUST set it to its own configured system identifier"
//! (line 94). The header layer only carries a client-supplied value into
//! [`UpdateAudit::system_id`]; the server default is asserted at the versioning
//! seam, not here.
//!
//! PORT NOTE (wire, spec-silent): the per-attribute value grammar is given only
//! by example — the spec states no formal ABNF for the `attr.key="value"` list,
//! its quoting, or escaping. We parse a tolerant comma-separated list of
//! `path="value"` (or bare `path=value`) pairs, treating a quoted value as
//! opaque (so a `description` value may itself contain commas). A header that
//! does not yield the attribute it targets is ignored (the server default
//! stands), never an error — the spec only says "merge whatever is provided".
//!
//! PORT NOTE (wire, spec-silent): the `committer` `external_ref.id` is wrapped
//! as a `HIER_OBJECT_ID` (the spec example is a UUID and gives no `OBJECT_ID`
//! subtype); if the assembled `PARTY_IDENTIFIED` fails to type, the committer is
//! left at the server default.

use http::HeaderMap;
use indexmap::IndexMap;
use serde_json::json;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::version_update::UpdateVersion;

/// New-form (development edition) header names — the attribute path lives in the
/// value.
const H_VERSION: &str = "openehr-version";
const H_AUDIT_DETAILS: &str = "openehr-audit-details";

/// Deprecated (Release-1.0.3) header names — the attribute path is the name
/// suffix, the value is a bare `key="value"` list. Kept accepted per
/// §"Deprecated headers" (a MAY).
const H_DEP_LIFECYCLE: &str = "openEHR-VERSION.lifecycle_state";
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
    let attrs = collect_attrs(headers);

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
            attrs.insert(target.to_owned(), parse_attr_pairs(&raw));
        }
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
            for (full_key, value) in parse_attr_pairs(&raw) {
                let (target, subkey) = match full_key.split_once('.') {
                    Some((t, k)) => (t.to_owned(), k.to_owned()),
                    // No dot ⇒ the whole key is a scalar target (e.g. `system_id`).
                    None => (full_key.clone(), String::new()),
                };
                dev.entry(target).or_default().push((subkey, value));
            }
        }
    }
    for (target, pairs) in dev {
        attrs.insert(target, pairs);
    }

    attrs
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
    let mut party = json!({ "_type": "PARTY_IDENTIFIED" });
    if let Some(name) = name {
        party["name"] = json!(name);
    }
    if let Some(id) = ext_id {
        party["external_ref"] = json!({
            "_type": "PARTY_REF",
            "namespace": pair(pairs, "external_ref.namespace").unwrap_or_else(|| "demographic".to_owned()),
            "type": pair(pairs, "external_ref.type").unwrap_or_else(|| "PERSON".to_owned()),
            "id": { "_type": "HIER_OBJECT_ID", "value": id },
        });
    }
    serde_json::from_value(party).ok()
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

/// Parse a tolerant comma-separated list of `key="value"` (or bare `key=value`)
/// attribute pairs. A double-quoted value is read opaquely (may contain commas);
/// a bare value runs to the next top-level comma. Whitespace around separators
/// and keys is trimmed. See the module PORT NOTE — the grammar is example-only.
fn parse_attr_pairs(input: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip leading separators/whitespace.
        while i < bytes.len() && (bytes[i] == b',' || bytes[i].is_ascii_whitespace()) {
            i += 1;
        }
        // Read the key up to '='.
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b',' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // No '=' — not a pair; skip to the next comma.
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            continue;
        }
        let key = input[key_start..i].trim().to_owned();
        i += 1; // consume '='
        // Read the value: quoted (opaque) or bare (to next comma).
        let value = if i < bytes.len() && bytes[i] == b'"' {
            i += 1; // consume opening quote
            let val_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let v = input[val_start..i].to_owned();
            if i < bytes.len() {
                i += 1; // consume closing quote
            }
            v
        } else {
            let val_start = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            input[val_start..i].trim().to_owned()
        };
        if !key.is_empty() {
            out.push((key, value));
        }
    }
    out
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
                committer: serde_json::from_value(
                    serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "default" }),
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
        let pairs = parse_attr_pairs("code_string=\"532\"");
        assert_eq!(pairs, vec![("code_string".to_owned(), "532".to_owned())]);
    }

    #[test]
    fn parses_bare_value() {
        let pairs = parse_attr_pairs("code_string=532");
        assert_eq!(pair(&pairs, "code_string").as_deref(), Some("532"));
    }

    #[test]
    fn quoted_value_may_contain_commas() {
        let pairs = parse_attr_pairs("value=\"an updated, comma-bearing description\"");
        assert_eq!(
            pair(&pairs, "value").as_deref(),
            Some("an updated, comma-bearing description")
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
        let committer = serde_json::to_value(&uv.audit.committer).unwrap();
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
