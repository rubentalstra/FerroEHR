//! Ingestion-side artefact-validity tests (B2 task 6).
//!
//! Each per-code test takes the vendored valid `minimal_evaluation.opt` and
//! breaks exactly one aspect (either as a targeted XML mutation or as a typed
//! model mutation for the terminology-side sets the minimal fixture does not
//! carry), asserting the matching AOM2 rule code surfaces. The corpus guard
//! asserts every vendored valid OPT still passes unchanged.

use std::path::{Path, PathBuf};

use openehr_base::prelude::{CodePhrase, TerminologyId};
use openehr_its::opt14::{
    self, ArchetypeTerm, Codedefinitionset, ConstraintBindingItem, Constraintbindingset,
    FlatArchetypeOntology, OperationalTemplate, TermBindingItem, Termbindingset,
};

use super::validate_opt_artefact;
use crate::service::ServiceError;

fn manifest() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

/// The vendored valid minimal OPT (the mutation base).
fn minimal_xml() -> String {
    let p = Path::new(manifest()).join(
        "../../docs/specs/openehr/CNF/tests/platform/robot/\
         _resources/test_data_sets/valid_templates/minimal/minimal_evaluation.opt",
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn parse(xml: &str) -> OperationalTemplate {
    opt14::from_xml(xml).expect("minimal OPT parses")
}

/// Assert the artefact is rejected with the given AOM2 rule code in the message.
fn expect_code(opt: &OperationalTemplate, code: &str) {
    let err = validate_opt_artefact(opt).expect_err("expected a violation");
    let msg = err.to_string();
    assert!(msg.contains(code), "expected `{code}` in error, got: {msg}");
}

/// Replace the first `from` occurring *after* `marker` with `to`.
fn mutate_after(xml: &str, marker: &str, from: &str, to: &str) -> String {
    let idx = xml
        .find(marker)
        .unwrap_or_else(|| panic!("marker not found: {marker}"));
    let (head, tail) = xml.split_at(idx);
    let new_tail = tail.replacen(from, to, 1);
    assert_ne!(new_tail, tail, "no `{from}` found after `{marker}`");
    format!("{head}{new_tail}")
}

fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
    CodePhrase {
        terminology_id: TerminologyId {
            value: terminology.to_owned(),
        },
        code_string: code.to_owned(),
        preferred_term: None,
    }
}

// ── sanity + corpus guard ───────────────────────────────────────────────────

#[test]
fn valid_minimal_passes() {
    let opt = parse(&minimal_xml());
    validate_opt_artefact(&opt).expect("the vendored minimal OPT is valid");
}

/// Every vendored valid OPT (the 91-file corpus, the same set the
/// `openehr-its` `opt14_corpus` gate parses) must still upload — the new
/// ingestion checks may never mis-reject a legitimate template.
#[test]
fn corpus_all_valid_opts_pass() {
    let dir = PathBuf::from(manifest()).join("tests/resources/service");
    let mut files = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("read {}: {e}", d.display()))
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "opt") {
                files.push(path);
            }
        }
    }
    files.sort();
    assert!(
        files.len() >= 90,
        "expected the full OPT corpus (~91 files), found {}",
        files.len()
    );

    let mut failures = Vec::new();
    for path in &files {
        let xml = std::fs::read_to_string(path).expect("read opt");
        let opt = match opt14::from_xml(&xml) {
            Ok(opt) => opt,
            // Parseability is the `opt14_corpus` gate's job, not ours.
            Err(_) => continue,
        };
        if let Err(e) = validate_opt_artefact(&opt) {
            failures.push(format!("{}: {e}", path.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "these valid vendored OPTs were mis-rejected by the ingestion checks:\n{}",
        failures.join("\n")
    );
}

#[test]
fn rejection_is_400_bad_request() {
    // VCARM path — assert the wire status is 400 (CNF `validate_opt-invalid_opt`
    // asserts "status code 400"; `ServiceError::BadRequest` → ITS-REST 400).
    let xml = minimal_xml().replace(
        "<rm_attribute_name>category</rm_attribute_name>",
        "<rm_attribute_name>bogus_attr</rm_attribute_name>",
    );
    let opt = parse(&xml);
    let err = validate_opt_artefact(&opt).unwrap_err();
    assert!(
        matches!(err, ServiceError::BadRequest(_)),
        "expected BadRequest (400), got {err:?}"
    );
}

// ── RM conformance (VCORM/VCARM/VCAEX/VCACA/VCAM) ────────────────────────────

#[test]
fn vcarm_unknown_rm_attribute() {
    // COMPOSITION has no `bogus_attr` attribute.
    let xml = minimal_xml().replace(
        "<rm_attribute_name>category</rm_attribute_name>",
        "<rm_attribute_name>bogus_attr</rm_attribute_name>",
    );
    expect_code(&parse(&xml), "VCARM");
}

#[test]
fn vcorm_unknown_rm_type() {
    // `NOT_A_TYPE` is not a reference-model class.
    let xml = minimal_xml().replace(
        "<rm_type_name>DV_QUANTITY</rm_type_name>",
        "<rm_type_name>NOT_A_TYPE</rm_type_name>",
    );
    expect_code(&parse(&xml), "VCORM");
}

#[test]
fn vcam_container_on_single_valued_attribute() {
    // `content` is a C_MULTIPLE_ATTRIBUTE; `language` is single-valued in the RM.
    let xml = minimal_xml().replace(
        "<rm_attribute_name>content</rm_attribute_name>",
        "<rm_attribute_name>language</rm_attribute_name>",
    );
    expect_code(&parse(&xml), "VCAM");
}

#[test]
fn vcaex_widened_existence_on_mandatory_attribute() {
    // `category` is mandatory in COMPOSITION; drop its existence lower to 0.
    let xml = mutate_after(
        &minimal_xml(),
        "<rm_attribute_name>category</rm_attribute_name>",
        "<lower>1</lower>",
        "<lower>0</lower>",
    );
    expect_code(&parse(&xml), "VCAEX");
}

// ── VACMCO / VCOC (occurrences vs cardinality) ──────────────────────────────

#[test]
fn vacmco_children_exceed_cardinality_upper() {
    // Bound the `items` container cardinality to an upper of 1, and require the
    // single ELEMENT child to occur at least twice → the mandatory children
    // cannot fit within the cardinality range.
    let xml = minimal_xml();
    let xml = mutate_after(
        &xml,
        "<rm_type_name>ELEMENT</rm_type_name>",
        "<lower>0</lower>",
        "<lower>2</lower>",
    );
    // The first open cardinality upper after ELEMENT is the `items` container's.
    let xml = mutate_after(
        &xml,
        "<rm_type_name>ELEMENT</rm_type_name>",
        "<upper_unbounded>true</upper_unbounded>",
        "<upper_unbounded>false</upper_unbounded><upper>1</upper>",
    );
    expect_code(&parse(&xml), "VACMCO");
}

// ── VATID (node-id codes defined in terminology) ────────────────────────────

#[test]
fn vatid_undefined_node_id() {
    // Point a node at an at-code that no term definition defines.
    let xml = minimal_xml().replace("<node_id>at0001</node_id>", "<node_id>at9999</node_id>");
    expect_code(&parse(&xml), "VATID");
}

// ── VTTBK / VTCBK / VTLC (terminology sets — typed mutations) ────────────────

#[test]
fn vttbk_undefined_term_binding_key() {
    let mut opt = parse(&minimal_xml());
    opt.definition.term_bindings.push(Termbindingset {
        terminology: "SNOMED-CT".to_owned(),
        items: vec![TermBindingItem {
            // at7777 is not a defined archetype term, nor a path.
            code: "at7777".to_owned(),
            value: code_phrase("SNOMED-CT", "1234"),
        }],
    });
    expect_code(&opt, "VTTBK");
}

#[test]
fn vtcbk_undefined_constraint_binding_key() {
    let mut opt = parse(&minimal_xml());
    opt.ontology = Some(FlatArchetypeOntology {
        archetype_id: "openEHR-EHR-COMPOSITION.minimal.v1".to_owned(),
        term_definitions: Vec::new(),
        constraint_definitions: Vec::new(), // no ac-codes defined
        term_bindings: Vec::new(),
        constraint_bindings: vec![Constraintbindingset {
            terminology: "SNOMED-CT".to_owned(),
            items: vec![ConstraintBindingItem {
                code: "ac9999".to_owned(),
                value: "http://example.org/vs".to_owned(),
            }],
        }],
    });
    expect_code(&opt, "VTCBK");
}

#[test]
fn vtlc_language_code_set_mismatch() {
    let mut opt = parse(&minimal_xml());
    let term = |code: &str| ArchetypeTerm {
        code: code.to_owned(),
        items: Default::default(),
    };
    opt.ontology = Some(FlatArchetypeOntology {
        archetype_id: "openEHR-EHR-COMPOSITION.minimal.v1".to_owned(),
        term_definitions: vec![
            Codedefinitionset {
                language: "en".to_owned(),
                items: vec![term("at0000"), term("at0001")],
            },
            Codedefinitionset {
                language: "de".to_owned(),
                // at0001 missing in `de` → language inconsistency.
                items: vec![term("at0000")],
            },
        ],
        constraint_definitions: Vec::new(),
        term_bindings: Vec::new(),
        constraint_bindings: Vec::new(),
    });
    expect_code(&opt, "VTLC");
}
