//! `Proper_interval<T>` — type representing a 'proper' Interval, i.e. any
//! two-sided or one-sided interval.
//!
//! openEHR class: `Proper_interval<T>`, package
//! `base.foundation_types.interval`.
//! Inherits: `Interval<T>`.
use super::interval::Interval;
use crate::primitive_types::ordered::Ordered;

/// `Proper_interval` declares no attributes or functions of its own beyond
/// those inherited from `Interval<T>` — its only spec content is the class
/// invariant `Inv_not_point`. Per ADR-001 §3, the parent's six attributes
/// are embedded by value as an `Interval<T>` field, matching the same
/// composition shape used by the sibling `PointInterval` in this module (see
/// `point_interval.rs`), so `Interval<T>` and `Proper_interval<T>` /
/// `Point_interval<T>` stay structurally identical apart from which
/// invariant each enforces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProperInterval<T: Ordered> {
    /// Embedded parent state; see the struct-level PORT NOTE for why this is
    /// composition rather than a flattened field list.
    pub interval: Interval<T>,
}

impl<T: Ordered> ProperInterval<T> {
    /// Constructs a `Proper_interval<T>` from an already-built `Interval<T>`
    /// value.
    ///
    /// TODO(port): the spec's invariant `Inv_not_point: lower /= upper` is
    /// not enforced here — unlike `PointInterval::new` (which can fix its
    /// invariant unconditionally by construction), `Inv_not_point` depends
    /// on comparing two `Option<T>` values that may each be absent
    /// (`lower_unbounded`/`upper_unbounded`), so a faithful check needs a
    /// `Validate`-style fallible constructor (context + path + error
    /// accumulator, per `.claude/rules/rm-transcription.md`) rather than an
    /// infallible one. Left as a plain constructor plus this TODO rather
    /// than silently omitting the invariant.
    pub fn new(interval: Interval<T>) -> Self {
        ProperInterval { interval }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/proper_interval.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / proper_interval.adoc §Proper_interval Class
//   confidence: medium
//   todos: 1
//   note: Inv_not_point (lower /= upper) not yet enforced; needs a Validate-style fallible constructor since both limits can be Option::None (unbounded) rather than always-comparable values.
// ─────────────────────────────────────────────
