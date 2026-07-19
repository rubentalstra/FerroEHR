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
//! Phases so far: the [`lexer`], the outer artefact parser ([`source`]), the
//! shared typed [`error`] catalogue, and the cADL definition-section parser
//! ([`cadl`], phase A3a) that builds the generated `openehr_am::am24::aom2`
//! constraint model. ODIN sections are parsed by the `openehr_lang::odin`
//! reader (ODIN is a LANG-component spec); the `rules` body and slot
//! assertion expressions are captured as raw text for a later phase.

pub mod assemble;
pub mod cadl;
pub mod codes;
pub mod error;
pub mod lexer;
pub mod paths;
pub mod printer;
pub mod rules;
pub mod source;
pub mod validate;
