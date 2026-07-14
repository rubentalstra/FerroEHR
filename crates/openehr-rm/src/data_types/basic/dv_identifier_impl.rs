//! Hand-written RM class invariant for `DV_IDENTIFIER`.
//!
//! `Id_valid` (archie `DvIdentifier`, `nullOrNotEmpty`): `id` must be non-empty.

use crate::data_types::basic::dv_identifier::DvIdentifier;
use crate::validate::{InvariantViolation, Validate};

/// The `Id_valid` core over the projected input — one source for the typed
/// impl and the value-level fast path (`validate::fast`).
pub(crate) fn push_dv_identifier_invariants(id: &str, out: &mut Vec<InvariantViolation>) {
    if id.is_empty() {
        out.push(InvariantViolation::here(
            "Invariant Id_valid failed on type DV_IDENTIFIER",
        ));
    }
}

impl Validate for DvIdentifier {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_dv_identifier_invariants(&self.id, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(id: &str) -> DvIdentifier {
        DvIdentifier {
            issuer: None,
            assigner: None,
            id: id.to_owned(),
            r#type: None,
        }
    }

    #[test]
    fn valid_id() {
        assert!(identifier("NHS-12345").invariants().is_empty());
    }

    #[test]
    fn empty_id_invalid() {
        let v = identifier("").invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Id_valid failed on type DV_IDENTIFIER"
        );
    }
}
