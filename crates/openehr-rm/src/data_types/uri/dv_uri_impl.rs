//! Hand-written RM class invariant (ADR-003) for `DV_URI`.
//!
//! `Value_valid` (archie `DvURI`): the URI value must be non-empty.

use crate::data_types::uri::dv_uri::DvUriData;
use crate::validate::{InvariantViolation, Validate};

impl Validate for DvUriData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_URI",
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
            DvUriData {
                value: "http://example.org/x".to_owned()
            }
            .invariants()
            .is_empty()
        );
        let v = DvUriData {
            value: String::new(),
        }
        .invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].message, "Invariant Value_valid failed on type DV_URI");
    }
}
