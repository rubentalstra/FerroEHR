//! The openEHR **SM Platform Service Model** native API.
//!
//! One Rust trait per SM platform-service interface, organised **one module
//! per SM chapter**, plus the shared `platform.common` package and the
//! call-status error model — the seam between the platform component
//! (`ehrbase`) and every protocol adapter (`ehrbase-rest` for ITS-REST
//! 1.0.3, `EhrScape`, management HTTP, …), exactly the architecture the SM
//! assumes (`master02-overview.adoc` §General Assumptions: a nominal *native
//! API* reached through protocol adapters).
//!
//! Crate map (mirrors the spec's own package structure, master02 §Package
//! Structure: `common` + service components + their interfaces):
//! - [`common`] — the `platform.common` package (master03) + the master02
//!   global conventions: call status / [`SmError`], the version-commit
//!   envelope, `PLATFORM_SERVICE`, `I_VALIDITY_CHECKER`, list cursors.
//! - [`services`] — one module per chapter: `definition` (04), `ehr` (05),
//!   `demographic` (06), `ehr_index` (07), `query` (08), `message` (09),
//!   `subject_proxy` (10), `terminology` (12), `admin` (15), `system_log`.
//! - [`extensions`] — everything **no openEHR spec governs**: adapter
//!   support types, ITS-REST-only adapter traits, the `EHR_ACCESS` scheme.
//! - [`platform`] — the [`Platform`] union trait (the whole component map).
//!
//! Precedence: the SM governs decomposition, naming, and call semantics (its
//! pre/post-conditions are test oracles); **ITS-REST 1.0.3 + the CNF/ECC
//! schedule govern the wire** — divergences are recorded at the declaration
//! with `PORT NOTE`s. Chapter registers: `docs/design/sm-platform/`.

pub mod common;
pub mod config;
pub mod extensions;
pub mod platform;
pub mod services;

pub use config::{REDACTED, Secret, SecretUrl};

/// Multi-tenancy request context — physically an extension
/// ([`extensions::tenant`]; no openEHR spec governs tenancy), re-exported at
/// the root so `ehrbase_sm::tenant::{scope, current}` stays the call form.
pub use extensions::tenant;

pub use common::{
    CallStatus, CallStatusType, Page, PlatformService, SmError, UpdateAttestation, UpdateAudit,
    UpdateVersion, ValidityChecker,
};
pub use extensions::tenant::TenantContext;
pub use extensions::{
    AccessEntry, AccessLevel, CompositionOverride, ContributionAdapter, DefaultAccess,
    DefinitionAdapter, EHR_ACCESS_CONTROL_V1_SCHEME, EHR_ACCESS_CONTROL_V1_TYPE, EhrAccessAdapter,
    EhrAccessSettings, EventSubscriptionAdapter, FhirConnectorAdapter, ItemTagAdapter,
    MultimediaAdapter, Privacy, ResourceMeta, ServiceResponse, TenantAdapter, VersionMetaAdapter,
    WebTemplateService, principal_matches,
};
pub use platform::Platform;
pub use services::{
    AdminArchive, AdminDumpLoad, AdminService, AqlQueryRequest, AuditEvent, CompressionFormat,
    DataBinding, DataFrame, DataFrameSample, DataSetResult, DefinedTerm, DefinitionAdl2Service,
    DefinitionAdl14Service, DefinitionQueryService, DemographicService, DumpLoadFailReport,
    EhrCompositionHandle, EhrCompositionService, EhrContributionHandle, EhrContributionService,
    EhrDirectoryHandle, EhrDirectoryService, EhrExtractService, EhrIndexEntry, EhrIndexService,
    EhrService, EhrStatusHandle, EhrStatusService, EhrSummary, EmitOutcome, EnvBinding,
    EventActionCode, EventOutcome, ExportFormat, ExportSpec, FramePayload, IEhr, LocationDesc,
    ObjectClass, PartyKind, PartyRelationshipService, QueryDescriptor, QueryOutcome, QueryService,
    ResourceInstanceType, ResourceStatus, Sample, StatTimeRange, SubjectDataSet,
    SubjectProxyService, SubjectRef, SubjectVariable, SystemCall, SystemCallBody, SystemLog,
    TddService, TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation, TerminologyRelationError, TerminologyService, TimeRange, VariableSample,
    VariableValue,
};
