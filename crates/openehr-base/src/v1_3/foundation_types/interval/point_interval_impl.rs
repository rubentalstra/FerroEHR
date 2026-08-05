// @generated-from-template templates/openehr-base/foundation_types/interval/point_interval_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM/BASE class invariants for `Point_interval`.
//!
//! A point interval represents a single value (`lower == upper`, both included).
//! It shares the `Interval` boundary-flag invariants (archie reports them under
//! the base type `INTERVAL`).

use super::point_interval::PointInterval;
use crate::validate::{InvariantViolation, Validate};

impl<T: PartialOrd> Validate for PointInterval<T> {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.lower_unbounded && self.lower_included {
            out.push(InvariantViolation::here(
                "Invariant Lower_included_valid failed on type INTERVAL",
            ));
        }
        if self.upper_unbounded && self.upper_included {
            out.push(InvariantViolation::here(
                "Invariant Upper_included_valid failed on type INTERVAL",
            ));
        }
        if !self.lower_unbounded
            && !self.upper_unbounded
            && let (Some(l), Some(u)) = (self.lower.as_ref(), self.upper.as_ref())
            && l > u
        {
            out.push(InvariantViolation::here(
                "Invariant Limits_consistent failed on type INTERVAL",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_interval_valid() {
        let p = PointInterval {
            lower: Some(5),
            upper: Some(5),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        };
        assert!(p.invariants().is_empty());
    }
}
