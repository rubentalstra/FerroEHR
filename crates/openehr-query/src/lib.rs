// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: Apache-2.0

//! openEHR QUERY (AQL 1.1.0): a hand-written lexer, parser, and AST over the
//! canonical ANTLR4 grammar vendored at `vendor/grammar/`, with no ANTLR
//! runtime dependency. AQL has no BMM meta-model, so this crate is written by
//! hand against the grammar and tested against `vendor/examples/`.
//!
//! The crate stops at the AST: [`lexer`] tokenizes `AqlLexer.g4` with `logos`,
//! [`ast`] carries one type per `AqlParser.g4` rule, and [`parser`] builds an
//! [`ast::SelectQuery`] with `chumsky`. Query execution lives in the
//! application.

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
