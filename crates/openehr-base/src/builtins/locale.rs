//! `Locale` — access to the current locale.
//!
//! openEHR class: `Locale` (interface), package `base.base_types.builtins`.
//!
//! Class representing current Locale.
// TODO(port): `Terminology_code` belongs to
// `base.foundation_types.primitive_types` and has not been transcribed into
// `openehr-foundation` yet. The `use` path below names where it is expected
// to land per the crate layout (PORT_MASTER_PLAN.md Section 9); update once
// that file exists.
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;

/// `Locale` is a pure function interface (no attributes, no state), so it is
/// transcribed as a Rust trait per ADR-001 §1, mirroring `Env` in this same
/// package.
pub trait Locale {
    /// `primary_language` (): `Terminology_code`.
    ///
    /// Primary language of the current locale.
    ///
    /// TODO(port): no concrete `impl Locale` exists yet in this crate; the
    /// spec does not itself name a singleton accessor for "the current
    /// locale", so none is invented here.
    fn primary_language(&self) -> TerminologyCode;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/locale.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / locale.adoc §Locale Interface
//   confidence: medium
//   todos: 2
//   note: forward-references Terminology_code, not transcribed yet in openehr-foundation::primitive_types; trait has no impl (no concrete locale source specified by the spec itself).
// ─────────────────────────────────────────────
