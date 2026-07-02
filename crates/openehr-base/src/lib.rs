// TODO(port): P17 warning burn-down — pedantic findings Phase-A transcription
// legitimately trips; remove and fix per-site at P17.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    clippy::module_inception,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::struct_excessive_bools
)]

//! openEHR BASE Release-1.2.0 — Base Types (definitions, builtins,
//! identification, resource).
//!
//! Spec crate written from the specifications; no Java counterpart in this
//! repository. Transcribed in P1 alongside `openehr-foundation`; module
//! wiring pulled forward from P17 so the crate compiles and the IDE can
//! index it.

pub mod builtins;
pub mod definitions;
pub mod identification;
pub mod resource;
