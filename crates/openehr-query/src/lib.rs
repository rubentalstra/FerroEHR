//! openEHR **QUERY (AQL 1.1.0)**: a hand-written lexer, parser, and AST,
//! reimplemented natively from the canonical ANTLR4 grammar (vendored at
//! `vendor/grammar/`) — **no ANTLR runtime** is a dependency (see
//! `.claude/rules/aql-engine.md`). AQL has no BMM meta-model, so unlike the
//! generated openEHR spec crates this one is written by hand against the
//! grammar, with the worked-example corpus (`vendor/examples/`) as tests.
//!
//! Pipeline boundary: this crate produces a parsed, semantically-analyzable
//! AST. The AST→ASL→SQL engine is **not** here — it lives in
//! `app/ehrbase/src/aql/` (`EHRbase`'s own IR, ported from Java).
//!
//! Layers (built incrementally):
//! - [`lexer`] — `logos` tokenizer from `AqlLexer.g4`. ✅
//! - [`ast`] — the syntax tree, one type per `AqlParser.g4` rule. ✅
//! - [`parser`] — `chumsky` parser tokens → [`ast::SelectQuery`]. ✅
//! - `semantics` — path analysis against Web Templates. (later)

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod printer;

/// The openEHR specification version this crate implements — the crate
/// version itself: the spec crates are versioned by the specification they
/// implement (`docs/VERSIONS.md` §Product and crate versioning), so
/// consumers read the pin from the package, never from a hand-typed literal.
pub const SPEC_VERSION: &str = env!("CARGO_PKG_VERSION");
