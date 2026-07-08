//! Moved to `ehrbase-sm` (SM-1, ADR-010): the native API lives in the
//! `ehrbase-sm` crate; this module re-exports it for existing paths.

pub use ehrbase_sm::backend::{Backend, StubBackend};
pub use ehrbase_sm::services::{
    AdminService, DemographicService, EhrCompositionService, EhrContributionService,
    EhrDirectoryService, EhrService, EhrStatusService, QueryService, SystemLog, ValidityChecker,
    WebTemplateService,
};
pub use ehrbase_sm::types::{AqlQueryRequest, PartyKind, QueryOutcome};
