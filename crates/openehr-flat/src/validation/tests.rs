//! Per-rule unit tests for the composition validator, built on hand-shaped
//! `WebTemplate` nodes + minimal instances (no OPT parsing) so each rule is
//! exercised in isolation. End-to-end corpus tests live in `tests/validation.rs`.

use indexmap::IndexMap;
use serde_json::{Value, json};

use super::*;
use crate::webtemplate::{
    WebTemplate, WebTemplateCardinality, WebTemplateCodedValue, WebTemplateExistence,
    WebTemplateInput, WebTemplateInputType, WebTemplateNode, WebTemplateRange,
    WebTemplateValidation,
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
    root.card_all = vec![WebTemplateCardinality {
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

// ── terminology: code-set slots (ISO / IANA), F-07-03 / F-11-02 ──────────────────

/// A bare `CODE_PHRASE` node for an external (ISO/IANA) code-set slot.
fn code_phrase(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
        "code_string": code,
    })
}

fn coded_text(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": "x",
        "defining_code": code_phrase(terminology, code),
    })
}

#[test]
fn dv_text_bad_language_and_encoding_reported() {
    // F-11-02: DV_TEXT.language (ISO 639-1) and .encoding (IANA character sets).
    let bad_lang = json!({
        "_type": "DV_TEXT", "value": "hi",
        "language": code_phrase("ISO_639-1", "zz"),
    });
    let msgs = validate_rm_and_terminology(&bad_lang);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology && m.message.contains("language")),
        "expected a bad-language Terminology violation, got {msgs:?}"
    );

    let bad_enc = json!({
        "_type": "DV_TEXT", "value": "hi",
        "encoding": code_phrase("IANA_character-sets", "NOT-A-CHARSET"),
    });
    let msgs = validate_rm_and_terminology(&bad_enc);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology && m.message.contains("character set")),
        "expected a bad-encoding Terminology violation, got {msgs:?}"
    );

    // Valid language + encoding: clean.
    let good = json!({
        "_type": "DV_TEXT", "value": "hi",
        "language": code_phrase("ISO_639-1", "en"),
        "encoding": code_phrase("IANA_character-sets", "UTF-8"),
    });
    assert!(
        !kinds(&validate_rm_and_terminology(&good)).contains(&ValidationKind::Terminology),
        "valid en/UTF-8 should be clean"
    );
}

#[test]
fn composition_bad_territory_reported() {
    // F-07-03: COMPOSITION.territory (ISO 3166-1 countries).
    let inst = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "language": code_phrase("ISO_639-1", "en"),
        "territory": code_phrase("ISO_3166-1", "ZZ"),
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology && m.message.contains("country")),
        "expected a bad-territory Terminology violation, got {msgs:?}"
    );
}

#[test]
fn entry_bad_encoding_reported() {
    // F-07-03: ENTRY (OBSERVATION) encoding (IANA character sets).
    let inst = json!({
        "_type": "OBSERVATION", "archetype_node_id": "a",
        "language": code_phrase("ISO_639-1", "en"),
        "encoding": code_phrase("IANA_character-sets", "BOGUS-CHARSET"),
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology && m.message.contains("character set")),
        "expected a bad-encoding Terminology violation on the ENTRY, got {msgs:?}"
    );
}

// ── terminology: openEHR-group slots, F-07-03 / F-11-03 / F-11-04 / F-11-05 ───────

#[test]
fn ism_transition_bad_transition_reported() {
    // F-11-03: ISM_TRANSITION.transition (instruction_transitions group).
    let inst = json!({
        "_type": "ISM_TRANSITION",
        "current_state": coded_text("openehr", "245"), // active (valid)
        "transition": coded_text("openehr", "99999"),  // invalid
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Terminology
            && m.message.contains("instruction transition")),
        "expected a bad-transition Terminology violation, got {msgs:?}"
    );
}

#[test]
fn term_mapping_bad_purpose_reported() {
    // F-11-04: TERM_MAPPING.purpose (term_mapping_purpose group), reached via a
    // DV_TEXT.mappings[].
    let inst = json!({
        "_type": "DV_TEXT", "value": "hi",
        "mappings": [{
            "_type": "TERM_MAPPING", "match": "=",
            "purpose": coded_text("openehr", "99999"),
            "target": code_phrase("SNOMED-CT", "123"),
        }]
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology
                && m.message.contains("term mapping purpose")),
        "expected a bad-purpose Terminology violation, got {msgs:?}"
    );
}

