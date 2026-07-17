//! Hand-written RM class invariants for `DV_PROPORTION`.
//!
//! Mirrors archie `DvProportion` (`ProportionKind` = ratio 0, unitary 1,
//! percent 2, fraction 3, integer_fraction 4), plus the inherited DV_AMOUNT /
//! DV_QUANTIFIED invariants.

use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::data_types::quantity::dv_proportion::DvProportion;
use crate::validate::{InvariantViolation, Validate, is_integral, push_dv_amount_invariants};

// ProportionKind codes.
const PK_UNITARY: i32 = 1;
const PK_PERCENT: i32 = 2;
const PK_FRACTION: i32 = 3;
const PK_INTEGER_FRACTION: i32 = 4;

/// The DV_PROPORTION own-invariant core over the projected inputs — one
/// source for the typed impl and the value-level fast path (`validate::fast`).
/// The inherited DV_AMOUNT / DV_ORDERED invariants are pushed by the callers.
// PORT NOTE: openEHR/archie compare denominator against 0/1/100 by exact
// value (`denominator.equals(0d)` etc.), so exact float comparison is the
// intended semantics here.
#[allow(clippy::float_cmp)]
pub(crate) fn push_dv_proportion_invariants(
    numerator: f64,
    denominator: f64,
    kind: i32,
    precision: Option<i32>,
    out: &mut Vec<InvariantViolation>,
) {
    let integral = is_integral(numerator) && is_integral(denominator);

    // Type_validity: type in 0..=4.
    if !(0..=4).contains(&kind) {
        out.push(InvariantViolation::here(
            "Invariant Type_validity failed on type DV_PROPORTION",
        ));
    }
    // Valid_denominator: denominator != 0.
    if denominator == 0.0 {
        out.push(InvariantViolation::here(
            "Invariant Valid_denominator failed on type DV_PROPORTION",
        ));
    }
    // Precision_validity: precision 0 implies integral numerator & denominator.
    if precision == Some(0) && !integral {
        out.push(InvariantViolation::here(
            "Invariant Precision_validity failed on type DV_PROPORTION",
        ));
    }
    // Fraction_validity: fraction / integer_fraction kinds are integral.
    if (kind == PK_FRACTION || kind == PK_INTEGER_FRACTION) && !integral {
        out.push(InvariantViolation::here(
            "Invariant Fraction_validity failed on type DV_PROPORTION",
        ));
    }
    // Unitary_validity: unitary kind has denominator 1.
    if kind == PK_UNITARY && denominator != 1.0 {
        out.push(InvariantViolation::here(
            "Invariant Unitary_validity failed on type DV_PROPORTION",
        ));
    }
    // Percent_validity: percent kind has denominator 100.
    if kind == PK_PERCENT && denominator != 100.0 {
        out.push(InvariantViolation::here(
            "Invariant Percent_validity failed on type DV_PROPORTION",
        ));
    }
}

impl Validate for DvProportion {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_dv_proportion_invariants(
            self.numerator,
            self.denominator,
            self.r#type,
            self.precision,
            out,
        );

        // Inherited DV_AMOUNT + DV_QUANTIFIED invariants.
        push_dv_amount_invariants(
            out,
            "DV_PROPORTION",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
        );
        // Inherited DV_ORDERED Normal_range_and_status_consistency.
        push_normal_range_consistency(
            out,
            "DV_PROPORTION",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            self,
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    fn proportion(
        numerator: f64,
        denominator: f64,
        ty: i32,
        precision: Option<i32>,
    ) -> DvProportion {
        DvProportion {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            numerator,
            denominator,
            r#type: ty,
            precision,
        }
    }

    fn messages(p: &DvProportion) -> Vec<String> {
        p.invariants().into_iter().map(|v| v.message).collect()
    }

    #[test]
    fn valid_proportions() {
        assert!(
            proportion(5.0, 1.0, PK_UNITARY, None)
                .invariants()
                .is_empty()
        );
        assert!(
            proportion(1.0, 100.0, PK_PERCENT, None)
                .invariants()
                .is_empty()
        );
        assert!(
            proportion(5.0, 100.0, PK_FRACTION, None)
                .invariants()
                .is_empty()
        );
        assert!(proportion(0.5, 100.6, 0, None).invariants().is_empty()); // ratio
    }

    #[test]
    fn unitary_requires_denominator_one() {
        assert!(
            messages(&proportion(5.5, 2.0, PK_UNITARY, None))
                .contains(&"Invariant Unitary_validity failed on type DV_PROPORTION".to_owned())
        );
    }

    #[test]
    fn percent_requires_denominator_hundred() {
        assert!(
            messages(&proportion(5.5, 2.0, PK_PERCENT, None))
                .contains(&"Invariant Percent_validity failed on type DV_PROPORTION".to_owned())
        );
    }

    #[test]
    fn fraction_requires_integral() {
        assert!(
            messages(&proportion(5.5, 2.0, PK_INTEGER_FRACTION, None))
                .contains(&"Invariant Fraction_validity failed on type DV_PROPORTION".to_owned())
        );
    }

    #[test]
    fn denominator_zero_invalid() {
        assert!(
            messages(&proportion(5.5, 0.0, 0, None))
                .contains(&"Invariant Valid_denominator failed on type DV_PROPORTION".to_owned())
        );
    }

    #[test]
    fn type_out_of_range_invalid() {
        assert!(
            messages(&proportion(5.5, 1.0, -1, None))
                .contains(&"Invariant Type_validity failed on type DV_PROPORTION".to_owned())
        );
        assert!(
            messages(&proportion(5.5, 1.0, 5, None))
                .contains(&"Invariant Type_validity failed on type DV_PROPORTION".to_owned())
        );
    }

    #[test]
    fn precision_zero_requires_integral() {
        assert!(
            messages(&proportion(5.5, 1.0, 0, Some(0)))
                .contains(&"Invariant Precision_validity failed on type DV_PROPORTION".to_owned())
        );
    }
}
