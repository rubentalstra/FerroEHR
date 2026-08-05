// @generated-from-template templates/openehr-rm/data_types/quantity/dv_proportion_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `DV_PROPORTION`.
//!
//! Spec:
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_proportion.adoc`
//! §Invariants (the seven own invariants), over the kind constants of
//! `…org.openehr.rm.data_types.proportion_kind.adoc` §Constants (`pk_ratio` 0,
//! `pk_unitary` 1, `pk_percent` 2, `pk_fraction` 3, `pk_integer_fraction` 4),
//! plus the inherited DV_AMOUNT / DV_QUANTIFIED / DV_ORDERED invariants.

use crate::v1_2::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::v1_2::data_types::quantity::dv_proportion::DvProportion;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvProportion {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        // DV_PROPORTION own invariants (Type_validity, Valid_denominator,
        // Precision_validity, Fraction_validity, Unitary_validity,
        // Percent_validity) via the generated core.
        crate::v1_2::validate::generated::dv_proportion_core(
            self.numerator,
            self.denominator,
            self.r#type,
            self.precision,
            out,
        );

        // Inherited DV_AMOUNT + DV_QUANTIFIED invariants.
        crate::v1_2::validate::generated::dv_amount_core(
            "DV_PROPORTION",
            self.accuracy,
            self.accuracy_is_percent,
            self.magnitude_status.as_deref(),
            out,
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
mod tests {
    use super::*;

    // ProportionKind codes, as readable test inputs (the runtime codes live in
    // the generated `validate::generated` core).
    const PK_UNITARY: i32 = 1;
    const PK_PERCENT: i32 = 2;
    const PK_FRACTION: i32 = 3;
    const PK_INTEGER_FRACTION: i32 = 4;

    fn proportion(
        numerator: f64,
        denominator: f64,
        ty: i32,
        precision: Option<i32>,
    ) -> DvProportion {
        DvProportion {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
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

    /// CNF `master17.3-content_tc_data_types-quantity.adoc`
    /// (CONT-DV_PROPORTION-validate_open): `type 3, num 10, den 500, precision 1
    /// | rejected | fraction_validity` (and the type-4 analogue). A
    /// fraction / integer_fraction with a present, non-zero precision is not
    /// integral (`is_integral()` is "True … if precision is 0", RM
    /// dv_proportion.adoc §Functions), so `Fraction_validity` must reject it even
    /// though the numerator/denominator are whole numbers.
    #[test]
    fn fraction_with_nonzero_precision_rejected() {
        let fraction_validity =
            "Invariant Fraction_validity failed on type DV_PROPORTION".to_owned();
        assert!(
            messages(&proportion(10.0, 500.0, PK_FRACTION, Some(1))).contains(&fraction_validity)
        );
        assert!(
            messages(&proportion(10.0, 500.0, PK_INTEGER_FRACTION, Some(1)))
                .contains(&fraction_validity)
        );
        // Precision 0 with integer numerator/denominator is the valid fraction
        // shape (CNF `type 3, 10/100, precision 0 | accepted`).
        assert!(
            proportion(10.0, 100.0, PK_FRACTION, Some(0))
                .invariants()
                .is_empty()
        );
        assert!(
            proportion(10.0, 100.0, PK_INTEGER_FRACTION, Some(0))
                .invariants()
                .is_empty()
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
