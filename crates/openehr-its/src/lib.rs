// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: BUSL-1.1 AND Apache-2.0

//! openEHR **ITS** — Implementation Technology Specifications. This crate
#![allow(
    clippy::doc_markdown,
    reason = "module docs are prose with many proper nouns"
)]
//!
//! mirrors the four `specifications-ITS-*` sub-repos (aggregated by
//! `specifications-ITS`): how openEHR RM instances are serialized and exposed.
//!
//! - [`json`] — **ITS-JSON**: canonical JSON. The named entry points
//!   (`to_canonical_json`/`from_canonical_json`/`from_canonical_value`, over the
//!   native [`json_codec`]) + the vendored ITS-JSON schema + the interop
//!   fidelity gate (round-trip a vendored real-world canonical-JSON corpus,
//!   `tests/`).
//! - [`json_codec`] — **ITS-JSON**, native codec: emitted `ToJson`/`FromJson`
//!   impls over a hand-written writer/reader runtime — the canonical-JSON
//!   (de)serialization for every spec type (there is no serde derive on the spec
//!   types; the codec owns the `_type` / number-typing / omission contract).
//! - [`wire_validate`] — the wire-boundary RM class-invariant dispatch layer:
//!   reads a canonical-JSON node, deserializes via the codec, and runs the RM
//!   invariant cores of `openehr_rm::v1_2::validate` (the `Validate` impls and every
//!   value-level decision stay in `openehr-rm`/`openehr-base`).
//! - [`rm_instance`] — the template-independent validation of a whole RM
//!   instance tree: the RM-invariant and terminology passes, the
//!   [`rm_instance::ValidationMessage`] report shape, and the composed
//!   [`rm_instance::validate_composition`] entry point.
//! - [`xml`] — **ITS-XML**: canonical XML via `quick-xml`, validated against the
//!   vendored XSDs (`schemas/xml/`). (Implementation is P5.)
//! - [`rest`] — **ITS-REST**: the openEHR REST API contract. The machine-readable
//!   OpenAPI specs are vendored (`vendor/rest-oas/`); the server that implements
//!   them is `ferroehr-rest` (P6).
//! - **ITS-BMM** has no module here: the vendored BMM meta-model that drives
//!   code generation lives in `openehr-codegen/vendor/bmm`, and the runtime
//!   BMM object model is `openehr-lang`.
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
//! # Features
//!
//! The default `full` feature is every surface above. The graph underneath it
//! is layered so a consumer takes only what it reads: `json` is the canonical
//! JSON base, `xml` adds the XML codecs, `opt14` the operational-template
//! reader, and `flat` the Simplified Formats. A `wasm32-unknown-unknown`
//! consumer takes `default-features = false, features = ["flat"]`, which pulls
//! that whole chain; `cache` (`moka`), `schema-validation` (`jsonschema` plus
//! the compiled-in ITS-JSON RM schema) and `rest-server` (the generated server
//! contract, `axum`) stay outside it.
//!
//! Taken with `default-features = false` and nothing else the crate compiles to
//! [`rest::smart_scopes`] alone — the std-only SMART scope grammar, with no
//! dependency of any kind — so a REST client that must parse scope strings in
//! the browser (the viewer's scope previewer) shares the very grammar the CDR
//! enforces instead of carrying a second parser.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
#[cfg(feature = "xml")]
pub mod aom2;
#[cfg(feature = "xml")]
pub mod aom2_model;
#[cfg(feature = "flat")]
pub mod flat;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "json")]
pub mod json_codec;
#[cfg(feature = "opt14")]
pub mod opt14;
pub mod rest;
// `rm_instance` composes the template pass of `flat::validation` over the RM
// pass, and `flat::validation` reads its report shape back from here, so the
// two are one layer rather than two.
#[cfg(feature = "flat")]
pub mod rm_instance;
#[cfg(feature = "json")]
pub mod wire_validate;
#[cfg(feature = "xml")]
pub mod xml;

/// The openEHR specification version this crate implements.
///
/// The pin is deliberately independent of the crates.io package version,
/// which is the crate's own `SemVer` line and moves only with this
/// implementation's code.
pub const SPEC_VERSION: &str = "1.1.0";
