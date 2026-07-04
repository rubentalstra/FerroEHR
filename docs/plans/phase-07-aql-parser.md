# Phase 07 — AQL parser + AST

- Status: **done** (semantic path analysis deferred to P16, where it belongs)
- Build order: complete (spec foundation for the AQL engine)
- Consumes: QUERY 1.1.0 grammar (vendored)

## Outcome

`openehr-query` hand-writes the AQL 1.1.0 front end: a `logos` lexer, the AST,
and a `chumsky` parser covering the full `AqlParser.g4` (nodePredicate AND/OR +
`MATCHES`/`CONTAINED_REGEX`, precedence, EOF enforcement). Decision (settled):
`logos`+`chumsky`, **not** `antlr-rust` (unmaintained; violates the no-ANTLR-
runtime rule). Grammar + example corpus vendored at `crates/openehr-query/vendor/`.

**Semantic path analysis** (resolving AQL paths against WebTemplates) is **not**
here — it needs the WebTemplate builder (P14) and is done as part of the AQL
engine (P16).

## Verification

`openehr-query` builds clean; `tests/corpus.rs` parses every SELECT block in the
official `AQL_examples` (standard-AQL blocks parse; documented exclusions for
embedded cADL / SQL-subquery examples).
