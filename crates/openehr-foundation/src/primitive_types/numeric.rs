//! `Numeric` — abstract parent class of numeric types.
//!
//! openEHR class: `Numeric` (abstract), package `base.foundation_types.primitive_types`.
//! Inherits: `Any`.
//!
//! Abstract parent class of numeric types, which are types having various
//! arithmetic and comparison operators defined.
use super::any::Any;

/// `Numeric` is modelled as a Rust trait with `Any` as a supertrait, mirroring
/// the spec's single-parent inheritance (`Numeric` inherits `Any`).
///
/// Symbolic operators (`+`, `-`, `*`, `/`, `^`, unary `-`) are named methods,
/// not `std::ops` overloads, per the RM transcription rules.
///
/// # Covariant redefinition (PORT NOTE)
///
/// The abstract spec signatures type both the parameter and the result as
/// the open ancestor type `Numeric` itself (e.g. `add(other: Numeric):
/// Numeric`), with the spec's own commentary noting: "Actual type of result
/// depends on arithmetic balancing rules." That is deliberately open in the
/// spec's own type system and is not directly representable as a Rust trait
/// method with a single associated `Self`-typed signature while still
/// allowing every concrete effector observed in this cluster (`Integer`,
/// `Integer64`, `Real`, `Double`) to freely vary both operand and result
/// type (e.g. `Integer.divide` takes an `Integer` but returns a `Double`;
/// `Integer64.add` takes an `Integer` but returns an `Integer64`).
///
/// This trait therefore narrows the abstract signature to the *closed,
/// same-type* case — `fn add(&self, other: &Self) -> Self` — which is the
/// shape every concrete `Numeric` effector in this cluster uses for the
/// same-type overloads of `add`/`subtract`/`multiply`/`negative`. The
/// heterogeneous cross-type overloads (`Integer::divide -> Double`,
/// `Integer::exponent(Double) -> Double`, `Integer64::add(Integer) ->
/// Integer64`, etc.) are transcribed as additional *inherent* methods on the
/// concrete types themselves (see `integer.rs`, `integer64.rs`, `real.rs`,
/// `double.rs`), not through this trait, since Rust traits cannot express an
/// open/self-widening return type without associated types keyed per
/// call-site, which the spec does not itself specify formally.
///
/// PORT NOTE: the spec's abstract `divide` and `exponent` members are *not*
/// carried on this trait at all. Every concrete effector in the cluster
/// narrows them to `Double`-involving signatures (`Integer.divide(Integer)
/// -> Double`, `Integer64.divide(Integer) -> Double`, `Real.divide(Real) ->
/// Double`, `Double.divide(Double) -> Double`), so a same-type `&Self ->
/// Self` trait method would match no concrete effector and previously
/// existed only as a `todo!()` stub on three of the four types. With all
/// four concrete `Numeric` types now transcribed, the stubs are resolved by
/// dropping the two members from the trait — the spec-accurate `divide`/
/// `exponent` live as inherent methods on each concrete type, where the
/// per-class tables actually declare their signatures.
pub trait Numeric: Any {
    /// `add` __alias__ `"+"` `(other: Numeric) -> Numeric` (abstract).
    ///
    /// Sum with `other` (commutative). Actual type of result depends on
    /// arithmetic balancing rules (see trait-level PORT NOTE for how the
    /// open result type is narrowed here).
    fn add(&self, other: &Self) -> Self;

    /// `subtract` __alias__ `"-"` `(other: Numeric) -> Numeric` (abstract).
    ///
    /// Result of subtracting `other`. Actual type of result depends on
    /// arithmetic balancing rules.
    fn subtract(&self, other: &Self) -> Self;

    /// `multiply` __alias__ `"*"` `(other: Numeric) -> Numeric` (abstract).
    ///
    /// Product by `other`. Actual type of result depends on arithmetic
    /// balancing rules.
    fn multiply(&self, other: &Self) -> Self;

    /// `negative` __alias__ `"-"` `(): Numeric` (abstract).
    ///
    /// Generate the negative of the current value.
    fn negative(&self) -> Self;
}

// PORT NOTE: raw `i64` stands in for `Integer64` where covariant
// redefinition (`DV_COUNT.magnitude`, ADR-001 §6) uses the bare primitive;
// resolves the P17-flagged bound conflict. See the matching `Any` impl in
// `any.rs` and `Ordered` impl in `ordered.rs`. Bodies delegate to `i64`'s
// native operators.
impl Numeric for i64 {
    fn add(&self, other: &Self) -> Self {
        self + other
    }

    fn subtract(&self, other: &Self) -> Self {
        self - other
    }

    fn multiply(&self, other: &Self) -> Self {
        self * other
    }

    fn negative(&self) -> Self {
        -self
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/numeric.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / numeric.adoc §Numeric Class
//   confidence: medium
//   todos: 0
//   note: the abstract divide/exponent members are not carried on the trait (PORT NOTE at the trait) — every concrete effector narrows them to Double-involving signatures, transcribed as inherent methods on Integer/Integer64/Real/Double per their per-class tables; the trait carries only the same-type add/subtract/multiply/negative shape.
// ─────────────────────────────────────────────
