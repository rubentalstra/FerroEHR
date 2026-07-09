//! The openEHR **SM Platform Service Model** native API (ADR-010).
//!
//! One Rust trait per SM platform-service interface, the shared service
//! types, and the call-status error model — the seam between the platform
//! component (`ehrbase`) and every protocol adapter (`ehrbase-rest` for
//! ITS-REST 1.0.3, `EhrScape`, management HTTP, …), exactly the architecture
//! the SM assumes (`docs/specs/openehr/SM/docs/openehr_platform/
//! master02-overview.adoc` §General Assumptions: a nominal *native API*
//! reached through protocol adapters).
//!
//! Precedence (ADR-010): the SM governs decomposition, naming, and call
//! semantics (its pre/post-conditions are test oracles); **ITS-REST 1.0.3 +
//! the CNF/ECC schedule govern the wire** — divergences are recorded at the
//! declaration with `PORT NOTE`s. Design set: `docs/design/sm-platform/`.

pub mod backend;
pub mod error;
pub mod services;
pub mod types;

pub use backend::{Backend, StubBackend};
pub use error::{CallStatus, CallStatusType};
pub use services::{
    AdminService, DefinitionAdl14Service, DefinitionQueryService, DemographicService,
    EhrCompositionService, EhrContributionService, EhrDirectoryService, EhrService,
    EhrStatusService, QueryService, SystemLog, ValidityChecker, WebTemplateService,
};
pub use types::{
    AqlQueryRequest, EhrSummary, Page, PartyKind, QueryDescriptor, QueryOutcome, ResourceMeta,
    ServiceResponse, UpdateAttestation, UpdateAudit, UpdateVersion,
};
