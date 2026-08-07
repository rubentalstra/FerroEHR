# openehr-query

**A hand-written openEHR AQL 1.1 front end for Rust**: lexer, parser, and
typed AST for the Archetype Query Language — native `logos` + `chumsky`, no
ANTLR runtime.

## What it provides

- A complete AQL 1.1 lexer and parser producing a typed AST (`SELECT`,
  `FROM`/`CONTAINS`, `WHERE`, `ORDER BY`, `LIMIT`, path expressions with
  predicates, parameters, functions), corpus-validated against the official
  grammar's example set.
- Typed parse errors: `ParseError` separates a lexing refusal (carrying the
  lexer's error as a real `source()`) from a grammar refusal (carrying every
  position the parser reported, as token-stream indices), so a caller branches
  on the failure instead of reading its message. `Display` is the located
  diagnostic to show a human.
- `printer::to_aql` — canonical AQL rendering of the AST, the parser's
  inverse, for programmatic query construction (corpus-verified fixed point:
  `parse(to_aql(ast)) == ast`).
- The AST is engine-agnostic: this crate deliberately stops at the syntax
  layer, so any execution engine (SQL generation, in-memory evaluation) can
  build on it.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification. The implemented
spec version is always available at runtime as the crate-level constant
`openehr_query::SPEC_VERSION` (`"1.1.0"` — QUERY/AQL), independent of the
package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

MIT ([`LICENSE-MIT`](LICENSE-MIT)). The crate is hand-written; the openEHR
specification text it implements is the authority but is not embedded in the
package.

## Part of FerroEHR

This crate is the query-language layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
