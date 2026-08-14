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
  The `_type` → decode dispatch and declared-key table
  (`json_codec::generated::structural`) span **every** emitted generation of
  the spec crates at once; the XML, REST and OPT surfaces below are generated
  over their current generations (RM 1.2.0 / BASE 1.3.0 / AM 2.4.0).
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

This crate is **half generated, half hand-written**, and every generated file
carries a `// @generated` header. The generated halves are emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR artifacts: the XML codec from the ITS-XML XSDs plus
the BMM field model, the REST contract from the ITS-REST OpenAPI documents,
the `_type` dispatch table from the BMM, and the three archetype XML codecs
(`opt14`, `aom2`, `aom2_model`) from their XSD closures. Changes to those
belong in the emitter, never in the generated output. The hand-written halves
— the `quick-xml` and REST runtimes, the canonical-JSON entry points, wire
validation, `flat`, and `rest::smart_scopes` — are edited normally.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification. The implemented
spec version is always available at runtime as the crate-level constant
`openehr_its::SPEC_VERSION` (`"1.1.0"`, the ITS-REST release this crate
implements — Simplified Formats is one of its sub-specifications, and the
ITS-XML and ITS-JSON artifacts are pinned alongside), independent of the
package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## Attribution — the embedded openEHR artifact

Exactly one third-party file travels inside this package:

- **Packaged path:** `schemas/json/openehr_rm_1.1.0_all.json`
- **Upstream:** [`openEHR/specifications-ITS-JSON`](https://github.com/openEHR/specifications-ITS-JSON),
  path `components/openehr_rm_1.1.0_all.json`, commit
  `5acae056248e917a4b4c56f7e712f4fcfeb616a6` (`master` — ITS-JSON is
  DEVELOPMENT status and has no numbered release)
- **Copyright:** openEHR Foundation; redistributed **verbatim** under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0),
  <https://github.com/openEHR/specifications-ITS-JSON/blob/master/LICENSE>)
- **Role here:** the consolidated ITS-JSON RM 1.1.0 JSON Schema, embedded as
  `openehr_its::json::RM_SCHEMA_JSON` and used as the validation oracle for
  canonical-JSON output — it is not a code source.

If you redistribute this crate, that file and this attribution travel with it.
The rest of the vendored ITS-JSON tree, and the ITS-XML XSDs and ITS-REST
OpenAPI documents the generated code was emitted from, stay in the repository
and are not packaged.

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated
  codecs/contract derive from the vendored ITS XSD/OpenAPI/BMM artifacts, and
  the package embeds the official ITS-JSON RM schema attributed above.

## Part of FerroEHR

This crate is the serialization and REST-contract layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
