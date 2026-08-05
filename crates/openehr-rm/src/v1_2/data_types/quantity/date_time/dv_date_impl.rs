// @generated-from-template templates/openehr-rm/data_types/quantity/date_time/dv_date_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `DV_DATE`.
//!
//! `Value_valid`: `value` is a valid (possibly partial) ISO-8601 date. Plus the
//! inherited DV_QUANTIFIED `Magnitude_status_valid`.
//!
//! NOTE: archie has no `@Invariant` for date well-formedness — it enforces
//! it at parse time via a typed temporal parse. Our `value` is a `String`, so we
//! express that guarantee explicitly (see `crate::v1_2::validate` ISO-8601 helpers).

use crate::v1_2::data_types::quantity::date_time::dv_date::DvDate;
use crate::v1_2::data_types::quantity::dv_ordered::DvOrdered;
use crate::v1_2::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::v1_2::validate::is_valid_iso_date;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvDate {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_2::validate::generated::temporal_value_core(
            "DV_DATE",
            is_valid_iso_date(&self.value),
            out,
        );
        crate::v1_2::validate::generated::magnitude_status_core(
            "DV_DATE",
            self.magnitude_status.as_deref(),
            out,
        );
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
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
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
