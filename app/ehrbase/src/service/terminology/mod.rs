//! The **Terminology** component of the platform crate: the concrete
//! realization of the SM `I_TERMINOLOGY_SERVICE` interface on
//! [`EhrbaseService`](crate::service::EhrbaseService), plus the AQL
//! terminology seam.
//!
//! Spec: `docs/specs/openehr/SM/docs/openehr_platform/
//! master12-terminology_service.adoc` + `UML/classes/i_terminology_service.adoc`
//! (the 9 calls + preconditions) and the extract model
//! (`terminology_extract.adoc` &c.). Context:
//! `BASE/docs/architecture_overview/master12-terminology.adoc` models the
//! concrete backend as an external "terminology query server"; the SM defines
//! a **single** interface. The interface/provider split is therefore
//! *logical*, realized by one service routing among providers.
//!
//! Module tree, one file per concern:
//!
//! - [`types`] — the SM extract data model (`Terminology_description`,
//!   `Terminology_extract`, `Term_code`/`Defined_term`, relationships).
//! - [`bundle`] — the in-process `openehr-term` bundle provider (TERM 3.1.0):
//!   the enumerable local default.
//! - [`fhir`] — [`FhirTerminologyProvider`], a remote FHIR R4 TS client
//!   (opt-in via [`ExternalTerminologyConfig`]).
//! - [`routing`] — the 9 SM calls on `EhrbaseService`, routing between the
//!   two providers (the G-4 rule below).
//! - [`expander`] — the AQL `TERMINOLOGY()` seam
//!   ([`crate::aql::terminology::TerminologyExpander`] on `EhrbaseService`).
//! - [`config`] — the `[terminology]` config section (no openEHR spec governs
//!   configuration — our own design).
//!
//! # Provider routing
//!
//! - **Enumeration** (`get_terminology_ids`, `has_terminology`,
//!   `get_terminology_description`) is answered **only by the bundle** — a
//!   FHIR TS is a validation/expansion backend, not an enumerable openEHR
//!   terminology (`fhir.rs` NOTE). A FHIR-only deployment still answers
//!   these.
//! - **Lookup / validation** (`has_term`, `get_term`, `subsumes`,
//!   `value_set_validate`, `has_value_set`, `get_value_set`) is answered by
//!   the bundle when it knows the terminology, else routed to the configured
//!   FHIR provider, else falls through to the bundle's `Pre_has_terminology`
//!   → `NotFound`. With no FHIR provider configured (the default) this is
//!   byte-identical to a bundle-only service.

mod bundle;
pub mod config;
mod expander;
pub mod fhir;
mod routing;

pub mod types;
