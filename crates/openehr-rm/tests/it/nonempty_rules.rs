#![allow(
    clippy::expect_used,
    reason = "test fixtures/diagnostics — a malformed fixture should fail loudly"
)]
//! Pins for the `x /= Void implies not x.is_empty` rules the #1623 register
//! sweep moved out of the Unrealized bucket: each rule of the generated
//! `NONEMPTY_LIST_RULES` table refuses a PRESENT-but-empty optional list and
//! accepts both the absent and the populated forms
//! (`openehr_rm::validate::nonempty_list_violations`, evaluated at the wire
//! boundary by `openehr-its`).

use serde_json::json;

/// `(class, attribute, invariant)` — the four rows #1623 re-adjudicated from
/// Unrealized to the generated core (RM `UML/classes` §Invariants of
/// `org.openehr.rm.common.party_identified.adoc`,
/// `org.openehr.rm.common.original_version.adoc`,
/// `org.openehr.rm.ehr_extract.extract_update_spec.adoc`).
const REGISTERED: &[(&str, &str, &str)] = &[
    ("PARTY_IDENTIFIED", "identifiers", "Identifiers_valid"),
    ("ORIGINAL_VERSION", "attestations", "Attestations_valid"),
    (
        "ORIGINAL_VERSION",
        "other_input_version_uids",
        "Other_input_version_uids_valid",
    ),
    (
        "EXTRACT_UPDATE_SPEC",
        "trigger_events",
        "Trigger_events_validity",
    ),
];

#[test]
fn present_but_empty_refuses_absent_and_populated_pass() {
    for (class, attribute, invariant) in REGISTERED {
        // A silently-dropped table row fails the violation assertion below,
        // so table presence needs no direct probe (the table is crate-private
        // by design — the public seam is `nonempty_list_violations`).
        let mut out = Vec::new();
        openehr_rm::validate::nonempty_list_violations(class, &json!({ *attribute: [] }), &mut out);
        assert!(
            out.iter()
                .any(|v| v.message.contains(invariant) && v.message.contains(class)),
            "{class}.{attribute} = [] must violate {invariant}, got {out:?}"
        );

        let mut absent = Vec::new();
        openehr_rm::validate::nonempty_list_violations(class, &json!({}), &mut absent);
        assert!(
            absent.is_empty(),
            "an ABSENT {class}.{attribute} is legal (0..1), got {absent:?}"
        );

        let mut populated = Vec::new();
        openehr_rm::validate::nonempty_list_violations(
            class,
            &json!({ *attribute: [{"_type": "X"}] }),
            &mut populated,
        );
        assert!(
            populated.is_empty(),
            "a populated {class}.{attribute} passes this rule, got {populated:?}"
        );
    }
}
