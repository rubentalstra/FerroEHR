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
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

impl DvCount {
    // NOTE: `less_than` and `is_strictly_comparable_to` are realized for every
    // DV_ORDERED descendant by the `ordered_limit!` macro in `dv_ordered_impl`,
    // which is why they are not written here.

    /// Sum of this count and `other`, or `None` on overflow.
    ///
    /// Spec: `dv_count.adoc` §Functions `add` — "Sum of this `DV_COUNT` and
    /// `other`."
    ///
    /// The spec types the result as a `DV_COUNT`, not as a fallible one, but
    /// `magnitude` is an `Integer64` and the sum of two of them need not be
    /// one. Returning `Option` rather than wrapping or panicking follows the
    /// project's arithmetic rule: a load-bearing computation says so at the
    /// type level instead of producing a silently wrong clinical value.
    ///
    /// The other fields are NOT carried over: `accuracy`, `normal_range` and
    /// the reference ranges describe the measurement THIS value came from, and
    /// the spec attaches no meaning to their combination under addition.
    #[must_use]
    pub fn add(&self, other: &Self) -> Option<Self> {
        Some(Self::of_magnitude(
            self.magnitude.checked_add(other.magnitude)?,
        ))
    }

    /// Difference of this count and `other`, or `None` on overflow.
    ///
    /// Spec: `dv_count.adoc` §Functions `subtract`. A negative result is NOT
    /// refused: `magnitude` is a signed `Integer64` and the class declares no
    /// invariant bounding it below.
    #[must_use]
    pub fn subtract(&self, other: &Self) -> Option<Self> {
        Some(Self::of_magnitude(
            self.magnitude.checked_sub(other.magnitude)?,
        ))
    }

    /// Product of this count and `factor`, or `None` when the result is not a
    /// representable whole number.
    ///
    /// Spec: `dv_count.adoc` §Functions `multiply` — the factor is a `Real`
    /// while `magnitude` is an `Integer64`, and the spec says nothing about how
    /// a fractional product becomes a count. Rather than pick a rounding rule
    /// and present it as normative, a product that is not already whole is
    /// refused: `count * 2.5` is an answer openEHR does not define, and
    /// inventing one would be a silent wrong value in a clinical record.
    ///
    /// The arithmetic runs in [`Decimal`], not in binary floating point. The
    /// question "is this product exactly a whole count" is a decimal question,
    /// and asking it of an `f64` means casting and then guessing from a
    /// round-trip whether the answer survived. `Decimal` answers it directly:
    /// `is_integer` and `to_i64` are exact, and each step that can fail says so.
    #[must_use]
    pub fn multiply(&self, factor: f64) -> Option<Self> {
        // `from_f64_retain` keeps the float's exact value rather than rounding
        // to a display form, so nothing is lost before the check below; it
        // returns `None` for NaN and the infinities.
        let product =
            Decimal::from(self.magnitude).checked_mul(Decimal::from_f64_retain(factor)?)?;
        product
            .is_integer()
            .then(|| product.to_i64().map(Self::of_magnitude))?
    }

    /// A count of `magnitude` with every measurement-specific field cleared.
    fn of_magnitude(magnitude: i64) -> Self {
        Self {
            magnitude,
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            normal_range: None,
            normal_status: None,
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
        }
    }
}

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

    /// `dv_count.adoc` §Functions: add/subtract are magnitude arithmetic, and
    /// the result carries NONE of the measurement-specific fields — accuracy
    /// and the ranges describe the measurement each operand came from, and the
    /// spec gives no meaning to combining them.
    #[test]
    fn add_and_subtract_are_magnitude_arithmetic() {
        let a = count();
        let mut b = count();
        b.magnitude = 4;

        let sum = a.add(&b).expect("no overflow");
        assert_eq!(sum.magnitude, a.magnitude + 4);
        assert!(
            sum.accuracy.is_none() && sum.normal_range.is_none(),
            "measurement-specific fields belong to the operands, not the result"
        );
        assert_eq!(
            a.subtract(&b).expect("no overflow").magnitude,
            a.magnitude - 4
        );
    }

    /// Overflow is refused rather than wrapped. `magnitude` is an Integer64 and
    /// the sum of two need not be one; a wrapped count is a silently wrong
    /// clinical value.
    #[test]
    fn overflow_is_refused_not_wrapped() {
        let mut a = count();
        a.magnitude = i64::MAX;
        let mut b = count();
        b.magnitude = 1;
        assert!(a.add(&b).is_none());

        a.magnitude = i64::MIN;
        assert!(a.subtract(&b).is_none());
    }

    /// `multiply` takes a Real while `magnitude` is an Integer64, and the spec
    /// says nothing about how a fractional product becomes a count. A whole
    /// product is returned; a fractional one is REFUSED rather than rounded by
    /// a rule this implementation would be inventing.
    #[test]
    fn multiply_returns_whole_products_and_refuses_the_rest() {
        let mut a = count();
        a.magnitude = 6;
        assert_eq!(a.multiply(2.0).expect("whole").magnitude, 12);
        assert_eq!(a.multiply(0.5).expect("whole").magnitude, 3);
        assert!(a.multiply(0.4).is_none(), "2.4 is not a count");
        assert!(a.multiply(f64::INFINITY).is_none());
        assert!(a.multiply(f64::NAN).is_none());
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