#[test]
fn party_related_bad_relationship_reported() {
    // F-11-05: PARTY_RELATED.relationship (subject_relationship group).
    let inst = json!({
        "_type": "PARTY_RELATED",
        "relationship": coded_text("openehr", "99999"),
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology
                && m.message.contains("subject relationship")),
        "expected a bad-relationship Terminology violation, got {msgs:?}"
    );
}

#[test]
fn dv_ordered_bad_normal_status_reported() {
    // F-11-05: DV_ORDERED.normal_status (normal_statuses code set) — checked on
    // any node carrying `normal_status`.
    let inst = json!({
        "_type": "DV_QUANTITY", "magnitude": 1.0, "units": "kg",
        "normal_status": code_phrase("openehr", "X"),
    });
    let msgs = validate_rm_and_terminology(&inst);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology && m.message.contains("normal status")),
        "expected a bad-normal-status Terminology violation, got {msgs:?}"
    );
    // A valid normal status ("N") is clean.
    let good = json!({
        "_type": "DV_QUANTITY", "magnitude": 1.0, "units": "kg",
        "normal_status": code_phrase("openehr", "N"),
    });
    assert!(!kinds(&validate_rm_and_terminology(&good)).contains(&ValidationKind::Terminology));
}

// ── AOM 1.4 C_ATTRIBUTE.existence (F-07-04) ───────────────────────────────────────

#[test]
fn existence_mandatory_attribute_missing_reported() {
    // A node requiring a mandatory `value` attribute (existence {1..1}); the
    // instance omits it → Required.
    let mut n = node("ELEMENT", "/items[at0001]");
    n.existence = vec![WebTemplateExistence {
        min: 1,
        max: 1,
        path: "/items[at0001]/value".to_owned(),
    }];
    let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
        "name": {"_type": "DV_TEXT", "value": "e"}});
    let msgs = walk_only(&inst, &n);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Required && m.path.ends_with("/value")),
        "expected a Required existence violation for the missing value, got {msgs:?}"
    );
}

#[test]
fn existence_present_attribute_is_clean() {
    let mut n = node("ELEMENT", "/items[at0001]");
    n.existence = vec![WebTemplateExistence {
        min: 1,
        max: 1,
        path: "/items[at0001]/value".to_owned(),
    }];
    let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
        "name": {"_type": "DV_TEXT", "value": "e"},
        "value": {"_type": "DV_TEXT", "value": "v"}});
    assert!(
        walk_only(&inst, &n).is_empty(),
        "a present mandatory value should be clean"
    );
}

#[test]
fn existence_empty_array_counts_as_absent() {
    let mut n = node("COMPOSITION", "");
    n.existence = vec![WebTemplateExistence {
        min: 1,
        max: -1,
        path: "/content".to_owned(),
    }];
    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
    let msgs = walk_only(&inst, &n);
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Required),
        "an empty mandatory container attribute is absent, got {msgs:?}"
    );
}

// ── path parsing ─────────────────────────────────────────────────────────────────

#[test]
fn segment_parsing_respects_brackets() {
    // Parsing now routes through the single `openehr_rm::paths` implementation
    // via `crate::path`; this asserts the validator sees the same segments.
    let segs = crate::path::parse("/content[openEHR-EHR-SECTION.x.v1]/items[at0004,'Sys']/value");
    assert_eq!(segs.len(), 3);
    assert_eq!(segs[0].attribute, "content");
    assert_eq!(
        segs[0].predicate.archetype_node_id.as_deref(),
        Some("openEHR-EHR-SECTION.x.v1")
    );
    assert_eq!(segs[0].predicate.name_value, None);
    assert_eq!(
        segs[1].predicate.archetype_node_id.as_deref(),
        Some("at0004")
    );
    assert_eq!(segs[1].predicate.name_value.as_deref(), Some("Sys"));
    assert!(segs[2].predicate.is_empty());
}

// ── CNF-hardening additions (master15/16/17 truth tables) ────────────────────

use crate::webtemplate::{WebTemplateCodeList, WebTemplateSlot};

/// `1..*` container cardinality with zero members → Cardinality (master15
/// CONT-COMP-content_card_1plus; AOM 1.4 §cardinality).
#[test]
fn cardinality_one_plus_empty_rejected() {
    let mut root = node("COMPOSITION", "");
    root.card_all = vec![WebTemplateCardinality {
        min: Some(1),
        max: -1,
        ids: None,
        path: "/content".to_owned(),
    }];
    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
    let msgs = walk_only(&inst, &root);
    assert!(
        kinds(&msgs).contains(&ValidationKind::Cardinality),
        "expected Cardinality for 1..* with 0 members, got {msgs:?}"
    );
}

