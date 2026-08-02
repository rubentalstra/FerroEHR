#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test assertions/diagnostics/fixtures"
)]
//! Canonical JSON and canonical XML get the same conformance answer.
//!
//! ITS-REST overview `Resources.md` §"Data representation": "Services MUST
//! support at least one of the openEHR **XML** or **JSON** canonical
//! formats" — the FORMAT is negotiable, the semantics are not. The #1431
//! escape presented exactly the forbidden split (the same invariant-violating
//! COMPOSITION was refused over XML and committed over JSON), so this suite
//! pins the property at the validation seam: for every instance expressible
//! in both canonical formats, the RM-invariant + terminology verdict set is
//! IDENTICAL whether the instance arrived as raw canonical JSON or through
//! the canonical-XML round-trip (typed decode → `ToXml` → `FromXml` → the
//! canonical value the XML wire arm validates).

use openehr_its::rm_instance::validate_rm_and_terminology;
use openehr_rm::composition::composition::Composition;
use serde_json::Value;

use crate::common::corpus_files;

/// The rendered verdict set for direct canonical-JSON arrival.
fn json_route(doc: &Value) -> Vec<String> {
    render(validate_rm_and_terminology(doc))
}

/// The rendered verdict set for canonical-XML arrival: the document is decoded
/// typed, serialized to canonical XML, parsed back, and re-rendered as the
/// canonical value the XML wire arm hands to validation. `None` when the
/// instance is not expressible in XML at all (it does not deserialize into the
/// typed `COMPOSITION` — e.g. a structurally invalid fixture whose defect IS
/// the missing mandatory attribute), which the caller records rather than
/// skips silently.
fn xml_route(doc: &Value) -> Option<Vec<String>> {
    let typed: Composition = openehr_its::json::from_canonical_value(doc).ok()?;
    let xml = openehr_its::xml::to_canonical_xml(&typed, "composition")
        .expect("gate-proven ToXml serializes a decoded COMPOSITION");
    let back: Composition = openehr_its::xml::from_canonical_xml(&xml)
        .expect("gate-proven FromXml parses its own ToXml output");
    let value = openehr_its::json::to_canonical_value(&back);
    Some(render(validate_rm_and_terminology(&value)))
}

fn render(msgs: Vec<openehr_its::rm_instance::ValidationMessage>) -> Vec<String> {
    let mut out: Vec<String> = msgs
        .into_iter()
        .map(|m| format!("{}|{:?}|{}", m.path, m.kind, m.message))
        .collect();
    out.sort();
    out
}

/// **Parity over the valid corpus:** every corpus COMPOSITION yields the
/// identical (empty or not) verdict set on both routes.
#[test]
fn corpus_verdicts_agree_across_canonical_formats() {
    let mut checked = 0usize;
    let mut inexpressible = Vec::new();
    for path in corpus_files() {
        let text = std::fs::read_to_string(&path).expect("read corpus file");
        let Ok(doc) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if doc.get("_type").and_then(Value::as_str) != Some("COMPOSITION") {
            continue;
        }
        let Some(via_xml) = xml_route(&doc) else {
            // The vendored SDK corpus deliberately carries a few structurally
            // defective documents (a missing mandatory attribute cannot exist
            // in XML — the typed decode is the XML arm's first step). Parity
            // for those means the JSON route REFUSES them too.
            assert!(
                !json_route(&doc).is_empty(),
                "typed-undecodable corpus document produced no violation on \
                 the JSON route: {}",
                path.display()
            );
            inexpressible.push(path.display().to_string());
            continue;
        };
        assert_eq!(
            json_route(&doc),
            via_xml,
            "JSON and XML routes disagree for {}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 20, "expected a real corpus, checked {checked}");
    // The known structurally-defective vendor entries only — growth here means
    // a decode regression, shrinkage means a fixture started decoding and the
    // list needs re-adjudication.
    assert_eq!(
        inexpressible.len(),
        3,
        "unexpected typed-undecodable corpus set: {inexpressible:?}"
    );
}

/// **Parity over the invalid CNF fixtures:** every composition fixture the
/// catalogue adjudicates `invalid` that is still typed-decodable (its defect
/// is semantic, not structural) yields the identical NON-EMPTY verdict set on
/// both routes — the refusal happens in both formats, for the same reasons.
/// Structurally undecodable fixtures are counted (their defect cannot exist
/// in XML: the typed decode is the XML arm's first step), never silently
/// skipped.
#[test]
fn invalid_fixture_verdicts_agree_across_canonical_formats() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/cnf-runner/artifacts/corpus/fixtures/composition");
    let mut agree = 0usize;
    let mut structural = 0usize;
    for entry in std::fs::read_dir(&dir).expect("catalogue fixture dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json")
            || !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.contains("invalid")
                    || n.contains("location_empty")
                    || n.contains("cluster_no_items")
            })
        {
            continue;
        }
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
                .expect("fixture parses as JSON");
        let via_json = json_route(&doc);
        match xml_route(&doc) {
            Some(via_xml) => {
                assert_eq!(
                    via_json,
                    via_xml,
                    "JSON and XML routes disagree for {}",
                    path.display()
                );
                assert!(
                    !via_json.is_empty(),
                    "an invalid-adjudicated fixture produced no violation on \
                     either route: {}",
                    path.display()
                );
                agree += 1;
            }
            None => structural += 1,
        }
    }
    assert!(
        agree >= 4,
        "expected the semantic invalid fixtures (territory/language/category/\
         setting/mode/location), saw {agree} agreeing + {structural} structural"
    );
}
