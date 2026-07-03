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

// PORT NOTE: the spec's three class-level invariants below are stated over
// arbitrary self-values, not over a specific constructed instance, so they
// are properties of the *operations* (encoded structurally by delegating to
// Rust's native `bool` operators above) rather than a runtime check any
// single `Boolean` value could fail — there is no `Validate` impl to write.
// Per ADR-003 decision 8's intent (invariants become working checks now),
// they are verified exhaustively over the type's entire two-value domain in
// the test module below:
//
// - `Involutive_negation`: `is_equal (not (not self))`
// - `Non_contradiction`: `not (self and (not self))`
// - `Completeness`: `self or else (not self)`

#[cfg(test)]
mod tests {
    use super::Boolean;
    use crate::primitive_types::any::Any;

    const DOMAIN: [Boolean; 2] = [Boolean(false), Boolean(true)];

    // Spec invariant Involutive_negation: `is_equal (not (not self))`.
    #[test]
    fn involutive_negation_holds_over_the_whole_domain() {
        for b in DOMAIN {
            assert!(b.is_equal(&b.negation().negation()));
        }
    }

    // Spec invariant Non_contradiction: `not (self and (not self))`.
    #[test]
    fn non_contradiction_holds_over_the_whole_domain() {
        for b in DOMAIN {
            assert!(b.conjunction(&b.negation()).negation().0);
        }
    }

    // Spec invariant Completeness: `self or else (not self)`.
    #[test]
    fn completeness_holds_over_the_whole_domain() {
        for b in DOMAIN {
            assert!(b.semistrict_disjunction(|| b.negation()).0);
        }
    }

    // Spec Post_de_Morgan on conjunction: `Result = not (not self or not
    // other)`; and on disjunction: `Result = not (not self and not other)`.
    #[test]
    fn de_morgan_postconditions_hold_for_all_pairs() {
        for a in DOMAIN {
            for b in DOMAIN {
                assert!(
                    a.conjunction(&b)
                        .is_equal(&a.negation().disjunction(&b.negation()).negation())
                );
                assert!(
                    a.disjunction(&b)
                        .is_equal(&a.negation().conjunction(&b.negation()).negation())
                );
            }
        }
    }

    // Semi-strict ("and then" / "or else") operators short-circuit: the
    // second operand must not be evaluated when the first decides the
    // result.
    #[test]
    fn semistrict_operators_short_circuit() {
        let poison = || -> Boolean { unreachable!("second operand must not be evaluated") };
        assert!(!Boolean(false).semistrict_conjunction(poison).0);
        assert!(Boolean(true).semistrict_disjunction(poison).0);
        assert!(Boolean(false).implication(poison).0);
    }

    // Spec Post_definition on exclusive_disjunction:
    // `Result = ((self or other) and not (self and other))`.
    #[test]
    fn exclusive_disjunction_matches_its_postcondition() {
        for a in DOMAIN {
            for b in DOMAIN {
                let expected = a.disjunction(&b).conjunction(&a.conjunction(&b).negation());
                assert!(a.exclusive_disjunction(&b).is_equal(&expected));
            }
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/boolean.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / boolean.adoc §Boolean Class
//   confidence: high
//   todos: 0
//   note: three spec invariants (Involutive_negation, Non_contradiction, Completeness) are laws over the operations with no per-instance state to Validate; verified exhaustively over the two-value domain in the in-file tests, alongside the De Morgan/xor postconditions and short-circuit checks.
// ─────────────────────────────────────────────
