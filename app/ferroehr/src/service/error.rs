//! The service-layer error model and its status-mapping tables — the single
//! SM ↔ [`ServiceError`] ↔ HTTP correspondence.
//!
//! One error vocabulary, three faces: SM call statuses
//! (`master03-common_package.adoc` §Representing Call Status — the
//! [`CallStatusType`] codes of [`super::status`]), this service-internal
//! [`ServiceError`], and the ITS-REST wire codes (ITS-REST overview §HTTP
//! status codes, realized by [`ApiError`]). The tables here keep the three in
//! lock-step; consistency is test-enforced below.

use openehr_base::validate::InvariantViolation;
use openehr_its::json::JsonParseError;
use openehr_its::rest::runtime::ApiError;

use super::status::{CallStatusType, SmError};

/// One violated spec rule, carried as DATA rather than as a pre-formatted
/// sentence.
///
/// A refusal has up to four independently useful facts — the RM attribute
/// PATH it is about, the named INVARIANT it breaks, what is wrong (`detail`),
/// and the machine-produced CAUSES a nested pass reported
/// ([`InvariantViolation`], the RM validation data type). Formatting them into
/// one string at the throw site destroys all four; this type keeps them and
/// renders them exactly once, at the protocol edge
/// (`From<ServiceError> for ApiError` / `for SmError`).
///
/// The rendering is `[<path> ]<detail>[: <cause>; …][ (<invariant>)]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The RM attribute path the violation is about (`ATTESTATION.items`).
    path: Option<String>,
    /// The named RM/spec invariant broken (`ATTESTATION.Items_valid`).
    invariant: Option<String>,
    /// What is wrong, WITHOUT the path/invariant/cause decoration.
    detail: String,
    /// Violations a nested machine pass reported (an RM class-invariant run, a
    /// canonical-JSON decode failure with its JSON path).
    causes: Vec<InvariantViolation>,
}

impl Violation {
    /// A violation stating `detail`, with no path, invariant or causes — the
    /// shape for a refusal whose only fact IS the sentence (a third-party
    /// parser's message, a shape rule with no named invariant).
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            path: None,
            invariant: None,
            detail: detail.into(),
            causes: Vec::new(),
        }
    }

    /// Attach the RM attribute path the violation is about.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Attach the name of the RM/spec invariant this violation breaks.
    #[must_use]
    pub fn with_invariant(mut self, invariant: impl Into<String>) -> Self {
        self.invariant = Some(invariant.into());
        self
    }

    /// Attach the violations a nested machine pass reported.
    #[must_use]
    pub fn with_causes(mut self, causes: Vec<InvariantViolation>) -> Self {
        self.causes = causes;
        self
    }

    /// Attach a canonical-JSON decode failure as the single cause, keeping its
    /// JSON path queryable ([`JsonParseError::path`]).
    #[must_use]
    pub fn with_decode_failure(mut self, error: &JsonParseError) -> Self {
        self.causes = vec![InvariantViolation::at(
            error.path().concat(),
            error.to_string(),
        )];
        self
    }

    /// The RM attribute path this violation is about, when it names one.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The named RM/spec invariant this violation breaks, when it names one.
    #[must_use]
    pub fn invariant(&self) -> Option<&str> {
        self.invariant.as_deref()
    }

    /// What is wrong, without the path/invariant/cause decoration.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The violations a nested machine pass reported.
    #[must_use]
    pub fn causes(&self) -> &[InvariantViolation] {
        &self.causes
    }
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{path} ")?;
        }
        f.write_str(&self.detail)?;
        if !self.causes.is_empty() {
            let joined = self
                .causes
                .iter()
                .map(|c| c.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            write!(f, ": {joined}")?;
        }
        if let Some(invariant) = &self.invariant {
            write!(f, " ({invariant})")?;
        }
        Ok(())
    }
}

