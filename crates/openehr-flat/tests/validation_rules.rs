//! Per-rule unit tests for the composition validator, built on hand-shaped
//! `WebTemplate` nodes + minimal instances (no OPT parsing) so each rule is
//! exercised in isolation. End-to-end corpus tests live in `tests/validation.rs`.

#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
use indexmap::IndexMap;
use serde_json::{Value, json};

use openehr_flat::validation::*;
use openehr_flat::webtemplate::{
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
    validate_archetype_conformance(instance, &wt_of(root.clone()))
}

/// Timing helper for the public-entry perf diagnostics.
fn time_pass(iters: u32, mut f: impl FnMut() -> usize) -> f64 {
    let start = std::time::Instant::now();
    let mut sink = 0usize;
    for _ in 0..iters {
        sink = sink.wrapping_add(f());
    }
    std::hint::black_box(sink);
    start.elapsed().as_secs_f64() * 1e6 / f64::from(iters)
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

// ── incomplete-lifecycle (553) relaxation ────────────────────────────────────────
// RM common master06 §"Incomplete Content": existence/occurrences/cardinality
// lower limits treated as zero; upper limits and value/type constraints stay.

/// Run the archetype-conformance pass with the `553|incomplete|` relaxation.
fn walk_incomplete(instance: &Value, root: &WebTemplateNode) -> Vec<ValidationMessage> {
    validate_archetype_conformance_incomplete(instance, &wt_of(root.clone()))
}

#[test]
fn incomplete_suppresses_required_missing_occurrences() {
    let mut root = node("COMPOSITION", "");
    let mut sec = node("SECTION", "/content[at0001]");
    sec.min = Some(1);
    sec.max = 1;
    root.children = vec![sec];

    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
    // Strict: a mandatory node is missing.
    assert!(kinds(&walk_only(&inst, &root)).contains(&ValidationKind::Required));
    // Incomplete: the lower bound is zeroed, so nothing is emitted.
    assert!(
        walk_incomplete(&inst, &root).is_empty(),
        "incomplete commit must tolerate a missing mandatory node"
    );
}

#[test]
fn incomplete_suppresses_too_few_but_keeps_too_many() {
    // too few (min 2, one present) → suppressed under incomplete.
    let mut root = node("COMPOSITION", "");
    let mut sec = node("SECTION", "/content[at0001]");
    sec.min = Some(2);
    sec.max = 3;
    root.children = vec![sec];
    let one = json!({"_type": "SECTION", "archetype_node_id": "at0001",
                     "name": {"_type": "DV_TEXT", "value": "s"}});
    let too_few = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x", "content": [one.clone()]
    });
    assert!(
        walk_incomplete(&too_few, &root).is_empty(),
        "incomplete commit must tolerate too-few occurrences"
    );

    // too many (three present, max 3 ok; four exceeds) → still enforced (upper
    // bound is not relaxed: missing is tolerated, wrong is not).
    let too_many = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [one.clone(), one.clone(), one.clone(), one]
    });
    assert!(
        walk_incomplete(&too_many, &root)
            .iter()
            .any(|m| m.kind == ValidationKind::Occurrences && m.message.contains("too many")),
        "incomplete commit must still reject too-many occurrences"
    );
}

#[test]
fn incomplete_suppresses_cardinality_lower_but_keeps_upper() {
    // Lower-bound cardinality violation → suppressed.
    let mut low = node("COMPOSITION", "");
    low.card_all = vec![WebTemplateCardinality {
        min: Some(2),
        max: -1,
        ids: None,
        path: "/content".to_owned(),
    }];
    let entry = json!({"_type": "OBSERVATION", "archetype_node_id": "a"});
    let one_child = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x", "content": [entry.clone()]
    });
    assert!(
        walk_incomplete(&one_child, &low).is_empty(),
        "incomplete commit must tolerate a below-minimum container"
    );

    // Upper-bound cardinality violation → still enforced.
    let mut high = node("COMPOSITION", "");
    high.card_all = vec![WebTemplateCardinality {
        min: Some(1),
        max: 2,
        ids: None,
        path: "/content".to_owned(),
    }];
    let three = json!({
        "_type": "COMPOSITION", "archetype_node_id": "x",
        "content": [entry.clone(), entry.clone(), entry]
    });
    assert!(
        kinds(&walk_incomplete(&three, &high)).contains(&ValidationKind::Cardinality),
        "incomplete commit must still reject an over-maximum container"
    );
}

