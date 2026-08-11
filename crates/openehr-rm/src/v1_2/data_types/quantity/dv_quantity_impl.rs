// @generated-from-template templates/openehr-rm/data_types/quantity/dv_quantity_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
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

impl DvQuantity {
    /// Sum of this quantity and `other`, or `None` when they are not strictly
    /// comparable or the result is not a finite magnitude.
    ///
    /// Spec: `dv_quantity.adoc` §Functions `add` — "Sum of this `DV_QUANTITY`
    /// and `other`."
    ///
    /// The spec states no pre-condition on `add`, but `DV_ORDERED` says
    /// instances of `DV_QUANTITY` "can only be compared if they measure the
    /// same kind of physical quantity", and a sum of unlike quantities is no
    /// more meaningful than a comparison of them. So this requires
    /// [`Self::is_strictly_comparable_to`] — the same units, and the same
    /// `units_system` where one is stated — rather than silently producing a
    /// number whose unit is a guess.
    #[must_use]
    pub fn add(&self, other: &Self) -> Option<Self> {
        self.combine(other, self.magnitude + other.magnitude)
    }

    /// Difference of this quantity and `other`, under the same conditions as
    /// [`Self::add`].
    ///
    /// Spec: `dv_quantity.adoc` §Functions `subtract`.
    #[must_use]
    pub fn subtract(&self, other: &Self) -> Option<Self> {
        self.combine(other, self.magnitude - other.magnitude)
    }

    /// Product of this quantity and `factor`, or `None` when the result is not
    /// a finite magnitude.
    ///
    /// Spec: `dv_quantity.adoc` §Functions `multiply` — "Product of this
    /// `DV_QUANTITY` and `factor`."
    ///
    /// Unlike `DV_COUNT.multiply`, a fractional result needs no adjudication:
    /// a quantity's magnitude is a `Real`, so scaling one stays in the type.
    /// The units are unchanged — a scalar factor carries no dimension.
    #[must_use]
    pub fn multiply(&self, factor: f64) -> Option<Self> {
        self.rescaled(self.magnitude * factor)
    }

    /// This quantity combined with `other`, if they are strictly comparable.
    fn combine(&self, other: &Self, magnitude: f64) -> Option<Self> {
        if !self.is_strictly_comparable_to(other) {
            return None;
        }
        self.rescaled(magnitude)
    }

    /// This quantity at a new magnitude, keeping the units and dropping every
    /// field the spec gives no combination rule for.
    ///
    /// `precision`, `accuracy` and the reference ranges describe how THIS value
    /// was measured. The spec says nothing about the precision of a sum or the
    /// accuracy of a scaled value, so carrying either would be this
    /// implementation inventing a rule and presenting it as the model's.
    fn rescaled(&self, magnitude: f64) -> Option<Self> {
        // A non-finite magnitude is not a quantity: the class types it as a
        // `Real`, and NaN or an infinity would flow into comparisons and
        // serialization as a value no reader can act on.
        if !magnitude.is_finite() {
            return None;
        }
        Some(Self {
            magnitude,
            units: self.units.clone(),
            units_system: self.units_system.clone(),
            units_display_name: self.units_display_name.clone(),
            precision: None,
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            normal_range: None,
            normal_status: None,
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
        })
    }
}

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

    /// Unlike quantities do not add. `DV_ORDERED` says instances "can only be
    /// compared if they measure the same kind of physical quantity", and a sum
    /// of kg and m is no more meaningful than a comparison of them — so the
    /// refusal is asserted, not just the success.
    #[test]
    fn unlike_units_do_not_combine() {
        let kg = quantity();
        let mut metres = quantity();
        metres.units = "m".to_owned();

        assert!(kg.add(&metres).is_none(), "kg + m has no unit");
        assert!(kg.subtract(&metres).is_none());

        let mut other_kg = quantity();
        other_kg.magnitude = 7.0;
        let sum = kg.add(&other_kg).expect("same units");
        assert!((sum.magnitude - 50.0).abs() < f64::EPSILON);
        assert_eq!(sum.units, "kg", "the sum stays in the operands' units");
    }

    /// A units_system stated on one side and not the other is NOT the same
    /// quantity kind: the existing comparability rule requires both to match,
    /// and arithmetic follows the same rule rather than a looser one.
    #[test]
    fn a_units_system_mismatch_does_not_combine() {
        let plain = quantity();
        let mut systematised = quantity();
        systematised.units_system = Some("http://unitsofmeasure.org".to_owned());
        assert!(plain.add(&systematised).is_none());
    }

    /// The measurement-specific fields describe how THIS value was measured.
    /// The spec defines neither the precision of a sum nor the accuracy of a
    /// scaled value, so the result carries neither rather than this
    /// implementation inventing a rule.
    #[test]
    fn arithmetic_drops_what_the_spec_gives_no_rule_for() {
        let mut a = quantity();
        a.accuracy = Some(0.5);
        a.accuracy_is_percent = Some(false);
        let scaled = a.multiply(2.0).expect("finite");
        assert!((scaled.magnitude - 86.0).abs() < f64::EPSILON);
        assert_eq!(scaled.units, "kg");
        assert!(scaled.precision.is_none() && scaled.accuracy.is_none());
    }

    /// A magnitude that is not finite is not a quantity: NaN or an infinity
    /// would flow into comparison and serialization as a value no reader can
    /// act on.
    #[test]
    fn a_non_finite_result_is_refused() {
        let q = quantity();
        assert!(q.multiply(f64::INFINITY).is_none());
        assert!(q.multiply(f64::NAN).is_none());
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
