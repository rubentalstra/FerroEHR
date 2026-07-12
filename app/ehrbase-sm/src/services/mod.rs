//! The openEHR SM Platform Service Model interfaces, one Rust trait per
//! interface, transcribed literally: each trait carries its
//! interface's exact SM call names, parameter names and types, spec returns,
//! and pre/post-conditions (in the per-method doc-comments). Native errors are
//! [`SmError`](crate::error::SmError) over `CALL_STATUS_TYPE` — **no** ITS-REST
//! types in the EHR-core catalog.
//!
//! The EHR mega-service is split along the SM's own interface boundaries
//! (`I_EHR_SERVICE` / `I_EHR_STATUS` / `I_EHR_COMPOSITION` / `I_EHR_DIRECTORY`
//! / `I_EHR_CONTRIBUTION`), with the per-EHR `I_EHR` accessor realized as the
//! generic [`IEhr`](crate::IEhr) handle. ITS-REST-only operations the SM does
//! not define (item-tag CRUD, `*_latest_meta` decoration) live in the
//! [`adapter`] extension traits, clearly separated with `PORT NOTE`s.

pub mod adapter;
pub mod admin;
pub mod composition;
pub mod contribution;
pub mod definition;
pub mod demographic;
pub mod directory;
pub mod ehr;
pub mod ehr_access;
pub mod ehr_index;
pub mod ehr_status;
pub mod message;
pub mod query;
pub mod relationship;
pub mod subject_proxy;
pub mod system_log;
pub mod tdd;
pub mod terminology;
pub mod validity;
pub mod web_template;

pub use adapter::{
    ContributionAdapter, DefinitionAdapter, EventSubscriptionAdapter, FhirConnectorAdapter,
    ItemTagAdapter, MultimediaAdapter, TenantAdapter, VersionMetaAdapter,
};
pub use admin::{
    AdminArchive, AdminDumpLoad, AdminService, CompressionFormat, DumpLoadFailReport, ExportFormat,
    ExportSpec, StatTimeRange,
};
pub use composition::EhrCompositionService;
pub use contribution::{EhrContributionService, TimeRange};
pub use definition::{DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService};
pub use demographic::DemographicService;
pub use directory::EhrDirectoryService;
pub use ehr::EhrService;
pub use ehr_access::{
    AccessEntry, AccessLevel, CompositionOverride, DefaultAccess, EHR_ACCESS_CONTROL_V1_SCHEME,
    EHR_ACCESS_CONTROL_V1_TYPE, EhrAccessAdapter, EhrAccessSettings, Privacy, principal_matches,
};
pub use ehr_index::EhrIndexService;
pub use ehr_status::EhrStatusService;
pub use message::EhrExtractService;
pub use query::QueryService;
pub use relationship::PartyRelationshipService;
pub use subject_proxy::{
    DataBinding, DataFrame, DataFrameSample, DataSetResult, EnvBinding, FramePayload, Sample,
    SubjectDataSet, SubjectProxyService, SubjectVariable, SystemCall, SystemCallBody,
    VariableSample, VariableValue,
};
pub use system_log::{
    AuditEvent, EmitOutcome, EventActionCode, EventOutcome, ObjectClass, SystemLog,
};
pub use tdd::TddService;
pub use terminology::{
    DefinedTerm, TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation, TerminologyRelationError, TerminologyService,
};
pub use validity::ValidityChecker;
pub use web_template::WebTemplateService;
