//! `Cardinality` — express constraints on the cardinality of container
//! objects which are the values of multiply-valued attributes, including
//! uniqueness and ordering.
//!
//! openEHR class: `Cardinality`, package `base.foundation_types.interval`.
//! Inherits: `Any` (implicit — see the struct-level PORT NOTE).
//!
//! Provides the means to state that a container acts like a logical list,
//! set or bag.
use super::multiplicity_interval::MultiplicityInterval;
use crate::primitive_types::any::Any;

/// PORT NOTE: the spec table for `Cardinality` has no `Inherit` row at all
/// (contrast every other class in this cluster, which states `Inherit:
/// Any`/`Interval`/`Proper_interval` explicitly). Per the chapter overview's
/// standing convention (documented on `Any` itself — see
/// `primitive_types/any.rs`: every foundation type inherits `Any`, the
/// inheritance diagram omits it "only for convenience"), `Cardinality` is
/// transcribed as implementing `Any` even though its own per-class table is
/// silent on the point, rather than treating the missing row as "inherits
/// nothing."
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cardinality {
    /// `interval`: `Multiplicity_interval` (1..1). The interval of this
    /// cardinality.
    pub interval: MultiplicityInterval,
    /// `is_ordered`: `Boolean` (1..1). True if the members of the container
    /// attribute to which this cardinality refers are ordered.
    pub is_ordered: bool,
    /// `is_unique`: `Boolean` (1..1). True if the members of the container
    /// attribute to which this cardinality refers are unique.
    pub is_unique: bool,
}

impl Cardinality {
    /// `is_bag` `(): Boolean`.
    ///
    /// True if the semantics of this cardinality represent a bag, i.e.
    /// unordered, non-unique membership.
    #[must_use]
    pub fn is_bag(&self) -> bool {
        !self.is_ordered && !self.is_unique
    }

    /// `is_list` `(): Boolean`.
    ///
    /// True if the semantics of this cardinality represent a list, i.e.
    /// ordered, non-unique membership.
    #[must_use]
    pub fn is_list(&self) -> bool {
        self.is_ordered && !self.is_unique
    }

    /// `is_set` `(): Boolean`.
    ///
    /// True if the semantics of this cardinality represent a set, i.e.
    /// unordered, unique membership.
    #[must_use]
    pub fn is_set(&self) -> bool {
        !self.is_ordered && self.is_unique
    }
}

impl Any for Cardinality {
    /// `is_equal(other: Cardinality) -> Boolean`.
    ///
    /// TODO(port): `Cardinality`'s per-class table does not state an
    /// `is_equal` row at all (unlike `Interval`, which marks it
    /// `(effected)` explicitly) — this is transcribed here only because
    /// every foundation type is expected to satisfy `Any` per the
    /// struct-level PORT NOTE, using straightforward field-wise equality as
    /// the natural reading, not a literal spec effecting.
    fn is_equal(&self, other: &Self) -> bool {
        self.interval == other.interval
            && self.is_ordered == other.is_ordered
            && self.is_unique == other.is_unique
    }

    fn type_of(&self) -> String {
        "Cardinality".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/cardinality.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / cardinality.adoc §Cardinality Class
//   confidence: medium
//   todos: 1
//   note: spec table has no Inherit row and no is_equal row; Any is implemented per the crate-wide "every foundation type inherits Any" convention rather than an explicit per-class instruction, flagged as an inference rather than a literal transcription.
// ─────────────────────────────────────────────
