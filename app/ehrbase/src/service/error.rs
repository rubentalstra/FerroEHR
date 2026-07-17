//! The service-layer error model and its status-mapping tables — the single
//! SM ↔ [`ServiceError`] ↔ HTTP correspondence.
//!
//! One error vocabulary, three faces: SM call statuses
//! (`master03-common_package.adoc` §Representing Call Status — the
//! [`CallStatusType`] codes of [`super::status`]), this service-internal
//! [`ServiceError`], and the ITS-REST wire codes (ITS-REST overview §HTTP
//! status codes, realized by [`ApiError`]). The tables here keep the three in
//! lock-step; consistency is test-enforced below.

use openehr_its::rest::runtime::ApiError;

use super::status::{CallStatusType, SmError};

/// Service-layer error, mapped to the ITS-REST [`ApiError`] at the protocol
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
    Storage(#[from] crate::storage::error::StorageError),
    /// A database failure.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    /// A JSON (de)serialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A version-signing or read-time integrity failure (RM common master06
    /// §Digital Signature) — either signing at commit failed, or
    /// `verify_on_read = strict` found a stored signature that does not match
    /// the served version.
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
    /// (statuses per `CALL_STATUS_TYPE` + descendants, [`super::status`]).
    ///
    /// Consistency with the wire is test-enforced: for every status,
    /// `ApiError::from(ServiceError::sm(s, m))` and the protocol adapter's
    /// SM → HTTP row (`ehrbase-rest::overview::error::sm_api_error`) produce
    /// the same HTTP status.
    #[must_use]
    pub fn sm(status: CallStatusType, message: impl Into<String>) -> Self {
        use super::status::CallStatusType as S;
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
            | S::EhrForSubjectAlreadyExists
            // A storage-classified generic conflict is also a `409`.
            | S::Conflict => ServiceError::Conflict(m),
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
            //
            // `ServiceOverloaded` originates only at the storage bridge and
            // flows *up* to the wire as an `SmError` (→ `503`); it never
            // round-trips back into a `ServiceError`. `ServiceError` has no
            // overload variant, so this defensive (unreachable) reverse
            // mapping degrades to a server fault.
            S::NotImplemented | S::ServiceOverloaded => ServiceError::Internal(m),
        }
    }
}

impl From<ServiceError> for SmError {
    /// Map a service failure onto the SM native `CALL_STATUS_TYPE` error the
    /// chapter methods return. This is the mirror of the
    /// `From<ServiceError> for ApiError` table below, expressed in SM status
    /// terms — the protocol adapter (`ehrbase-rest`)
    /// then maps the status back to the ITS-REST status code
    /// (`ehrbase-rest::overview::error::sm_api_error`), so the wire outcome is
    /// identical row-for-row:
    ///
    /// | `ServiceError`            | `CallStatusType`             | HTTP |
    /// |---------------------------|------------------------------|------|
    /// | `NotFound`                | `VersionedObjectDoesNotExist`| 404  |
    /// | `VersionConflict`         | `VersionMismatch`            | 412  |
    /// | `Conflict`                | `CompositionAlreadyExists`   | 409  |
    /// | `Unprocessable`           | `ContentInvalid`             | 422  |
    /// | `ValidationFailed`        | `ContentInvalid`             | 422  |
    /// | `BadRequest`              | `PreconditionViolation`      | 400  |
    /// | `Storage`/`Database`      | classified: 409/503/500      |      |
    /// | `Json`/`Signing`/`Internal` | `Exception`                | 500  |
    ///
    /// `NotFound` cannot recover the concrete resource kind, so it maps to the
    /// generic `versioned_object_does_not_exist` (all 404s); a chapter that
    /// knows the precise kind constructs its own `SmError` instead (e.g. the
    /// EHR-index chapter's `IndexError`). `Conflict` maps to a representative
    /// already-exists status (all 409s).
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
        use super::status::CallStatusType as S;
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
            ServiceError::Storage(e) => SmError::from(e),
            // A raw `sqlx` error carries SQLSTATE/constraint detail: classify it
            // (integrity/serialization conflict → 409, pool exhaustion → 503)
            // instead of collapsing every database error to a blanket 500.
            // The classifier emits the structured trace.
            ServiceError::Database(e) => crate::storage::error::classify_sqlx(&e),
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
            // Storage/DB failures carry SQLSTATE/constraint detail: classify
            // them (integrity/serialization conflict → 409, pool exhaustion →
            // 503) rather than blanket-500. A genuine fault stays 500. This
            // path is secondary to the SM `SmError` bridge, but must stay
            // consistent with it.
            ServiceError::Storage(e) => sqlx_conflict_api_error(SmError::from(e)),
            ServiceError::Database(e) => {
                sqlx_conflict_api_error(crate::storage::error::classify_sqlx(&e))
            }
            // A JSON (de)serialization failure at the service boundary is a
            // malformed client payload → 400.
            ServiceError::Json(e) => ApiError::BadRequest(e.to_string()),
            // Signing/integrity failures and generic faults are server-side
            // (5xx).
            ServiceError::Signing(m) | ServiceError::Internal(m) => ApiError::Internal(m),
        }
    }
}

/// Map a storage-classified [`SmError`] (from [`crate::storage::error::classify_sqlx`])
/// to the ITS-REST [`ApiError`] on the direct `ServiceError → ApiError` path.
/// Only the storage-classified statuses occur here — a database conflict
/// (`409`), pool exhaustion (`503`), or a genuine fault (`500`) — mirroring the
/// `sm_api_error` rows the SM bridge uses (`ehrbase-rest::overview::error`).
/// The `503` is our own overload contract (no openEHR spec governs overload;
/// RFC 9110 §15.6.4 is the HTTP authority).
fn sqlx_conflict_api_error(sm: SmError) -> ApiError {
    use super::status::CallStatusType as S;
    match sm.status {
        S::Conflict | S::EhrForSubjectAlreadyExists => ApiError::Conflict(sm.message),
        S::ServiceOverloaded => ApiError::ServiceUnavailable(sm.message),
        _ => ApiError::Internal(sm.message),
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use openehr_its::rest::runtime::ApiError;

    use super::ServiceError;
    use crate::service::status::CallStatusType as S;

    /// `ServiceError::sm(status)` routed to the ITS-REST [`ApiError`] must land
    /// on the HTTP status the SM row prescribes. The SM → `ApiError` half of
    /// the table lives in the protocol adapter
    /// (`ehrbase-rest::overview::error::sm_api_error`) and is tested there
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
