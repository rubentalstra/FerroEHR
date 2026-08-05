//! Hand-written RM class invariants for `DV_QUANTITY`.
//!
//! `DV_QUANTITY` declares no own invariants; it inherits the DV_AMOUNT
//! (`Accuracy_is_percent_validity`, `Accuracy_valid`) and DV_QUANTIFIED
//! (`Magnitude_status_valid`) invariants — see the shared helper.
//!
//! Plus the DV_ORDERED `Normal_range_and_status_consistency` invariant, via
//! the ordered-magnitude machinery in `dv_ordered_impl`.
//!
//! NOTE: the DV_ORDERED `Normal_status_validity` invariant (terminology)
//! is deferred to the composition validator + `openehr-term` (this crate has
//! no terminology dependency).

use crate::v1_2::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::v1_2::data_types::quantity::dv_quantity::DvQuantity;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvQuantity {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_2::validate::generated::dv_amount_core(
            "DV_QUANTITY",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
            out,
        );
        push_normal_range_consistency(
            out,
            "DV_QUANTITY",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            self,
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
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
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
                .contains(&"Invariant Accuracy_validity failed on type DV_QUANTITY".to_owned())
        );
        q.accuracy = Some(-1.0);
        assert!(
            messages(&q)
                .contains(&"Invariant Accuracy_validity failed on type DV_QUANTITY".to_owned())
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

    #[test]
    fn normal_range_and_status_consistency() {
        use crate::v1_2::data_types::quantity::dv_interval::DvInterval;
        use crate::v1_2::data_types::text::code_phrase::CodePhrase;
        use openehr_base::v1_3::prelude::TerminologyId;

        let range = |lo: f64, hi: f64| {
            Box::new(DvInterval {
                lower: Some(quantity_with(lo)),
                upper: Some(quantity_with(hi)),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            })
        };
        let status = |code: &str| CodePhrase {
            terminology_id: TerminologyId {
                value: "openehr_normal_statuses".to_owned(),
            },
            code_string: code.to_owned(),
            preferred_term: None,
        };

        // In range + status "N": consistent.
        let mut q = quantity(); // magnitude 43.0
        q.normal_range = Some(range(0.0, 100.0));
        q.normal_status = Some(status("N"));
        assert!(q.invariants().is_empty());

        // In range + status "H": inconsistent.
        let mut q = quantity();
        q.normal_range = Some(range(0.0, 100.0));
        q.normal_status = Some(status("H"));
        assert!(messages(&q).contains(
            &"Invariant Normal_range_and_status_consistency failed on type DV_QUANTITY".to_owned()
        ));

        // Out of range + status "N": inconsistent.
        let mut q = quantity();
        q.normal_range = Some(range(100.0, 200.0));
        q.normal_status = Some(status("N"));
        assert!(messages(&q).contains(
            &"Invariant Normal_range_and_status_consistency failed on type DV_QUANTITY".to_owned()
        ));

        // Out of range + status "H": consistent.
        let mut q = quantity();
        q.normal_range = Some(range(100.0, 200.0));
        q.normal_status = Some(status("H"));
        assert!(q.invariants().is_empty());
    }

    fn quantity_with(magnitude: f64) -> DvQuantity {
        let mut q = quantity();
        q.magnitude = magnitude;
        q
    }
}