#[test]
fn incomplete_suppresses_existence() {
    let mut root = node("COMPOSITION", "");
    root.existence = vec![WebTemplateExistence {
        min: 1,
        max: 1,
        path: "/context".to_owned(),
    }];
    // The mandatory `context` field is absent.
    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x"});
    assert!(kinds(&walk_only(&inst, &root)).contains(&ValidationKind::Required));
    assert!(
        walk_incomplete(&inst, &root).is_empty(),
        "incomplete commit must tolerate a missing mandatory attribute (existence)"
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
    // DV_TEXT.language (ISO 639-1) and .encoding (IANA character sets).
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
    // COMPOSITION.territory (ISO 3166-1 countries).
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
    // ENTRY (OBSERVATION) encoding (IANA character sets).
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
    // ISM_TRANSITION.transition (instruction_transitions group).
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
    // TERM_MAPPING.purpose (term_mapping_purpose group), reached via a
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
    // PARTY_RELATED.relationship (subject_relationship group).
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
    // DV_ORDERED.normal_status (normal_statuses code set) — checked on
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

// ── AOM 1.4 C_ATTRIBUTE.existence ───────────────────────────────────────

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
    // via `openehr_flat::path`; this asserts the validator sees the same segments.
    let segs =
        openehr_flat::path::parse("/content[openEHR-EHR-SECTION.x.v1]/items[at0004,'Sys']/value");
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

use openehr_flat::webtemplate::{WebTemplateCodeList, WebTemplateSlot};

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
/// present (master15 `context_mand`; AOM 1.4 §existence).
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
/// (master16 §`ITEM_STRUCTURE/§EVENT` "Class not allowed").
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

/// `C_INTEGER.list` on `DV_COUNT.magnitude` (master17.3 CONT-DV_COUNT-validate_list).
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

/// `DV_PROPORTION` `type` kind membership (master17.3 CONT-DV_PROPORTION-*).
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

/// `C_DATE` pattern + range (master17.4 CONT-DV_DATE-validate_constraint/-range).
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

/// `C_TIME` pattern: a partial time violates HH:MM:SS (master17.4 CONT-DV_TIME).
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

/// `C_DURATION` allowed fields + range (master17.4 CONT-DV_DURATION-*).
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

/// An enumerated **external** `C_CODE_PHRASE` list constrains membership
/// (master17.2 CONT-DV_CODED_TEXT-validate_ext_term; AOM 1.4 §`C_CODE_PHRASE`).
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

/// `C_CODE_PHRASE` on a coded attribute outside `defining_code`
/// (`DV_MULTIMEDIA.media_type` — master17.6 CONT-DV_MULTIMEDIA-validate_media_type).
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

// ── closed-archetype walk ─────────────────────────────────────────

use openehr_flat::webtemplate::{WebTemplateArchetypeSlot, WebTemplateClosedAttribute};

/// A COMPOSITION whose `content` is closed to `openEHR-EHR-SECTION.x.v1`: the
/// defined section is accepted, a foreign OBSERVATION is rejected as unexpected
/// (closed-world rule 1).
#[test]
fn closed_world_rejects_foreign_content() {
    let mut root = node("COMPOSITION", "");
    root.closed_attributes = vec![WebTemplateClosedAttribute {
        path: "/content".to_owned(),
        allowed_ids: vec!["openEHR-EHR-SECTION.x.v1".to_owned()],
        slots: vec![],
    }];
    let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
        {"_type": "SECTION", "archetype_node_id": "openEHR-EHR-SECTION.x.v1",
         "name": {"_type": "DV_TEXT", "value": "s"}},
        {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.foreign.v1",
         "name": {"_type": "DV_TEXT", "value": "o"}}
    ]});
    // the closed-world admission rule (scope amendment, B2 close): an unmatched
    // *archetype-rooted* child is tolerated (the flat OPT does not enumerate
    // the full slot-fill universe; the CNF corpus itself commits such ENTRYs).
    let msgs = walk_only(&inst, &root);
    assert!(
        msgs.is_empty(),
        "foreign archetype-rooted content is tolerated (the closed-world admission rule), got {msgs:?}"
    );
    // At-coded children remain closed: an at-coded child matching no sibling
    // constraint is rejected (closed-world rule 1).
    let at_foreign = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
        {"_type": "SECTION", "archetype_node_id": "at0099",
         "name": {"_type": "DV_TEXT", "value": "s"}}
    ]});
    let msgs = walk_only(&at_foreign, &root);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Unexpected && m.message.contains("at0099")),
        "expected an Unexpected violation for the foreign at-coded child, got {msgs:?}"
    );
    let ok = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
        {"_type": "SECTION", "archetype_node_id": "openEHR-EHR-SECTION.x.v1",
         "name": {"_type": "DV_TEXT", "value": "s"}}]});
    assert!(
        walk_only(&ok, &root).is_empty(),
        "the defined section is admitted"
    );
}

