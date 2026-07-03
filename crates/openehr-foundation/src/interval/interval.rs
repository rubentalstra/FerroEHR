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
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Interval<T: Ordered> {
    /// `lower`: `T` (0..1). Lower bound.
    ///
    /// `#[serde(skip_serializing_if = "Option::is_none")]` per P4: the
    /// canonical ITS-JSON schema types this property as a plain (non-
    /// nullable) `T` object when present, so an explicit JSON `null` (what
    /// serde emits by default for a `None` field with no such attribute)
    /// fails schema validation; omitting the key entirely when unbounded
    /// is both schema-valid and matches `lower_unbounded` already
    /// conveying the "no lower limit" semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower: Option<T>,
    /// `upper`: `T` (0..1). Upper bound.
    ///
    /// See the `#[serde(skip_serializing_if = ...)]` note on `lower` above;
    /// the same reasoning applies symmetrically.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// The effective lower constraint: `None` when this side imposes no
    /// constraint, otherwise the limit value and whether it is included.
    ///
    /// PORT NOTE: a side declared bounded (`lower_unbounded = false`) whose
    /// limit value is nonetheless absent is an inconsistent state the spec
    /// never describes (it has no invariant tying `lower_unbounded` to
    /// `lower`'s presence); treated as unconstrained here rather than
    /// panicking, consistently across `has`/`intersects`/`contains`.
    fn effective_lower(&self) -> Option<(&T, bool)> {
        if self.lower_unbounded {
            None
        } else {
            self.lower.as_ref().map(|v| (v, self.lower_included))
        }
    }

    /// The effective upper constraint; see `effective_lower`.
    fn effective_upper(&self) -> Option<(&T, bool)> {
        if self.upper_unbounded {
            None
        } else {
            self.upper.as_ref().map(|v| (v, self.upper_included))
        }
    }

    /// `has` (abstract) `(e: T[1]) -> Boolean`.
    ///
    /// True if the value `e` is properly contained in this Interval.
    ///
    /// Spec `Post_result`: `Result = (lower_unbounded or lower_included and
    /// v >= lower) or v > lower and (upper_unbounded or upper_included and v
    /// <= upper or v < upper)`.
    ///
    /// PORT NOTE (published-spec defect, resolved): the `Post_result`
    /// expression as printed is ambiguously parenthesized (and names the
    /// argument `v` while the signature declares `e`). The Meaning column of
    /// the same table row spells out the intended grouping — `(lower_unbounded
    /// or ((lower_included and v >= lower) or v > lower)) and (upper_unbounded
    /// or ((upper_included and v <= upper) or v < upper))`, i.e. a
    /// lower-side condition AND an upper-side condition — which is the
    /// standard interval-membership reading implemented here.
    pub fn has(&self, e: &T) -> bool {
        let lower_ok = match self.effective_lower() {
            None => true,
            Some((lower, included)) => {
                if included {
                    e.greater_than_or_equal(lower)
                } else {
                    e.greater_than(lower)
                }
            }
        };
        let upper_ok = match self.effective_upper() {
            None => true,
            Some((upper, included)) => {
                if included {
                    e.less_than_or_equal(upper)
                } else {
                    e.less_than(upper)
                }
            }
        };
        lower_ok && upper_ok
    }

    /// True if this interval lies entirely below `other` — i.e. this
    /// interval's upper limit and `other`'s lower limit exclude any shared
    /// value. Helper for `intersects`.
    fn is_disjoint_below(&self, other: &Interval<T>) -> bool {
        let (Some((su, su_included)), Some((ol, ol_included))) =
            (self.effective_upper(), other.effective_lower())
        else {
            return false;
        };
        if su.less_than(ol) {
            return true;
        }
        // At this point su >= ol; touching limits (su = ol) share a value
        // only when both sides include it.
        !ol.less_than(su) && !(su_included && ol_included)
    }

    /// `intersects` (abstract) `(other: Interval[1]) -> Boolean`.
    ///
    /// True if there is any overlap between intervals represented by
    /// Current and `other`.
    ///
    /// PORT NOTE (published-spec prose defect): the Meaning column's second
    /// sentence — "True if at least one limit of `other` is strictly inside
    /// the limits of this interval" — contradicts its own first sentence:
    /// when `other` strictly contains this interval, no limit of `other`
    /// lies inside this interval, yet the two plainly overlap. Implemented
    /// as the first sentence's mathematical overlap (some value belongs to
    /// both intervals): neither interval lies entirely below the other,
    /// with touching limits overlapping only when both sides include the
    /// shared limit value.
    pub fn intersects(&self, other: &Interval<T>) -> bool {
        !self.is_disjoint_below(other) && !other.is_disjoint_below(self)
    }

    /// True if this interval's lower side admits every value that `other`'s
    /// lower side admits. Helper for `contains`.
    fn lower_side_contains(&self, other: &Interval<T>) -> bool {
        match (self.effective_lower(), other.effective_lower()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some((sl, sl_included)), Some((ol, ol_included))) => {
                if sl.less_than(ol) {
                    true
                } else if ol.less_than(sl) {
                    false
                } else {
                    // Equal limits: only fails when other admits the limit
                    // value itself but self excludes it.
                    sl_included || !ol_included
                }
            }
        }
    }

    /// Upper-side counterpart of `lower_side_contains`.
    fn upper_side_contains(&self, other: &Interval<T>) -> bool {
        match (self.effective_upper(), other.effective_upper()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some((su, su_included)), Some((ou, ou_included))) => {
                if ou.less_than(su) {
                    true
                } else if su.less_than(ou) {
                    false
                } else {
                    su_included || !ou_included
                }
            }
        }
    }

    /// `contains` (abstract) `(other: Interval[1]) -> Boolean`.
    ///
    /// True if current interval properly contains `other`? True if all
    /// points of `other` are inside the current interval.
    ///
    /// PORT NOTE: implemented per the Meaning column's operative sentence
    /// ("all points of `other` are inside the current interval"), i.e.
    /// non-strict set inclusion `other ⊆ self` — an interval contains
    /// itself. The stray "properly contains ... ?" wording in the same cell
    /// (with its literal question mark) is an editorial artifact and defines
    /// no strict-inclusion semantics.
    pub fn contains(&self, other: &Interval<T>) -> bool {
        self.lower_side_contains(other) && self.upper_side_contains(other)
    }

    /// Invariant `Lower_included_valid`: `lower_unbounded implies not
    /// lower_included`.
    pub fn lower_included_valid(&self) -> bool {
        !self.lower_unbounded || !self.lower_included
    }

    /// Invariant `Upper_included_valid`: `upper_unbounded implies not
    /// upper_included`.
    pub fn upper_included_valid(&self) -> bool {
        !self.upper_unbounded || !self.upper_included
    }

    /// Invariant `Limits_consistent`: `(not upper_unbounded and not
    /// lower_unbounded) implies lower <= upper`.
    ///
    /// PORT NOTE: when a bounded side's limit value is absent (the
    /// inconsistent state noted on `effective_lower`), there is nothing to
    /// compare and the implication holds vacuously.
    pub fn limits_consistent(&self) -> bool {
        if self.upper_unbounded || self.lower_unbounded {
            return true;
        }
        match (&self.lower, &self.upper) {
            (Some(lower), Some(upper)) => lower.less_than_or_equal(upper),
            _ => true,
        }
    }

    /// All four class invariants combined, as a working validity method per
    /// ADR-003 decision 8.
    ///
    /// PORT NOTE: the fourth invariant, `Limits_comparable`
    /// (`lower.strictly_comparable_to(upper)` when both bounded), is
    /// satisfied structurally — both limits are the same Rust type `T`, so
    /// they are always mutually comparable via `Ordered`; `strictly_
    /// comparable_to` itself appears in no BASE per-class table and needs no
    /// runtime encoding here.
    pub fn is_valid(&self) -> bool {
        self.lower_included_valid() && self.upper_included_valid() && self.limits_consistent()
    }
}

