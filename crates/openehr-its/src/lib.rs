//! openEHR **ITS** — Implementation Technology Specifications. This crate
#![allow(clippy::doc_markdown)] // module docs are prose with many proper nouns
//!
//! mirrors the four `specifications-ITS-*` sub-repos (aggregated by
//! `specifications-ITS`): how openEHR RM instances are serialized and exposed.
//!
//! - [`json`] — **ITS-JSON**: canonical JSON. The named entry points
//!   (`to_canonical_json`/`from_canonical_json`/`from_canonical_value`, over the
//!   native [`json_codec`]) + the vendored ITS-JSON schema + the interop
//!   fidelity gate (round-trip the EHRbase corpus, `tests/`).
//! - [`json_codec`] — **ITS-JSON**, native codec: emitted `ToJson`/`FromJson`
//!   impls over a hand-written writer/reader runtime — the canonical-JSON
//!   (de)serialization for every spec type (there is no serde derive on the spec
//!   types; the codec owns the `_type` / number-typing / omission contract).
//! - [`rm_validate`] — the wire-boundary RM class-invariant dispatcher: reads a
//!   canonical-JSON node, deserializes via the codec, and runs the RM invariants
//!   (the `Validate` impls stay in `openehr-rm`/`openehr-base`).
//! - [`xml`] — **ITS-XML**: canonical XML via `quick-xml`, validated against the
//!   vendored XSDs (`schemas/xml/`). (Implementation is P5.)
//! - [`rest`] — **ITS-REST**: the openEHR REST API contract. The machine-readable
//!   OpenAPI specs are vendored (`vendor/rest-oas/`); the server that implements
//!   them is `ehrbase-rest` (P6).
//! - [`bmm`] — **ITS-BMM**: BMM serialization. The vendored BMM meta-model that
//!   drives code generation lives in `openehr-codegen/vendor/bmm`; the generated
//!   runtime BMM object model is `openehr-lang`.
//! - [`flat`] — **ITS-REST Formats**: openEHR Simplified Formats
//!   (FLAT / STRUCTURED data instances + the Web Template model). The
//!   *Formats* specification is a STABLE ITS-REST 1.1.0 sub-specification
//!   (`docs/specs/openehr/ITS-REST/docs/simplified_formats/`), so it lives
//!   here alongside the other ITS surfaces; hand-written (BMM has no
//!   simplified-format model), unlike the emitted JSON/XML/REST codecs.
//!
//! Serialization here is native codec machinery: the canonical-JSON `ToJson`/
//! `FromJson` codec and the canonical-XML codec are generated over hand-written
//! runtimes, so the spec crates carry no serde derive.
//!
//! # Feature `full` (default)
//!
//! Every surface above rides the default `full` feature. Taken with
//! `default-features = false` the crate compiles to `rest::smart_scopes` alone —
//! the std-only SMART scope grammar, with no dependency of any kind — so a REST
//! client that must parse scope strings on `wasm32-unknown-unknown` (the admin
//! console's scope previewer) shares the very grammar the CDR enforces instead
//! of carrying a second parser.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
#[cfg(feature = "full")]
pub mod bmm;
#[cfg(feature = "full")]
pub mod flat;
#[cfg(feature = "full")]
pub mod json;
#[cfg(feature = "full")]
pub mod json_codec;
#[cfg(feature = "full")]
pub mod opt14;
pub mod rest;
#[cfg(feature = "full")]
pub mod rm_terminology;
#[cfg(feature = "full")]
pub mod rm_validate;
#[cfg(feature = "full")]
pub mod xml;

/// The openEHR specification version this crate implements — the crate
/// version itself: the spec crates are versioned by the specification they
/// implement (`docs/VERSIONS.md` §Product and crate versioning), so
/// consumers read the pin from the package, never from a hand-typed literal.
pub const SPEC_VERSION: &str = env!("CARGO_PKG_VERSION");
