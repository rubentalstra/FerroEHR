//! The application service layer (P12): turns ITS-REST calls into persisted,
//! versioned openEHR data on the greenfield PG18 schema (ADR-008).
//!
//! Design (grounded in the P10 storage foundation + sqlx/PG18 best practices):
//!
//! - **One versioned-object machinery** ([`vobject`]) serves COMPOSITION,
//!   `EHR_STATUS` and FOLDER uniformly: a write decomposes the canonical JSON into
//!   `node` rows (the P10 [`crate::storage`] codec), inserts a `vo_version` row
//!   (temporal `sys_period`, PG18 `WITHOUT OVERLAPS`), and emits a
//!   `contribution` + `audit` row — all inside one `sqlx` transaction; a read
//!   loads the version's nodes and reassembles them.
//! - **Versioning** is temporal: the current version is the `upper_inf` row; an
//!   update closes the current `sys_period` at `now()` and inserts the next
//!   `sys_version`. No current/`_history` duplication; `ALL_VERSIONS` is the
//!   same table unfiltered.
//! - The service implements the generated ITS-REST server traits (see
//!   [`api`]); `ehrbase-rest` calls them through its `Backend` seam.
//!
//! Static SQL uses runtime `sqlx::query*` (no compile-time macro → no
//! database needed at build time, matching the P10 spike); dynamic multi-row
//! node inserts use `sqlx::QueryBuilder`. `sea-query` is reserved for the AQL
//! engine (P16).

mod admin;
mod api;
mod aql_query;
mod codes;
mod composition;
mod contribution;
mod definition;
mod demographic;
mod directory;
mod dump_load;
mod ehr;
mod ehr_index;
mod event_subscription;
mod item_tag;
mod message;
mod opt_validation;
mod relationship;
mod stored_query;
mod subject_proxy;
mod tdd;
mod template;
mod tenant;
mod terminology;
mod version_id;
mod versioned;
mod vobject;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::signing::Signer;
use crate::system_log::AuditSender;
use ehrbase_sm::TenantContext;
use openehr_flat::cache::WebTemplateCache;
use openehr_its::rest::runtime::ApiError;
use sqlx::PgPool;

/// In-process cache of resolved tenants, keyed by the claim/header value (a
/// tenant name or uuid string) the middleware resolves per request (ADR-015 §4;
/// "cache in-process"). Shared across service clones (single registry view);
/// cleared wholesale on any tenant CRUD write.
type TenantCache = Arc<RwLock<HashMap<String, TenantContext>>>;

/// The default openEHR system identifier stamped into `OBJECT_VERSION_ID`s and
/// audit rows. Configurable per deployment (P18 wires it from config).
pub const DEFAULT_SYSTEM_ID: &str = "ehrbase-rs.local";

/// The DB-backed application service — the concrete [`Backend`](ehrbase_rest::Backend).
#[derive(Debug, Clone)]
pub struct EhrbaseService {
    pool: PgPool,
    system_id: String,
    /// Cache of `WebTemplate`s built from stored OPTs, used by composition
    /// validation (P15) on create/update. Cheaply cloneable (moka-backed).
    web_templates: WebTemplateCache,
    /// Version signer (`VERSION.signature`, RM common §"Digital Signature";
    /// `docs/design/version-signing.md`). Defaults to server-side `digest`
    /// signing; `main.rs` wires the configured [`Signer`].
    signer: Arc<Signer>,
    /// The optional IHE ATNA audit sender realizing the SM `I_SYSTEM_LOG`
    /// component (`crate::system_log`). `None` = auditing off; the binary wires
    /// the configured [`AuditSender`] via [`Self::with_audit`]. Read only
    /// through the [`SystemLog`](ehrbase_sm::SystemLog) impl in
    /// `crate::system_log`.
    pub(crate) audit: Option<AuditSender>,
    /// The optional external terminology provider (FHIR R4), selected when a
    /// deployment opts in ([`crate::terminology::ExternalTerminologyConfig`]).
    /// Used by AQL `TERMINOLOGY('expand', 'hl7.org/fhir/…', …)` resolution (B4);
    /// `None` keeps AQL terminology expansion on the in-process `openehr-term`
    /// bundle only. Read through the [`TerminologyExpander`](crate::aql::TerminologyExpander)
    /// impl in `service::api::terminology`.
    pub(crate) external_terminology: Option<Arc<crate::terminology::FhirTerminologyProvider>>,
    /// Multi-tenancy tenant registry cache (ADR-015 §4). Only ever populated
    /// when tenancy is on (the middleware resolves through it); in single-tenant
    /// mode it stays empty and is never consulted.
    tenant_cache: TenantCache,
}

