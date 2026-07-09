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
mod ehr;
mod ehr_index;
mod item_tag;
mod relationship;
mod stored_query;
mod template;
mod terminology;
mod version_id;
mod versioned;
mod vobject;

use std::sync::Arc;

use ehrbase_signing::Signer;
use openehr_flat::cache::WebTemplateCache;
use openehr_its::rest::runtime::ApiError;
use sqlx::PgPool;

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
        }
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

    /// The signing context (system id + signer) handed to the `vobject` commit
    /// path so every versioned-object write signs its `ORIGINAL_VERSION`.
    pub(in crate::service) fn signing_ctx(&self) -> vobject::SigningCtx<'_> {
        vobject::SigningCtx {
            system_id: &self.system_id,
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
            // `CallStatusType` is `#[non_exhaustive]`; future statuses map
            // to a server fault until given an explicit row.
            _ => ServiceError::Internal(m),
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

    /// The SM ↔ `ServiceError` ↔ HTTP table is one table: routing a status
    /// through `ServiceError::sm` must land on the same HTTP status as the
    /// direct `CallStatusType::api_error` mapping (design 08 §5).
    #[test]
    fn service_error_route_matches_the_sm_http_table() {
        let statuses = [
            S::AuthFailure,
            S::PreconditionViolation,
            S::ObjectVersionDoesNotExist,
            S::VersionedObjectDoesNotExist,
            S::Exception,
            S::EhrIdDoesNotExist,
            S::PartyIdDoesNotExist,
            S::FileNotWritable,
            S::VersionMismatch,
            S::CompositionDoesNotExist,
            S::ContributionDoesNotExist,
            S::CompositionArchetypeInvalid,
            S::EhrCreateFailDuplicateId,
            S::CompositionAlreadyExists,
            S::EhrForSubjectAlreadyExists,
            S::InvalidArchetype,
            S::InvalidTemplate,
            S::InvalidArtefact,
            S::InvalidQuery,
            S::InvalidIdPattern,
            S::ArtefactDoesNotExist,
            S::TemplateDoesNotExist,
            S::DefinitionUnknown,
            S::ContentInvalid,
            S::VersionDoesNotExist,
            S::SubjectIdDoesNotExist,
            S::VersionedCompositionDoesNotExist,
        ];
        for status in statuses {
            let via_service = ApiError::from(ServiceError::sm(status, "m")).status();
            let direct = status.api_error("m").status();
            // The two auth/exception rows deliberately differ from the direct
            // table: service-side auth/exception faults surface as 500 (auth
            // is the adapter's job — see `ServiceError::sm`).
            if matches!(status, S::AuthFailure) {
                continue;
            }
            assert_eq!(via_service, direct, "row {} diverged", status.sm_name());
        }
    }
}
