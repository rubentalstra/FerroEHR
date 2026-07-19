//! **Native canonical-JSON codec** (ITS-JSON, Serialize side).
//!
//! A hand-written [`runtime`] (the [`runtime::JsonWriter`] + [`runtime::ToJson`]
//! trait + primitive/leaf impls) under emitted per-type impls
//! ([`generated`], produced by `openehr-codegen -- emit-json`). Mirrors the
//! canonical-XML codec's split ([`crate::xml`]).
//!
//! This is the emitted-code replacement for the `#[derive(OpenEhrType)]` serde
//! `Serialize`: it produces byte-identical output (proven by the parity gate in
//! `tests/json_codec_parity.rs`), owning the `_type`-first / BMM-order / number-
//! typing / omission contract as explicit generated code rather than inherited
//! serde behaviour. The `json::to_canonical_json` entry point is not yet switched
//! onto it — that (and retiring the serde derive) is a later phase.

pub mod generated;
pub mod runtime;
