//! Hand-written RM/BASE class invariants for `PARTY_REF`.
//!
//! Mirrors archie `PartyRef`:
//! - `Type_validity`: `type` is one of the demographic/party types.
//! - `Namespace_valid`: inherited from `OBJECT_REF`.

use super::object_ref_impl::namespace_valid;
use super::party_ref::PartyRef;
use crate::validate::{InvariantViolation, Validate};

/// The party/demographic `type` values a `PARTY_REF` may point at.
///
/// The named subtypes + the abstract supertypes `PARTY`/`ACTOR` are the closed
/// set of the BASE `Type_validity` invariant
/// (`docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.party_ref.adoc`,
/// §Invariants). The class's own Description sanctions **abstract supertypes**
/// "if the referenced object is of a type not known by the current
/// implementation"; `ANY` is the openEHR Foundation-Types universal supertype
/// and the value the CNF platform corpus uses on its **positive** commit
/// fixtures (`CNF/.../create_composition-persistent.robot` "Alternative flow 1 …
/// TDD"; every `..__full` COMPOSITION/TDD sets `external_ref.type = "ANY"`). Per
/// By the CNF-outranks-prose rule the positive case wins over the strict enumeration, so
/// `ANY` is admitted; an *unknown* type string (e.g. a typo) is still rejected.
// NOTE (spec vs CNF): the normative invariant lists a closed set that does
// not include `ANY`; the CNF positive corpus commits `type="ANY"`. Admitting
// exactly `ANY` (the universal supertype) reconciles the two without opening the
// invariant to arbitrary strings — the intent the Description states.
const VALID_PARTY_TYPES: &[&str] = &[
    "PERSON",
    "ORGANISATION",
    "GROUP",
    "AGENT",
    "ROLE",
    "PARTY",
    "ACTOR",
    "ANY",
];

impl Validate for PartyRef {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !VALID_PARTY_TYPES.contains(&self.r#type.as_str()) {
            out.push(InvariantViolation::here(
                "Invariant Type_validity failed on type PARTY_REF",
            ));
        }
        if !namespace_valid(&self.namespace) {
            out.push(InvariantViolation::here(
                "Invariant Namespace_valid failed on type PARTY_REF",
            ));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;
    use crate::base_types::identification::hier_object_id::HierObjectId;
    use crate::base_types::identification::object_id::ObjectId;

    fn party_ref(namespace: &str, ty: &str) -> PartyRef {
        PartyRef {
            namespace: namespace.to_owned(),
            r#type: ty.to_owned(),
            id: ObjectId::HierObjectId(HierObjectId {
                value: "abc".to_owned(),
            }),
        }
    }

    #[test]
    fn valid_party_ref() {
        assert!(party_ref("local", "PERSON").invariants().is_empty());
    }

    /// `ANY` (the universal supertype) is admitted — the value the CNF positive
    /// commit corpus uses (see `VALID_PARTY_TYPES` doc; CNF wins).
    #[test]
    fn valid_party_ref_any_supertype() {
        assert!(party_ref("local", "ANY").invariants().is_empty());
    }

    #[test]
    fn invalid_type() {
        let v = party_ref("local", "SOMEONE").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Type_validity failed on type PARTY_REF"),
            "got {v:?}"
        );
    }

    #[test]
    fn invalid_namespace() {
        let v = party_ref("A*", "AGENT").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Namespace_valid failed on type PARTY_REF"),
            "got {v:?}"
        );
    }
}
