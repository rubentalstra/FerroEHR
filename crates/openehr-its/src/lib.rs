//! openEHR **ITS** — Implementation Technology Specifications. This crate
#![allow(clippy::doc_markdown)] // module docs are prose with many proper nouns
//!
//! mirrors the four `specifications-ITS-*` sub-repos (aggregated by
//! `specifications-ITS`): how openEHR RM instances are serialized and exposed.
//!
//! - [`json`] — **ITS-JSON**: canonical JSON. The `_type` self-tagging is on
//!   the RM types via `#[derive(OpenEhrType)]` (`openehr-derive`); this module
//!   is the named entry points + the vendored ITS-JSON schema + the interop
//!   fidelity gate (round-trip the EHRbase corpus, `tests/`).
//! - [`json_codec`] — **ITS-JSON**, native codec: emitted `ToJson` impls over a
//!   hand-written writer runtime (the Serialize-side replacement for the serde
//!   derive; byte-identical, proven by `tests/json_codec_parity.rs`).
//! - [`xml`] — **ITS-XML**: canonical XML via `quick-xml`, validated against the
//!   vendored XSDs (`schemas/xml/`). (Implementation is P5.)
//! - [`rest`] — **ITS-REST**: the openEHR REST API contract. The machine-readable
//!   OpenAPI specs are vendored (`vendor/rest-oas/`); the server that implements
//!   them is `ehrbase-rest` (P6).
//! - [`bmm`] — **ITS-BMM**: BMM serialization. The vendored BMM meta-model that
//!   drives code generation lives in `openehr-codegen/vendor/bmm`; the generated
//!   runtime BMM object model is `openehr-lang`.
//!
//! Serialization here is format machinery only — it stays generic over
//! `serde::Serialize`/`Deserialize` so it does not depend on `openehr-rm`
//! (the fidelity-gate tests do, as dev-dependencies).

pub mod bmm;
pub mod json;
pub mod json_codec;
pub mod opt14;
pub mod rest;
pub mod xml;
