//! `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal request headers.
//!
//! ITS-REST 1.0.3 (overview §"openEHR-VERSION and openEHR-AUDIT_DETAILS",
//! `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
//! lines 47–82) makes it a **MUST** that a service accept these custom request
//! headers on the direct-commit change-controlled writes (COMPOSITION /
//! `EHR_STATUS` / directory FOLDER `POST`/`PUT`/`DELETE`) and merge whatever is
//! provided with the server's default VERSION + `commit_audit` attributes:
//!
//! > "None of these headers are mandatory, but whatever is provided it MUST be
//! >  merged with the default VERSION and VERSION.audit_details attributes on
//! >  commit runtime."
//!
//! The four headers the spec defines (by worked example, lines 60–65):
//!
//! | Header | Value form | RM target |
//! |---|---|---|
//! | `openEHR-VERSION.lifecycle_state` | `code_string="532"` | `VERSION.lifecycle_state` |
//! | `openEHR-AUDIT_DETAILS.change_type` | `code_string="251"` | `commit_audit.change_type` |
//! | `openEHR-AUDIT_DETAILS.description` | `value="…"` | `commit_audit.description` |
//! | `openEHR-AUDIT_DETAILS.committer` | `name="…", external_ref.id="…", external_ref.namespace="…", external_ref.type="…"` | `commit_audit.committer` |
//!
//! PORT NOTE (wire, spec-silent): the per-attribute value grammar is given only
//! by example — the spec states no formal ABNF for the `key="value"` list, its
//! quoting, or escaping. We parse a tolerant comma-separated list of
//! `key="value"` (or bare `key=value`) pairs, treating a quoted value as opaque
//! (so a `description` value may itself contain commas). A header that does not
//! yield the attribute it targets is ignored (the server default stands), never
//! an error — the spec only says "merge whatever is provided".
//!
//! PORT NOTE (wire, spec-silent): the `openEHR-AUDIT_DETAILS.committer`
//! `external_ref.id` is wrapped as a `HIER_OBJECT_ID` (the spec example is a
//! UUID and gives no `OBJECT_ID` subtype); if the assembled `PARTY_IDENTIFIED`
//! fails to type, the committer is left at the server default.

use http::HeaderMap;
use serde_json::json;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase_sm::types::UpdateVersion;

const H_LIFECYCLE: &str = "openEHR-VERSION.lifecycle_state";
const H_CHANGE_TYPE: &str = "openEHR-AUDIT_DETAILS.change_type";
const H_DESCRIPTION: &str = "openEHR-AUDIT_DETAILS.description";
const H_COMMITTER: &str = "openEHR-AUDIT_DETAILS.committer";

/// An `openehr`-terminology coded value from a numeric `code_string`.
fn openehr_code(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// Merge any present `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal
/// headers into a synthesized [`UpdateVersion`] commit envelope, overriding the
/// server defaults set by the caller. Absent headers leave the defaults intact.
pub(crate) fn merge_committal_headers(uv: &mut UpdateVersion, headers: &HeaderMap) {
    if let Some(code) = header_pairs(headers, H_LIFECYCLE).and_then(|p| pair(&p, "code_string")) {
        uv.lifecycle_state = openehr_code(&code);
    }
    if let Some(code) = header_pairs(headers, H_CHANGE_TYPE).and_then(|p| pair(&p, "code_string")) {
        uv.audit.change_type = openehr_code(&code);
    }
    if let Some(desc) = header_pairs(headers, H_DESCRIPTION).and_then(|p| pair(&p, "value")) {
        uv.audit.description = Some(desc);
    }
    if let Some(pairs) = header_pairs(headers, H_COMMITTER)
        && let Some(committer) = build_committer(&pairs)
    {
        uv.audit.committer = committer;
    }
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

/// The parsed `key="value"` pairs of a request header, if present and decodable.
fn header_pairs(headers: &HeaderMap, name: &str) -> Option<Vec<(String, String)>> {
    let raw = headers.get(name)?.to_str().ok()?;
    Some(parse_attr_pairs(raw))
}

/// The value of the first `key` in a parsed pair list.
fn pair(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
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
    use ehrbase_sm::types::UpdateAudit;
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
            },
            signature: None,
        }
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
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

    #[test]
    fn parses_committer_multi_pair() {
        let pairs = parse_attr_pairs(
            "name=\"John Doe\", external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\", \
             external_ref.namespace=\"demographic\", external_ref.type=\"PERSON\"",
        );
        assert_eq!(pair(&pairs, "name").as_deref(), Some("John Doe"));
        assert_eq!(
            pair(&pairs, "external_ref.id").as_deref(),
            Some("BC8132EA-8F4A-11E7-BB31-BE2E44B06B34")
        );
        assert_eq!(pair(&pairs, "external_ref.type").as_deref(), Some("PERSON"));
    }

    #[test]
    fn merges_lifecycle_change_type_and_description() {
        let mut uv = base_uv();
        let h = headers(&[
            (H_LIFECYCLE, "code_string=\"523\""),
            (H_CHANGE_TYPE, "code_string=\"251\""),
            (H_DESCRIPTION, "value=\"An updated composition\""),
        ]);
        merge_committal_headers(&mut uv, &h);
        assert_eq!(uv.lifecycle_state.code_string, "523");
        assert_eq!(uv.audit.change_type.code_string, "251");
        assert_eq!(
            uv.audit.description.as_deref(),
            Some("An updated composition")
        );
    }

    #[test]
    fn merges_committer_party_identified() {
        let mut uv = base_uv();
        let h = headers(&[(
            H_COMMITTER,
            "name=\"John Doe\", external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\", \
             external_ref.namespace=\"demographic\", external_ref.type=\"PERSON\"",
        )]);
        merge_committal_headers(&mut uv, &h);
        assert!(
            matches!(uv.audit.committer, PartyProxy::PartyIdentified(_)),
            "expected PARTY_IDENTIFIED, got {:?}",
            uv.audit.committer
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
    fn absent_headers_leave_defaults() {
        let mut uv = base_uv();
        merge_committal_headers(&mut uv, &HeaderMap::new());
        assert_eq!(uv.lifecycle_state.code_string, "532");
        assert_eq!(uv.audit.change_type.code_string, "249");
        assert_eq!(uv.audit.description.as_deref(), Some("default"));
    }
}
