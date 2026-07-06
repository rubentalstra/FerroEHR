//! Hand-written RM class invariant (ADR-003) for `ARCHETYPED`.
//!
//! `Rm_version_valid` (archie `Archetyped`, `nullOrNotEmpty`): `rm_version` must
//! be non-empty.

use crate::common::archetyped::archetyped::Archetyped;
use crate::validate::{InvariantViolation, Validate};

impl Validate for Archetyped {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.rm_version.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Rm_version_valid failed on type ARCHETYPED",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_base::prelude::ArchetypeId;

    fn archetyped(rm_version: &str) -> Archetyped {
        Archetyped {
            archetype_id: ArchetypeId {
                value: "openEHR-EHR-COMPOSITION.example.v1".to_owned(),
            },
            template_id: None,
            rm_version: rm_version.to_owned(),
        }
    }

    #[test]
    fn valid_rm_version() {
        assert!(archetyped("1.1.0").invariants().is_empty());
    }

    #[test]
    fn empty_rm_version_invalid() {
        assert_eq!(
            archetyped("").invariants()[0].message,
            "Invariant Rm_version_valid failed on type ARCHETYPED"
        );
    }
}
