//! Hand-written RM class invariant for `DV_IDENTIFIER`.
//!
//! `Id_valid` (archie `DvIdentifier`, `nullOrNotEmpty`): `id` must be non-empty.

use crate::data_types::basic::dv_identifier::DvIdentifier;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvIdentifier {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::validate::generated::dv_identifier_core(&self.id, out);
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
