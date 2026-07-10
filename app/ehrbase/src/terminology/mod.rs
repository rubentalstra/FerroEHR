//! External terminology-server integration (B4): the FHIR R4 terminology
//! **client** the composition validator uses for external value-set bindings.
//!
//! - [`config`] — the `figment` configuration
//!   (`docs/terminology-validation.md` §4); external terminology is **off by
//!   default** (openEHR-bundle-only), FHIR is opt-in.
//! - [`fhir`] — [`FhirTerminologyProvider`], a remote FHIR R4 TS client that
//!   implements the SM `I_TERMINOLOGY_SERVICE` trait
//!   ([`ehrbase_sm::TerminologyService`]).
//!
//! The in-process `openehr-term` bundle ([`crate::service::terminology`])
//! stays the local default provider; this module adds the *remote* provider
//! selected when a deployment configures a FHIR terminology server.

pub mod config;
pub mod fhir;

pub use config::{ExternalTerminologyConfig, FhirOperation, FhirProviderConfig, ProviderKind};
pub use fhir::FhirTerminologyProvider;
