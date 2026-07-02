//! `Multiplicity_interval` — an Interval of Integer, used to represent
//! multiplicity, cardinality and optionality in models.
//!
//! openEHR class: `Multiplicity_interval`, package
//! `base.foundation_types.interval`.
//! Inherits: `Proper_interval` (i.e. `Proper_interval<Integer>` — see the
//! struct-level PORT NOTE).
use super::proper_interval::ProperInterval;
use crate::primitive_types::integer::Integer;

/// The spec table's `Inherit` line reads `Proper_interval` (not
/// `Proper_interval<T>` with an explicit type argument), which is only
/// meaningful once `T` is bound to a concrete `Ordered` type — the class
/// description states outright that this is "An Interval of Integer", so
/// `T = Integer` here. Equivalently, since `Proper_interval<T>` itself
/// narrows `Interval<T>` with no attributes of its own beyond the invariant,
/// `Multiplicity_interval` can be read as `Interval<Integer>` at the
/// semantic level the class description uses informally — both framings
/// describe the same embedded shape; this file follows the spec table's
/// literal `Inherit: Proper_interval` line for the embedded field type.
///
/// Per ADR-001 §3, the parent's state is embedded by value rather than
/// flattened, matching `PointInterval`/`ProperInterval` in this same module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiplicityInterval {
    /// Embedded parent state (`Proper_interval<Integer>`); see the
    /// struct-level PORT NOTE for the `Inherit: Proper_interval` → `T =
    /// Integer` reasoning.
    pub proper_interval: ProperInterval<Integer>,
}

impl MultiplicityInterval {
    /// `Multiplicity_range_marker`: `String = ".."` (1..1 constant).
    ///
    /// Marker to use in string form of interval between limits.
    pub const MULTIPLICITY_RANGE_MARKER: &'static str = "..";

    /// `Multiplicity_unbounded_marker`: `char = '*'` (1..1 constant).
    ///
    /// Symbol to use to indicate upper limit unbounded.
    pub const MULTIPLICITY_UNBOUNDED_MARKER: char = '*';

    /// `is_open` `(): Boolean`.
    ///
    /// True if this interval imposes no constraints, i.e. is set to `0..*`.
    #[must_use]
    pub fn is_open(&self) -> bool {
        let lower_is_zero = self
            .proper_interval
            .interval
            .lower
            .as_ref()
            .is_some_and(|l| l.0 == 0);
        lower_is_zero && self.proper_interval.interval.upper_unbounded
    }

    /// `is_optional` `(): Boolean`.
    ///
    /// True if this interval expresses optionality, i.e. `0..1`.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        let lower_is_zero = self
            .proper_interval
            .interval
            .lower
            .as_ref()
            .is_some_and(|l| l.0 == 0);
        let upper_is_one = self
            .proper_interval
            .interval
            .upper
            .as_ref()
            .is_some_and(|u| u.0 == 1);
        lower_is_zero && !self.proper_interval.interval.upper_unbounded && upper_is_one
    }

    /// `is_mandatory` `(): Boolean`.
    ///
    /// True if this interval expresses mandation, i.e. `1..1`.
    #[must_use]
    pub fn is_mandatory(&self) -> bool {
        let lower_is_one = self
            .proper_interval
            .interval
            .lower
            .as_ref()
            .is_some_and(|l| l.0 == 1);
        let upper_is_one = self
            .proper_interval
            .interval
            .upper
            .as_ref()
            .is_some_and(|u| u.0 == 1);
        lower_is_one && !self.proper_interval.interval.upper_unbounded && upper_is_one
    }

    /// `is_prohibited` `(): Boolean`.
    ///
    /// True if this interval is set to `0..0`.
    #[must_use]
    pub fn is_prohibited(&self) -> bool {
        let lower_is_zero = self
            .proper_interval
            .interval
            .lower
            .as_ref()
            .is_some_and(|l| l.0 == 0);
        let upper_is_zero = self
            .proper_interval
            .interval
            .upper
            .as_ref()
            .is_some_and(|u| u.0 == 0);
        lower_is_zero && !self.proper_interval.interval.upper_unbounded && upper_is_zero
    }
}

// TODO(port): the four predicate bodies above (`is_open`, `is_optional`,
// `is_mandatory`, `is_prohibited`) are transcribed from the class
// description's informal "i.e. 0..*" / "i.e. 0..1" / "i.e. 1..1" / "i.e.
// 0..0" prose rather than a formal Post_result postcondition — the spec
// table gives no postcondition for any of these four functions, only the
// meaning column quoted in each doc comment above. Flagged since this is an
// inference from prose, not a literal postcondition transcription like the
// rest of this cluster.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/multiplicity_interval.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / multiplicity_interval.adoc §Multiplicity_interval Class
//   confidence: medium
//   todos: 1
//   note: is_open/is_optional/is_mandatory/is_prohibited bodies are inferred from the class description's informal "i.e. 0..*" etc. phrasing, not a formal Post_result the spec table does not provide for these four functions.
// ─────────────────────────────────────────────
