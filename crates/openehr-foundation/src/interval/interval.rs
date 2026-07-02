//! `Interval<T>` — interval abstraction, featuring upper and lower limits
//! that may be open or closed, included or not included.
//!
//! openEHR class: `Interval<T>` (abstract), package
//! `base.foundation_types.interval`.
//! Inherits: `Any`.
//!
//! Interval of ordered items. The definition of `Interval<T>` here is an
//! intensional one, i.e. it states its members by implication from its
//! limits, rather than enumerating them. If `Interval<X>` is defined as the
//! type of a feature in a class in an openEHR model, where `X` is some
//! descendant of `Ordered`, then at runtime, either a `Point_interval` or a
//! `Proper_interval` may be attached — see `point_interval.rs` and
//! `proper_interval.rs` in this module.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;

/// `Interval` is declared `abstract` in the spec table, with two concrete
/// descendants (`Point_interval<T>`, `Proper_interval<T>`) that are
/// substitutable for it. Per ADR-001 §3 ("abstract class with attributes →
/// embedded struct + marker trait"), this struct carries the six spec
/// attributes directly and is embedded by value in both concrete
/// descendants, rather than being instantiated on its own — nothing in this
/// crate constructs a bare `Interval<T>`.
///
/// The generic parameter `T` carries the `T: Ordered` bound from the spec's
/// own constraint ("`X` is some descendant of `Ordered`"), matching
/// ADR-001 §5 (`Interval<T: Ordered>` is the canonical constrained-generic
/// example named there).
///
/// `has`, `intersects`, and `contains` are declared `(abstract)` in the spec
/// table — no body is given at this level, so they are stubbed with
/// `todo!()` here since the acting concrete types (`PointInterval`,
/// `ProperInterval`) provide the actual bodies (both effectively use the
/// same limit-comparison logic; the spec text describes the abstract
/// contract at this level and leaves the effecting to descendants). `is_equal`
/// is marked `(effected)` at this level, so it is given a real body,
/// following `Any`'s value-equality contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval<T: Ordered> {
    /// `lower`: `T` (0..1). Lower bound.
    pub lower: Option<T>,
    /// `upper`: `T` (0..1). Upper bound.
    pub upper: Option<T>,
    /// `lower_unbounded`: `Boolean` (1..1). `lower` boundary open (i.e. =
    /// -infinity).
    pub lower_unbounded: bool,
    /// `upper_unbounded`: `Boolean` (1..1). `upper` boundary open (i.e. =
    /// +infinity).
    pub upper_unbounded: bool,
    /// `lower_included`: `Boolean` (1..1). `lower` boundary value included
    /// in range if not `lower_unbounded`.
    pub lower_included: bool,
    /// `upper_included`: `Boolean` (1..1). `upper` boundary value included
    /// in range if not `upper_unbounded`.
    pub upper_included: bool,
}

impl<T: Ordered> Interval<T> {
    /// `has` (abstract) `(e: T[1]) -> Boolean`.
    ///
    /// True if the value `e` is properly contained in this Interval.
    ///
    /// Spec `Post_result`: `Result = (lower_unbounded or lower_included and
    /// v >= lower) or v > lower and (upper_unbounded or upper_included and v
    /// <= upper or v < upper)`.
    ///
    /// TODO(port): declared `(abstract)` at this level; the parenthesization
    /// of the postcondition as published is ambiguous between `(A or B) and
    /// (C or D)` and a flatter `A or B or C or D` reading — needs the
    /// boundary-comparison operators (`>=`, `<=` as named `Ordered` methods)
    /// wired through before the exact logic can be encoded without guessing
    /// which grouping the spec authors intended.
    pub fn has(&self, _e: &T) -> bool {
        todo!("Interval::has: abstract in spec, see per-class postcondition ambiguity note")
    }

    /// `intersects` (abstract) `(other: Interval[1]) -> Boolean`.
    ///
    /// True if there is any overlap between intervals represented by
    /// Current and `other`. True if at least one limit of `other` is
    /// strictly inside the limits of this interval.
    pub fn intersects(&self, _other: &Interval<T>) -> bool {
        todo!("Interval::intersects: abstract in spec")
    }

    /// `contains` (abstract) `(other: Interval[1]) -> Boolean`.
    ///
    /// True if current interval properly contains `other`? True if all
    /// points of `other` are inside the current interval.
    pub fn contains(&self, _other: &Interval<T>) -> bool {
        todo!("Interval::contains: abstract in spec")
    }
}

impl<T: Ordered> Any for Interval<T> {
    /// `is_equal` (effected) `(other: Any[1]) -> Boolean`.
    ///
    /// True if current object's interval is semantically same as `other`.
    ///
    /// PORT NOTE: the spec types `other` as `Any`, but a semantic-equality
    /// comparison over interval limits is only meaningful against another
    /// `Interval<T>` of the same `T`; narrowed to `&Self` here, matching the
    /// same pattern used throughout `primitive_types` (see `any.rs`'s
    /// `not_equal` PORT NOTE for the precedent of the spec typing an `Any`
    /// function's parameter more loosely than the concrete comparison it
    /// actually performs).
    fn is_equal(&self, other: &Self) -> bool {
        self.lower_unbounded == other.lower_unbounded
            && self.upper_unbounded == other.upper_unbounded
            && self.lower_included == other.lower_included
            && self.upper_included == other.upper_included
            && self.lower == other.lower
            && self.upper == other.upper
    }

    fn type_of(&self) -> String {
        "Interval".to_string()
    }
}

// TODO(port): the four class invariants below are not yet encoded as a
// `Validate` impl — they need the `Ordered::less_than_or_equal` /
// `strictly_comparable_to` methods wired through generically over `T`, and
// `strictly_comparable_to` itself is not part of the `Ordered` trait as
// transcribed in `primitive_types/ordered.rs` (it does not appear in that
// per-class table either; likely a BASE-wide `Comparable`-style notion not
// yet transcribed). Left as documented TODOs per the RM transcription
// invariant rule rather than silently omitted.
//
// - `Lower_included_valid`: `lower_unbounded implies not lower_included`
// - `Upper_included_valid`: `upper_unbounded implies not upper_included`
// - `Limits_consistent`: `(not upper_unbounded and not lower_unbounded)
//   implies lower <= upper`
// - `Limits_comparable`: `(not upper_unbounded and not lower_unbounded)
//   implies lower.strictly_comparable_to (upper)`

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/interval.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / interval.adoc §Interval Class
//   confidence: medium
//   todos: 2
//   note: has/intersects/contains are abstract in the spec and stubbed todo!() pending resolution of the has postcondition's ambiguous parenthesization; the four class invariants need Ordered::strictly_comparable_to, which is not yet part of the Ordered trait transcribed in primitive_types.
// ─────────────────────────────────────────────
