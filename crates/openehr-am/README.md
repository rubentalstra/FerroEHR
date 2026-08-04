# openehr-am

**openEHR AM — the Archetype Model for Rust**, generated deterministically
from the official openEHR machine-readable BMM meta-model. Both extant AM
generations ship side by side, exactly as the openEHR architecture mandates:
`am14` (ADL/AOM 1.4) and `am24` (ADL 2 / AOM 2.4).

## What it provides

- `am14` — the AOM 1.4 constraint model (`C_COMPLEX_OBJECT`, `C_PRIMITIVE_*`,
  the openEHR archetype profile: `C_DV_QUANTITY`, `C_CODE_PHRASE`, …) as used
  by ADL 1.4 archetypes and OPT 1.4 operational templates.
- `am24` — the AOM 2 model: archetypes, templates, the persistence (`P_`)
  classes, rules/assertion expression classes, and the AOM profile.
- Per-generation `SPEC_VERSION` constants (`am14::SPEC_VERSION = "1.4.0"`,
  `am24::SPEC_VERSION = "2.4.0"`).

For an ADL 2 *engine* (parser, validation, flattener, OPT2 generation, ADL
1.4→2 conversion) over this model, see the companion crate `openehr-adl`.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification (AM 2.4.0 (with AM 1.4 as `am14`)). The implemented spec version is always
available at runtime as `openehr_am::SPEC_VERSION` (`"2.4.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the generated types and their doc text derive from the vendored AM BMM schemas.

## Part of FerroEHR

This crate is the specification layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
