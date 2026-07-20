//! The ADL 1.4 phase-1 validation subset
//! ([`openehr_adl::validate::validate_source_phase1_adl14`]).
//!
//! A 1.4 source is judged **as 1.4** (never post-conversion): the checks that
//! correspond to an ADL 1.4 / AOM 1.4 standalone rule run; the AOM2-only rules
//! that would false-reject a valid 1.4 archetype (VARAV/VARRV/VCOID-strict/
//! VATCV-form/VCOSU-archetype-wide/WOUC) are suppressed. Oracle:
//! `docs/specs/openehr/AM/docs/ADL1.4/master08-adl.adoc` §Validity Rules +
//! `docs/specs/openehr/AM/docs/AOM1.4/` class invariants.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use openehr_adl::validate::{Severity, validate_source_phase1_adl14};

/// A minimal, valid standalone ADL 1.4 archetype (`adl_version=1.4`, no
/// `rm_release`, at-code node ids, `ontology` keyword) — the exact shape a 1.4
/// upload carries.
const VALID_14: &str = "\u{feff}archetype (adl_version=1.4)
\topenEHR-EHR-OBSERVATION.test14.v1

concept
\t[at0000]\t-- Test 14
language
\toriginal_language = <[ISO_639-1::en]>
description
\tlifecycle_state = <\"Initial\">
\tdetails = <
\t\t[\"en\"] = <
\t\t\tlanguage = <[ISO_639-1::en]>
\t\t\tpurpose = <\"testing\">
\t\t>
\t>
definition
\tOBSERVATION[at0000] matches {
\t\tdata matches {
\t\t\tHISTORY matches {*}
\t\t}
\t}
ontology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\titems = <
\t\t\t\t[\"at0000\"] = <
\t\t\t\t\ttext = <\"Test 14\">
\t\t\t\t\tdescription = <\"A test archetype.\">
\t\t\t\t>
\t\t\t>
\t\t>
\t>
";

fn errors(src: &str) -> Vec<String> {
    let issues = validate_source_phase1_adl14(src).expect("1.4 source parses");
    issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code.mnemonic().to_owned())
        .collect()
}

#[test]
fn valid_14_archetype_is_clean() {
    // A well-formed 1.4 archetype validates clean — and specifically does NOT
    // raise VARAV (adl_version=1.4) or VARRV (no rm_release), the AOM2-only
    // rules the 1.4 subset suppresses.
    let errs = errors(VALID_14);
    assert!(errs.is_empty(), "expected clean, got {errs:?}");
    assert!(
        !errs.contains(&"VARAV".to_owned()) && !errs.contains(&"VARRV".to_owned()),
        "the adl_version/rm_release 3-part rules must not apply to 1.4"
    );
}

#[test]
fn definition_type_mismatch_raises_vardt() {
    // ADL1.4 master08 §Validity Rules VARDT: the topmost definition typename
    // must match the RM class of the archetype id. Corresponds to the engine's
    // VARDT (a shared ADL 1.4 / AOM2 rule) — it still fires in the 1.4 subset.
    let bad = VALID_14.replace("OBSERVATION[at0000] matches", "CLUSTER[at0000] matches");
    assert!(
        errors(&bad).contains(&"VARDT".to_owned()),
        "a definition type not matching the id class must raise VARDT"
    );
}

#[test]
fn undefined_node_code_raises_vatdf() {
    // ADL1.4 master08 §Validity Rules VATDF: every at-code used as a node
    // identifier must be defined in the ontology. Add a data node with an
    // at-code the ontology does not define.
    let bad = VALID_14.replace(
        "HISTORY matches {*}",
        "HISTORY[at0001] matches {*}", // at0001 is not in term_definitions
    );
    assert!(
        errors(&bad).contains(&"VATDF".to_owned()),
        "an undefined node-id at-code must raise VATDF, got {:?}",
        errors(&bad)
    );
}

#[test]
fn unparseable_source_is_a_parse_error() {
    assert!(validate_source_phase1_adl14("this is not an archetype").is_err());
}
