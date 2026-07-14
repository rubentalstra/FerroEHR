//! Hand-written RM class invariants for `DV_DURATION`.
//!
//! `Value_valid`: `value` is a valid ISO-8601 duration (openEHR permits a
//! leading sign and a `W` designator mixed with the others). Plus the inherited
//! DV_AMOUNT / DV_QUANTIFIED invariants (`DV_DURATION` extends `DV_AMOUNT`). See
//! `dv_date_impl` for the PORT NOTE on why value well-formedness is explicit.

use crate::data_types::quantity::date_time::dv_duration::DvDuration;
use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::validate::{
    InvariantViolation, Validate, is_valid_iso_duration, push_dv_amount_invariants,
    push_temporal_value_valid,
};

impl Validate for DvDuration {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_temporal_value_valid(out, "DV_DURATION", is_valid_iso_duration(&self.value));
        push_dv_amount_invariants(
            out,
            "DV_DURATION",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
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
            other_reference_ranges: Vec::new(),
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
