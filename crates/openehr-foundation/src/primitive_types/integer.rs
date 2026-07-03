//! `Integer` — minimal interface of the built-in Integer type.
//!
//! openEHR class: `Integer`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered_Numeric`.
//!
//! Type representing the minimal interface of a built-in Integer type:
//! 32-bit integers.
use super::any::Any;
use super::double::Double;
use super::numeric::Numeric;
use super::ordered::Ordered;
use serde::{Deserialize, Serialize};
// PORT NOTE: `OrderedNumeric` is not implemented explicitly here — it is
// blanket-implemented in `ordered_numeric.rs` for any type already
// satisfying both `Ordered` and `Numeric` (which `Integer` does, below), so
// an explicit `impl OrderedNumeric for Integer {}` would conflict with the
// blanket impl rather than being redundant with it.

/// Transcribed as a transparent newtype over `i32` per `docs/PORTING.md`
/// Section 14.2 (`int` → `i32`), matching the spec's explicit "32-bit
/// integers" description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Integer(pub i32);

impl Integer {
    /// `divide` __alias__ `"/"` `(other: Integer) -> Double` (effected).
    ///
    /// Integer division.
    ///
    /// PORT NOTE: covariant redefinition — the spec narrows this concrete
    /// effector's *result* type from the abstract `Numeric` to `Double`
    /// specifically (not `Integer`, not `Real`). Transcribed as an inherent
    /// method rather than an override of `Numeric::divide` (see the
    /// trait-level PORT NOTE in `numeric.rs` for why the `Numeric` trait
    /// itself cannot carry this heterogeneous-result shape).
    #[must_use]
    pub fn divide(&self, other: &Integer) -> Double {
        Double(f64::from(self.0) / f64::from(other.0))
    }

    /// `exponent` __alias__ `"^"` `(other: Double) -> Double` (effected).
    ///
    /// Integer exponentiation.
    ///
    /// PORT NOTE: covariant redefinition on *both* sides — the spec types
    /// this effector's parameter as `Double`, not `Integer`, and its result
    /// as `Double`, not `Integer`. Transcribed literally as read from the
    /// per-class table rather than "corrected" to an all-`Integer`
    /// signature.
    #[must_use]
    pub fn exponent(&self, other: &Double) -> Double {
        Double(f64::from(self.0).powf(other.0))
    }

    /// `modulo` __alias__ `"mod"`, `"\\"` `(mod: Integer) -> Integer`.
    ///
    /// Return `self` modulo `other`.
    ///
    /// PORT NOTE: the spec does not state the sign convention for negative
    /// operands; per ADR-003 decision 4 this is truncated division (the
    /// result takes the dividend's sign) — Rust's native `%`, identical to
    /// Java's `%`, i.e. the behaviour a faithful EHRbase port exercises.
    #[must_use]
    pub fn modulo(&self, other: &Integer) -> Integer {
        Integer(self.0 % other.0)
    }
}

impl Any for Integer {
    /// `is_equal(other: Integer) -> Boolean` (effected).
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// `equal` __alias__ `"="`, `"=="` `(other: Integer) -> Boolean` (redefined).
    ///
    /// Spec marks this `(redefined)` explicitly at the `Integer` level;
    /// the default `Any::equal` (delegating to `is_equal`) already produces
    /// the same result for this value type, so the override exists here
    /// only to make the spec's `(redefined)` marker visible at the correct
    /// concrete type, per the RM transcription rule for covariant/redefined
    /// members.
    fn equal(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    fn type_of(&self) -> String {
        "Integer".to_string()
    }
}

impl Ordered for Integer {
    /// `less_than` __alias__ `"<"` `(other: Integer) -> Boolean` (effected).
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

impl Numeric for Integer {
    /// `add` __alias__ `"+"` `(other: Integer) -> Integer` (effected).
    fn add(&self, other: &Self) -> Self {
        Integer(self.0 + other.0)
    }

    /// `subtract` __alias__ `"-"` `(other: Integer) -> Integer` (effected).
    fn subtract(&self, other: &Self) -> Self {
        Integer(self.0 - other.0)
    }

    /// `multiply` __alias__ `"*"` `(other: Integer) -> Integer` (effected).
    fn multiply(&self, other: &Self) -> Self {
        Integer(self.0 * other.0)
    }

    // PORT NOTE: the spec's `divide`/`exponent` effectors for Integer
    // return/take `Double` and live as the inherent methods above; the
    // `Numeric` trait deliberately does not carry those two members (see
    // the PORT NOTE on the trait in `numeric.rs`).

    /// `negative` __alias__ `"-"` `(): Integer` (effected).
    fn negative(&self) -> Self {
        Integer(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Integer;
    use crate::primitive_types::double::Double;
    use crate::primitive_types::numeric::Numeric;
    use crate::primitive_types::ordered::Ordered;

    // Spec: divide (alias "/") "Integer division", result covariantly
    // narrowed to Double.
    #[test]
    fn divide_returns_a_double() {
        assert_eq!(Integer(7).divide(&Integer(2)), Double(3.5));
        assert_eq!(Integer(-6).divide(&Integer(4)), Double(-1.5));
    }

    // Spec: exponent (alias "^") "Integer exponentiation", parameter and
    // result both Double.
    #[test]
    fn exponent_takes_and_returns_double() {
        assert_eq!(Integer(2).exponent(&Double(10.0)), Double(1024.0));
        assert_eq!(Integer(9).exponent(&Double(0.5)), Double(3.0));
    }

    // Spec: modulo (alias "mod") "Return self modulo other"; sign
    // convention is truncated division per ADR-003 decision 4 (result takes
    // the dividend's sign, matching Rust's and Java's `%`).
    #[test]
    fn modulo_uses_truncated_division_per_adr_003() {
        assert_eq!(Integer(7).modulo(&Integer(3)), Integer(1));
        assert_eq!(Integer(-7).modulo(&Integer(3)), Integer(-1));
        assert_eq!(Integer(7).modulo(&Integer(-3)), Integer(1));
        assert_eq!(Integer(-7).modulo(&Integer(-3)), Integer(-1));
    }

    // Spec Numeric effectors: add/subtract/multiply/negative (same-type).
    #[test]
    fn same_type_arithmetic_effectors() {
        assert_eq!(Integer(2).add(&Integer(3)), Integer(5));
        assert_eq!(Integer(2).subtract(&Integer(3)), Integer(-1));
        assert_eq!(Integer(2).multiply(&Integer(3)), Integer(6));
        assert_eq!(Integer(2).negative(), Integer(-2));
    }

    // Spec Ordered: less_than (effected) plus the Post_result-derived
    // default comparisons.
    #[test]
    fn ordered_comparisons() {
        assert!(Integer(1).less_than(&Integer(2)));
        assert!(!Integer(2).less_than(&Integer(1)));
        assert!(Integer(2).less_than_or_equal(&Integer(2)));
        assert!(Integer(3).greater_than(&Integer(2)));
        assert!(Integer(3).greater_than_or_equal(&Integer(3)));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/integer.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / integer.adoc §Integer Class
//   confidence: high
//   todos: 0
//   note: spec-accurate divide (Integer -> Double) and exponent (Double -> Double) live as inherent methods since the Numeric trait no longer carries those members (numeric.rs PORT NOTE); modulo uses truncated division per ADR-003 decision 4 (spec is silent on the sign convention).
// ─────────────────────────────────────────────
