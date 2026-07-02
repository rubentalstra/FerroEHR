// TODO(port): P17 warning burn-down — these allows cover pedantic findings
// that Phase-A spec-faithful transcription legitimately trips (spec-typed
// Integer counts force usize->i32 casts; spec field sets force bool clusters;
// interval/interval.rs mirrors the spec package/class name). Remove the block
// at P17 and fix or justify each site individually.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    clippy::float_cmp,
    clippy::module_inception,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::struct_excessive_bools
)]

//! openEHR BASE Release-1.2.0 — Foundation Types.
//!
//! Spec crate written from the openEHR specifications; it has no Java
//! counterpart in this repository (`EHRbase` consumed these types through
//! the external `archie`/SDK libraries). Transcribed in P1
//! (`docs/plans/phase-01-foundation-identification.md`); module wiring
//! pulled forward from P17 so the crate compiles and the IDE can index it.

pub mod functional;
pub mod interval;
pub mod primitive_types;
pub mod serde_support;
pub mod structure_types;
pub mod terminology_types;
pub mod time;
