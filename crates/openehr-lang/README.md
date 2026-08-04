# openehr-lang

**openEHR LANG for Rust** — the BMM (Basic Meta-Model) object model,
generated from the official machine-readable schemas, plus a hand-written ODIN
instance parser. Both published BMM generations are emitted side by side: the
stable v2.x model (`bmm`, `bmm_persistence`, `beom`) and the v3 development
line (`bmm3`).

## What it provides

- `bmm` / `bmm_persistence` / `beom` — the stable v2.x BMM object model,
  P_BMM persistence classes, and the basic expression object model.
- `bmm3` — the v3 BMM development line (entities, features, literal values,
  expressions, statements), emitted completely at its own package paths.
- `odin` — a hand-written ODIN instance parser (`logos` + `chumsky`, from the
  official `odin.g4`/`odin_values.g4` grammars), producing insertion-ordered
  typed values.

This crate is what a BMM-consuming tool (such as a code generator for openEHR
models) reads schemas with.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version follows a **pre-stabilisation `0.0.x` line** while the API
settles; once stable, the crate adopts the version of the openEHR
specification it implements (LANG 1.0.0). The implemented spec version is always
available at runtime as `openehr_lang::SPEC_VERSION` (`"1.0.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated types and their doc text derive from the vendored LANG BMM schemas.

## Part of FerroEHR

This crate is the meta-model layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
