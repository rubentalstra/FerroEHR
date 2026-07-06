//! Hand-written RM class invariants (ADR-003) for `DV_INTERVAL`.
//!
//! `DV_INTERVAL` surfaces the `Interval` boundary-flag invariants
//! (`Lower_included_valid`, `Upper_included_valid`).
//!
//! PORT NOTE: archie reports these under the base type `INTERVAL` (its
//! `DvInterval` composes an inner `Interval`); our `DvInterval` is flat, so we
//! report the concrete type `DV_INTERVAL`. `Limits_consistent` (`lower <= upper`)
//! is **not** implemented here: `T` is a `DV_ORDERED`, whose comparison is
//! openEHR ordered-magnitude semantics — the P16 `openehr_magnitude` concern —
//! not Rust `PartialOrd`. The composition validator handles cross-value ordering
//! when the magnitude machinery lands.

use crate::data_types::quantity::dv_interval::DvInterval;
use crate::validate::{InvariantViolation, Validate};

impl<T> Validate for DvInterval<T> {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval() -> DvInterval<i32> {
        DvInterval {
            lower: Some(1),
            upper: Some(4),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }
    }

    #[test]
    fn valid_interval() {
        assert!(interval().invariants().is_empty());
    }

    #[test]
    fn lower_included_invalid() {
        let mut i = interval();
        i.lower_unbounded = true; // still lower_included
        let v = i.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Lower_included_valid failed on type DV_INTERVAL"),
            "got {v:?}"
        );
    }
}
