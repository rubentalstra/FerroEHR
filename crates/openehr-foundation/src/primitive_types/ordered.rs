//! `Ordered` — abstract parent class of ordered types.
//!
//! openEHR class: `Ordered` (abstract), package `base.foundation_types.primitive_types`.
//! Inherits: `Any`.
//!
//! Abstract parent class of ordered types, i.e. types on which the `<`
//! operator is defined.
use super::any::Any;

/// `Ordered` is modelled as a Rust trait with `Any` as a supertrait, mirroring
/// the spec's single-parent inheritance (`Ordered` inherits `Any`).
///
/// Symbolic operators (`<`, `<=`/`≤`, `>`, `>=`/`≥`) are named methods, not
/// `std::cmp`/`std::ops` overloads, per the RM transcription rules — this
/// keeps the trait's shape a direct mirror of the spec table rather than an
/// idiomatic `PartialOrd` impl (a concrete type may still also derive
/// `PartialOrd` separately for internal Rust ergonomics; that is additive,
/// not a replacement for these named methods).
pub trait Ordered: Any {
    /// `less_than` __alias__ `"<"` `(other: Ordered) -> Boolean` (abstract).
    ///
    /// Arithmetic value comparison. Returns `true` if the current object is
    /// less than `other`. Effected and/or redefined in descendants to
    /// provide the appropriate ordering semantics for concrete types.
    ///
    /// In conjunction with `=`, enables the definition of the related
    /// functions `greater_than()` etc. (see below).
    fn less_than(&self, other: &Self) -> bool;

    /// `less_than_or_equal` __alias__ `"<="`, `"≤"` `(other: Ordered) -> Boolean`.
    ///
    /// True if the current object is less than or equal to `other`.
    ///
    /// Spec postcondition (`Post_result`): `not (other < self)`.
    fn less_than_or_equal(&self, other: &Self) -> bool {
        !other.less_than(self)
    }

    /// `greater_than` __alias__ `">"` `(other: Ordered) -> Boolean`.
    ///
    /// True if the current object is greater than `other`.
    ///
    /// Spec postcondition (`Post_result`): `Result = (other < self)`.
    fn greater_than(&self, other: &Self) -> bool {
        other.less_than(self)
    }

    /// `greater_than_or_equal` __alias__ `">="`, `"≥"` `(other: Ordered) -> Boolean`.
    ///
    /// True if the current object is greater than or equal to `other`.
    ///
    /// Spec postcondition (`Post_result`): `Result = (other <= self)`.
    fn greater_than_or_equal(&self, other: &Self) -> bool {
        other.less_than_or_equal(self)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/ordered.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / ordered.adoc §Ordered Class
//   confidence: high
//   todos: 0
//   note: default-method bodies encode the spec's Post_result postconditions directly; concrete Ordered types only need to effect less_than.
// ─────────────────────────────────────────────