impl EhrbaseService {
    /// Construct the service over a connection pool with the default system id
    /// and the default (server-side `digest`) version signer.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            system_id: DEFAULT_SYSTEM_ID.to_owned(),
            web_templates: WebTemplateCache::default(),
            signer: Arc::new(Signer::digest_default()),
            audit: None,
            external_terminology: None,
            tenant_cache: TenantCache::default(),
        }
    }

    /// The openEHR `system_id` in effect for the current request: the resolved
    /// tenant's own `system_id` when tenancy is on (ADR-015 §1), else the
    /// service's configured default. Every request (read or write) runs inside
    /// its tenant's task-local scope, so version ids / audits / `EHR.system_id`
    /// pick up the right value ambiently; with tenancy off the task-local is
    /// never set and this is byte-identical to the configured `system_id`.
    pub(super) fn effective_system_id(&self) -> String {
        ehrbase_sm::tenant::current().map_or_else(|| self.system_id.clone(), |t| t.system_id)
    }

    /// Override the openEHR system id (identifies this CDR in versions/audit).
    #[must_use]
    pub fn with_system_id(mut self, system_id: impl Into<String>) -> Self {
        self.system_id = system_id.into();
        self
    }

    /// Install the configured version [`Signer`] (RM common §"Digital
    /// Signature").
    #[must_use]
    pub fn with_signer(mut self, signer: Arc<Signer>) -> Self {
        self.signer = signer;
        self
    }

    /// Install the IHE ATNA audit sender realizing the SM `I_SYSTEM_LOG`
    /// component (`crate::system_log`); the binary boots it and wires it here.
    /// Without it, [`SystemLog`](ehrbase_sm::SystemLog) auditing is off.
    #[must_use]
    pub fn with_audit(mut self, sender: AuditSender) -> Self {
        self.audit = Some(sender);
        self
    }

    /// Install an external FHIR R4 terminology provider (opt-in), used by AQL
    /// `TERMINOLOGY('expand', 'hl7.org/fhir/…', …)` resolution (B4). Without it,
    /// AQL terminology expansion routes only to the in-process `openehr-term`
    /// bundle (`service_api = "openehr"`).
    #[must_use]
    pub fn with_external_terminology(
        mut self,
        provider: Arc<crate::terminology::FhirTerminologyProvider>,
    ) -> Self {
        self.external_terminology = Some(provider);
        self
    }

    /// The signing context (system id + signer) handed to the `vobject` commit
    /// path so every versioned-object write signs its `ORIGINAL_VERSION`.
    pub(in crate::service) fn signing_ctx(&self) -> vobject::SigningCtx<'_> {
        vobject::SigningCtx {
            system_id: self.effective_system_id(),
            signer: &self.signer,
        }
    }

    /// The configured version [`Signer`] (used for read-time verification).
    pub(super) fn signer(&self) -> &Signer {
        &self.signer
    }
}

/// Service-layer error, mapped to the ITS-REST [`ApiError`] at the trait
/// boundary so the REST layer stays free of persistence concerns.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// The requested resource does not exist.
    #[error("{0} not found")]
    NotFound(String),
    /// The request is malformed at the semantic level (e.g. a stale/invalid
    /// `preceding_version_uid`, or an operation on an already-deleted object) —
    /// ITS-REST `400 Bad Request` (`400_already_deleted.yaml`).
    #[error("bad request: {0}")]
    BadRequest(String),
    /// The request conflicts with current state (e.g. EHR already exists).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Optimistic-concurrency precondition (`If-Match`) failed.
    #[error("version conflict: {0}")]
    VersionConflict(String),
    /// The submitted payload is malformed or fails a structural rule.
    #[error("unprocessable: {0}")]
    Unprocessable(String),
    /// A well-formed payload that fails semantic (template/RM/terminology)
    /// validation — carries the per-path violations for the ITS-REST 422 body.
    #[error("{} validation error(s)", .0.len())]
    ValidationFailed(Vec<openehr_its::rest::runtime::ValidationError>),
    /// A storage/codec failure.
    #[error("storage: {0}")]
    Storage(#[from] crate::storage::StorageError),
    /// A database failure.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    /// A JSON (de)serialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A version-signing or read-time integrity failure (RM common §"Digital
    /// Signature") — either signing at commit failed, or `verify_on_read =
    /// strict` found a stored signature that does not match the served version.
    #[error("signing: {0}")]
    Signing(String),
    /// A server-side fault with no more specific variant (SM
    /// `CALL_STATUS_TYPE.exception` / `file_not_writable` — → HTTP 500).
    #[error("internal: {0}")]
    Internal(String),
}

