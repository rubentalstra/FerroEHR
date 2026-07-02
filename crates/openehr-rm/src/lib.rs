// TODO(port): P17 warning burn-down — pedantic findings Phase-A transcription
// legitimately trips; remove and fix per-site at P17.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    // The closed spec enums (DataValue, ContentItem, Party, ...) mirror
    // subtype sets whose members legitimately differ in size; boxing
    // variants would reshape the transcription — revisit at P17/P19.
    clippy::large_enum_variant,
    clippy::module_inception,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::struct_excessive_bools
)]

//! openEHR RM Release-1.1.0 — the Reference Model, transcribed literally:
//! `data_types`, `data_structures`, `common`, `ehr`, `demographic`,
//! `integration` (`ehr_extract` behind the `ehr-extract` feature), `support`.
//!
//! Spec crate written from the specifications; no Java counterpart in this
//! repository. Transcribed in P3 (`docs/plans/phase-03-rm.md`); canonical
//! JSON serde landed in P4; module wiring pulled forward from P17 so the
//! crate compiles and the IDE can index it.

pub mod common;
pub mod data_structures;
pub mod data_types;
pub mod demographic;
pub mod ehr;
pub mod integration;
pub mod serde_support;
pub mod support;
