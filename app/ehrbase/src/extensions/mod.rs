//! The spec-homeless surface — deliberate local extensions (W-3f register 12).
//!
//! **No openEHR specification governs anything in this module tree — every
//! submodule is our own design/extension.** This is the *inverse* of registers
//! 01–11: the modules the `ehrbase` binary offers that no openEHR spec requires
//! or describes, consolidated in one place so the spec-homeless surface is
//! visible and auditable (previously scattered across three top-level modules
//! plus three buried under `service/`).
//!
//! The no-spec-home finding is verified, not assumed: the only three openEHR
//! chapters that touch integration / deployment / security-of-access were
//! checked as the last possible spec home and none claims these features —
//! `docs/design/platform/12-extensions.md` (register 12) records the <file:line>
//! evidence and the master13/master14/master07 cross-check. In particular:
//!
//! * **master14** ("Integrating openEHR with other Systems") governs
//!   archetype-to-archetype data conversion (`GENERIC_ENTRY` + `FEEDER_AUDIT`),
//!   NOT FHIR resources, message brokers, topic routing, or outbound emission.
//! * **master13** is informative deployment guidance and prescribes no
//!   eventing, multi-tenancy, or blob-offload mechanism.
//! * **master07** governs the `EHR_ACCESS` object and authn-at-deployment, not a
//!   broker, a tenant registry, or object storage.
//!
//! (The one place a submodule touches spec-defined data is the FHIR connector's
//! `FEEDER_AUDIT` builder — RM common `feeder_audit` — which carries its own RM
//! citation in [`fhir`]'s `feeder_audit` submodule.)
//!
//! ## Two non-negotiables every extension keeps
//! 1. **Flag header** — each submodule doc comment carries the explicit flag
//!    *"no openEHR spec governs this — our own design/extension"*.
//! 2. **Config gate, off by default** — the feature does nothing unless a
//!    `figment` config explicitly enables it; with its gate off the commit/read
//!    paths are byte-identical to the no-extension behaviour (the zero-drift
//!    invariant).
//!
//! ## Members
//! | Submodule | G-row | Gate (env, default off) |
//! |---|---|---|
//! | [`events`] | G-12-01/02 | `EHRBASE_EVENTS_ENABLED` |
//! | [`fhir`] | G-12-03/04 | `ehrbase-rest` FHIR routes / `EHRBASE_FHIR_OUTBOUND_ENABLED` |
//! | [`multimedia`] | G-12-05 | `EHRBASE_MULTIMEDIA_ENABLED` |
//! | [`tenancy`] | G-12-06 | tenancy-resolution middleware (configured) |
//!
//! **Not here (reassigned, register §5):** `ehr_access_cache` (`EHR_ACCESS` is
//! RM — register 01, `service/ehr/access.rs`) and `codes.rs` (change-control
//! terminology codes — RM common master06 + TERM, `versioning/`). Both are
//! spec-governed and stay out of this bucket.
//
// Wiring is orchestrator-owned (`lib.rs`, `main.rs`; see the return note):
//   * `lib.rs`: replace the three scattered `pub mod events; pub mod
//     fhir_outbound; pub mod multimedia;` with a single `pub mod extensions;`.
//   * `main.rs`: `ehrbase::events::*` → `ehrbase::extensions::events::*`,
//     `ehrbase::multimedia::*` → `ehrbase::extensions::multimedia::*`,
//     `ehrbase::fhir_outbound::*` → `ehrbase::extensions::fhir::*`.
//   * `service/mod.rs`: the `multimedia` field type
//     `crate::multimedia::MultimediaEngine` → `crate::extensions::multimedia::MultimediaEngine`.

pub mod events;
pub mod fhir;
pub mod multimedia;
pub mod tenancy;
