# openehr-term

**openEHR TERM 3.1.0 for Rust** — the terminology data model, generated
from the official openEHR machine-readable BMM meta-model, plus a hand-written
loader over the official openEHR terminology XML, which is embedded in the
package (English, Spanish, Japanese, Portuguese, Chinese).

## What it provides

- The generated TERM data classes (`TERMINOLOGY`, `CODE_SET`, concept/group
  structures).
- `bundle` — a zero-I/O, `include_str!`-embedded bundle of the official
  openEHR terminology XML releases (`assets/`, provenance-stamped), parsed
  once and served through typed lookups: terminology groups, code sets,
  rubrics per language.
- The support-terminology queries RM invariant enforcement needs (e.g. "is
  this code in group X"), as consumed by `openehr-rm::validate`.

## Generated code — do not edit

Every type file carries a `// @generated` header. The crate is emitted
deterministically by [`openehr-codegen`](https://github.com/rubentalstra/FerroEHR/tree/develop/tools/openehr-codegen)
from the vendored openEHR BMM meta-model; hand-written spec behaviour
(invariants, spec functions) lives in sibling `*_impl.rs` files the generator
never rewrites. Changes belong in the emitter, never in the generated output.

## Versioning

The package version is the crate's **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification (TERM 3.1.0). The implemented spec version is always
available at runtime as `openehr_term::SPEC_VERSION` (`"3.1.0"`), independent of
the package version.

## Minimum supported Rust version

Rust 1.96 (edition 2024).

## License

Licensed as **MIT AND Apache-2.0**:

- The Rust code and hand-written parts are MIT ([`LICENSE-MIT`](LICENSE-MIT)).
- The package embeds material derived from the official openEHR
  machine-readable specification artifacts, which openEHR publishes under
  Apache-2.0 ([`LICENSE-APACHE-2.0`](LICENSE-APACHE-2.0)) — the embedded terminology XML in `assets/` and the generated data classes derive from official openEHR artifacts.

## Part of FerroEHR

This crate is the terminology layer of [FerroEHR](https://github.com/rubentalstra/FerroEHR), a pure-Rust,
openEHR-spec-conformant Clinical Data Repository (ITS-REST 1.1.0 + AQL 1.1 on
PostgreSQL 18). The crates are usable standalone; FerroEHR is the reference
consumer.
