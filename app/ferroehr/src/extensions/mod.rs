// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The spec-homeless surface — deliberate local extensions.
//!
//! **No openEHR specification governs anything in this module tree — every
//! submodule is our own design/extension.** This is the inverse of the
//! spec-governed modules: the features the `ferroehr` binary offers that no
//! openEHR spec requires or describes, consolidated in one place so the
//! spec-homeless surface stays visible and auditable.
//!
//! The no-spec-home finding is verified, not assumed — the only three openEHR
//! chapters that touch integration / deployment / security-of-access were
//! checked as the last possible spec home and none claims these features:
//!
//! * **master14** ("Integrating openEHR with other Systems") governs
//!   archetype-to-archetype data conversion (`GENERIC_ENTRY` + `FEEDER_AUDIT`),
//!   NOT FHIR resources, message brokers, topic routing, or outbound emission.
//! * **master13** is informative deployment guidance and prescribes no
//!   eventing, multi-tenancy, or blob-offload mechanism.
//! * **master07** governs the `EHR_ACCESS` object and authn-at-deployment, not
//!   a broker, a tenant registry, or object storage.
//!
//! (The one place a submodule touches spec-defined data is the FHIR connector's
//! `FEEDER_AUDIT` builder — RM common `feeder_audit` — which carries its own RM
//! citation in [`fhir`]'s `feeder_audit` submodule, and the multimedia offload's
//! `DV_MULTIMEDIA` rewrite, which cites RM data types in [`multimedia`].)
//!
//! ## Two non-negotiables every extension keeps
//! 1. **Flag header** — each submodule doc comment carries the explicit flag
//!    *"no openEHR spec governs this — our own design/extension"*.
//! 2. **Config gate, off by default** — the feature does nothing unless the
//!    configuration explicitly enables it; with its gate off the commit/read
//!    paths are byte-identical to the no-extension behaviour (the zero-drift
//!    invariant).
//!
//! ## Members
//! | Submodule | Gate (config path, default off) |
//! |---|---|
//! | [`events`] | `events.enabled` |
//! | [`fhir`] | `fhir.api_enabled` (routes) / `fhir.outbound.enabled` (emitter) |
//! | [`multimedia`] | `multimedia.enabled` |
//! | [`tenancy`] + [`tenant_context`] | tenancy-resolution middleware (configured) |

pub mod events;
pub mod fhir;
pub mod multimedia;
pub mod tenancy;
pub mod tenant_context;
