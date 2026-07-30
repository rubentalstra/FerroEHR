//! Hand-written RM class invariants + interval functions for
//! `DV_INTERVAL`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_interval.adoc`.
//!
//! - The base `Interval` boundary-flag invariants (`Lower_included_valid`,
//!   `Upper_included_valid`).
//! - `Limits_consistent`: `(not upper_unbounded and not lower_unbounded)
//!   implies (lower.is_strictly_comparable_to(upper) and lower <= upper)` —
//!   enforced through the [`OrderedLimit`] ordered-magnitude surface
//!   (`dv_ordered_impl`); undecidable comparisons (e.g. a `serde_json::Value`
//!   element, or a malformed temporal magnitude) are not reported as
//!   violations — value well-formedness is caught by the element's own
//!   `Value_valid` invariant.
//! - `has(v)`: the base `Interval.has` membership test, honouring the
//!   `*_included` boundary flags (used by `REFERENCE_RANGE.is_in_range` and
//!   `DV_ORDERED.is_normal`).
//!
//! NOTE: archie reports the boundary-flag invariants under the base type
//! `INTERVAL` (its `DvInterval` composes an inner `Interval`); our `DvInterval`
//! is flat, so we report the concrete type `DV_INTERVAL`. The indexed SQL
//! realisation of the same ordering is the AQL engine's `openehr_magnitude`
//! function; this impl is the in-process authority.

use crate::data_types::quantity::dv_interval::DvInterval;
use crate::data_types::quantity::dv_ordered_impl::OrderedLimit;
use crate::validate::{InvariantViolation, Validate};

impl<T: OrderedLimit> DvInterval<T> {
    /// The base `Interval.has(v)` membership test: `true` when `v` lies inside
    /// this interval, honouring `lower_included` / `upper_included`. `None`
    /// when a required bound is missing or a comparison is undecidable.
    #[must_use]
    pub fn has(&self, v: &T) -> Option<bool> {
        let lower_ok = if self.lower_unbounded {
            true
        } else {
            let lower = self.lower.as_ref()?;
            if self.lower_included {
                lower.less_or_equal(v)?
            } else {
                lower.less_than(v)?
            }
        };
        if !lower_ok {
            return Some(false);
        }
        let upper_ok = if self.upper_unbounded {
            true
        } else {
            let upper = self.upper.as_ref()?;
            if self.upper_included {
                v.less_or_equal(upper)?
            } else {
                v.less_than(upper)?
            }
        };
        Some(upper_ok)
    }
}

