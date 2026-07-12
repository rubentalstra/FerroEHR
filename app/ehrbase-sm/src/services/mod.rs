//! The openEHR SM Platform Service Model interfaces — **one module per SM
//! chapter** (`master04`–`master15`), one Rust trait per interface,
//! transcribed literally: each trait carries its interface's exact SM call
//! names, parameter names and types, spec returns, and pre/post-conditions
//! (in the per-method doc-comments). Native errors are
//! [`SmError`](crate::SmError) over `CALL_STATUS_TYPE` (chapter 03,
//! [`crate::common`]) — **no** ITS-REST types in the catalog.
//!
//! Chapter map (mirrors `docs/design/sm-platform/`): [`definition`]
//! (master04), [`ehr`] (master05), [`demographic`] (master06), [`ehr_index`]
//! (master07), [`query`] (master08), [`message`] (master09),
//! [`subject_proxy`] (master10), [`terminology`] (master12), [`admin`]
//! (master15), [`system_log`] (the master02 System Log component: "IHE
//! ATNA-compliant system log"). ITS-REST-only operations the SM does not
//! define live in [`crate::extensions`], flagged with `PORT NOTE`s.

pub mod admin;
pub mod definition;
pub mod demographic;
pub mod ehr;
pub mod ehr_index;
pub mod message;
pub mod query;
pub mod subject_proxy;
pub mod system_log;
pub mod terminology;

pub use admin::{
    AdminArchive, AdminDumpLoad, AdminService, CompressionFormat, DumpLoadFailReport, ExportFormat,
    ExportSpec, StatTimeRange,
};
pub use definition::{
    DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService, QueryDescriptor,
};
pub use demographic::{DemographicService, PartyKind, PartyRelationshipService};
pub use ehr::{
    EhrCompositionHandle, EhrCompositionService, EhrContributionHandle, EhrContributionService,
    EhrDirectoryHandle, EhrDirectoryService, EhrService, EhrStatusHandle, EhrStatusService,
    EhrSummary, IEhr, TimeRange,
};
pub use ehr_index::{
    EhrIndexEntry, EhrIndexService, LocationDesc, ResourceInstanceType, ResourceStatus, SubjectRef,
};
pub use message::{EhrExtractService, TddService};
pub use query::{AqlQueryRequest, QueryOutcome, QueryService};
pub use subject_proxy::{
    DataBinding, DataFrame, DataFrameSample, DataSetResult, EnvBinding, FramePayload, Sample,
    SubjectDataSet, SubjectProxyService, SubjectVariable, SystemCall, SystemCallBody,
    VariableSample, VariableValue,
};
pub use system_log::{
    AuditEvent, EmitOutcome, EventActionCode, EventOutcome, ObjectClass, SystemLog,
};
pub use terminology::{
    DefinedTerm, TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation, TerminologyRelationError, TerminologyService,
};
