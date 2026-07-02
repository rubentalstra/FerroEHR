//! `Double` — double-precision decimal number type.
//!
//! openEHR class: `Double`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered_Numeric`.
//!
//! Type used to represent double-precision decimal numbers. Corresponds to a
//! double-precision floating point value in most languages. The
//! primitive-types overview table separately describes it as "64-bit real
//! numbers, in any interoperable representation including double-precision
//! IEEE floating point."
use super::any::Any;
use super::integer::Integer;
use super::numeric::Numeric;
use super::ordered::Ordered;
// PORT NOTE: `OrderedNumeric` is blanket-implemented in `ordered_numeric.rs`
// for any type satisfying both `Ordered` and `Numeric`; no explicit impl
// needed or possible here (see the equivalent note in `integer.rs`).

/// Transcribed as a transparent newtype over `f64` per `docs/PORTING.md`
/// Section 14.2 (`double` → `f64`), matching the spec's explicit
/// "double-precision"/"64-bit" description exactly — unlike `Real` in this
/// same cluster, `Double`'s Rust backing type needs no directed deviation
/// (see the PORT NOTE on `Real` for that unrelated case).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Double(pub f64);

impl Double {
    /// `floor(): Integer`.
    ///
    /// Return the greatest integer no greater than the value of this
    /// object.
    #[must_use]
    pub fn floor(&self) -> Integer {
        Integer(self.0.floor() as i32)
    }
}

impl Any for Double {
    /// `is_equal(other: Double) -> Boolean` (effected).
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// `equal` __alias__ `"="`, `"=="` `(other: Double) -> Boolean` (redefined).
    fn equal(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    fn type_of(&self) -> String {
        "Double".to_string()
    }
}

impl Ordered for Double {
    /// `less_than` __alias__ `"<"` `(other: Double) -> Boolean` (effected).
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

impl Numeric for Double {
    /// `add` __alias__ `"+"` `(other: Double) -> Double` (effected).
    fn add(&self, other: &Self) -> Self {
        Double(self.0 + other.0)
    }

    /// `subtract` __alias__ `"-"` `(other: Double) -> Double` (effected).
    fn subtract(&self, other: &Self) -> Self {
        Double(self.0 - other.0)
    }

    /// `multiply` __alias__ `"*"` `(other: Double) -> Double` (effected).
    fn multiply(&self, other: &Self) -> Self {
        Double(self.0 * other.0)
    }

    /// `divide` __alias__ `"/"` `(other: Double) -> Double` (effected).
    ///
    /// PORT NOTE: `Double` is the one concrete type in this cluster whose
    /// spec-declared `divide` result actually matches `Numeric::divide`'s
    /// same-type trait shape (`Double, Double -> Double`) — no inherent
    /// override needed, unlike `Integer`/`Integer64`/`Real`, whose `divide`
    /// narrows to `Double` from a *different* concrete type.
    fn divide(&self, other: &Self) -> Self {
        Double(self.0 / other.0)
    }

    /// `exponent` __alias__ `"^"` `(other: Double) -> Double` (effected).
    ///
    /// Same same-type-shape note as `divide` above applies to `exponent`.
    fn exponent(&self, other: &Self) -> Self {
        Double(self.0.powf(other.0))
    }

    /// `negative` __alias__ `"-"` `(): Double` (effected).
    fn negative(&self) -> Self {
        Double(-self.0)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/double.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / double.adoc §Double Class
//   confidence: high
//   todos: 0
//   note: only concrete Numeric effector in this cluster whose divide/exponent match the trait's same-type shape exactly, so Numeric is fully implemented here with no stub/todo split (contrast Integer/Integer64/Real).
// ─────────────────────────────────────────────