/// A metadata value (no `archetype_node_id`, i.e. non-LOCATABLE) under a closed
/// attribute is never flagged (closed-world rule 2 — the `archetype_node_id`
/// discriminator).
#[test]
fn closed_world_ignores_metadata_values() {
    let mut root = node("ELEMENT", "/items[at0001]");
    root.closed_attributes = vec![WebTemplateClosedAttribute {
        path: "/items[at0001]/value".to_owned(),
        allowed_ids: vec!["at9999".to_owned()],
        slots: vec![],
    }];
    let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
        "name": {"_type": "DV_TEXT", "value": "e"},
        "value": {"_type": "DV_QUANTITY", "magnitude": 1.0, "units": "kg"}});
    assert!(
        walk_only(&inst, &root).is_empty(),
        "a DATA_VALUE with no archetype_node_id must not be flagged by closure"
    );
}

// ── ARCHETYPE_SLOT enforcement ─────────────────────────────────────

fn obs_slot(includes: &[&str], excludes: &[&str], min: i32, max: i32) -> WebTemplateNode {
    let mut root = node("COMPOSITION", "");
    root.closed_attributes = vec![WebTemplateClosedAttribute {
        path: "/content".to_owned(),
        allowed_ids: vec![],
        slots: vec![WebTemplateArchetypeSlot {
            rm_type: "OBSERVATION".to_owned(),
            min,
            max,
            includes: includes.iter().map(|s| (*s).to_owned()).collect(),
            excludes: excludes.iter().map(|s| (*s).to_owned()).collect(),
        }],
    }];
    root
}

fn content_obs(archetype_id: &str, rm_type: &str) -> Value {
    json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
        {"_type": rm_type, "archetype_node_id": archetype_id,
         "name": {"_type": "DV_TEXT", "value": "o"}}]})
}

#[test]
fn slot_admits_include_rejects_others() {
    let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\..*"], &[], 0, -1);
    assert!(
        walk_only(
            &content_obs("openEHR-EHR-OBSERVATION.bp.v1", "OBSERVATION"),
            &root
        )
        .is_empty(),
        "an include-matching filler is admitted"
    );
    let msgs = walk_only(
        &content_obs("openEHR-EHR-EVALUATION.x.v1", "EVALUATION"),
        &root,
    );
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Unexpected),
        "a wrong-rm-type filler is rejected, got {msgs:?}"
    );
}

#[test]
fn slot_exclude_rejects_matching_filler() {
    let root = obs_slot(
        &[r"openEHR-EHR-OBSERVATION\..*"],
        &[r"openEHR-EHR-OBSERVATION\.secret\..*"],
        0,
        -1,
    );
    let msgs = walk_only(
        &content_obs("openEHR-EHR-OBSERVATION.secret.v1", "OBSERVATION"),
        &root,
    );
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Unexpected),
        "an exclude-matching filler is rejected, got {msgs:?}"
    );
}

