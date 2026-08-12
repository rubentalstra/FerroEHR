// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The **Terminology** component of the platform crate.
//!
//! The concrete realization of the SM `I_TERMINOLOGY_SERVICE` interface on
//! [`FerroEhrService`](crate::service::FerroEhrService), plus the AQL
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
//! - `bundle` — the in-process `openehr-term` bundle provider (TERM 3.1.0):
//!   the enumerable local default.
//! - [`fhir`] — [`FhirTerminologyProvider`](fhir::FhirTerminologyProvider), a
//!   remote FHIR R4B TS client (opt-in via
//!   [`ExternalTerminologyConfig`](config::ExternalTerminologyConfig)).
//! - [`oauth2`] — [`TokenSource`](oauth2::TokenSource), the `OAuth2`
//!   client-credentials authentication a provider may carry.
//! - [`tls`] — the per-provider outbound TLS material: the client certificate
//!   presented for mutual TLS and the trust anchors the server is verified
//!   with (no openEHR spec governs transport security — our own design).
//! - [`router`] — [`TerminologyRouter`](router::TerminologyRouter): every
//!   configured server, materialised, with the terminology → provider routing.
//! - `routing` — the 9 SM calls on `FerroEhrService`, routing between the
//!   bundle and the routed remote provider (the routing rule below).
//! - `binding` — commit-time resolution of archetype **constraint bindings**
//!   (ac-code value sets) against the routed terminology service.
//! - `expander` — the AQL `TERMINOLOGY()` seam
//!   ([`crate::aql::terminology::TerminologyExpander`] on `FerroEhrService`).
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
//!   the bundle when it knows the terminology, else routed **by terminology**
//!   to one of the configured FHIR providers
//!   ([`router::TerminologyRouter`]), else falls through to the bundle's
//!   `Pre_has_terminology` → `NotFound`. With no FHIR provider configured (the
//!   default) this is byte-identical to a bundle-only service.
//! - Several servers answer **simultaneously**: SNOMED CT may live on one
//!   server and LOINC on another in the same running instance, which is the
//!   deployment reality BASE
//!   `docs/architecture_overview/master12-terminology.adoc` §Overview
//!   describes ("LOINC, `ICDx`, ICPC, SNOMED CT and the many other terminologies
//!   and vocabularies used in healthcare").

mod binding;
mod bundle;
pub mod config;
mod expander;
pub mod fhir;
pub mod oauth2;
pub mod router;
mod routing;
pub mod tls;

pub mod types;