// PORT NOTE: `+ PartialEq` narrows this impl beyond the spec's bare
// `T: Ordered` — limit-field equality in `is_equal` needs structural
// comparison, and the transcribed `Ordered` trait deliberately does not
// declare an `is_equal` over `Option<T>` limits. Every concrete Ordered
// type is PartialEq, so nothing is excluded in practice.
impl<T: Ordered + PartialEq> Any for Interval<T> {
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

#[cfg(test)]
mod tests {
    use super::Interval;
    use crate::primitive_types::integer::Integer;

    fn bounded(
        lower: i32,
        upper: i32,
        lower_included: bool,
        upper_included: bool,
    ) -> Interval<Integer> {
        Interval {
            lower: Some(Integer(lower)),
            upper: Some(Integer(upper)),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included,
            upper_included,
        }
    }

    fn lower_unbounded_upto(upper: i32) -> Interval<Integer> {
        Interval {
            lower: None,
            upper: Some(Integer(upper)),
            lower_unbounded: true,
            upper_unbounded: false,
            lower_included: false,
            upper_included: true,
        }
    }

    fn upper_unbounded_from(lower: i32) -> Interval<Integer> {
        Interval {
            lower: Some(Integer(lower)),
            upper: None,
            lower_unbounded: false,
            upper_unbounded: true,
            lower_included: true,
            upper_included: false,
        }
    }

    // Spec has() Meaning: "(lower_unbounded or ((lower_included and v >=
    // lower) or v > lower)) and (upper_unbounded or ((upper_included and
    // v <= upper) or v < upper))".
    #[test]
    fn has_respects_included_and_excluded_limits() {
        let closed = bounded(1, 5, true, true);
        assert!(closed.has(&Integer(1)));
        assert!(closed.has(&Integer(3)));
        assert!(closed.has(&Integer(5)));
        assert!(!closed.has(&Integer(0)));
        assert!(!closed.has(&Integer(6)));

        let open = bounded(1, 5, false, false);
        assert!(!open.has(&Integer(1)));
        assert!(open.has(&Integer(2)));
        assert!(open.has(&Integer(4)));
        assert!(!open.has(&Integer(5)));
    }

