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
    /// TODO(port): as with `Integer::modulo`, the spec does not state the
    /// sign convention for negative operands; Rust's `%` (truncated
    /// remainder) is used as the direct primitive-operator translation
    /// pending clarification.
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

    fn divide(&self, _other: &Self) -> Self {
        // TODO(port): Numeric::divide cannot express Integer64's true
        // (Integer) -> Double result; see Integer64::divide for the
        // spec-accurate overload.
        todo!(
            "Integer64 as Numeric::divide: spec-accurate divide returns Double, see Integer64::divide"
        )
    }

    fn exponent(&self, _other: &Self) -> Self {
        // TODO(port): Numeric::exponent cannot express Integer64's true
        // (Double) -> Double signature; see Integer64::exponent.
        todo!(
            "Integer64 as Numeric::exponent: spec-accurate exponent takes/returns Double, see Integer64::exponent"
        )
    }

    fn negative(&self) -> Self {
        Integer64(-self.0)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/integer64.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / integer64.adoc §Integer64 Class
//   confidence: medium
//   todos: 2
//   note: spec's actual arithmetic overloads all take a 32-bit Integer operand (asymmetric widening) transcribed as inherent methods; the Numeric trait impl provides a same-type Integer64+Integer64 specialization instead since the trait cannot express the widening shape, with divide/exponent stubbed as in Integer.
// ─────────────────────────────────────────────
