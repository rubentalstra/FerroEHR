//! Hand-written RM/BASE class invariant (ADR-003) for `ISO_OID`.
//!
//! `UID.Value_valid`: the identifier string must be non-empty (archie `UID`,
//! `!Strings.isNullOrEmpty(value)`). Surfaces under the concrete type name.

use super::iso_oid::IsoOid;
use crate::validate::{InvariantViolation, Validate};

impl Validate for IsoOid {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type ISO_OID",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_valid() {
        assert!(
            IsoOid {
                value: "1.2.840".to_owned()
            }
            .invariants()
            .is_empty()
        );
        let v = IsoOid {
            value: String::new(),
        }
        .invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].message, "Invariant Value_valid failed on type ISO_OID");
    }
}
