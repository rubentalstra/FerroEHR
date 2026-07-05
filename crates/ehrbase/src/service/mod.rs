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
mod composition;
mod contribution;
mod directory;
mod ehr;
mod versioned;
mod vobject;

use sqlx::PgPool;

use openehr_its::rest::runtime::ApiError;

/// The default openEHR system identifier stamped into `OBJECT_VERSION_ID`s and
/// audit rows. Configurable per deployment (P18 wires it from config).
pub const DEFAULT_SYSTEM_ID: &str = "ehrbase-rs.local";

/// The DB-backed application service — the concrete [`Backend`](ehrbase_rest::Backend).
#[derive(Debug, Clone)]
pub struct EhrbaseService {
    pool: PgPool,
    system_id: String,
}

impl EhrbaseService {
    /// Construct the service over a connection pool with the default system id.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            system_id: DEFAULT_SYSTEM_ID.to_owned(),
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
    /// The request conflicts with current state (e.g. EHR already exists).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Optimistic-concurrency precondition (`If-Match`) failed.
    #[error("version conflict: {0}")]
    VersionConflict(String),
    /// The submitted payload is malformed or fails a structural rule.
    #[error("unprocessable: {0}")]
    Unprocessable(String),
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
            ServiceError::Conflict(m) => ApiError::Conflict(m),
            ServiceError::VersionConflict(m) => ApiError::PreconditionFailed(m),
            ServiceError::Unprocessable(m) => ApiError::Unprocessable(m),
            // Storage/DB/JSON failures are our fault, not the client's.
            ServiceError::Storage(e) => ApiError::Internal(e.to_string()),
            ServiceError::Database(e) => ApiError::Internal(e.to_string()),
            ServiceError::Json(e) => ApiError::BadRequest(e.to_string()),
        }
    }
}
