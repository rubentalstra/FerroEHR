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

mod api;
mod aql_query;
mod codes;
mod composition;
mod contribution;
mod directory;
mod ehr;
mod item_tag;
mod stored_query;
mod template;
mod version_id;
mod versioned;
mod vobject;

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
}

impl EhrbaseService {
    /// Construct the service over a connection pool with the default system id.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            system_id: DEFAULT_SYSTEM_ID.to_owned(),
            web_templates: WebTemplateCache::default(),
        }
    }

    /// Override the openEHR system id (identifies this CDR in versions/audit).
    #[must_use]
    pub fn with_system_id(mut self, system_id: impl Into<String>) -> Self {
        self.system_id = system_id.into();
        self
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
        }
    }
}
