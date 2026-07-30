//! The #679 reproduction: the CNF corpus fixture uses the ANONYMOUS
//! archetype-slot form ADL1.4 master05-cadl.adoc §Archetype Slots itself
//! writes (`allow_archetype ITEM_TREE matches { include … }`), and must
//! parse + phase-1-validate as ADL 1.4.

use openehr_adl::validate::validate_source_phase1_adl14;

#[test]
fn cnf_adl14_medication_fixture_parses_and_validates() {
    let src = include_str!(
        "../../../../tools/cnf-runner/artifacts/corpus/fixtures/archetypes/adl14.medication.v1.adl"
    );
    let issues = validate_source_phase1_adl14(src).expect(
        "the fixture must parse as ADL 1.4 (anonymous slot form, master05 §Archetype Slots)",
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == openehr_adl::validate::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected phase-1 errors: {errors:?}");
}