#[test]
fn slot_blanket_exclude_ignored_when_includes_present() {
    // ADL 1.4 closed-slot idiom: specific include + a blanket `.*` exclude. The
    // specific include wins (AOM 1.4 has no is_closed) — the filler passes.
    let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\.bp\.v1"], &[".*"], 0, -1);
    assert!(
        walk_only(
            &content_obs("openEHR-EHR-OBSERVATION.bp.v1", "OBSERVATION"),
            &root
        )
        .is_empty(),
        "a specific include overrides a blanket `.*` exclude"
    );
}

#[test]
fn slot_occurrences_min_and_max() {
    let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\..*"], &[], 1, 1);
    let empty = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
    assert!(
        walk_only(&empty, &root)
            .iter()
            .any(|m| m.kind == ValidationKind::Required),
        "an unfilled mandatory slot is Required"
    );
    let two = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
        {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.a.v1",
         "name": {"_type": "DV_TEXT", "value": "a"}},
        {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.b.v1",
         "name": {"_type": "DV_TEXT", "value": "b"}}]});
    assert!(
        walk_only(&two, &root)
            .iter()
            .any(|m| m.kind == ValidationKind::Occurrences),
        "too many slot fillers is Occurrences"
    );
}

// ── DV_ORDINAL / DV_SCALE (symbol, value) pairing ──────────────────

fn ordinal_node(rm: &str, scale: bool) -> WebTemplateNode {
    let mut n = node(rm, "/value");
    let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, None);
    let mk = |code: &str, v: i32| {
        let mut cv = WebTemplateCodedValue::new(code, None);
        if scale {
            cv.scale = Some(f64::from(v));
        } else {
            cv.ordinal = Some(v);
        }
        cv
    };
    input.list = vec![mk("at0014", 0), mk("at0015", 1)];
    n.inputs = vec![input];
    n
}

fn ordinal_value(rm: &str, v: &Value, code: &str) -> Value {
    json!({"_type": rm, "value": v, "symbol": {"_type": "DV_CODED_TEXT",
        "value": "s", "defining_code": {"_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"}, "code_string": code}}})
}

#[test]
fn ordinal_pair_must_match() {
    let n = ordinal_node("DV_ORDINAL", false);
    assert!(walk_only(&ordinal_value("DV_ORDINAL", &json!(0), "at0014"), &n).is_empty());
    assert!(
        kinds(&walk_only(
            &ordinal_value("DV_ORDINAL", &json!(1), "at0014"),
            &n
        ))
        .contains(&ValidationKind::CodedValue),
        "value 1 does not pair with symbol at0014"
    );
    assert!(
        kinds(&walk_only(
            &ordinal_value("DV_ORDINAL", &json!(0), "at0666"),
            &n
        ))
        .contains(&ValidationKind::CodedValue),
        "symbol at0666 is off the list"
    );
}

#[test]
fn scale_pair_must_match() {
    let n = ordinal_node("DV_SCALE", true);
    assert!(walk_only(&ordinal_value("DV_SCALE", &json!(0.0), "at0014"), &n).is_empty());
    assert!(
        kinds(&walk_only(
            &ordinal_value("DV_SCALE", &json!(1.0), "at0014"),
            &n
        ))
        .contains(&ValidationKind::CodedValue),
        "scale 1.0 does not pair with symbol at0014"
    );
}

// ── C_STRING fail-closed with fancy-regex fallback ─────────────────

#[test]
fn c_string_backreference_is_enforced() {
    // `(a)\1` uses a backreference the `regex` crate rejects; `fancy-regex`
    // compiles it, so the pattern is enforced instead of silently passing.
    let mut n = node("DV_TEXT", "/text");
    let mut input = WebTemplateInput::new(WebTemplateInputType::Text, None);
    input.validation = Some(WebTemplateValidation {
        pattern: Some(r"(a)\1".to_owned()),
        ..Default::default()
    });
    n.inputs = vec![input];
    assert!(
        kinds(&walk_only(&json!({"_type": "DV_TEXT", "value": "ab"}), &n))
            .contains(&ValidationKind::PatternError),
        "`ab` fails the backreference pattern (was silently passing before F-07-11)"
    );
    assert!(walk_only(&json!({"_type": "DV_TEXT", "value": "aa"}), &n).is_empty());
}