/// A bare mandatory attribute (existence `1..1`, no value constraint) must be
/// present (master15 context_mand; AOM 1.4 §existence).
#[test]
fn bare_mandatory_attribute_absent_rejected() {
    let mut root = node("COMPOSITION", "");
    root.existence = vec![WebTemplateExistence {
        min: 1,
        max: 1,
        path: "/context".to_owned(),
    }];
    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x"});
    let msgs = walk_only(&inst, &root);
    assert!(
        kinds(&msgs).contains(&ValidationKind::Required),
        "expected Required for absent mandatory context, got {msgs:?}"
    );
}

/// A hoisted-wrapper slot narrowed to a concrete subtype rejects a sibling
/// subtype and accepts the narrowed one; an abstract slot accepts any subtype
/// (master16 §ITEM_STRUCTURE/§EVENT "Class not allowed").
#[test]
fn slot_narrowing() {
    let mut eval = node("EVALUATION", "/content[at0001]");
    eval.slots = vec![WebTemplateSlot {
        path: "/content[at0001]/data[at0002]".to_owned(),
        rm_type: "ITEM_LIST".to_owned(),
    }];
    let wrong = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
        "data": {"_type": "ITEM_TREE", "archetype_node_id": "at0002", "items": []}});
    let msgs = walk_only(&wrong, &eval);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::WrongType && m.message.contains("not allowed")),
        "expected WrongType for ITEM_TREE in an ITEM_LIST slot, got {msgs:?}"
    );
    let right = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
        "data": {"_type": "ITEM_LIST", "archetype_node_id": "at0002", "items": []}});
    assert!(
        walk_only(&right, &eval).is_empty(),
        "narrowed subtype accepted"
    );

    eval.slots[0].rm_type = "ITEM_STRUCTURE".to_owned();
    let any = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
        "data": {"_type": "ITEM_TABLE", "archetype_node_id": "at0002", "rows": []}});
    assert!(
        walk_only(&any, &eval).is_empty(),
        "abstract ITEM_STRUCTURE slot admits any subtype"
    );
}

/// `C_INTEGER.list` on DV_COUNT.magnitude (master17.3 CONT-DV_COUNT-validate_list).
#[test]
fn numeric_list_membership() {
    let mut count = node("DV_COUNT", "/value");
    count.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Integer, None)];
    count.numeric_lists = vec![("magnitude".to_owned(), vec![3.0])];
    let bad = json!({"_type": "DV_COUNT", "magnitude": 7});
    let msgs = walk_only(&bad, &count);
    assert!(
        kinds(&msgs).contains(&ValidationKind::CodedValue),
        "expected CodedValue for magnitude off the list, got {msgs:?}"
    );
    let good = json!({"_type": "DV_COUNT", "magnitude": 3});
    assert!(walk_only(&good, &count).is_empty());
}

