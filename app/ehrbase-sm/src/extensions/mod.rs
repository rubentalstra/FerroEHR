//! Extension surface — **no openEHR spec governs anything in this module**.
//!
//! Everything here is our own design, quarantined away from the SM chapter
//! modules so the spec-governed surface stays clean: the protocol-adapter
//! support types ([`response`]), the ITS-REST-only adapter traits
//! ([`adapters`]), and the `EHR_ACCESS` scheme realization
//! ([`ehr_access`] — the scheme *slot* is RM-governed
//! (`EHR_ACCESS.settings`), the concrete scheme is ours because openEHR
//! publishes none).

pub mod adapters;
pub mod ehr_access;
pub mod response;

pub use adapters::{
    ContributionAdapter, DefinitionAdapter, EventSubscriptionAdapter, FhirConnectorAdapter,
    ItemTagAdapter, MultimediaAdapter, TenantAdapter, VersionMetaAdapter,
};
pub use ehr_access::{
    AccessEntry, AccessLevel, CompositionOverride, DefaultAccess, EHR_ACCESS_CONTROL_V1_SCHEME,
    EHR_ACCESS_CONTROL_V1_TYPE, EhrAccessAdapter, EhrAccessSettings, Privacy, principal_matches,
};
pub use response::{ResourceMeta, ServiceResponse};
