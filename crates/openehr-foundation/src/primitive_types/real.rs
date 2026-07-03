//! `Real` — decimal number type, single-precision in the published spec.
//!
//! openEHR class: `Real`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered_Numeric`.
//!
//! Type used to represent decimal numbers. The specification text states:
//! "Corresponds to a single-precision floating point value in most
//! languages," and the primitive-types overview table separately describes
//! it as "32-bit real numbers in any interoperable representation, including
//! single-width IEEE floating point."
use super::any::Any;
use super::double::Double;
use super::integer::Integer;
use super::numeric::Numeric;
use super::ordered::Ordered;
use serde::{Deserialize, Serialize};
// PORT NOTE: `OrderedNumeric` is blanket-implemented in `ordered_numeric.rs`
// for any type satisfying both `Ordered` and `Numeric`; no explicit impl
// needed or possible here (see the equivalent note in `integer.rs`).

/// PORT NOTE (directed deviation from the literal spec text): the published
/// BASE 1.2.0 spec describes `Real` as a **single-precision** (32-bit)
/// floating-point type — see the module doc comment above, quoting both the
/// per-class table and the primitive-types overview table verbatim. A
/// strictly literal transcription would therefore back this type with
/// `f32`, mirroring the equally literal `Double` (`f64`, 64-bit) sibling.
///
/// This type is instead backed by `f64`, matching `Double`, per an explicit
/// instruction for this transcription pass rather than an inference of my
/// own. This is a deliberate, directed deviation from the literal spec
/// text, not a silent choice — recorded here, on the type, and in the PORT
/// STATUS trailer below, and worth revisiting explicitly (rather than
/// re-litigating implicitly) if a future phase needs true single-precision
/// fidelity for `Real` (e.g. for exact round-trip parity against a
/// reference EHRbase/Java `float`-backed value, or for
/// `rust_decimal`-adjacent `DV_QUANTITY`/`DV_COUNT` precision work in
/// `openehr-rm`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Real(pub f64);

impl Real {
    /// `floor(): Integer`.
    ///
    /// Return the greatest integer no greater than the value of this
    /// object.
    #[must_use]
    pub fn floor(&self) -> Integer {
        Integer(self.0.floor() as i32)
    }

    /// `divide` __alias__ `"/"` `(other: Real) -> Double` (effected).
    ///
    /// Real number division.
    ///
    /// PORT NOTE: covariant redefinition — the spec narrows this concrete
    /// effector's result type to `Double`, not `Real`. Transcribed as an
    /// inherent method rather than through `Numeric::divide` (same-type
    /// shape only); see the trait-level PORT NOTE in `numeric.rs`.
    #[must_use]
    pub fn divide(&self, other: &Real) -> Double {
        Double(self.0 / other.0)
    }

    /// `exponent` __alias__ `"^"` `(other: Double) -> Double` (effected).
    ///
    /// Real number exponentiation.
    ///
    /// PORT NOTE: covariant redefinition on both sides, transcribed
    /// literally as read from the per-class table (parameter and result
    /// both `Double`, not `Real`).
    #[must_use]
    pub fn exponent(&self, other: &Double) -> Double {
        Double(self.0.powf(other.0))
    }
}

impl Any for Real {
    /// `is_equal(other: Real) -> Boolean` (effected).
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// `equal` __alias__ `"="`, `"=="` `(other: Real) -> Boolean` (redefined).
    fn equal(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    fn type_of(&self) -> String {
        "Real".to_string()
    }
}

impl Ordered for Real {
    /// `less_than` __alias__ `"<"` `(other: Real) -> Boolean` (effected).
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

impl Numeric for Real {
    fn add(&self, other: &Self) -> Self {
        Real(self.0 + other.0)
    }

    fn subtract(&self, other: &Self) -> Self {
        Real(self.0 - other.0)
    }

    fn multiply(&self, other: &Self) -> Self {
        Real(self.0 * other.0)
    }

    // PORT NOTE: the spec's `divide` (Real -> Double) and `exponent`
    // (Double -> Double) effectors live as the inherent methods above; the
    // `Numeric` trait deliberately does not carry those two members (see
    // the PORT NOTE on the trait in `numeric.rs`).

    /// `negative` __alias__ `"-"` `(): Real` (effected).
    fn negative(&self) -> Self {
        Real(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Real;
    use crate::primitive_types::double::Double;
    use crate::primitive_types::integer::Integer;
    use crate::primitive_types::numeric::Numeric;

    // Spec: floor "Return the greatest integer no greater than the value of
    // this object".
    #[test]
    fn floor_is_greatest_integer_no_greater() {
        assert_eq!(Real(3.7).floor(), Integer(3));
        assert_eq!(Real(-3.2).floor(), Integer(-4));
        assert_eq!(Real(5.0).floor(), Integer(5));
    }

    // Spec: divide "Real number division" -> Double; exponent takes and
    // returns Double.
    #[test]
    fn divide_and_exponent_involve_double() {
        assert_eq!(Real(7.0).divide(&Real(2.0)), Double(3.5));
        assert_eq!(Real(2.0).exponent(&Double(3.0)), Double(8.0));
    }

    // Spec Numeric effectors (same-type shape).
    #[test]
    fn same_type_arithmetic_effectors() {
        assert_eq!(Real(1.5).add(&Real(2.5)), Real(4.0));
        assert_eq!(Real(1.5).subtract(&Real(0.5)), Real(1.0));
        assert_eq!(Real(1.5).multiply(&Real(2.0)), Real(3.0));
        assert_eq!(Real(1.5).negative(), Real(-1.5));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/real.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / real.adoc §Real Class
//   confidence: medium
//   todos: 0
//   note: DIRECTED DEVIATION — spec text literally describes Real as single-precision (32-bit); backed by f64 (same as Double) per explicit transcription-pass instruction, not spec literalism. Flag for review if float-precision parity ever matters. Spec-accurate divide/exponent are inherent methods; the Numeric trait no longer carries those members (numeric.rs PORT NOTE).
// ─────────────────────────────────────────────