impl<T: OrderedLimit> Validate for DvInterval<T> {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.lower_unbounded && self.lower_included {
            out.push(InvariantViolation::here(
                "Invariant Lower_included_valid failed on type DV_INTERVAL",
            ));
        }
        if self.upper_unbounded && self.upper_included {
            out.push(InvariantViolation::here(
                "Invariant Upper_included_valid failed on type DV_INTERVAL",
            ));
        }
        // NOTE: an absent bound with a false `*_unbounded` flag is ACCEPTED.
        // BASE `org.openehr.base.foundation_types.interval.adoc` declares
        // `lower`/`upper` as 0..1 and its §Invariants set is closed
        // (Limits_consistent, Limits_comparable, Lower/Upper_included_valid) —
        // no invariant requires a bound VALUE when its flag is false; the
        // guarded `Limits_consistent`/`Limits_comparable` implications are
        // simply unevaluable then and are skipped, never a rejection
        // (AMB-43 disposition in the CNF ambiguity register).
        // Limits_consistent: bounded on both sides implies the limits are
        // strictly comparable and lower <= upper.
        if !self.lower_unbounded
            && !self.upper_unbounded
            && let (Some(lower), Some(upper)) = (self.lower.as_ref(), self.upper.as_ref())
        {
            let violated = match lower.strictly_comparable(upper) {
                Some(false) => true,
                Some(true) => lower.less_or_equal(upper) == Some(false),
                None => false, // undecidable at this element type/value
            };
            if violated {
                out.push(InvariantViolation::here(
                    "Invariant Limits_consistent failed on type DV_INTERVAL",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_ordered::DvOrdered;
    use crate::data_types::quantity::dv_quantity::DvQuantity;

    fn quantity(magnitude: f64, units: &str) -> DvQuantity {
        DvQuantity {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            magnitude,
            precision: None,
            units: units.to_owned(),
            units_system: None,
            units_display_name: None,
        }
    }

    fn interval(lower: f64, upper: f64) -> DvInterval<DvQuantity> {
        DvInterval {
            lower: Some(quantity(lower, "kg")),
            upper: Some(quantity(upper, "kg")),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }
    }

    #[test]
    fn valid_interval() {
        assert!(interval(1.0, 4.0).invariants().is_empty());
        // Equal limits are consistent (lower <= upper).
        assert!(interval(4.0, 4.0).invariants().is_empty());
    }

    #[test]
    fn lower_included_invalid() {
        let mut i = interval(1.0, 4.0);
        i.lower_unbounded = true; // still lower_included
        let v = i.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Lower_included_valid failed on type DV_INTERVAL"),
            "got {v:?}"
        );
    }

    #[test]
    fn absent_bound_with_bounded_flag_is_accepted() {
        // BASE Interval: `lower`/`upper` are 0..1 and the closed invariant
        // set has no bound-presence rule — an absent bound with a false
        // `*_unbounded` flag violates nothing (the guarded Limits_consistent
        // implication is unevaluable and skipped; AMB-43, ACCEPTED).
        let half_open = DvInterval::<DvQuantity> {
            lower: None,
            upper: Some(quantity(1.0, "s")),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: false,
            upper_included: false,
        };
        assert!(half_open.invariants().is_empty());
        // The flagged half-open form is equally clean.
        let proper = DvInterval::<DvQuantity> {
            lower: None,
            upper: Some(quantity(1.0, "s")),
            lower_unbounded: true,
            upper_unbounded: false,
            lower_included: false,
            upper_included: true,
        };
        assert!(proper.invariants().is_empty());
        // Mirror on the upper side.
        let no_upper = DvInterval::<DvQuantity> {
            lower: Some(quantity(1.0, "s")),
            upper: None,
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: false,
        };
        assert!(no_upper.invariants().is_empty());
    }

    #[test]
    fn inverted_limits_fail_limits_consistent() {
        let v = interval(10.0, 2.0).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Limits_consistent failed on type DV_INTERVAL"),
            "got {v:?}"
        );
    }

    #[test]
    fn incomparable_limits_fail_limits_consistent() {
        let mut i = interval(1.0, 4.0);
        i.upper = Some(quantity(4.0, "mm[Hg]")); // different units
        let v = i.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Limits_consistent failed on type DV_INTERVAL"),
            "got {v:?}"
        );
    }

    #[test]
    fn unbounded_side_skips_limits_consistent() {
        let mut i = interval(10.0, 2.0);
        i.upper_unbounded = true;
        i.upper_included = false;
        i.upper = None;
        assert!(i.invariants().is_empty());
    }

    #[test]
    fn undecidable_value_elements_skip_limits_consistent() {
        // serde_json::Value elements (the dispatcher fallback) can't be
        // compared → only boundary-flag invariants run.
        let i: DvInterval<serde_json::Value> = DvInterval {
            lower: Some(serde_json::json!({"magnitude": 10.0})),
            upper: Some(serde_json::json!({"magnitude": 2.0})),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        };
        assert!(i.invariants().is_empty());
    }

    #[test]
    fn has_membership() {
        let i = interval(1.0, 4.0);
        assert_eq!(i.has(&quantity(2.0, "kg")), Some(true));
        assert_eq!(i.has(&quantity(1.0, "kg")), Some(true)); // inclusive lower
        assert_eq!(i.has(&quantity(5.0, "kg")), Some(false));
        assert_eq!(i.has(&quantity(2.0, "mm[Hg]")), None); // incomparable

        let mut open = interval(1.0, 4.0);
        open.lower_included = false;
        assert_eq!(open.has(&quantity(1.0, "kg")), Some(false)); // exclusive lower
    }

    #[test]
    fn has_over_dv_ordered_enum() {
        let i: DvInterval<DvOrdered> = DvInterval {
            lower: Some(DvOrdered::DvQuantity(quantity(1.0, "kg"))),
            upper: Some(DvOrdered::DvQuantity(quantity(4.0, "kg"))),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        };
        assert_eq!(
            i.has(&DvOrdered::DvQuantity(quantity(3.0, "kg"))),
            Some(true)
        );
        assert!(i.invariants().is_empty());
    }
}