impl ServiceError {
    /// Construct the [`ServiceError`] variant for an SM call status — the
    /// service-side entry into the single SM ↔ `ServiceError` ↔ HTTP table
    /// (design `docs/design/sm-platform/08-target-architecture.md` §5;
    /// statuses per `CALL_STATUS_TYPE` + descendants, `ehrbase-sm::error`).
    ///
    /// Consistency with the wire is test-enforced: for every status,
    /// `ApiError::from(ServiceError::sm(s, m))` and
    /// [`CallStatusType::api_error`] produce the same HTTP status.
    #[must_use]
    pub fn sm(status: ehrbase_sm::CallStatusType, message: impl Into<String>) -> Self {
        use ehrbase_sm::CallStatusType as S;
        let m = message.into();
        match status {
            // `success` is not an error; constructing it is a server bug.
            // Auth is decided at the adapter (401/403 before dispatch), so a
            // service-side auth failure is likewise a server fault.
            S::Success | S::Exception | S::FileNotWritable | S::AuthFailure => {
                ServiceError::Internal(m)
            }
            S::PreconditionViolation | S::InvalidIdPattern => ServiceError::BadRequest(m),
            S::ObjectVersionDoesNotExist
            | S::VersionedObjectDoesNotExist
            | S::EhrIdDoesNotExist
            | S::PartyIdDoesNotExist
            | S::CompositionDoesNotExist
            | S::ContributionDoesNotExist
            | S::ArtefactDoesNotExist
            | S::TemplateDoesNotExist
            | S::VersionDoesNotExist
            | S::SubjectIdDoesNotExist
            | S::VersionedCompositionDoesNotExist => ServiceError::NotFound(m),
            S::VersionMismatch => ServiceError::VersionConflict(m),
            S::EhrCreateFailDuplicateId
            | S::CompositionAlreadyExists
            | S::EhrForSubjectAlreadyExists => ServiceError::Conflict(m),
            S::CompositionArchetypeInvalid
            | S::InvalidArchetype
            | S::InvalidTemplate
            | S::InvalidArtefact
            | S::InvalidQuery
            | S::DefinitionUnknown
            | S::ContentInvalid => ServiceError::Unprocessable(m),
            // No service-side `ServiceError::NotImplemented`; a not-implemented
            // status surfaces as a server fault (the service implements every
            // catalog call, so this row is unreachable in practice).
            S::NotImplemented => ServiceError::Internal(m),
        }
    }
}

impl From<ServiceError> for ehrbase_sm::SmError {
    /// Map a service failure onto the SM native `CALL_STATUS_TYPE` error the
    /// catalog traits return (ADR-011). This is the mirror of the
    /// [`From<ServiceError> for ApiError`] table above, expressed in SM status
    /// terms — the protocol adapter (`ehrbase-rest`) then maps the status back
    /// to the ITS-REST status code via [`ehrbase_sm::CallStatusType::api_error`],
    /// so the wire outcome is identical row-for-row:
    ///
    /// | `ServiceError`            | `CallStatusType`             | HTTP |
    /// |---------------------------|------------------------------|------|
    /// | `NotFound`                | `VersionedObjectDoesNotExist`| 404  |
    /// | `VersionConflict`         | `VersionMismatch`            | 412  |
    /// | `Conflict`                | `CompositionAlreadyExists`   | 409  |
    /// | `Unprocessable`           | `ContentInvalid`             | 422  |
    /// | `ValidationFailed`        | `ContentInvalid`             | 422  |
    /// | `BadRequest`              | `PreconditionViolation`      | 400  |
    /// | `Storage`/`Database`/`Json`/`Signing`/`Internal` | `Exception` | 500 |
    ///
    /// `NotFound` cannot recover the concrete resource kind, so it maps to the
    /// generic `versioned_object_does_not_exist` (all 404s); `Conflict` maps to
    /// a representative already-exists status (all 409s).
    ///
    /// PORT NOTE (wire): the structured per-path violations of `ValidationFailed`
    /// (the ITS-REST `Error.validationErrors[]` array) do **not** survive the SM
    /// boundary — `SmError` carries only a status + message (the SM `I_STATUS`
    /// shape). The violations are joined into the message so the detail is not
    /// wholly lost; the `422` body renders as `{ error, message }` rather than
    /// `{ message, validationErrors[] }`. This is spec-permitted:
    /// `422_COMPOSITION.yaml` declares no `content`/`schema` (the `422` body is
    /// spec-silent; the `Error` object is formally bound only to `400`).
    fn from(e: ServiceError) -> Self {
        use ehrbase_sm::CallStatusType as S;
        use ehrbase_sm::SmError;
        match e {
            ServiceError::NotFound(m) => SmError::new(S::VersionedObjectDoesNotExist, m),
            ServiceError::VersionConflict(m) => SmError::new(S::VersionMismatch, m),
            ServiceError::Conflict(m) => SmError::new(S::CompositionAlreadyExists, m),
            ServiceError::Unprocessable(m) => SmError::new(S::ContentInvalid, m),
            ServiceError::ValidationFailed(v) => {
                let joined = v
                    .into_iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                SmError::new(S::ContentInvalid, joined)
            }
            ServiceError::BadRequest(m) => SmError::new(S::PreconditionViolation, m),
            ServiceError::Storage(e) => SmError::new(S::Exception, e.to_string()),
            ServiceError::Database(e) => SmError::new(S::Exception, e.to_string()),
            ServiceError::Json(e) => SmError::new(S::Exception, e.to_string()),
            ServiceError::Signing(m) | ServiceError::Internal(m) => SmError::new(S::Exception, m),
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound(m) => ApiError::NotFound(m),
            ServiceError::BadRequest(m) => ApiError::BadRequest(m),
            ServiceError::Conflict(m) => ApiError::Conflict(m),
            ServiceError::VersionConflict(m) => ApiError::PreconditionFailed(m),
            ServiceError::Unprocessable(m) => ApiError::Unprocessable(m),
            ServiceError::ValidationFailed(v) => ApiError::ValidationFailed(v),
            // Storage/DB/JSON failures are our fault, not the client's.
            ServiceError::Storage(e) => ApiError::Internal(e.to_string()),
            ServiceError::Database(e) => ApiError::Internal(e.to_string()),
            ServiceError::Json(e) => ApiError::BadRequest(e.to_string()),
            // Signing/integrity failures and generic faults are server-side
            // (5xx).
            ServiceError::Signing(m) | ServiceError::Internal(m) => ApiError::Internal(m),
        }
    }
}

