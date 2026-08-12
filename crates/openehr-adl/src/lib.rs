// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! openEHR ADL 2.4.0 — the hand-written ADL2 engine: ADL2/cADL/ODIN source
//! parser, the AOM2 validation catalogue, specialisation flattening, OPT2
//! generation, and ADL 1.4→2 conversion, built over the generated
//! `openehr_am::v2_4` object model.
//!
//! Spec oracle: `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/` (+ the ODIN
//! spec at `docs/specs/openehr/LANG/docs/odin/`). The normative ANTLR4
//! grammars are vendored under `vendor/grammar/` (version-scoped by AM
//! generation: `v1_4/`, `v2_4/`) as reference input for the
//! hand-written `logos`/`chumsky` implementation — no ANTLR runtime.
//!
//! Components: the outer artefact parser ([`source`]), the
//! shared typed [`error`] catalogue, and the cADL definition-section parser
//! ([`parse`]) that builds the generated `openehr_am::v2_4::aom2` constraint
//! model. Under all of it sits the shared substrate every component reads
//! through: [`aom`] (AOM2 field accessors, constructors, interval arithmetic),
//! [`artefact`] (artefact views + the parent/supplier repository), [`hrid`]
//! (the archetype-id grammar), and [`odin`] (the ODIN reading bridge + the
//! `master03` lexical decoding). Above it:
//! [`codes`]/[`paths`] (code math + ADL paths), [`validate`] (the AOM2
//! validation catalogue), [`flatten`] + [`opt`] (specialisation flattening +
//! OPT2), the ADL2 serializer [`mod@print`], and [`adl14`] (ADL 1.4→2 conversion — our own
//! design, no openEHR spec). ODIN sections are parsed by the
//! `openehr_lang::v1_1::odin` reader (ODIN is a LANG-component spec). **This crate
//! has no lexer of its own**: the cADL token stream is the shared workspace
//! lexical layer read under its ADL reading
//! ([`openehr_lang::v1_1::lexer::lex_adl`]). The `rules` body and the slot
//! include/exclude assertions are full BEL/beom expression trees
//! ([`rules`], over `openehr_lang::v1_1::bel`); their string form is rendered back
//! from the tree ([`print::assertion_text`]), never parsed out of it.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
pub mod adl14;
pub mod aom;
pub mod artefact;
pub mod assemble;
pub mod codes;
pub mod error;
pub mod flatten;
pub mod hrid;
pub mod meta;
pub mod odin;
pub mod opt;
pub mod parse;
pub mod paths;
pub mod print;
pub mod rules;
pub mod source;
pub mod validate;

/// The openEHR specification version this crate implements.
///
/// The pin is deliberately independent of the crates.io package version,
/// which is the crate's own `SemVer` line and moves only with this
/// implementation's code.
pub const SPEC_VERSION: &str = "2.4.0";