// ── C_TIME / C_DATE_TIME timezone_validity (F-07 temporal) ───────────────────

#[test]
fn timezone_validity_mandatory_and_disallowed() {
    let mut n = node("DV_TIME", "/value");
    n.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Time, None)];
    // 1001 = mandatory timezone.
    n.tz_validity = Some(1001);
    assert!(
        kinds(&walk_only(
            &json!({"_type": "DV_TIME", "value": "10:30:00"}),
            &n
        ))
        .contains(&ValidationKind::PatternError),
        "a missing mandatory timezone is rejected"
    );
    assert!(walk_only(&json!({"_type": "DV_TIME", "value": "10:30:00Z"}), &n).is_empty());
    // 1003 = disallowed timezone.
    n.tz_validity = Some(1003);
    assert!(
        kinds(&walk_only(
            &json!({"_type": "DV_TIME", "value": "10:30:00+01:00"}),
            &n
        ))
        .contains(&ValidationKind::PatternError),
        "a present disallowed timezone is rejected"
    );
    assert!(walk_only(&json!({"_type": "DV_TIME", "value": "10:30:00"}), &n).is_empty());
}

// ── LOCATABLE.Archetyped_valid (A1 rm-common-change-control-R46) ─────────────────

/// A non-root node — `archetype_node_id` an `at`/`id` term code — must not
/// carry `archetype_details` (`locatable.adoc` §`Archetyped_valid`); an
/// archetype-HRID node carrying them, and a nested archetype root *without*
/// them (the CNF-corpus-sanctioned shape), both stay valid.
#[test]
fn non_root_node_with_archetype_details_rejected() {
    let details = json!({"_type": "ARCHETYPED",
        "archetype_id": {"_type": "ARCHETYPE_ID", "value": "openEHR-EHR-CLUSTER.x.v1"},
        "rm_version": "1.0.2"});
    for bad_id in ["at0001", "id42", "at0001.1"] {
        let inst = json!({
            "_type": "CLUSTER", "archetype_node_id": bad_id,
            "name": {"_type": "DV_TEXT", "value": "c"},
            "archetype_details": details,
            "items": []
        });
        let msgs = validate_rm_and_terminology(&inst);
        assert!(
            msgs.iter()
                .any(|m| m.kind == ValidationKind::Invariant
                    && m.message.contains("Archetyped_valid")),
            "expected an Archetyped_valid violation for {bad_id}, got {msgs:?}"
        );
    }
    // Root shapes stay valid: HRID with details, and HRID without details
    // (nested archetype root as the CNF corpus ships it).
    for (id, with_details) in [
        ("openEHR-EHR-CLUSTER.x.v1", true),
        ("openEHR-EHR-CLUSTER.x.v1", false),
    ] {
        let mut inst = json!({
            "_type": "CLUSTER", "archetype_node_id": id,
            "name": {"_type": "DV_TEXT", "value": "c"},
            "items": []
        });
        if with_details {
            inst.as_object_mut()
                .unwrap()
                .insert("archetype_details".into(), details.clone());
        }
        let msgs = validate_rm_and_terminology(&inst);
        assert!(
            !msgs.iter().any(|m| m.message.contains("Archetyped_valid")),
            "root shape (details={with_details}) must not violate Archetyped_valid: {msgs:?}"
        );
    }
}

// ── "present implies non-empty" list invariants (A1 rm-composition) ──────────────