#[cfg(test)]
mod sm_error_table_tests {
    use ehrbase_sm::CallStatusType as S;
    use openehr_its::rest::runtime::ApiError;

    use super::ServiceError;

    /// `ServiceError::sm(status)` routed to the ITS-REST [`ApiError`] must land
    /// on the HTTP status the SM row prescribes (design 08 §5). The SM →
    /// `ApiError` half of the table now lives in the protocol adapter
    /// (`ehrbase-rest::error::sm_api_error`, ADR-011) and is tested there
    /// end-to-end; here we verify the service-side `ServiceError::sm` +
    /// `From<ServiceError> for ApiError` composition against the expected code
    /// per status.
    #[test]
    fn service_error_routes_to_the_expected_http_status() {
        use http::StatusCode as C;
        let rows = [
            (S::PreconditionViolation, C::BAD_REQUEST),
            (S::InvalidIdPattern, C::BAD_REQUEST),
            (S::ObjectVersionDoesNotExist, C::NOT_FOUND),
            (S::VersionedObjectDoesNotExist, C::NOT_FOUND),
            (S::EhrIdDoesNotExist, C::NOT_FOUND),
            (S::PartyIdDoesNotExist, C::NOT_FOUND),
            (S::CompositionDoesNotExist, C::NOT_FOUND),
            (S::ContributionDoesNotExist, C::NOT_FOUND),
            (S::ArtefactDoesNotExist, C::NOT_FOUND),
            (S::TemplateDoesNotExist, C::NOT_FOUND),
            (S::VersionDoesNotExist, C::NOT_FOUND),
            (S::SubjectIdDoesNotExist, C::NOT_FOUND),
            (S::VersionedCompositionDoesNotExist, C::NOT_FOUND),
            (S::VersionMismatch, C::PRECONDITION_FAILED),
            (S::EhrCreateFailDuplicateId, C::CONFLICT),
            (S::CompositionAlreadyExists, C::CONFLICT),
            (S::EhrForSubjectAlreadyExists, C::CONFLICT),
            (S::CompositionArchetypeInvalid, C::UNPROCESSABLE_ENTITY),
            (S::InvalidArchetype, C::UNPROCESSABLE_ENTITY),
            (S::InvalidTemplate, C::UNPROCESSABLE_ENTITY),
            (S::InvalidArtefact, C::UNPROCESSABLE_ENTITY),
            (S::InvalidQuery, C::UNPROCESSABLE_ENTITY),
            (S::DefinitionUnknown, C::UNPROCESSABLE_ENTITY),
            (S::ContentInvalid, C::UNPROCESSABLE_ENTITY),
            // Service-side auth/exception faults surface as 500 (auth is the
            // adapter's job before dispatch — see `ServiceError::sm`).
            (S::Exception, C::INTERNAL_SERVER_ERROR),
            (S::FileNotWritable, C::INTERNAL_SERVER_ERROR),
            (S::AuthFailure, C::INTERNAL_SERVER_ERROR),
        ];
        for (status, expected) in rows {
            let got = ApiError::from(ServiceError::sm(status, "m")).status();
            assert_eq!(got, expected, "row {} diverged", status.sm_name());
        }
    }
}
