//! `Math` — mathematical computation.
//!
//! openEHR class: `Math` (interface), package `base.base_types.builtins`.
//!
//! Mathematical computation.
use openehr_foundation::primitive_types::numeric::Numeric;

/// `Math` is a pure function interface (no attributes, no state), so it is
/// transcribed as a Rust trait per ADR-001 §1, generic over the numeric
/// operand type per the spec's own `Numeric` parameter type, mirroring
/// `Statistical_evaluator` in this same package.
pub trait Math<T: Numeric> {
    /// `ln` (v: `Numeric[1]`): `Double`.
    ///
    /// Compute natural log of `v`.
    ///
    /// TODO(port): return type is spec-declared `Double`
    /// (`openehr_foundation::primitive_types::double::Double`); left as
    /// `f64` pending that type's presence in this crate's dependency graph
    /// at time of writing, per the primitive-type std mapping (ADR-001 §7).
    fn ln(&self, v: &T) -> f64;

    /// `log` (v: `Numeric[1]`): `Double`.
    ///
    /// Compute base 10 log of `v`.
    fn log(&self, v: &T) -> f64;

    /// `sin` (v: `Numeric[1]`): `Double`.
    ///
    /// Compute `sin(v)`.
    fn sin(&self, v: &T) -> f64;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/math.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / math.adoc §Math Interface
//   confidence: medium
//   todos: 1
//   note: Double return type narrowed to f64 pending that type's transcription being wired into this crate's dependency graph; the spec table lists only ln/log/sin (no other functions) despite the class-level "Mathematical computation" description reading broader.
// ─────────────────────────────────────────────