/// Service-layer error, mapped to the ITS-REST [`ApiError`] at the protocol
/// boundary so the REST layer stays free of persistence concerns.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// The requested resource does not exist. Carries the granular SM
    /// does-not-exist status ([`CallStatusType`] `*_does_not_exist` family,
    /// `master03-common_package.adoc` §Representing Call Status) so the
    /// [`SmError`] conversion below restores it losslessly — construct via
    /// [`ServiceError::sm`] naming the precise status; extension resources the
    /// SM has no status for (tenants, event subscriptions, FHIR mappings, item
    /// tags) use the generic `versioned_object_does_not_exist`.
    #[error("{0} not found")]
    NotFound(SmError),
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
    /// The submitted payload is malformed or fails a structural rule —
    /// carrying the violated rule as DATA ([`Violation`]: path, invariant,
    /// causes), rendered into prose only at the protocol edge.
    #[error("unprocessable: {0}")]
    Unprocessable(Violation),
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
    /// SM → HTTP row (`ferroehr-rest::overview::error::sm_api_error`) produce
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
            // The does-not-exist family keeps its granular status inside the
            // variant, so the `SmError` round-trip below is lossless — no
            // per-method boundary re-raise needed.
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
            | S::VersionedCompositionDoesNotExist => {
                ServiceError::NotFound(SmError::new(status, m))
            }
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
            // An SM status arrives with a message and no separable facts, so
            // the violation it becomes carries only its detail.
            | S::ContentInvalid => ServiceError::Unprocessable(Violation::new(m)),
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
    /// terms — the protocol adapter (`ferroehr-rest`)
    /// then maps the status back to the ITS-REST status code
    /// (`ferroehr-rest::overview::error::sm_api_error`), so the wire outcome is
    /// identical row-for-row:
    ///
    /// | `ServiceError`            | `CallStatusType`             | HTTP |
    /// |---------------------------|------------------------------|------|
    /// | `NotFound`                | its carried granular status  | 404  |
    /// | `VersionConflict`         | `VersionMismatch`            | 412  |
    /// | `Conflict`                | `CompositionAlreadyExists`   | 409  |
    /// | `Unprocessable`           | `ContentInvalid`             | 422  |
    /// | `ValidationFailed`        | `ContentInvalid`             | 422  |
    /// | `BadRequest`              | `PreconditionViolation`      | 400  |
    /// | `Storage`/`Database`      | classified: 409/503/500      |      |
    /// | `Json`/`Signing`/`Internal` | `Exception`                | 500  |
    ///
    /// `NotFound` carries the granular does-not-exist [`SmError`] it was
    /// constructed with ([`ServiceError::sm`]), so the round-trip restores the
    /// precise status — `ehr_id_does_not_exist` stays `ehr_id_does_not_exist`,
    /// never a resurrected generic. `Conflict` maps to a representative
    /// already-exists status (all 409s).
    ///
    /// NOTE (wire — settled, adjudicated divergence): the structured per-path
    /// violations of `ValidationFailed` (the ITS-REST `Error.validationErrors[]`
    /// array) do **not** survive the SM boundary. That is the SM's own shape,
    /// not an omission here: `I_STATUS` returns a `CALL_STATUS`
    /// (`SM/docs/UML/classes/i_status.adoc` — "Class status object for last
    /// call"), and `CALL_STATUS`
    /// (`SM/docs/UML/classes/call_status.adoc` §Attributes) declares exactly
    /// `code` + `call_name` + `call_string` + `meaning` + `message` — five
    /// scalars, with no slot for a per-path violation list. The violations are
    /// therefore joined into `message` so the detail is not wholly lost, and an
    /// SM-routed `422` renders as `{ error, message }` where the direct
    /// `ApiError` route renders `{ message, validationErrors[] }`.
    ///
    /// The resulting route-dependence of the `422` BODY is spec-permitted:
    /// the ITS-REST docs text assigns the 422 row a meaning only ("The request
    /// was well-formed but was unable to be followed due to semantic errors",
    /// overview `Requests_and_responses.md` §HTTP status codes) and no body
    /// shape, and the released OAS `responses/422_COMPOSITION.yaml` declares no
    /// `content`/`schema` at all (the `Error` object is formally bound to
    /// `400`). Both renderings therefore satisfy the release; the status code —
    /// the part the spec DOES assign — is identical on both routes, which is
    /// what the conversion table above and its tests pin.
    fn from(e: ServiceError) -> Self {
        use super::status::CallStatusType as S;
        match e {
            // Lossless: the granular does-not-exist status travels inside the
            // variant (see `ServiceError::sm`).
            ServiceError::NotFound(e) => e,
            ServiceError::VersionConflict(m) => SmError::new(S::VersionMismatch, m),
            ServiceError::Conflict(m) => SmError::new(S::CompositionAlreadyExists, m),
            // The ONE rendering point of a `Violation` on the SM bridge: the
            // facts travel as data all the way from the throw site to here.
            ServiceError::Unprocessable(v) => SmError::new(S::ContentInvalid, v.to_string()),
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
            // Every does-not-exist status is a wire 404; the message is the
            // carried `SmError`'s text (unchanged from construction).
            ServiceError::NotFound(e) => ApiError::NotFound(e.message),
            ServiceError::BadRequest(m) => ApiError::BadRequest(m),
            ServiceError::Conflict(m) => ApiError::Conflict(m),
            ServiceError::VersionConflict(m) => ApiError::PreconditionFailed(m),
            // The ONE rendering point of a `Violation` on the wire bridge.
            ServiceError::Unprocessable(v) => ApiError::Unprocessable(v.to_string()),
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
/// `sm_api_error` rows the SM bridge uses (`ferroehr-rest::overview::error`).
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
mod tests {
    use openehr_its::rest::runtime::ApiError;

    use openehr_base::validate::InvariantViolation;

    use super::{ServiceError, Violation};
    use crate::service::status::CallStatusType as S;

    /// A [`Violation`] renders `[<path> ]<detail>[: <cause>; …][ (<invariant>)]`
    /// — the ONE place the facts become prose. Each fact is independently
    /// readable back off the value, which is what the refusal tests assert.
    #[test]
    fn violation_renders_its_facts_and_keeps_them_queryable() {
        let plain = Violation::new("contribution must contain versions");
        assert_eq!(plain.to_string(), "contribution must contain versions");
        assert_eq!(plain.path(), None);
        assert_eq!(plain.invariant(), None);
        assert!(plain.causes().is_empty());

        let rule = Violation::new("must be a non-empty list when present")
            .with_path("ATTESTATION.items")
            .with_invariant("ATTESTATION.Items_valid");
        assert_eq!(
            rule.to_string(),
            "ATTESTATION.items must be a non-empty list when present \
             (ATTESTATION.Items_valid)"
        );
        assert_eq!(rule.path(), Some("ATTESTATION.items"));
        assert_eq!(rule.invariant(), Some("ATTESTATION.Items_valid"));

        let nested = Violation::new("is not a valid PARTY_PROXY")
            .with_path("AUDIT_DETAILS.committer")
            .with_causes(vec![
                InvariantViolation::here("Invariant Basic_validity failed"),
                InvariantViolation::at("name", "Invariant Name_valid failed"),
            ]);
        assert_eq!(
            nested.to_string(),
            "AUDIT_DETAILS.committer is not a valid PARTY_PROXY: \
             Invariant Basic_validity failed; Invariant Name_valid failed"
        );
        assert_eq!(nested.causes().len(), 2);
        assert_eq!(nested.causes()[1].path, "name");
    }

    /// The two 422 routes of ONE `ValidationFailed`: the same status on both,
    /// a different body shape by the SM's own model.
    ///
    /// `CALL_STATUS` (`SM/docs/UML/classes/call_status.adoc` §Attributes)
    /// declares five scalars and no per-path violation list, so the SM-routed
    /// failure carries its violations JOINED INTO `message`; the direct
    /// `ApiError` route keeps them as data. The status code — the part the
    /// released text assigns ("The request was well-formed but was unable to be
    /// followed due to semantic errors", ITS-REST overview
    /// `Requests_and_responses.md` §HTTP status codes) — is the same on both,
    /// and neither body shape is contradicted by the release (the OAS
    /// `responses/422_COMPOSITION.yaml` declares no schema). This test pins that
    /// divergence so it can only change deliberately.
    #[test]
    fn validation_failed_renders_two_bodies_but_one_status() {
        let violations = || {
            vec![
                openehr_its::rest::runtime::ValidationError {
                    path: "/content[0]/data".to_owned(),
                    message: "missing mandatory attribute".to_owned(),
                },
                openehr_its::rest::runtime::ValidationError {
                    path: "/context/start_time".to_owned(),
                    message: "not a valid DV_DATE_TIME".to_owned(),
                },
            ]
        };

        // The SM seam: status + one message, every violation still readable.
        let sm =
            crate::service::status::SmError::from(ServiceError::ValidationFailed(violations()));
        assert_eq!(sm.status, S::ContentInvalid);
        assert_eq!(
            sm.message,
            "/content[0]/data: missing mandatory attribute; \
             /context/start_time: not a valid DV_DATE_TIME"
        );

        // The direct wire seam: the violations survive as data.
        match ApiError::from(ServiceError::ValidationFailed(violations())) {
            ApiError::ValidationFailed(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0].path, "/content[0]/data");
                assert_eq!(v[1].message, "not a valid DV_DATE_TIME");
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }

        // One status on both routes.
        assert_eq!(
            ApiError::from(ServiceError::ValidationFailed(violations())).status(),
            ApiError::from(ServiceError::sm(S::ContentInvalid, "x")).status(),
        );
    }

    /// `ServiceError::sm(status)` routed to the ITS-REST [`ApiError`] must land
    /// on the HTTP status the SM row prescribes. The SM → `ApiError` half of
    /// the table lives in the protocol adapter
    /// (`ferroehr-rest::overview::error::sm_api_error`) and is tested there
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

    /// Every granular does-not-exist status must survive the
    /// `ServiceError` round-trip verbatim — `ServiceError::sm(s, m)` back
    /// through `From<ServiceError> for SmError` yields `s`, never a
    /// resurrected generic `versioned_object_does_not_exist` (the #141-era
    /// lossy seam). The SM models these as distinct `CALL_STATUS_TYPE`
    /// codes (`master03-common_package.adoc` §Representing Call Status).
    #[test]
    fn does_not_exist_statuses_round_trip_losslessly() {
        let granular = [
            S::ObjectVersionDoesNotExist,
            S::VersionedObjectDoesNotExist,
            S::EhrIdDoesNotExist,
            S::PartyIdDoesNotExist,
            S::CompositionDoesNotExist,
            S::ContributionDoesNotExist,
            S::ArtefactDoesNotExist,
            S::TemplateDoesNotExist,
            S::VersionDoesNotExist,
            S::SubjectIdDoesNotExist,
            S::VersionedCompositionDoesNotExist,
        ];
        for status in granular {
            let service_err = ServiceError::sm(status, "m");
            assert!(
                matches!(service_err, ServiceError::NotFound(_)),
                "{} must classify as NotFound",
                status.sm_name()
            );
            let restored = crate::service::status::SmError::from(service_err);
            assert_eq!(
                restored.status,
                status,
                "{} resurrected as {}",
                status.sm_name(),
                restored.status.sm_name()
            );
            assert_eq!(restored.message, "m", "message must survive unchanged");
        }
    }
}
