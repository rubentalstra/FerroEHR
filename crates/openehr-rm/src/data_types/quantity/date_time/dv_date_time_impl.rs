//! Hand-written RM class invariants (ADR-003) for `DV_DATE_TIME`.
//!
//! `Value_valid`: `value` is a valid (possibly partial) ISO-8601 date-time. Plus
//! the inherited DV_QUANTIFIED `Magnitude_status_valid`. See `dv_date_impl` for
//! the PORT NOTE on why this is an explicit invariant.

use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
use crate::validate::{
    InvariantViolation, Validate, is_valid_iso_date_time, push_magnitude_status_valid,
};

impl Validate for DvDateTime {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !is_valid_iso_date_time(&self.value) {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_DATE_TIME",
            ));
        }
        push_magnitude_status_valid(out, "DV_DATE_TIME", self.magnitude_status.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_time(value: &str) -> DvDateTime {
        DvDateTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            value: value.to_owned(),
        }
    }

    #[test]
    fn valid_date_time() {
        assert!(date_time("2021-05-17T10:30:00").invariants().is_empty());
        assert!(date_time("2021-05-17T10:30:00Z").invariants().is_empty());
    }

    #[test]
    fn invalid_date_time() {
        let v = date_time("2021-05-17T99:00:00").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Value_valid failed on type DV_DATE_TIME")
        );
    }
}
