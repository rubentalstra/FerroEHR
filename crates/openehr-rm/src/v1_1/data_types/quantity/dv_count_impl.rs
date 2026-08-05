// @generated-from-template templates/openehr-rm/data_types/quantity/dv_count_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `DV_COUNT`.
//!
//! `DV_COUNT` inherits the DV_AMOUNT / DV_QUANTIFIED invariants
//! (`Accuracy_is_percent_validity`, `Accuracy_valid`, `Magnitude_status_valid`)
//! plus the DV_ORDERED `Normal_range_and_status_consistency` (via the
//! ordered-magnitude machinery in `dv_ordered_impl`).

use crate::v1_1::data_types::quantity::dv_count::DvCount;
use crate::v1_1::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvCount {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_1::validate::generated::dv_amount_core(
            "DV_COUNT",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
            out,
        );
        push_normal_range_consistency(
            out,
            "DV_COUNT",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            self,
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
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
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
