//! Hand-written RM class invariants (ADR-003) for `DV_TIME`.
//!
//! `Value_valid`: `value` is a valid (possibly partial) ISO-8601 time. Plus the
//! inherited DV_QUANTIFIED `Magnitude_status_valid`. See `dv_date_impl` for the
//! PORT NOTE on why this is an explicit invariant.

use crate::data_types::quantity::date_time::dv_time::DvTime;
use crate::validate::{
    InvariantViolation, Validate, is_valid_iso_time, push_magnitude_status_valid,
};

impl Validate for DvTime {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !is_valid_iso_time(&self.value) {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_TIME",
            ));
        }
        push_magnitude_status_valid(out, "DV_TIME", self.magnitude_status.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(value: &str) -> DvTime {
        DvTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
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
