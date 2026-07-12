//! The openEHR **SM Platform Service Model** native API.
//!
//! One Rust trait per SM platform-service interface, the shared service
//! types, and the call-status error model — the seam between the platform
//! component (`ehrbase`) and every protocol adapter (`ehrbase-rest` for
//! ITS-REST 1.0.3, `EhrScape`, management HTTP, …), exactly the architecture
//! the SM assumes (`docs/specs/openehr/SM/docs/openehr_platform/
//! master02-overview.adoc` §General Assumptions: a nominal *native API*
//! reached through protocol adapters).
//!
//! Precedence: the SM governs decomposition, naming, and call
//! semantics (its pre/post-conditions are test oracles); **ITS-REST 1.0.3 +
//! the CNF/ECC schedule govern the wire** — divergences are recorded at the
//! declaration with `PORT NOTE`s. Design set: `docs/design/sm-platform/`.

pub mod ehr_handle;
pub mod error;
pub mod platform;
pub mod services;
pub mod tenant;
pub mod types;

pub use ehr_handle::{
    EhrCompositionHandle, EhrContributionHandle, EhrDirectoryHandle, EhrStatusHandle, IEhr,
};
pub use error::{CallStatus, CallStatusType, SmError};
pub use platform::Platform;
pub use services::{
    AdminArchive, AdminDumpLoad, AdminService, AuditEvent, CompressionFormat, DataBinding,
    DataFrame, DataFrameSample, DataSetResult, DefinedTerm, DefinitionAdapter,
    DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService, DemographicService,
    DumpLoadFailReport, EhrCompositionService, EhrContributionService, EhrDirectoryService,
    EhrExtractService, EhrService, EhrStatusService, EmitOutcome, EnvBinding, EventActionCode,
    EventOutcome, EventSubscriptionAdapter, ExportFormat, ExportSpec, FhirConnectorAdapter,
    FrameMethod, FramePayload, ItemTagAdapter, ObjectClass, QueryService, Sample, StatTimeRange,
    SubjectDataSet, SubjectProxyService, SubjectVariable, SystemLog, TddService, TenantAdapter,
    TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation, TerminologyRelationError, TerminologyService, TimeRange, ValidityChecker,
    VariableSample, VariableValue, VersionMetaAdapter, WebTemplateService,
};
pub use tenant::TenantContext;
pub use types::{
    AqlQueryRequest, EhrSummary, Page, PartyKind, PlatformService, QueryDescriptor, QueryOutcome,
    ResourceMeta, ServiceResponse, UpdateAttestation, UpdateAudit, UpdateVersion,
};
