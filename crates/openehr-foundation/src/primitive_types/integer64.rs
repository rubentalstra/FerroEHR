//! `Integer64` — minimal interface of the built-in Integer64 type.
//!
//! openEHR class: `Integer64`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered_Numeric`.
//!
//! Type representing the minimal interface of a built-in Integer64 type:
//! 64-bit integers.
use super::any::Any;
use super::double::Double;
use super::integer::Integer;
use super::numeric::Numeric;
use super::ordered::Ordered;
// PORT NOTE: `OrderedNumeric` is blanket-implemented in `ordered_numeric.rs`
// for any type satisfying both `Ordered` and `Numeric`; no explicit impl
// needed or possible here (see the equivalent note in `integer.rs`).

/// Transcribed as a transparent newtype over `i64` per `docs/PORTING.md`
/// Section 14.2 (`long` → `i64`), matching the spec's explicit "64-bit
/// integers" description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Integer64(pub i64);

impl Integer64 {
    /// `add` __alias__ `"+"` `(other: Integer) -> Integer64` (effected).
    ///
    /// Large integer addition.
    ///
    /// PORT NOTE: covariant/asymmetric parameter typing straight from the
    /// spec table — the operand `other` is typed `Integer` (32-bit), not
    /// `Integer64`, while the result is `Integer64`. Transcribed literally
    /// as an inherent method rather than through the `Numeric` trait (which
    /// requires a same-type `&Self -> Self` shape); see `numeric.rs` for the
    /// same-type trait method that `Numeric::add` provides in addition to
    /// this spec-accurate widening overload.
    #[must_use]
    pub fn add(&self, other: &Integer) -> Integer64 {
        Integer64(self.0 + i64::from(other.0))
    }

    /// `subtract` __alias__ `"-"` `(other: Integer) -> Integer64` (effected).
    ///
    /// Large integer subtraction.
    #[must_use]
    pub fn subtract(&self, other: &Integer) -> Integer64 {
        Integer64(self.0 - i64::from(other.0))
    }

    /// `multiply` __alias__ `"*"` `(other: Integer) -> Integer64` (effected).
    ///
    /// Large integer multiplication.
    #[must_use]
    pub fn multiply(&self, other: &Integer) -> Integer64 {
        Integer64(self.0 * i64::from(other.0))
    }

    /// `divide` __alias__ `"/"` `(other: Integer) -> Double` (effected).
    ///
    /// Large integer division.
    #[must_use]
    pub fn divide(&self, other: &Integer) -> Double {
        Double(self.0 as f64 / f64::from(other.0))
    }

    /// `exponent` __alias__ `"^"` `(other: Double) -> Double` (effected).
    ///
    /// Large integer exponentiation.
    #[must_use]
    pub fn exponent(&self, other: &Double) -> Double {
        Double((self.0 as f64).powf(other.0))
    }

    /// `modulo` __alias__ `"mod"`, `"\\"` `(mod: Integer) -> Integer64`.
    ///
    /// Large integer modulus.
    ///
    /// PORT NOTE: as with `Integer::modulo`, the spec does not state the
    /// sign convention for negative operands; per ADR-003 decision 4 this
    /// is truncated division (Rust's/Java's `%` — the result takes the
    /// dividend's sign).
    #[must_use]
    pub fn modulo(&self, other: &Integer) -> Integer64 {
        Integer64(self.0 % i64::from(other.0))
    }

    /// `negative` __alias__ `"-"` `(): Integer64` (effected).
    ///
    /// Generate the negative of the current Integer64 value.
    ///
    /// PORT NOTE: also provided as the `Numeric::negative` trait
    /// implementation below (a same-type unary operation, so no
    /// trait/inherent split is needed for this particular member, unlike
    /// `add`/`subtract`/`multiply`/`divide`/`exponent` above).
    #[must_use]
    pub fn negative_value(&self) -> Integer64 {
        Integer64(-self.0)
    }
}

