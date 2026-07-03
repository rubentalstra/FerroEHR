//! openEHR **LANG 1.0.0** component: ODIN, BMM, and the Expression Language (EL).
//!
//! - [`odin`]: ODIN parser (canonical grammar: specifications-BASE `odin.g4`).
//! - [`bmm`]: BMM object model + `P_BMM` (persisted BMM, schema v2.3) reader.
//!   BMM schema files are themselves ODIN documents, so `bmm` builds on `odin`.
//! - `el`: Expression Language — added when a consuming phase needs it.
//!
//! `bmm` is also consumed at build time by the `openehr-codegen` tool, which
//! generates the spec crates from the vendored openEHR BMM meta-model.
//!
//! Populated in P8 (`docs/plans/phase-08-odin-bmm.md`) and pulled forward by
//! the spec-driven codegen work (ADR-004).

pub mod bmm;
pub mod odin;
