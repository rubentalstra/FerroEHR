//! **Native canonical-JSON codec** (ITS-JSON, both directions).
//!
//! A hand-written [`runtime`] (the [`runtime::JsonWriter`] + [`runtime::ToJson`]
//! trait on the write side, the [`runtime::JsonNode`] reader + borrowing
//! tokenizer + [`runtime::FromJson`] trait on the read side, plus the
//! primitive/leaf impls) under emitted per-type impls ([`generated`], produced by
//! `openehr-codegen -- emit-json`). Mirrors the canonical-XML codec's split
//! ([`crate::xml`]).
//!
//! This is the emitted-code replacement for `#[derive(OpenEhrType)]`: the
//! serialize side produces byte-identical output (proven by the parity gate in
//! `tests/json_codec_parity.rs`), and the deserialize side reproduces the derive's
//! tolerance rules verbatim (unknown keys ignored, out-of-order members,
//! present-but-wrong `_type` rejected, absent `_type` accepted on a concrete slot,
//! polymorphic `_type` dispatch) while dropping the serde-`Value` enum
//! double-pass. It owns the `_type` / number-typing / omission contract as
//! explicit generated code rather than inherited serde behaviour.

pub mod generated;
pub mod runtime;
