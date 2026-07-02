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
// PORT NOTE: `OrderedNumeric` is not implemented explicitly here — it is
// blanket-implemented in `ordered_numeric.rs` for any type already
// satisfying both `Ordered` and `Numeric` (which `Integer` does, below), so
// an explicit `impl OrderedNumeric for Integer {}` would conflict with the
// blanket impl rather than being redundant with it.

/// Transcribed as a transparent newtype over `i32` per `docs/PORTING.md`
/// Section 14.2 (`int` → `i32`), matching the spec's explicit "32-bit
/// integers" description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// TODO(port): the spec does not state the sign convention (Euclidean
    /// vs. truncated remainder) for negative operands. Rust's `%` on `i32`
    /// is truncated (sign follows the dividend), used here as the direct
    /// translation of the underlying primitive operator pending a spec
    /// clarification; flagged rather than silently assumed correct for
    /// every RM call site that will eventually use this.
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

    /// PORT NOTE: `Numeric::divide` is same-type (`Self -> Self`) per the
    /// trait's shape, but the spec's actual `Integer.divide` effector
    /// returns `Double` (see the inherent `Integer::divide` method above,
    /// which is the spec-faithful one). This trait-required override exists
    /// only to satisfy `Numeric`'s object shape; TODO(port) tracks removing
    /// `divide`/`exponent` from the `Numeric` trait once every concrete
    /// `Numeric` type in this cluster exists and the trait can be
    /// re-shaped without an interim inconsistency.
    fn divide(&self, _other: &Self) -> Self {
        // TODO(port): Numeric::divide cannot express Integer's true
        // Integer -> Double result; see PORT NOTE above and Integer::divide.
        todo!(
            "Integer as Numeric::divide: spec-accurate divide returns Double, see Integer::divide"
        )
    }

    /// See `divide` PORT NOTE above; same trait/inherent-method split
    /// applies to `exponent`.
    fn exponent(&self, _other: &Self) -> Self {
        // TODO(port): Numeric::exponent cannot express Integer's true
        // (Double) -> Double signature; see PORT NOTE above and
        // Integer::exponent.
        todo!(
            "Integer as Numeric::exponent: spec-accurate exponent takes/returns Double, see Integer::exponent"
        )
    }

    /// `negative` __alias__ `"-"` `(): Integer` (effected).
    fn negative(&self) -> Self {
        Integer(-self.0)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/integer.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / integer.adoc §Integer Class
//   confidence: medium
//   todos: 3
//   note: Numeric trait's divide/exponent cannot carry Integer's true Integer->Double / Double->Double signatures, so the trait impls are stubbed todo!() and the spec-accurate versions live as inherent methods; modulo's sign convention for negative operands is unspecified by the spec.
// ─────────────────────────────────────────────
