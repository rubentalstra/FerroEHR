# openehr-base

**openEHR BASE 1.3.0 — foundation and base types for Rust**, generated
deterministically from the official openEHR machine-readable BMM meta-model.

## What it provides

- The complete BASE component type set: foundation types (intervals,
  ISO-8601 date/time types, `Uri`, terminology-neutral primitives) and base
  types (identification — `OBJECT_VERSION_ID`, `ARCHETYPE_ID`, `HIER_OBJECT_ID`
  and peers — plus `DV_`-supporting base classes and definitions).
- `serde_support` — the small hand-written runtime shared by the emitted
  canonical-JSON `serde` implementations of every openEHR spec crate
  (`_type`-first self-tagging, strict reading: undeclared and duplicate keys
  are refused).
- Validated constructors: types with BMM invariants construct through
  `new() -> Result`, so invalid spec values are unrepresentable.

Every other `openehr-*` crate builds on this one.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version follows a **pre-stabilisation `0.0.x` line** while the API
settles; once stable, the crate adopts the version of the openEHR
specification it implements (BASE 1.3.0). The implemented spec version is always
available at runtime as `openehr_base::SPEC_VERSION` (`"1.3.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated types and their doc text derive from the vendored BASE BMM schema.

## Part of FerroEHR

This crate is the specification foundation layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
