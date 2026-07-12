//! Hand-written RM class invariants for `DV_DATE`.
//!
//! `Value_valid`: `value` is a valid (possibly partial) ISO-8601 date. Plus the
//! inherited DV_QUANTIFIED `Magnitude_status_valid`.
//!
//! PORT NOTE: archie has no `@Invariant` for date well-formedness — it enforces
//! it at parse time via a typed temporal parse. Our `value` is a `String`, so we
//! express that guarantee explicitly (see `crate::validate` ISO-8601 helpers).

use crate::data_types::quantity::date_time::dv_date::DvDate;
use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::validate::{
    InvariantViolation, Validate, is_valid_iso_date, push_magnitude_status_valid,
};

impl Validate for DvDate {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !is_valid_iso_date(&self.value) {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_DATE",
            ));
        }
        push_magnitude_status_valid(out, "DV_DATE", self.magnitude_status.as_deref());
        // Inherited DV_ORDERED Normal_range_and_status_consistency (the
        // normal_range element type is the DvOrdered enum here).
        push_normal_range_consistency(
            out,
            "DV_DATE",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            &DvOrdered::DvDate(self.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> DvDate {
        DvDate {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            value: value.to_owned(),
        }
    }

    #[test]
    fn valid_date() {
        assert!(date("2021-05-17").invariants().is_empty());
        assert!(date("2021").invariants().is_empty());
    }

    #[test]
    fn invalid_date() {
        let v = date("2021-13-40").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Value_valid failed on type DV_DATE")
        );
    }
}