/// `COMPOSITION.Content_valid` / `SECTION.Items_valid` /
/// `INSTRUCTION.Activities_valid` etc.: a PRESENT empty list violates the
/// invariant; an absent list does not (`composition.adoc` §Invariants and
/// siblings).
#[test]
fn present_empty_lists_are_rejected() {
    let base = json!({
        "_type": "COMPOSITION", "archetype_node_id": "openEHR-EHR-COMPOSITION.x.v1",
        "name": {"_type": "DV_TEXT", "value": "c"}
    });
    // Absent content: no Content_valid violation.
    assert!(
        !validate_rm_and_terminology(&base)
            .iter()
            .any(|m| m.message.contains("Content_valid")),
        "absent content must not violate Content_valid"
    );
    // Present-empty content: violation.
    let mut empty = base.clone();
    empty
        .as_object_mut()
        .unwrap()
        .insert("content".into(), json!([]));
    let msgs = validate_rm_and_terminology(&empty);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Invariant && m.message.contains("Content_valid")),
        "present-empty content must violate Content_valid, got {msgs:?}"
    );
    // A nested SECTION with empty items: Items_valid violation at its path.
    let mut nested = base;
    nested.as_object_mut().unwrap().insert(
        "content".into(),
        json!([{
            "_type": "SECTION", "archetype_node_id": "at0001",
            "name": {"_type": "DV_TEXT", "value": "s"},
            "items": []
        }]),
    );
    let msgs = validate_rm_and_terminology(&nested);
    assert!(
        msgs.iter()
            .any(|m| m.message.contains("Items_valid") && m.path.contains("content")),
        "present-empty SECTION.items must violate Items_valid, got {msgs:?}"
    );
}

// ── data-structure shape duties (A1 rm-data-structures) ─────────────────────────

/// `CLUSTER.items` is 1..1 (`cluster.adoc`; the ITS-JSON CLUSTER schema
/// requires it) and one `HISTORY`'s events must all carry the same
/// `ITEM_STRUCTURE` subtype in `data` (RM `data_structures` master06 §History).
#[test]
fn data_structure_shapes_are_enforced() {
    // CLUSTER without items → violation.
    let cluster = json!({
        "_type": "CLUSTER", "archetype_node_id": "at0001",
        "name": {"_type": "DV_TEXT", "value": "c"}
    });
    let msgs = validate_rm_and_terminology(&cluster);
    assert!(
        msgs.iter()
            .any(|m| m.message.contains("CLUSTER.items is mandatory")),
        "items-less CLUSTER must be rejected, got {msgs:?}"
    );

    // HISTORY mixing ITEM_TREE and ITEM_LIST event data → violation.
    let history = json!({
        "_type": "HISTORY", "archetype_node_id": "at0002",
        "name": {"_type": "DV_TEXT", "value": "h"},
        "origin": {"_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z"},
        "events": [
            { "_type": "POINT_EVENT", "archetype_node_id": "at0003",
              "name": {"_type": "DV_TEXT", "value": "e1"},
              "time": {"_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z"},
              "data": {"_type": "ITEM_TREE", "archetype_node_id": "at0004",
                       "name": {"_type": "DV_TEXT", "value": "d"}, "items": []} },
            { "_type": "POINT_EVENT", "archetype_node_id": "at0003",
              "name": {"_type": "DV_TEXT", "value": "e2"},
              "time": {"_type": "DV_DATE_TIME", "value": "2026-01-01T01:00:00Z"},
              "data": {"_type": "ITEM_LIST", "archetype_node_id": "at0004",
                       "name": {"_type": "DV_TEXT", "value": "d"}, "items": []} }
        ]
    });
    let msgs = validate_rm_and_terminology(&history);
    assert!(
        msgs.iter()
            .any(|m| m.message.contains("same ITEM_STRUCTURE") && m.path.contains("events[1]")),
        "a HISTORY mixing event data types must be rejected, got {msgs:?}"
    );
}

// ── name-differentiated same-archetype-id siblings ──────────────────────────────
//
// A template may fill the same archetype twice under one container, the two
// fills differentiated by their runtime `name` (RM common
// `master03-archetyped_package.adoc` §"The `LOCATABLE` class" L33-35: a `name`
// distinguishes sibling nodes that share an `archetype_node_id`; AOM 1.4
// `master04-constraint_model_package.adoc` §`node_id` L41: node ids "guarantee
// sibling node unique identification"). Templates realise this by putting a
// fixed `name/value` `C_STRING` on all-but-one sibling, so one sibling stays
// *unqualified* (its `name` unconstrained). Each instance must be routed to
// exactly the sibling whose name it matches, and the unqualified sibling must
// admit only the instances no name-qualified sibling claims — otherwise the
// unqualified sibling (matched by `archetype_node_id` alone) captures a
// name-qualified sibling's instance and rejects it against the wrong overlay.