/// DV_PROPORTION `type` kind membership (master17.3 CONT-DV_PROPORTION-*).
#[test]
fn proportion_kind_membership() {
    let mut prop = node("DV_PROPORTION", "/value");
    prop.inputs = vec![WebTemplateInput::new(
        WebTemplateInputType::Decimal,
        Some("numerator"),
    )];
    prop.proportion_types = vec!["percent".to_owned()];
    let bad = json!({"_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 2.0, "type": 0});
    let msgs = walk_only(&bad, &prop);
    assert!(
        kinds(&msgs).contains(&ValidationKind::CodedValue),
        "expected CodedValue for ratio where percent required, got {msgs:?}"
    );
    let good =
        json!({"_type": "DV_PROPORTION", "numerator": 42.0, "denominator": 100.0, "type": 2});
    assert!(walk_only(&good, &prop).is_empty());
}

/// C_DATE pattern + range (master17.4 CONT-DV_DATE-validate_constraint/-range).
#[test]
fn temporal_pattern_and_range() {
    let mut date = node("DV_DATE", "/value");
    let mut input = WebTemplateInput::new(WebTemplateInputType::Date, None);
    input.validation = Some(WebTemplateValidation {
        pattern: Some("yyyy-mm-dd".to_owned()),
        range: Some(WebTemplateRange {
            min_op: Some(">=".to_owned()),
            min: Some(json!("2021-01-01")),
            max_op: Some("<=".to_owned()),
            max: Some(json!("2021-12-31")),
        }),
        precision: None,
    });
    date.inputs = vec![input];

    let partial = json!({"_type": "DV_DATE", "value": "2021"});
    assert!(
        kinds(&walk_only(&partial, &date)).contains(&ValidationKind::PatternError),
        "partial date violates yyyy-mm-dd"
    );
    let out = json!({"_type": "DV_DATE", "value": "2025-06-01"});
    assert!(
        kinds(&walk_only(&out, &date)).contains(&ValidationKind::RangeError),
        "date outside the range"
    );
    let ok = json!({"_type": "DV_DATE", "value": "2021-10-18"});
    assert!(walk_only(&ok, &date).is_empty());
}

/// C_TIME pattern: a partial time violates HH:MM:SS (master17.4 CONT-DV_TIME).
#[test]
fn time_pattern_partial_rejected() {
    let mut time = node("DV_TIME", "/value");
    let mut input = WebTemplateInput::new(WebTemplateInputType::Time, None);
    input.validation = Some(WebTemplateValidation {
        pattern: Some("HH:MM:SS".to_owned()),
        range: None,
        precision: None,
    });
    time.inputs = vec![input];
    assert!(
        kinds(&walk_only(
            &json!({"_type": "DV_TIME", "value": "22"}),
            &time
        ))
        .contains(&ValidationKind::PatternError)
    );
    assert!(walk_only(&json!({"_type": "DV_TIME", "value": "22:18:16"}), &time).is_empty());
}

/// C_DURATION allowed fields + range (master17.4 CONT-DV_DURATION-*).
#[test]
fn duration_fields_and_range() {
    let mut dur = node("DV_DURATION", "/value");
    dur.inputs = vec![
        WebTemplateInput::new(WebTemplateInputType::Integer, Some("hour")),
        WebTemplateInput::new(WebTemplateInputType::Integer, Some("minute")),
    ];
    dur.duration_range = Some(WebTemplateRange {
        min_op: Some(">=".to_owned()),
        min: Some(json!("PT0S")),
        max_op: Some("<=".to_owned()),
        max: Some(json!("PT1H")),
    });
    assert!(
        kinds(&walk_only(
            &json!({"_type": "DV_DURATION", "value": "P1Y"}),
            &dur
        ))
        .contains(&ValidationKind::PatternError),
        "year field forbidden by the pattern"
    );
    assert!(
        kinds(&walk_only(
            &json!({"_type": "DV_DURATION", "value": "PT5H"}),
            &dur
        ))
        .contains(&ValidationKind::RangeError),
        "PT5H outside [PT0S,PT1H]"
    );
    assert!(walk_only(&json!({"_type": "DV_DURATION", "value": "PT30M"}), &dur).is_empty());
}

/// An enumerated **external** C_CODE_PHRASE list constrains membership
/// (master17.2 CONT-DV_CODED_TEXT-validate_ext_term; AOM 1.4 §C_CODE_PHRASE).
#[test]
fn external_code_list_membership() {
    let mut coded = node("DV_CODED_TEXT", "/value");
    let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, Some("code"));
    input.terminology = Some("SNOMED-CT".to_owned());
    input.list = vec![WebTemplateCodedValue::new("73211009", None)];
    coded.inputs = vec![input];

    let bad = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
        "terminology_id": {"value": "SNOMED-CT"}, "code_string": "99999999"}});
    assert!(
        kinds(&walk_only(&bad, &coded)).contains(&ValidationKind::CodedValue),
        "external code off the enumerated list"
    );
    let good = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
        "terminology_id": {"value": "SNOMED-CT"}, "code_string": "73211009"}});
    assert!(walk_only(&good, &coded).is_empty());
    // A code from a different terminology than the constraint's is not judged.
    let other = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
        "terminology_id": {"value": "ICD10"}, "code_string": "A00"}});
    assert!(walk_only(&other, &coded).is_empty());
}

/// C_CODE_PHRASE on a coded attribute outside `defining_code`
/// (DV_MULTIMEDIA.media_type — master17.6 CONT-DV_MULTIMEDIA-validate_media_type).
#[test]
fn media_type_code_list() {
    let mut mm = node("DV_MULTIMEDIA", "/value");
    mm.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Text, None)];
    mm.code_lists = vec![WebTemplateCodeList {
        attr: "media_type".to_owned(),
        terminology: Some("IANA_media-types".to_owned()),
        codes: vec!["image/png".to_owned()],
    }];
    let bad = json!({"_type": "DV_MULTIMEDIA", "size": 1, "media_type": {
        "terminology_id": {"value": "IANA_media-types"}, "code_string": "image/gif"}});
    assert!(
        kinds(&walk_only(&bad, &mm)).contains(&ValidationKind::CodedValue),
        "media_type off the enumerated list"
    );
    let good = json!({"_type": "DV_MULTIMEDIA", "size": 1, "media_type": {
        "terminology_id": {"value": "IANA_media-types"}, "code_string": "image/png"}});
    assert!(walk_only(&good, &mm).is_empty());
}
