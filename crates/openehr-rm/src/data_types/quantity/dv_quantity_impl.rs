//! Hand-written RM class invariants (ADR-003) for `DV_QUANTITY`.
//!
//! `DV_QUANTITY` declares no own invariants; it inherits the DV_AMOUNT
//! (`Accuracy_is_percent_validity`, `Accuracy_valid`) and DV_QUANTIFIED
//! (`Magnitude_status_valid`) invariants — see the shared helper.
//!
//! PORT NOTE: the DV_ORDERED `Normal_status_validity` (terminology) and
//! `Normal_range_and_status_consistency` (magnitude comparison) invariants are
//! deferred — the former needs `openehr-term`, the latter the P16
//! `openehr_magnitude` machinery.

use crate::data_types::quantity::dv_quantity::DvQuantity;
use crate::validate::{InvariantViolation, Validate, push_dv_amount_invariants};

impl Validate for DvQuantity {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_dv_amount_invariants(
            out,
            "DV_QUANTITY",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quantity() -> DvQuantity {
        DvQuantity {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            magnitude: 43.0,
            precision: Some(0),
            units: "kg".to_owned(),
            units_system: None,
            units_display_name: None,
        }
    }

    fn messages(q: &DvQuantity) -> Vec<String> {
        q.invariants().into_iter().map(|v| v.message).collect()
    }

    #[test]
    fn valid_quantity() {
        let mut q = quantity();
        q.magnitude_status = Some("=".to_owned());
        q.accuracy = Some(100.0);
        q.accuracy_is_percent = Some(true);
        assert!(q.invariants().is_empty());
    }

    #[test]
    fn percent_accuracy_out_of_range() {
        let mut q = quantity();
        q.accuracy_is_percent = Some(true);
        q.accuracy = Some(101.0);
        assert!(
            messages(&q)
                .contains(&"Invariant Accuracy_valid failed on type DV_QUANTITY".to_owned())
        );
        q.accuracy = Some(-1.0);
        assert!(
            messages(&q)
                .contains(&"Invariant Accuracy_valid failed on type DV_QUANTITY".to_owned())
        );
    }

    #[test]
    fn zero_accuracy_cannot_be_percent() {
        let mut q = quantity();
        q.accuracy_is_percent = Some(true);
        q.accuracy = Some(0.0);
        assert!(messages(&q).contains(
            &"Invariant Accuracy_is_percent_validity failed on type DV_QUANTITY".to_owned()
        ));
    }

    #[test]
    fn invalid_magnitude_status() {
        let mut q = quantity();
        q.magnitude_status = Some("bigger than".to_owned());
        assert!(
            messages(&q).contains(
                &"Invariant Magnitude_status_valid failed on type DV_QUANTITY".to_owned()
            )
        );
    }
}
