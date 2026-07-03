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

    /// `divide` __alias__ `"/"` `(other: Double) -> Double` (effected).
    ///
    /// PORT NOTE: `Double` is the one concrete type in this cluster whose
    /// spec-declared `divide` happens to be same-type (`Double, Double ->
    /// Double`); it is nonetheless an inherent method, like every other
    /// concrete type's `divide`, because the `Numeric` trait deliberately
    /// does not carry the spec's `divide`/`exponent` members (see the PORT
    /// NOTE on the trait in `numeric.rs`).
    #[must_use]
    pub fn divide(&self, other: &Double) -> Double {
        Double(self.0 / other.0)
    }

    /// `exponent` __alias__ `"^"` `(other: Double) -> Double` (effected).
    ///
    /// Same inherent-method note as `divide` above.
    #[must_use]
    pub fn exponent(&self, other: &Double) -> Double {
        Double(self.0.powf(other.0))
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

    /// `negative` __alias__ `"-"` `(): Double` (effected).
    fn negative(&self) -> Self {
        Double(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Double;
    use crate::primitive_types::integer::Integer;
    use crate::primitive_types::numeric::Numeric;

    // Spec: floor "Return the greatest integer no greater than the value of
    // this object".
    #[test]
    fn floor_is_greatest_integer_no_greater() {
        assert_eq!(Double(2.9).floor(), Integer(2));
        assert_eq!(Double(-2.1).floor(), Integer(-3));
    }

    // Spec: divide/exponent are Double -> Double effectors.
    #[test]
    fn divide_and_exponent_are_same_type() {
        assert_eq!(Double(1.0).divide(&Double(4.0)), Double(0.25));
        assert_eq!(Double(2.0).exponent(&Double(-1.0)), Double(0.5));
    }

    // Spec Numeric effectors (same-type shape).
    #[test]
    fn same_type_arithmetic_effectors() {
        assert_eq!(Double(0.5).add(&Double(0.25)), Double(0.75));
        assert_eq!(Double(0.5).subtract(&Double(0.25)), Double(0.25));
        assert_eq!(Double(0.5).multiply(&Double(4.0)), Double(2.0));
        assert_eq!(Double(0.5).negative(), Double(-0.5));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/double.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / double.adoc §Double Class
//   confidence: high
//   todos: 0
//   note: divide/exponent are inherent methods (the one same-type pair in the cluster) since the Numeric trait no longer carries those members (numeric.rs PORT NOTE); trait carries add/subtract/multiply/negative.
// ─────────────────────────────────────────────
