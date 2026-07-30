//! Hand-written RM class invariants for `PARTY_IDENTIFIED`.
//!
//! Mirrors archie `PartyIdentified`:
//! - `Basic_validity`: at least one of `name`, `identifiers`, `external_ref`.
//! - `Name_valid`: if present, `name` is non-empty.

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
