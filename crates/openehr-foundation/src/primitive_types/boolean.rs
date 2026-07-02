//! `Boolean` — minimal interface of the built-in Boolean type.
//!
//! openEHR class: `Boolean`, package `base.foundation_types.primitive_types`.
//! Inherits: `Any`.
//!
//! Type representing the minimal interface of a built-in Boolean type:
//! logical True/False values, usually physically represented as an integer,
//! but need not be.
use super::any::Any;

/// Transcribed as a transparent newtype over `bool` per `docs/PORTING.md`
/// Section 14.2/14.4 (`boolean` → `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Boolean(pub bool);

impl Boolean {
    /// `conjunction` __alias__ `"and"`, `"∧"`, `"&"` `(other: Boolean) -> Boolean`.
    ///
    /// Logical conjunction of `self` with `other`.
    ///
    /// Spec postconditions:
    /// - `Post_de_Morgan`: `Result = not (not self or not other)`
    /// - `Post_commutative`: `Result = (other and self)`
    #[must_use]
    pub fn conjunction(&self, other: &Boolean) -> Boolean {
        Boolean(self.0 && other.0)
    }

    /// `semistrict_conjunction` __alias__ `"and then"`, `"&&"` `(other: Boolean) -> Boolean`.
    ///
    /// Boolean semi-strict conjunction with `other`.
    ///
    /// Spec postcondition (`Post_de_Morgan`):
    /// `Result = not (not self or else not other)`.
    ///
    /// Named method per the RM transcription rule for symbolic/keyword
    /// operators (`and then`), backed by Rust's native short-circuiting
    /// `&&` for the semi-strict evaluation semantics the spec describes.
    pub fn semistrict_conjunction(&self, other: impl FnOnce() -> Boolean) -> Boolean {
        Boolean(self.0 && other().0)
    }

    /// `disjunction` __alias__ `"or"`, `"∨"`, `"|"` `(other: Boolean) -> Boolean`.
    ///
    /// Boolean disjunction with `other`.
    ///
    /// Spec postconditions:
    /// - `Post_de_Morgan`: `Result = not (not self and not other)`
    /// - `Post_commutative`: `Result = (other or Current)`
    /// - `Post_consistent_with_semi_strict`: `Result implies (self or else other)`
    #[must_use]
    pub fn disjunction(&self, other: &Boolean) -> Boolean {
        Boolean(self.0 || other.0)
    }

    /// `semistrict_disjunction` __alias__ `"or else"`, `"||"` `(other: Boolean) -> Boolean`.
    ///
    /// Boolean semi-strict disjunction with `other`.
    ///
    /// Spec postcondition (`Post_de_Morgan`):
    /// `Result = not (not self and then not other)`.
    pub fn semistrict_disjunction(&self, other: impl FnOnce() -> Boolean) -> Boolean {
        Boolean(self.0 || other().0)
    }

    /// `exclusive_disjunction` __alias__ `"xor"`, `"⊻"` `(other: Boolean) -> Boolean`.
    ///
    /// Boolean exclusive or with `other`.
    ///
    /// Spec postcondition (`Post_definition`):
    /// `Result = ((self or other) and not (self and other))`.
    #[must_use]
    pub fn exclusive_disjunction(&self, other: &Boolean) -> Boolean {
        Boolean(self.0 ^ other.0)
    }

    /// `implication` __alias__ `"implies"`, `"⇒"` `(other: Boolean) -> Boolean`.
    ///
    /// Boolean implication of `other` (semi-strict).
    ///
    /// Spec postcondition (`Post_definition`):
    /// `Result = (not self or else other)`.
    pub fn implication(&self, other: impl FnOnce() -> Boolean) -> Boolean {
        Boolean(!self.0 || other().0)
    }

    /// `negation` __alias__ `"not"`, `"¬"`, `"!"` `(): Boolean`.
    ///
    /// Boolean negation of the current value.
    #[must_use]
    pub fn negation(&self) -> Boolean {
        Boolean(!self.0)
    }
}

impl Any for Boolean {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Boolean".to_string()
    }
}

// TODO(port): the spec's three class-level invariants below are stated over
// arbitrary self-values, not over a specific constructed instance, so they
// are properties of the *operations* (already encoded structurally by
// delegating to Rust's native `bool` operators above) rather than a runtime
// check any single `Boolean` value could fail. Left as a documented
// `Validate`-style TODO per the RM transcription invariant rule rather than
// silently omitted; a property-based test (`proptest`) exercising these
// three laws over arbitrary `Boolean` pairs is the natural verification once
// the test harness exists.
//
// - `Involutive_negation`: `is_equal (not (not self))`
// - `Non_contradiction`: `not (self and (not self))`
// - `Completeness`: `self or else (not self)`

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/boolean.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / boolean.adoc §Boolean Class
//   confidence: high
//   todos: 1
//   note: three spec invariants (Involutive_negation, Non_contradiction, Completeness) are laws over the operations, documented but not encoded as a Validate impl since there is no per-instance state to check; candidate for a proptest law-check later.
// ─────────────────────────────────────────────
