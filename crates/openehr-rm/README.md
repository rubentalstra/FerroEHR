# openehr-rm

**openEHR RM 1.2.0 — the Reference Model for Rust**, generated
deterministically from the official openEHR machine-readable BMM meta-model.
This is the domain model of an openEHR system: `COMPOSITION`, `EHR_STATUS`,
`OBSERVATION`, the data structures and data types, and the change-control
(versioning) classes.

## What it provides

- The complete RM 1.2.0 class set, package-mirrored (`ehr`, `composition`,
  `data_structures`, `data_types`, `common` incl. change control, `support`,
  `demographic`, `integration`), with closed subtype sets as Rust enums and
  recursion boxed.
- Emitted **manual** canonical-JSON `serde` implementations (`json_serde.rs`)
  — `_type`-first self-tagging in BMM declaration order, strict reader — no
  derives, no serde attributes on spec types.
- `validate` — machine-classified RM invariant cores generated from the BMM
  invariant expressions, plus terminology-backed invariant enforcement via
  `openehr-term`.
- `model` — the static RM attribute/type model (attribute → types,
  multiplicity, abstract → concrete descendant sets), the oracle an AQL
  planner or path validator needs, generated rather than reflected.
- Feature `ehr-extract` adds the EHR Extract package.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version follows a **pre-stabilisation `0.0.x` line** while the API
settles; once stable, the crate adopts the version of the openEHR
specification it implements (RM 1.2.0). The implemented spec version is always
available at runtime as `openehr_rm::SPEC_VERSION` (`"1.2.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated types and their doc text derive from the vendored RM BMM schema.

## Part of FerroEHR

This crate is the specification layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