    // Spec: lower_unbounded means the lower boundary is open (= -infinity),
    // and symmetrically for upper_unbounded.
    #[test]
    fn has_treats_unbounded_sides_as_infinite() {
        assert!(lower_unbounded_upto(5).has(&Integer(i32::MIN)));
        assert!(lower_unbounded_upto(5).has(&Integer(5)));
        assert!(!lower_unbounded_upto(5).has(&Integer(6)));
        assert!(upper_unbounded_from(1).has(&Integer(i32::MAX)));
        assert!(!upper_unbounded_from(1).has(&Integer(0)));
    }

    // Spec intersects(): "True if there is any overlap between intervals
    // represented by Current and other."
    #[test]
    fn intersects_detects_overlap_touching_and_disjointness() {
        let a = bounded(1, 5, true, true);
        assert!(a.intersects(&bounded(4, 8, true, true))); // partial overlap
        assert!(a.intersects(&bounded(2, 3, true, true))); // other inside self
        assert!(a.intersects(&bounded(0, 9, true, true))); // self inside other
        assert!(!a.intersects(&bounded(6, 9, true, true))); // disjoint
        // Touching limits: shared value only when both sides include it.
        assert!(a.intersects(&bounded(5, 9, true, true)));
        assert!(!a.intersects(&bounded(5, 9, false, true)));
        assert!(!bounded(1, 5, true, false).intersects(&bounded(5, 9, true, true)));
        // Unbounded sides overlap anything on that side.
        assert!(lower_unbounded_upto(5).intersects(&upper_unbounded_from(0)));
    }

    // Spec contains(): "True if all points of other are inside the current
    // interval."
    #[test]
    fn contains_is_non_strict_set_inclusion() {
        let a = bounded(1, 5, true, true);
        assert!(a.contains(&bounded(2, 4, true, true)));
        assert!(a.contains(&bounded(1, 5, true, true))); // an interval contains itself
        assert!(!a.contains(&bounded(0, 4, true, true)));
        assert!(!a.contains(&bounded(2, 6, true, true)));
        // Equal limit values: an open self limit cannot contain a closed
        // other limit, but the reverse is fine.
        let open = bounded(1, 5, false, false);
        assert!(!open.contains(&bounded(1, 4, true, true)));
        assert!(bounded(1, 5, true, true).contains(&bounded(1, 5, false, false)));
        // Bounded self cannot contain an unbounded other side.
        assert!(!a.contains(&upper_unbounded_from(2)));
        assert!(upper_unbounded_from(0).contains(&a));
    }

    // Invariants both directions (ADR-003 decision 8): Lower/Upper_included_
    // valid and Limits_consistent as working methods.
    #[test]
    fn invariants_hold_for_well_formed_intervals() {
        assert!(bounded(1, 5, true, true).is_valid());
        assert!(bounded(5, 5, true, true).is_valid()); // lower <= upper allows equality
        assert!(lower_unbounded_upto(5).is_valid());
        assert!(upper_unbounded_from(1).is_valid());
    }

    #[test]
    fn invariants_flag_each_violation() {
        // Lower_included_valid: lower_unbounded implies not lower_included.
        let mut bad = lower_unbounded_upto(5);
        bad.lower_included = true;
        assert!(!bad.lower_included_valid());
        assert!(!bad.is_valid());
        // Upper_included_valid: upper_unbounded implies not upper_included.
        let mut bad = upper_unbounded_from(1);
        bad.upper_included = true;
        assert!(!bad.upper_included_valid());
        assert!(!bad.is_valid());
        // Limits_consistent: both bounded implies lower <= upper.
        let bad = bounded(6, 5, true, true);
        assert!(!bad.limits_consistent());
        assert!(!bad.is_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/interval.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / interval.adoc §Interval Class
//   confidence: high
//   todos: 0
//   note: has() implemented from the Meaning column's unambiguous grouping (the printed Post_result parenthesization is a published defect, PORT-NOTEd); intersects() uses mathematical overlap (the "at least one limit strictly inside" prose is self-contradictory, PORT-NOTEd); contains() is non-strict inclusion. Invariants Lower/Upper_included_valid + Limits_consistent are working methods (is_valid combines them); Limits_comparable is satisfied structurally by the shared T. P4: added #[serde(skip_serializing_if = "Option::is_none")] on `lower`/`upper` — DV_INTERVAL's ITS-JSON schema types these as plain (non-nullable) objects when present, so the prior unconditional `null` emission for an unbounded limit failed schema validation.
// ─────────────────────────────────────────────
