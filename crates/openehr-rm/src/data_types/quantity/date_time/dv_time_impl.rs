//! Hand-written RM class invariants for `DV_TIME`.
//!
//! `Value_valid`: `value` is a valid (possibly partial) ISO-8601 time. Plus the
//! inherited DV_QUANTIFIED `Magnitude_status_valid`. See `dv_date_impl` for the
//! NOTE on why this is an explicit invariant.

use crate::data_types::quantity::date_time::dv_time::DvTime;
use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::validate::is_valid_iso_time;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvTime {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::validate::generated::temporal_value_core(
            "DV_TIME",
            is_valid_iso_time(&self.value),
            out,
        );
        crate::validate::generated::magnitude_status_core(
            "DV_TIME",
            self.magnitude_status.as_deref(),
            out,
        );
        // Inherited DV_ORDERED Normal_range_and_status_consistency.
        push_normal_range_consistency(
            out,
            "DV_TIME",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            &DvOrdered::DvTime(self.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(value: &str) -> DvTime {
        DvTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
            magnitude_status: None,
            accuracy: None,
            value: value.to_owned(),
        }
    }

    #[test]
    fn valid_time() {
        assert!(time("10:30:00").invariants().is_empty());
        assert!(time("10:30:00+01:00").invariants().is_empty());
    }

    #[test]
    fn invalid_time() {
        let v = time("25:99").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Value_valid failed on type DV_TIME")
        );
    }
}