/// Two same-archetype siblings under `items`, one unqualified (name "A", inner
/// `items` closed to `at0004`) and one name-qualified ('B', inner `items` closed
/// to `at0013`).
fn name_diff_parent() -> WebTemplateNode {
    let mut root = node("CLUSTER", "");
    let sib_a = {
        let mut n = node("CLUSTER", "/items[openEHR-EHR-CLUSTER.c.v1]");
        n.name = Some("A".to_owned());
        n.min = Some(0);
        n.max = 1;
        n.closed_attributes = vec![WebTemplateClosedAttribute {
            path: "/items[openEHR-EHR-CLUSTER.c.v1]/items".to_owned(),
            allowed_ids: vec!["at0004".to_owned()],
            slots: vec![],
        }];
        n
    };
    let sib_b = {
        let mut n = node("CLUSTER", "/items[openEHR-EHR-CLUSTER.c.v1,'B']");
        n.name = Some("B".to_owned());
        n.min = Some(0);
        n.max = 1;
        n.closed_attributes = vec![WebTemplateClosedAttribute {
            path: "/items[openEHR-EHR-CLUSTER.c.v1,'B']/items".to_owned(),
            allowed_ids: vec!["at0013".to_owned()],
            slots: vec![],
        }];
        n
    };
    root.children = vec![sib_a, sib_b];
    root
}

/// A same-archetype CLUSTER instance with `name` and a single at-coded child.
fn c_instance(name: &str, child_id: &str) -> Value {
    json!({
        "_type": "CLUSTER", "archetype_node_id": "openEHR-EHR-CLUSTER.c.v1",
        "name": {"_type": "DV_TEXT", "value": name},
        "items": [{"_type": "ELEMENT", "archetype_node_id": child_id,
                   "name": {"_type": "DV_TEXT", "value": "leaf"},
                   "value": {"_type": "DV_TEXT", "value": "v"}}]
    })
}

fn unexpected_of(msgs: &[ValidationMessage]) -> Vec<&ValidationMessage> {
    msgs.iter()
        .filter(|m| m.kind == ValidationKind::Unexpected)
        .collect()
}

#[test]
fn name_diff_siblings_route_each_instance_to_its_own_overlay() {
    let root = name_diff_parent();
    // Both instances present, each carrying its own overlay's child.
    let inst = json!({
        "_type": "CLUSTER", "archetype_node_id": "x",
        "name": {"_type": "DV_TEXT", "value": "root"},
        "items": [c_instance("A", "at0004"), c_instance("B", "at0013")]
    });
    let msgs = walk_only(&inst, &root);
    assert!(
        unexpected_of(&msgs).is_empty(),
        "both name-differentiated siblings should validate against their own \
         overlay, got {msgs:?}"
    );
}

#[test]
fn name_qualified_siblings_child_in_unqualified_instance_is_unexpected() {
    let root = name_diff_parent();
    // `at0013` belongs to sibling 'B' only; inside the instance named "A" it must
    // still be Unexpected (true rejections preserved).
    let inst = json!({
        "_type": "CLUSTER", "archetype_node_id": "x",
        "name": {"_type": "DV_TEXT", "value": "root"},
        "items": [c_instance("A", "at0013")]
    });
    let msgs = walk_only(&inst, &root);
    assert!(
        unexpected_of(&msgs)
            .iter()
            .any(|m| m.message.contains("at0013")),
        "at0013 in the unqualified sibling's instance must be Unexpected, got {msgs:?}"
    );
}

