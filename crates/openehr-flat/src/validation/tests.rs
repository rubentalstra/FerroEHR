//! Per-rule unit tests for the composition validator, built on hand-shaped
//! `WebTemplate` nodes + minimal instances (no OPT parsing) so each rule is
//! exercised in isolation. End-to-end corpus tests live in `tests/validation.rs`.

use indexmap::IndexMap;
use serde_json::{Value, json};

use super::*;
use crate::webtemplate::{
    WebTemplate, WebTemplateCardinality, WebTemplateCodedValue, WebTemplateInput,
    WebTemplateInputType, WebTemplateNode, WebTemplateRange, WebTemplateValidation,
};

fn node(rm: &str, path: &str) -> WebTemplateNode {
    WebTemplateNode::new(rm.to_owned(), path.to_owned())
}

fn wt_of(tree: WebTemplateNode) -> WebTemplate {
    WebTemplate {
        template_id: "t".to_owned(),
        sem_ver: None,
        version: "2.3".to_owned(),
        default_language: "en".to_owned(),
        languages: vec!["en".to_owned()],
        tree,
        other_details: IndexMap::new(),
    }
}

/// Run only the `WebTemplate` (archetype-conformance) pass for a matched root.
fn walk_only(instance: &Value, root: &WebTemplateNode) -> Vec<ValidationMessage> {
    let mut v = Validator::default();
    v.walk(instance, root);
    v.out
}

fn kinds(msgs: &[ValidationMessage]) -> Vec<ValidationKind> {
    msgs.iter().map(|m| m.kind).collect()
}

// ── occurrences ────────────────────────────────────────────────────────────────

#[test]
fn occurrences_too_few() {
    let mut root = node("COMPOSITION", "");
    let mut sec = node("SECTION", "/content[at0001]");
    sec.min = Some(2);
    sec.max = 5;
    root.children = vec![sec];

    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [{"_type": "SECTION", "archetype_node_id": "at0001",
                     "name": {"_type": "DV_TEXT", "value": "s"}}]
    });
    let msgs = walk_only(&inst, &root);
    assert!(
        kinds(&msgs).contains(&ValidationKind::Occurrences),
        "expected Occurrences (too few), got {msgs:?}"
    );
}

#[test]
fn occurrences_too_many() {
    let mut root = node("COMPOSITION", "");
    let mut sec = node("SECTION", "/content[at0001]");
    sec.min = Some(0);
    sec.max = 1;
    root.children = vec![sec];

    let one = json!({"_type": "SECTION", "archetype_node_id": "at0001",
                     "name": {"_type": "DV_TEXT", "value": "s"}});
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [one.clone(), one.clone(), one]
    });
    let msgs = walk_only(&inst, &root);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Occurrences && m.message.contains("too many")),
        "expected Occurrences (too many), got {msgs:?}"
    );
}

#[test]
fn occurrences_required_missing() {
    let mut root = node("COMPOSITION", "");
    let mut sec = node("SECTION", "/content[at0001]");
    sec.min = Some(1);
    sec.max = 1;
    root.children = vec![sec];

    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
    let msgs = walk_only(&inst, &root);
    assert!(
        kinds(&msgs).contains(&ValidationKind::Required),
        "expected Required, got {msgs:?}"
    );
}

// ── cardinality ─────────────────────────────────────────────────────────────────

#[test]
fn cardinality_violation() {
    let mut root = node("COMPOSITION", "");
    root.cardinalities = vec![WebTemplateCardinality {
        min: Some(1),
        max: 2,
        ids: None,
        path: "/content".to_owned(),
    }];
    let entry = json!({"_type": "OBSERVATION", "archetype_node_id": "a"});
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [entry.clone(), entry.clone(), entry]
    });
    let msgs = walk_only(&inst, &root);
    assert!(
        kinds(&msgs).contains(&ValidationKind::Cardinality),
        "expected Cardinality (>max), got {msgs:?}"
    );
}

// ── numeric range ────────────────────────────────────────────────────────────────

fn count_node_range(min: i64, max: i64) -> WebTemplateNode {
    let mut n = node("DV_COUNT", "/count");
    let mut input = WebTemplateInput::new(WebTemplateInputType::Integer, None);
    input.validation = Some(WebTemplateValidation {
        range: Some(WebTemplateRange {
            min_op: Some(">=".to_owned()),
            min: Some(Value::from(min)),
            max_op: Some("<=".to_owned()),
            max: Some(Value::from(max)),
        }),
        ..Default::default()
    });
    n.inputs = vec![input];
    n
}

#[test]
fn range_error_out_of_bounds() {
    let n = count_node_range(0, 10);
    let inst = json!({"_type": "DV_COUNT", "magnitude": 42});
    let msgs = walk_only(&inst, &n);
    assert_eq!(kinds(&msgs), vec![ValidationKind::RangeError], "{msgs:?}");
}

#[test]
fn range_ok_in_bounds() {
    let n = count_node_range(0, 10);
    let inst = json!({"_type": "DV_COUNT", "magnitude": 5});
    assert!(walk_only(&inst, &n).is_empty());
}

// ── string pattern ───────────────────────────────────────────────────────────────

