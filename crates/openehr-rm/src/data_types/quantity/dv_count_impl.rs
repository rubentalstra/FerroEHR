//! Hand-written RM class invariants (ADR-003) for `DV_COUNT`.
//!
//! `DV_COUNT` inherits the DV_AMOUNT / DV_QUANTIFIED invariants
//! (`Accuracy_is_percent_validity`, `Accuracy_valid`, `Magnitude_status_valid`).

use crate::data_types::quantity::dv_count::DvCount;
use crate::validate::{InvariantViolation, Validate, push_dv_amount_invariants};

impl Validate for DvCount {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_dv_amount_invariants(
            out,
            "DV_COUNT",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count() -> DvCount {
        DvCount {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            magnitude: 3,
        }
    }

    #[test]
    fn valid_count() {
        assert!(count().invariants().is_empty());
    }

    #[test]
    fn invalid_magnitude_status() {
        let mut c = count();
        c.magnitude_status = Some("approx".to_owned());
        assert!(
            c.invariants()
                .iter()
                .any(|v| v.message == "Invariant Magnitude_status_valid failed on type DV_COUNT")
        );
    }
}