#[test]
fn unqualified_sibling_admits_a_runtime_named_residual_instance() {
    let root = name_diff_parent();
    // An instance whose name matches NO name-qualified sibling ("other") routes
    // to the unqualified (residual) sibling — its `name` being unconstrained,
    // master03 §"The `LOCATABLE` class" L35. Its own overlay child (`at0004`)
    // therefore validates clean…
    let ok = json!({
        "_type": "CLUSTER", "archetype_node_id": "x",
        "name": {"_type": "DV_TEXT", "value": "root"},
        "items": [c_instance("other", "at0004")]
    });
    assert!(
        unexpected_of(&walk_only(&ok, &root)).is_empty(),
        "a residual-named instance must validate against the unqualified overlay"
    );
    // …but a child that overlay forbids (`at0013`) is still Unexpected there.
    let bad = json!({
        "_type": "CLUSTER", "archetype_node_id": "x",
        "name": {"_type": "DV_TEXT", "value": "root"},
        "items": [c_instance("other", "at0013")]
    });
    assert!(
        unexpected_of(&walk_only(&bad, &root))
            .iter()
            .any(|m| m.message.contains("at0013")),
        "the residual instance is still closed-world checked against the \
         unqualified overlay"
    );
}

// ── P20 item 15: validation-walk cost measurement (not a gate) ──────────────────

/// Count the `_type`-bearing nodes reachable in `v` (the units both
/// template-independent passes visit).
fn count_type_nodes(v: &Value) -> usize {
    match v {
        Value::Object(obj) => {
            let self_count = usize::from(obj.contains_key("_type"));
            self_count
                + obj
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .map(|(_, val)| count_type_nodes(val))
                    .sum::<usize>()
        }
        Value::Array(a) => a.iter().map(count_type_nodes).sum(),
        _ => 0,
    }
}

/// P20 overhead checklist item 31 — MEASUREMENT (not a correctness gate):
/// quantify the archetype-conformance **walk** (pass 3) over the populated IPS
/// example against its OPT-built `WebTemplate`. Pass 3 is where item 31's
/// per-node cost lives (per-visit `path::parse` of every constraint path, the
/// per-visit `groups`/sibling-index rebuild, and the per-node path allocations)
/// — the item-15 harness above only times the two template-independent passes.
/// Times each pass and the full `validate_composition` so the before/after of
/// the allocation-discipline rewrite is honest. Ignored by default (timing, not
/// correctness); run:
/// `cargo nextest run -p openehr-flat --run-ignored all \
///   -E 'test(measure_ips_validation_full_cost)' --no-capture`.
#[test]
#[ignore = "measurement, not a correctness gate — run with --run-ignored all"]
fn measure_ips_validation_full_cost() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/benchmark/templates/ckm"
    );
    let opt_xml = std::fs::read_to_string(format!("{dir}/international-patient-summary.opt"))
        .expect("read IPS OPT");
    let opt = openehr_its::opt14::from_xml(&opt_xml).expect("parse IPS OPT");
    let wt = openehr_flat::build_web_template(&opt).expect("build IPS WebTemplate");
    let comp: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{dir}/international-patient-summary.example.json"))
            .expect("read IPS example"),
    )
    .expect("parse IPS example");
    let node_count = count_type_nodes(&comp);

    // Warm up (allocator, branch predictors, the lazily-initialized bundle).
    for _ in 0..5 {
        std::hint::black_box(validate_composition(&comp, &wt).len());
    }

    // Public entry points only, so this harness compiles unchanged across the
    // item-31 rewrite (the internal pass signatures change; these do not).
    let iters = 50;
    let t_rmterm = time_pass(iters, || validate_rm_and_terminology(&comp).len());
    let t_walk = time_pass(iters, || validate_archetype_conformance(&comp, &wt).len());
    let t_all = time_pass(iters, || validate_composition(&comp, &wt).len());

    eprintln!("IPS full validation cost ({node_count} _type nodes, {iters} iters):");
    eprintln!("  passes 1+2 rm+terminology      : {t_rmterm:>8.1} us/op");
    eprintln!("  pass 3 walk (archetype conf.)  : {t_walk:>8.1} us/op");
    eprintln!("  full validate_composition      : {t_all:>8.1} us/op");
}
