//! Hand-written RM class invariants for `PARTY_IDENTIFIED`.
//!
//! Spec: RM `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.party_identified.adoc`
//! §Invariants declares three:
//!
//! - `Basic_validity`: `name /= Void or identifiers /= Void or external_ref
//!   /= Void` — realized here, through the generated `party_identified_core`.
//! - `Name_valid`: `name /= Void implies not name.is_empty` — same core.
//! - `Identifiers_valid`: `identifiers /= Void implies not
//!   identifiers.is_empty` — NOT realized here: the BMM `List` emits as a
//!   `Vec`, so an absent and a present-but-empty `identifiers` are the same
//!   value in the typed model and the rule has nothing to distinguish. The
//!   emittable-invariant realization register (`crate::validate::generated`
//!   module header) carries that verdict.

use crate::common::generic::party_identified::PartyIdentifiedData;
use crate::validate::{InvariantViolation, Validate};

impl Validate for PartyIdentifiedData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::validate::generated::party_identified_core(
            "PARTY_IDENTIFIED",
            self.name.as_deref(),
            !self.identifiers.is_empty(),
            self.external_ref.is_some(),
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(name: Option<&str>) -> PartyIdentifiedData {
        PartyIdentifiedData {
            external_ref: None,
            name: name.map(str::to_owned),
            identifiers: Vec::new(),
        }
    }

    #[test]
    fn named_party_valid() {
        assert!(party(Some("Dr Jones")).invariants().is_empty());
    }

    #[test]
    fn no_identity_invalid() {
        let v = party(None).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Basic_validity failed on type PARTY_IDENTIFIED"),
            "got {v:?}"
        );
    }

    #[test]
    fn empty_name_invalid() {
        let v = party(Some("")).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Name_valid failed on type PARTY_IDENTIFIED"),
            "got {v:?}"
        );
    }
}
