// @generated-from-template templates/openehr-rm/data_types/quantity/date_time/dv_duration_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `DV_DURATION`.
//!
//! `Value_valid`: `value` is a valid ISO-8601 duration (openEHR permits a
//! leading sign and a `W` designator mixed with the others). Plus the inherited
//! DV_AMOUNT / DV_QUANTIFIED invariants (`DV_DURATION` extends `DV_AMOUNT`). See
//! `dv_date_impl` for the NOTE on why value well-formedness is explicit.

use crate::v1_2::data_types::quantity::date_time::dv_duration::DvDuration;
use crate::v1_2::data_types::quantity::dv_ordered::DvOrdered;
use crate::v1_2::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::v1_2::validate::is_valid_iso_duration;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvDuration {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_2::validate::generated::temporal_value_core(
            "DV_DURATION",
            is_valid_iso_duration(&self.value),
            out,
        );
        crate::v1_2::validate::generated::dv_amount_core(
            "DV_DURATION",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
            out,
        );
        // Inherited DV_ORDERED Normal_range_and_status_consistency.
        push_normal_range_consistency(
            out,
            "DV_DURATION",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            &DvOrdered::DvDuration(self.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duration(value: &str) -> DvDuration {
        DvDuration {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            value: value.to_owned(),
        }
    }

    #[test]
    fn valid_duration() {
        assert!(duration("P1Y2M10DT2H30M").invariants().is_empty());
        assert!(duration("PT10H").invariants().is_empty());
    }

    #[test]
    fn invalid_duration() {
        let v = duration("10 hours").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Value_valid failed on type DV_DURATION")
        );
    }
}
