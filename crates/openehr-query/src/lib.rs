// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! openEHR **QUERY (AQL 1.1.0)**: a hand-written lexer, parser, and AST,
//! reimplemented natively from the canonical ANTLR4 grammar (vendored at
//! `vendor/grammar/`) — **no ANTLR runtime** is a dependency (see
//! `.claude/rules/aql-engine.md`). AQL has no BMM meta-model, so unlike the
//! generated openEHR spec crates this one is written by hand against the
//! grammar, with the worked-example corpus (`vendor/examples/`) as tests.
//!
//! Pipeline boundary: this crate produces a parsed, semantically-analyzable
//! AST. The AST→IR→SQL engine is **not** here — it lives in
//! `app/ferroehr/src/aql/` (our own typed query IR over the greenfield node
//! model; no openEHR spec governs the SQL shapes).
//!
//! Layers (built incrementally):
//! - [`lexer`] — `logos` tokenizer from `AqlLexer.g4`. ✅
//! - [`ast`] — the syntax tree, one type per `AqlParser.g4` rule. ✅
//! - [`parser`] — `chumsky` parser tokens → [`ast::SelectQuery`]. ✅
//! - `semantics` — path analysis against Web Templates. (later)

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod printer;

/// The openEHR specification version this crate implements.
///
/// The pin is deliberately independent of the crates.io package version,
/// which is the crate's own `SemVer` line and moves only with this
/// implementation's code.
pub const SPEC_VERSION: &str = "1.1.0";
