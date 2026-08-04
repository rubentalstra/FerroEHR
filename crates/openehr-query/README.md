# openehr-query

**A hand-written openEHR AQL 1.1 front end for Rust**: lexer, parser, and
typed AST for the Archetype Query Language — native `logos` + `chumsky`, no
ANTLR runtime.

## What it provides

- A complete AQL 1.1 lexer and parser producing a typed AST (`SELECT`,
  `FROM`/`CONTAINS`, `WHERE`, `ORDER BY`, `LIMIT`, path expressions with
  predicates, parameters, functions), corpus-validated against the official
  grammar's example set.
- Typed parse errors with source positions — a refusal names what and where.
- The AST is engine-agnostic: this crate deliberately stops at parsing, so
  any execution engine (SQL generation, in-memory evaluation) can build on
  it.

## Versioning

The package version follows a **pre-stabilisation `0.0.x` line** while the API
settles; once stable, the crate adopts the version of the openEHR
specification it implements (QUERY (AQL) 1.1.0). The implemented spec version is always
available at runtime as `openehr_query::SPEC_VERSION` (`"1.1.0"`), independent of
the package version.

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
