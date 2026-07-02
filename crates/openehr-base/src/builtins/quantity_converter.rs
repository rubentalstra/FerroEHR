//! `Quantity_converter` — quantity unit conversion.
//!
//! openEHR class: `Quantity_converter` (interface), package
//! `base.base_types.builtins`.
//!
//! Quantity conversion.
// TODO(port): `Terminology_code` belongs to
// `base.foundation_types.primitive_types` and has not been transcribed into
// `openehr-foundation` yet. The `use` path below names where it is expected
// to land per the crate layout (PORT_MASTER_PLAN.md Section 9); update once
// that file exists.
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;

/// `Quantity_converter` is a pure function interface (no attributes, no
/// state), so it is transcribed as a Rust trait per ADR-001 §1, mirroring
/// `Math`/`Statistical_evaluator`/`Env`/`Locale` in this same package.
pub trait QuantityConverter {
    /// `convert_value` (value: `Real[1]`, from_units: `String[1]`,
    /// to_units: `String[1]`, property: `Terminology_code[1]`): `Real`.
    ///
    /// Convert `value` of physical property type (e.g. 'pressure' etc) from
    /// one units to another.
    ///
    /// TODO(port): the spec's `Real` parameter/return type maps to this
    /// crate's `openehr_foundation::primitive_types::real::Real`, and the
    /// `String` parameters map directly to `std::string::String` per
    /// `docs/PORTING.md` §14.2 (an ordinary RM attribute of spec type
    /// `String`, distinct from the foundation-types `String`/
    /// `OpenEhrString` class itself — see the PORT NOTE on
    /// `OpenEhrString`); left as `f64`/`&str` pending `Real`'s presence in
    /// this crate's dependency graph at time of writing.
    fn convert_value(
        &self,
        value: f64,
        from_units: &str,
        to_units: &str,
        property: &TerminologyCode,
    ) -> f64;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/quantity_converter.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / quantity_converter.adoc §Quantity_converter Interface
//   confidence: medium
//   todos: 1
//   note: forward-references Terminology_code, not transcribed yet; Real narrowed to f64 pending that type's transcription being wired into this crate's dependency graph; trait has no impl (no concrete conversion table/service specified by the spec itself).
// ─────────────────────────────────────────────
