//! Hand-written RM/BASE class invariants for `Proper_interval`.
//!
//! Mirrors the reference implementation archie's `com.nedap.archie.base.Interval`
//! invariants; archie reports them under the base type name `INTERVAL`, which we
//! reproduce in the message. `DV_INTERVAL` surfaces the same invariants (see
//! `openehr-rm` `dv_interval_impl`).

use super::proper_interval::ProperIntervalData;
use crate::validate::{InvariantViolation, Validate};

// NOTE: `Limits_consistent` needs an ordering on `T`. For BASE intervals
// `T` is an ordered foundation type, so we bound on `PartialOrd`. (RM
// `DV_INTERVAL<T: DV_ORDERED>` cannot use this — openEHR ordered-magnitude
// comparison is the P16 `openehr_magnitude` concern — so it checks only the
// boundary-flag invariants; see `openehr-rm`.)
impl<T: PartialOrd> Validate for ProperIntervalData<T> {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        // Lower_included_valid: an unbounded lower boundary is not included.
        if self.lower_unbounded && self.lower_included {
            out.push(InvariantViolation::here(
                "Invariant Lower_included_valid failed on type INTERVAL",
            ));
        }
        // Upper_included_valid: an unbounded upper boundary is not included.
        if self.upper_unbounded && self.upper_included {
            out.push(InvariantViolation::here(
                "Invariant Upper_included_valid failed on type INTERVAL",
            ));
        }
        // Limits_consistent: with both boundaries bounded and present, lower <= upper.
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
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    fn interval(lower: Option<i32>, upper: Option<i32>) -> ProperIntervalData<i32> {
        ProperIntervalData {
            lower,
            upper,
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }
    }

    #[test]
    fn valid_interval_has_no_violations() {
        assert!(interval(Some(1), Some(3)).invariants().is_empty());
        assert!(interval(Some(3), Some(3)).invariants().is_empty());
        let mut open = interval(None, Some(3));
        open.lower_unbounded = true;
        open.lower_included = false;
        assert!(open.invariants().is_empty());
    }

    #[test]
    fn limits_inconsistent() {
        let v = interval(Some(4), Some(3)).invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Limits_consistent failed on type INTERVAL"
        );
    }

    #[test]
    fn lower_included_invalid() {
        let mut i = interval(Some(1), Some(4));
        i.lower_unbounded = true; // still lower_included == true
        let v = i.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Lower_included_valid failed on type INTERVAL"),
            "got {v:?}"
        );
    }

    #[test]
    fn upper_included_invalid() {
        let mut i = interval(Some(1), Some(4));
        i.upper_unbounded = true; // still upper_included == true
        let v = i.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Upper_included_valid failed on type INTERVAL"),
            "got {v:?}"
        );
    }
}
