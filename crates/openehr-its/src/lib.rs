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
//!
//! Serialization here is native codec machinery: the canonical-JSON `ToJson`/
//! `FromJson` codec and the canonical-XML codec are generated over hand-written
//! runtimes, so the spec crates carry no serde derive.

pub mod bmm;
pub mod json;
pub mod json_codec;
pub mod opt14;
pub mod rest;
pub mod rm_validate;
pub mod xml;
