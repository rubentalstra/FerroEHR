//! openEHR ADL 2.4.0 — the hand-written ADL2 engine: ADL2/cADL/ODIN source
//! parser, the AOM2 validation catalogue, specialisation flattening, OPT2
//! generation, and ADL 1.4→2 conversion, built over the generated
//! `openehr_am::am24` object model.
//!
//! Spec oracle: `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/` (+ the ODIN
//! spec at `docs/specs/openehr/LANG/docs/odin/`). The normative ANTLR4
//! grammars are vendored under `vendor/grammar/` as reference input for the
//! hand-written `logos`/`chumsky` implementation — no ANTLR runtime.
//!
//! Components: the [`lexer`], the outer artefact parser ([`source`]), the
//! shared typed [`error`] catalogue, and the cADL definition-section parser
//! ([`cadl`]) that builds the generated `openehr_am::am24::aom2` constraint
//! model. Under all of it sits the shared substrate every component reads
//! through: [`aom`] (AOM2 field accessors, constructors, interval arithmetic),
//! [`artefact`] (artefact views + the parent/supplier repository), [`hrid`]
//! (the archetype-id grammar), and [`odin`] (the ODIN reading bridge + the
//! `master03` lexical decoding). Above it:
//! [`codes`]/[`paths`] (code math + ADL paths), [`validate`] (the AOM2
//! validation catalogue), [`flatten`] + [`opt`] (specialisation flattening +
//! OPT2), the [`printer`], and [`adl14`] (ADL 1.4→2 conversion — our own
//! design, no openEHR spec). ODIN sections are parsed by the
//! `openehr_lang::odin` reader (ODIN is a LANG-component spec); the `rules`
//! body and slot assertion expressions are captured as raw text alongside a
//! structured placeholder.
//! TODO: model rule/slot assertion expressions as full BEL/beom trees.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
pub mod adl14;
pub mod aom;
pub mod artefact;
pub mod assemble;
pub mod cadl;
pub mod codes;
pub mod error;
pub mod flatten;
pub mod hrid;
pub mod lexer;
pub mod meta;
pub mod odin;
pub mod opt;
pub mod paths;
pub mod printer;
pub mod rules;
pub mod source;
pub mod validate;

/// The openEHR specification version this crate implements — the crate
/// version itself: the spec crates are versioned by the specification they
/// implement (`docs/VERSIONS.md` §Product and crate versioning), so
/// consumers read the pin from the package, never from a hand-typed literal.
pub const SPEC_VERSION: &str = env!("CARGO_PKG_VERSION");
