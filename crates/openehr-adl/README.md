# openehr-adl

**A hand-written openEHR ADL 2.4 engine for Rust**: ADL2/cADL/ODIN parsing,
AOM 2 validation, specialisation flattening, operational-template (OPT 2)
generation, and ADL 1.4 → ADL 2 conversion — over the generated
`openehr-am::am24` model.

## What it provides

- **Parser**: ADL 2 archetype text (cADL definition sections, ODIN metadata /
  terminology sections) into typed `openehr-am::am24` values, with precise,
  typed syntax errors.
- **Validation**: the AOM 2 validity catalogue (VxxXX codes) over parsed
  archetypes.
- **Flattener**: specialisation flattening and template expansion.
- **OPT 2**: operational template generation from a flattened archetype set.
- **ADL 1.4 → 2 conversion** for legacy archetype libraries.

The grammar authority is the vendored openEHR ADL 2.4 specification text; the
parser is native Rust (`logos` + `chumsky`), no ANTLR runtime.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification (ADL/AOM 2.4.0). The implemented spec version is always
available at runtime as `openehr_adl::SPEC_VERSION` (`"2.4.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

MIT ([`LICENSE-MIT`](LICENSE-MIT)). The crate is hand-written; the openEHR
specification text it implements is the authority but is not embedded in the
package.

## Part of FerroEHR

This crate is the archetype-tooling layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