impl Any for Integer64 {
    /// `is_equal(other: Integer64) -> Boolean` (effected).
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// `equal` __alias__ `"="`, `"=="` `(other: Integer64) -> Boolean` (redefined).
    fn equal(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    fn type_of(&self) -> String {
        "Integer64".to_string()
    }
}

impl Ordered for Integer64 {
    /// `less_than` __alias__ `"<"` `(other: Integer64) -> Boolean` (effected).
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

impl Numeric for Integer64 {
    /// PORT NOTE: `Numeric::add` requires a same-type `&Self -> Self`
    /// shape, but the spec's actual `Integer64.add` effector takes an
    /// `Integer` operand (see the inherent `Integer64::add` above, which is
    /// the spec-faithful widening overload). This same-type trait
    /// implementation is provided so `Integer64` satisfies `Numeric` /
    /// `OrderedNumeric`, treating `other` as an `Integer64 + Integer64`
    /// same-type addition — a reasonable same-type specialization that is
    /// not itself drawn from a distinct spec table row (the spec never
    /// states an `Integer64, Integer64 -> Integer64` overload explicitly).
    fn add(&self, other: &Self) -> Self {
        Integer64(self.0 + other.0)
    }

    fn subtract(&self, other: &Self) -> Self {
        Integer64(self.0 - other.0)
    }

    fn multiply(&self, other: &Self) -> Self {
        Integer64(self.0 * other.0)
    }

    // PORT NOTE: the spec's `divide`/`exponent` effectors for Integer64
    // take an `Integer`/`Double` operand and return `Double`, living as the
    // inherent methods above; the `Numeric` trait deliberately does not
    // carry those two members (see the PORT NOTE on the trait in
    // `numeric.rs`).

    fn negative(&self) -> Self {
        Integer64(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Integer64;
    use crate::primitive_types::double::Double;
    use crate::primitive_types::integer::Integer;

    // Spec: the arithmetic effectors take an Integer (32-bit) operand and
    // return Integer64 (asymmetric widening, straight from the table).
    #[test]
    fn widening_arithmetic_with_an_integer_operand() {
        assert_eq!(
            Integer64(5_000_000_000).add(&Integer(2)),
            Integer64(5_000_000_002)
        );
        assert_eq!(Integer64(10).subtract(&Integer(3)), Integer64(7));
        assert_eq!(Integer64(4).multiply(&Integer(3)), Integer64(12));
    }

    // Spec: divide "Large integer division" -> Double; exponent takes and
    // returns Double.
    #[test]
    fn divide_and_exponent_involve_double() {
        assert_eq!(Integer64(7).divide(&Integer(2)), Double(3.5));
        assert_eq!(Integer64(3).exponent(&Double(2.0)), Double(9.0));
    }

    // Spec: modulo "Large integer modulus"; truncated division per ADR-003
    // decision 4.
    #[test]
    fn modulo_uses_truncated_division_per_adr_003() {
        assert_eq!(Integer64(7).modulo(&Integer(3)), Integer64(1));
        assert_eq!(Integer64(-7).modulo(&Integer(3)), Integer64(-1));
        assert_eq!(Integer64(7).modulo(&Integer(-3)), Integer64(1));
    }

    // Spec: negative "Generate the negative of the current Integer64 value".
    #[test]
    fn negative_value_negates() {
        assert_eq!(Integer64(42).negative_value(), Integer64(-42));
        assert_eq!(Integer64(-1).negative_value(), Integer64(1));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/integer64.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / integer64.adoc §Integer64 Class
//   confidence: high
//   todos: 0
//   note: spec's actual arithmetic overloads all take a 32-bit Integer operand (asymmetric widening) transcribed as inherent methods; the Numeric trait impl provides a same-type Integer64+Integer64 specialization for add/subtract/multiply/negative, and the trait no longer carries divide/exponent (numeric.rs PORT NOTE). modulo uses truncated division per ADR-003 decision 4.
// ─────────────────────────────────────────────