#[test]
fn pattern_error() {
    let mut n = node("DV_TEXT", "/text");
    let mut input = WebTemplateInput::new(WebTemplateInputType::Text, None);
    input.validation = Some(WebTemplateValidation {
        pattern: Some("[A-Z]+".to_owned()),
        ..Default::default()
    });
    n.inputs = vec![input];

    let bad = json!({"_type": "DV_TEXT", "value": "abc"});
    assert_eq!(
        kinds(&walk_only(&bad, &n)),
        vec![ValidationKind::PatternError]
    );
    let good = json!({"_type": "DV_TEXT", "value": "ABC"});
    assert!(walk_only(&good, &n).is_empty());
}

// ── coded value membership ───────────────────────────────────────────────────────

fn coded_node(codes: &[&str]) -> WebTemplateNode {
    let mut n = node("DV_CODED_TEXT", "/coded");
    let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, Some("code"));
    input.list = codes
        .iter()
        .map(|c| WebTemplateCodedValue::new(*c, Some((*c).to_owned())))
        .collect();
    n.inputs = vec![input];
    n
}

#[test]
fn coded_value_not_in_list() {
    let n = coded_node(&["at0001", "at0002"]);
    let bad = json!({
        "_type": "DV_CODED_TEXT", "value": "x",
        "defining_code": {"_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
            "code_string": "at0099"}
    });
    assert_eq!(
        kinds(&walk_only(&bad, &n)),
        vec![ValidationKind::CodedValue]
    );

    let good = json!({
        "_type": "DV_CODED_TEXT", "value": "x",
        "defining_code": {"_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
            "code_string": "at0001"}
    });
    assert!(walk_only(&good, &n).is_empty());
}

#[test]
fn coded_value_external_terminology_is_skipped() {
    // A SNOMED code is not validated against the archetype's internal list.
    let n = coded_node(&["at0001"]);
    let external = json!({
        "_type": "DV_CODED_TEXT", "value": "x",
        "defining_code": {"_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "SNOMED-CT"},
            "code_string": "999"}
    });
    assert!(walk_only(&external, &n).is_empty());
}

// ── type conformance ─────────────────────────────────────────────────────────────

#[test]
fn wrong_type_reported() {
    let n = node("DV_QUANTITY", "/q");
    let inst = json!({"_type": "DV_TEXT", "value": "x"});
    assert_eq!(
        kinds(&walk_only(&inst, &n)),
        vec![ValidationKind::WrongType]
    );
}

#[test]
fn coded_text_in_text_slot_conforms() {
    // DV_CODED_TEXT is-a DV_TEXT: no WrongType.
    let n = node("DV_TEXT", "/t");
    let inst = json!({
        "_type": "DV_CODED_TEXT", "value": "x",
        "defining_code": {"_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
            "code_string": "at0001"}
    });
    assert!(
        !kinds(&walk_only(&inst, &n)).contains(&ValidationKind::WrongType),
        "DV_CODED_TEXT should conform to a DV_TEXT slot"
    );
}

// ── RM invariant surfacing (full pipeline) ───────────────────────────────────────

#[test]
fn rm_invariant_surfaces() {
    // An ELEMENT with both value AND null_flavour violates Inv_null_flavour_indicated.
    let wt = wt_of(node("COMPOSITION", ""));
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [{
            "_type": "ELEMENT", "archetype_node_id": "at0001",
            "name": {"_type": "DV_TEXT", "value": "e"},
            "value": {"_type": "DV_TEXT", "value": "v"},
            "null_flavour": {"_type": "DV_CODED_TEXT", "value": "unknown",
                "defining_code": {"_type": "CODE_PHRASE",
                    "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                    "code_string": "253"}}
        }]
    });
    let msgs = validate_composition(&inst, &wt);
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Invariant
            && m.message.contains("Inv_null_flavour_indicated")),
        "expected the ELEMENT XOR invariant, got {msgs:?}"
    );
}

// ── openEHR-terminology group (full pipeline) ────────────────────────────────────

#[test]
fn bad_composition_category_reported() {
    let wt = wt_of(node("COMPOSITION", ""));
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "archetype_details": {"_type": "ARCHETYPED",
            "archetype_id": {"_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.x.v1"},
            "rm_version": "1.0.2"},
        "category": {"_type": "DV_CODED_TEXT", "value": "bogus",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                "code_string": "99999"}}
    });
    let msgs = validate_composition(&inst, &wt);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology
                && m.message.contains("composition category")),
        "expected a bad-category Terminology violation, got {msgs:?}"
    );
}

#[test]
fn valid_composition_category_is_clean_of_terminology() {
    let wt = wt_of(node("COMPOSITION", ""));
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "category": {"_type": "DV_CODED_TEXT", "value": "event",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                "code_string": "433"}}
    });
    let msgs = validate_composition(&inst, &wt);
    assert!(
        !kinds(&msgs).contains(&ValidationKind::Terminology),
        "valid category 433 should not trip terminology, got {msgs:?}"
    );
}

// ── path parsing ─────────────────────────────────────────────────────────────────

#[test]
fn segment_parsing_respects_brackets() {
    let segs = parse_segments("/content[openEHR-EHR-SECTION.x.v1]/items[at0004,'Sys']/value");
    assert_eq!(segs.len(), 3);
    assert_eq!(segs[0].attr, "content");
    assert!(matches!(&segs[0].pred, Pred::Node(id) if id == "openEHR-EHR-SECTION.x.v1"));
    assert!(matches!(&segs[1].pred, Pred::NodeNamed(id, name) if id == "at0004" && name == "Sys"));
    assert!(matches!(segs[2].pred, Pred::Any));
}
