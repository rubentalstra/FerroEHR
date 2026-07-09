//! Moved to `ehrbase-sm` (SM-1, ADR-010): the native API lives in the
//! `ehrbase-sm` crate; this module re-exports it for existing paths.

pub use ehrbase_sm::backend::{Backend, StubBackend};
pub use ehrbase_sm::services::{
    AdminService, DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService,
    DemographicService, EhrCompositionService, EhrContributionService, EhrDirectoryService,
    EhrService, EhrStatusService, QueryService, SystemLog, ValidityChecker, WebTemplateService,
};
pub use ehrbase_sm::types::{
    AqlQueryRequest, EhrSummary, Page, PartyKind, QueryDescriptor, QueryOutcome,
};
