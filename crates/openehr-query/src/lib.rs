//! AQL 1.1.0 lexer, parser, AST, and semantic path analysis, reimplemented
//! natively from the canonical grammar (no ANTLR runtime).
//!
//! Populated in P12 (`docs/plans/phase-12-aql-parser.md`). The AST→ASL→SQL
//! engine is not here; it lives in `openehr-server/src/aql/`.
