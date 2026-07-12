//! The **`Platform`** supertrait — the SM platform is the set of all its
//! services.
//!
//! `Platform` is the union of every SM catalog interface plus the ITS-REST
//! adapter-support extension traits (`DefinitionAdapter`, `VersionMetaAdapter`,
//! `ItemTagAdapter`). The concrete DB-backed service in the `ehrbase` crate
//! implements every one of these, and the protocol adapter (`ehrbase-rest`) is
//! generic over `S: Platform` — no `Arc<dyn>`, no stub backend, no default
//! method bodies: a missing implementation is a build error, never a silent
//! runtime `501`.
//!
//! The generated ITS-REST `DefinitionApi` is deliberately **not** a supertrait
//! here (the protocol-free service seam): templates + stored queries are dispatched onto the
//! SM `I_DEFINITION_*` traits plus the wire-shaped [`DefinitionAdapter`]
//! extension, so this crate carries no `openehr-its` types.

use crate::services::{
    AdminArchive, AdminService, ContributionAdapter, DefinitionAdapter, DefinitionAdl2Service,
    DefinitionAdl14Service, DefinitionQueryService, DemographicService, EhrCompositionService,
    EhrContributionService, EhrDirectoryService, EhrIndexService, EhrService, EhrStatusService,
    EventSubscriptionAdapter, FhirConnectorAdapter, ItemTagAdapter, MultimediaAdapter,
    PartyRelationshipService, QueryService, SystemLog, TenantAdapter, TerminologyService,
    VersionMetaAdapter, WebTemplateService,
};

/// The full server platform: everything the ITS-REST surface dispatches to.
/// Implemented once, on the application's concrete service.
pub trait Platform:
    EhrService
    + EhrStatusService
    + EhrCompositionService
    + EhrDirectoryService
    + EhrContributionService
    + ContributionAdapter
    + EventSubscriptionAdapter
    + FhirConnectorAdapter
    + TenantAdapter
    + VersionMetaAdapter
    + ItemTagAdapter
    + MultimediaAdapter
    + DemographicService
    + PartyRelationshipService
    + EhrIndexService
    + DefinitionAdapter
    + DefinitionAdl14Service
    + DefinitionAdl2Service
    + DefinitionQueryService
    + WebTemplateService
    + QueryService
    + AdminService
    + AdminArchive
    + TerminologyService
    + SystemLog
    + Send
    + Sync
    + std::fmt::Debug
    + 'static
{
}

impl<T> Platform for T where
    T: EhrService
        + EhrStatusService
        + EhrCompositionService
        + EhrDirectoryService
        + EhrContributionService
        + ContributionAdapter
        + EventSubscriptionAdapter
        + FhirConnectorAdapter
        + TenantAdapter
        + VersionMetaAdapter
        + ItemTagAdapter
        + MultimediaAdapter
        + DemographicService
        + PartyRelationshipService
        + EhrIndexService
        + DefinitionAdapter
        + DefinitionAdl14Service
        + DefinitionAdl2Service
        + DefinitionQueryService
        + WebTemplateService
        + QueryService
        + AdminService
        + AdminArchive
        + TerminologyService
        + SystemLog
        + Send
        + Sync
        + std::fmt::Debug
        + 'static
{
}
