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
  position the parser reported), so a caller branches on the failure instead
  of reading its message. Each grammar fault gives the position twice — as
  token-stream indices and as a byte range of the source, so a diagnostic can
  underline the offending text. `Display` is the located diagnostic to show a
  human; it names the token indices.
- `lexer::lex_spanned` — the token stream with each token's source byte span,
  additive beside `lexer::lex`. `parser::parse_spanned` consumes it and is
  what `parser::parse_str` runs; `parser::parse` over a bare `&[Token]` still
  works and reports token indices only.
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

Apache License 2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)). The crate is
hand-written; the openEHR specification text it implements is the authority but
is not embedded in the package. The `openehr-*` crates are Apache-2.0 so any
Rust project can use them; the Business Source License of the FerroEHR
application does not apply to them.

## Part of FerroEHR

This crate is the query-language layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
