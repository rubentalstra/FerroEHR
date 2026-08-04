# openehr-its

**openEHR ITS (Implementation Technology Specifications) for Rust**: the
canonical JSON and canonical XML serialization of the openEHR RM, the
generated ITS-REST 1.1.0 API contract, OPT 1.4 template reading, and the
Simplified Data Formats (FLAT / STRUCTURED / Web Template).

## What it provides

- **Canonical JSON** — `json::{to_canonical_json, from_canonical_json,
  from_canonical_value}`: `_type`-first, BMM field order, a STRICT reader
  (undeclared/duplicate keys refused), refusal paths naming the offending
  JSON node, and validation against the embedded official ITS-JSON RM schema.
- **Canonical XML** — generated `ToXml`/`FromXml` implementations over a
  `quick-xml` runtime, serving both published XSD lineages (the root
  namespace is a serialize-time choice).
- **ITS-REST contract** — generated DTOs, `#[async_trait]` server traits, and
  route tables for every ITS-REST 1.1.0 API group, ready to implement over
  `axum`.
- **Simplified Data Formats** — the hand-written `flat` module: Web Template
  building, FLAT and STRUCTURED composition reading/writing.
- **OPT 1.4** (`opt14`) and both AOM2 archetype XML codecs.
- `default-features = false` leaves a dependency-free island:
  `rest::smart_scopes`, the SMART on openEHR scope grammar — parseable from
  `wasm32-unknown-unknown` clients.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification (ITS-REST 1.1.0 (with ITS-XML and ITS-JSON alongside)). The implemented spec version is always
available at runtime as `openehr_its::SPEC_VERSION` (`"1.1.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated codecs/contract derive from the vendored ITS XSD/OpenAPI/BMM artifacts, and the package embeds the official ITS-JSON RM schema (`schemas/json/`).

## Part of FerroEHR

This crate is the serialization and REST-contract layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
