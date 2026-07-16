//! Storage-layer error surface.
//!
//! No openEHR spec governs the physical storage — these are our own
//! codec/repository errors. Semantic outcomes (version conflict, not-found,
//! validation) belong to the versioning/service layer and are NOT modelled
//! here; storage reports only what the codec and the SQL row I/O can observe.

use uuid::Uuid;

/// Errors produced by the node-storage codec and the row repositories.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The root value handed to [`crate::storage::codec::decompose`] is not a
    /// decomposable versioned-object tree (its `_type` is not a versioned root).
    #[error("root object has no structure _type (found {0:?})")]
    NotAStructureRoot(Option<String>),

    /// A canonical-JSON array mixed structure and non-structure elements, which
    /// canonical RM JSON never does.
    #[error("array {attribute:?} mixes structure and non-structure elements")]
    MixedArray { attribute: String },

    /// Reassembly received rows that do not form one tree rooted at `num = 0`.
    #[error("invalid node rows: {0}")]
    InvalidRows(String),

    /// A client-supplied CONTRIBUTION uid is already in use (the insert hit
    /// `ON CONFLICT (id) DO NOTHING`). ITS-REST `contribution_create`: "if the
    /// `uid` is supplied it must not already be in use" — a duplicate is a
    /// conflict, never an overwrite. Mapped to 409 by the versioning layer.
    #[error("CONTRIBUTION uid {0:?} is already in use")]
    ContributionUidInUse(Option<Uuid>),

    /// The EHR row insert violated the one-EHR-per-subject unique index
    /// (`uq_ehr_subject`) — another EHR already names this subject
    /// `(id, namespace)` (RM ehr master04 §EHR Status: the subject 0..1
    /// identifies the EHR). Carries `(subject_id, namespace)` so the service
    /// layer can build the ITS-REST `409_EHR` body. Distinct from a duplicate
    /// EHR-id conflict (that surfaces as an absent `RETURNING` row).
    #[error("an EHR already exists for subject {0}@{1}")]
    SubjectInUse(String, String),

    /// A driver/pool/query error from `sqlx`.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<StorageError> for crate::service::status::SmError {
    /// Bridge a storage failure to the SM call-status model. Constraint/
    /// concurrency detail is preserved via [`classify_sqlx`] for the raw
    /// `sqlx` case; the typed conflicts keep their specific `409` meaning; codec
    /// faults are genuine server faults (`exception` → `500`). No openEHR spec
    /// governs the persistence mechanism (`docs/architecture.md` §Storage); the
    /// HTTP mapping the resulting status drives is the ITS-REST-governed one
    /// (overview §HTTP status codes).
    fn from(e: StorageError) -> Self {
        use crate::service::status::{CallStatusType, SmError};
        match e {
            // A raw driver/pool/query error carries SQLSTATE + constraint detail
            // — classify it instead of collapsing to a blanket 500.
            StorageError::Database(db) => classify_sqlx(&db),
            // A client-supplied CONTRIBUTION uid already in use → `409`
            // (normally intercepted at the versioning call site; this is the
            // correct fallback should it propagate).
            StorageError::ContributionUidInUse(_) => {
                SmError::new(CallStatusType::Conflict, e.to_string())
            }
            // The one-EHR-per-subject index → `409` (normally intercepted in
            // `service::ehr` to build the ITS-REST `409_EHR` body).
            StorageError::SubjectInUse(_, _) => {
                SmError::new(CallStatusType::EhrForSubjectAlreadyExists, e.to_string())
            }
            // Codec faults (a non-structure root, a mixed array, malformed node
            // rows) are our own bugs, not client errors → `500`.
            StorageError::NotAStructureRoot(_)
            | StorageError::MixedArray { .. }
            | StorageError::InvalidRows(_) => {
                SmError::new(CallStatusType::Exception, e.to_string())
            }
        }
    }
}

/// Classify a raw [`sqlx::Error`] at the storage boundary into the SM call
/// status it should surface, emitting **one** structured trace record so the
/// SQLSTATE code + constraint name are never lost (they were previously
/// stringified into a blanket `exception`). No openEHR spec governs the
/// persistence mechanism or server-overload behaviour — this is our own design;
/// the resulting HTTP status codes are the ITS-REST-governed ones (overview
/// §HTTP status codes: `409 Conflict`; RFC 9110 §15.6.4: `503 Service
/// Unavailable`).
///
/// - **SQLSTATE class 23** (`integrity_constraint_violation` — unique, foreign
/// key, check, not-null, exclusion) → [`CallStatusType::Conflict`] (`409`).
/// - **40001** (`serialization_failure`) / **40P01** (`deadlock_detected`) →
/// [`CallStatusType::Conflict`] (`409`, retryable).
/// - **[`sqlx::Error::PoolTimedOut`]** (pool exhausted under load) →
/// [`CallStatusType::ServiceOverloaded`] (`503` + `Retry-After`; the W-12
/// admission contract).
/// - anything else → [`CallStatusType::Exception`] (`500`, a genuine fault).
pub(crate) fn classify_sqlx(e: &sqlx::Error) -> crate::service::status::SmError {
    use crate::service::status::{CallStatusType, SmError};
    match e {
        sqlx::Error::PoolTimedOut => {
            tracing::warn!(
                error = %e,
                "storage bridge: connection-pool acquire timed out — shedding as service-overloaded (503)"
            );
            SmError::new(
                CallStatusType::ServiceOverloaded,
                "the server is temporarily overloaded; retry shortly".to_owned(),
            )
        }
        sqlx::Error::Database(db) => {
            let sqlstate = db.code();
            let sqlstate = sqlstate.as_deref().unwrap_or("");
            let constraint = db.constraint();
            if sqlstate.starts_with("23") {
                tracing::warn!(
                    sqlstate,
                    constraint = ?constraint,
                    error = %e,
                    "storage bridge: integrity-constraint violation → 409 conflict"
                );
                SmError::new(CallStatusType::Conflict, e.to_string())
            } else if sqlstate == "40001" || sqlstate == "40P01" {
                tracing::warn!(
                    sqlstate,
                    constraint = ?constraint,
                    error = %e,
                    "storage bridge: serialization/deadlock failure → 409 conflict (retryable)"
                );
                SmError::new(CallStatusType::Conflict, e.to_string())
            } else {
                tracing::error!(
                    sqlstate,
                    constraint = ?constraint,
                    error = %e,
                    "storage bridge: unclassified database error → 500"
                );
                SmError::new(CallStatusType::Exception, e.to_string())
            }
        }
        other => {
            tracing::error!(
                error = %other,
                "storage bridge: non-database sqlx error → 500"
            );
            SmError::new(CallStatusType::Exception, other.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::service::status::CallStatusType;

    use super::classify_sqlx;

    #[test]
    fn pool_timeout_is_service_overloaded() {
        // Pool exhaustion under load → 503 semantics (our W-12 admission
        // contract), never a blanket 500.
        let sm = classify_sqlx(&sqlx::Error::PoolTimedOut);
        assert_eq!(sm.status, CallStatusType::ServiceOverloaded);
    }

    #[test]
    fn non_database_error_is_exception() {
        // A row-not-found or other non-database sqlx error is a genuine fault.
        let sm = classify_sqlx(&sqlx::Error::RowNotFound);
        assert_eq!(sm.status, CallStatusType::Exception);
    }
}
