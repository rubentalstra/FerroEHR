//! The service-layer error model and its status-mapping tables — the single
//! SM ↔ [`ServiceError`] ↔ HTTP correspondence.
//!
//! One error vocabulary, three faces: SM call statuses
//! (`master03-common_package.adoc` §Representing Call Status — the
//! [`CallStatusType`] codes of [`super::status`]), this service-internal
//! [`ServiceError`], and the ITS-REST wire codes (ITS-REST overview §HTTP
//! status codes, realized by [`ApiError`]). The tables here keep the three in
//! lock-step; consistency is test-enforced below.

use std::sync::Arc;

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
#[derive(Debug, Clone)]
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
    /// The failure the refusal was raised FOR (a codec error, a template
    /// compile failure), reachable through [`std::error::Error::source`] and
    /// never part of the rendering above. Attach it with
    /// [`Violation::with_source`].
    source: Option<Arc<dyn std::error::Error + Send + Sync>>,
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
            source: None,
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

    /// Attach the failure that caused this refusal, leaving the rendering — and
    /// therefore the wire body — byte-identical.
    ///
    /// The cause is carried for the log and for anything walking the chain
    /// ([RFC 0201](https://rust-lang.github.io/rfcs/0201-error-chaining.html)).
    #[must_use]
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Arc::new(source));
        self
    }
}

/// Equality is over the FACTS a refusal states — path, invariant, detail and
/// the reported causes. The attached [`std::error::Error`] source is
/// diagnostic-only and no error type is comparable, so it takes no part.
impl PartialEq for Violation {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.invariant == other.invariant
            && self.detail == other.detail
            && self.causes == other.causes
    }
}

impl Eq for Violation {}

impl std::error::Error for Violation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.source {
            Some(source) => Some(&**source),
            None => None,
        }
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
    NotFound(#[source] SmError),
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
    Unprocessable(#[source] Violation),
    /// A well-formed payload that fails semantic (template/RM/terminology)
    /// validation — carries the per-path violations as the RM validation data
    /// type ([`InvariantViolation`]), which the protocol bridges below render
    /// into the ITS-REST 422 body. The service layer never names a protocol
    /// type: the wire shape is chosen at the edge, not at the throw site.
    #[error("{} validation error(s)", .0.len())]
    ValidationFailed(Vec<InvariantViolation>),
    /// A storage/codec failure.
    #[error("storage: {0}")]
    Storage(#[from] crate::storage::error::StorageError),
    /// A database failure.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    /// A JSON payload the server was asked to READ is malformed — a
    /// client-caused refusal (`400`) whose wire message NAMES the defect
    /// (offset, expected token), because that is the only thing the caller can
    /// act on. Constructed explicitly at a read site; never by `?`.
    #[error("malformed json: {0}")]
    JsonRead(#[source] serde_json::Error),
    /// The server failed to serialize its OWN data — a server-side fault
    /// (`500`). This is the `From<serde_json::Error>` default (a bare `?` on a
    /// `serde_json` failure lands here), because the safe classification of an
    /// unattributed codec failure is "our fault, say nothing": a serde
    /// diagnostic names Rust type and field names, which is server-internal
    /// detail no client can act on. The full diagnostic goes to `tracing`
    /// instead, and the wire body carries `INTERNAL_MESSAGE`.
    #[error("json serialization failed: {0}")]
    JsonWrite(#[from] serde_json::Error),
    /// A version-signing or read-time integrity failure (RM common master06
    /// §Digital Signature) — either signing at commit failed, or
    /// `verify_on_read = strict` found a stored signature that does not match
    /// the served version.
    ///
    /// The carried text is a LOG detail, not a wire message: both bridges
    /// below trace it and answer with `INTERNAL_MESSAGE`.
    #[error("signing: {0}")]
    Signing(String),
    /// A server-side fault with no more specific variant (SM
    /// `CALL_STATUS_TYPE.exception` / `file_not_writable` — → HTTP 500).
    ///
    /// The carried text is a LOG detail, not a wire message. Sites construct
    /// it with whatever diagnoses the fault (a codec error, an unexpected
    /// stored shape, a failed conversion); both bridges below put that on the
    /// trace record and answer with `INTERNAL_MESSAGE`, because a `500` is
    /// by definition something the client cannot act on.
    #[error("internal: {0}")]
    Internal(String),
    /// A failure carrying the error that CAUSED it, beside the SM status and
    /// message the variants above carry as a bare string.
    ///
    /// The carried status routes to exactly the wire outcome
    /// [`ServiceError::sm`] gives it, and the message is what the equivalent
    /// flat variant would carry, so no response body changes. What the flat
    /// variants cannot hold is the cause: a `String` payload has nowhere to put
    /// the `sqlx`/codec/transport error that actually failed, and
    /// `format!("…{e}")` destroys it before anything can walk it
    /// ([RFC 0201](https://rust-lang.github.io/rfcs/0201-error-chaining.html)).
    /// Here it rides [`SmError`]'s source field, out of the message — a
    /// `500`-class body must disclose no internal error value.
    ///
    /// Construct via [`ServiceError::internal`] (a `500`) or
    /// [`ServiceError::bad_request`] (a `400`).
    #[error("{0}")]
    Caused(#[source] SmError),
}

/// The client-visible message of a server-side fault (`500`). Deliberately
/// opaque: a serde/codec/driver diagnostic names Rust types, RM attribute
/// names, SQL aliases and schema objects — server-internal detail the client
/// can neither act on nor be trusted with. The diagnosis belongs in the
/// server's own logs, so every 500-class body carries this sentence and the
/// detail rides one structured `tracing` record instead. (No openEHR spec
/// governs error-body wording beyond the `{ error, message }` shape —
/// ITS-REST overview §HTTP status codes; the opacity is our own design.)
pub(crate) const INTERNAL_MESSAGE: &str = "the server encountered an internal error";

/// Record a server-side fault on the trace record and return the curated
/// opaque `exception` [`SmError`] its 500-class body carries.
///
/// `context` names the operation that failed (a static call-site label);
/// `detail` is the raw diagnostic — a serde error, a codec failure, a driver
/// string — which is written to `tracing` and NEVER to the wire. Use this at
/// every site that would otherwise render a foreign error's `Display` into a
/// 500 body.
pub(crate) fn internal_fault(context: &'static str, detail: &dyn std::fmt::Display) -> SmError {
    tracing::error!(context, error = %detail, "internal server fault → 500");
    SmError::new(CallStatusType::Exception, INTERNAL_MESSAGE.to_owned())
}

/// Record a server-side fault AND its whole cause chain on the trace record,
/// and return the curated opaque `exception` [`SmError`] its 500-class body
/// carries.
///
/// The sibling of [`internal_fault`] for a fault that carries a
/// [`std::error::Error`] source: the `cause` field is the walked
/// [`std::error::Error::source`] chain, which is the only place the underlying
/// `sqlx`/codec/transport diagnosis is readable — it never reaches the wire.
fn internal_fault_caused(
    context: &'static str,
    error: &(dyn std::error::Error + 'static),
) -> SmError {
    if let Some(cause) = error.source() {
        tracing::error!(
            context,
            error = %error,
            cause = %ErrorChain::new(cause),
            "internal server fault → 500"
        );
    } else {
        tracing::error!(context, error = %error, "internal server fault → 500");
    }
    SmError::new(CallStatusType::Exception, INTERNAL_MESSAGE.to_owned())
}

/// A [`std::fmt::Display`] view of an error and its remaining source chain.
///
/// The hops are joined with `: ` — the log rendering of a carried cause
/// ([RFC 0201](https://rust-lang.github.io/rfcs/0201-error-chaining.html)).
///
/// For a trace field only: a chain ends in whatever its innermost source knows
/// (SQL text, a DSN, an internal URL), which is exactly what a 5xx response
/// body must not disclose.
///
/// # Examples
///
/// ```
/// use ferroehr::service::error::ErrorChain;
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("outer")]
/// struct Outer(#[source] Inner);
/// #[derive(Debug, thiserror::Error)]
/// #[error("inner")]
/// struct Inner;
///
/// assert_eq!(ErrorChain::new(&Outer(Inner)).to_string(), "outer: inner");
/// ```
#[derive(Debug)]
pub struct ErrorChain<'a>(&'a (dyn std::error::Error + 'static));

impl<'a> ErrorChain<'a> {
    /// A chain view rooted at `error`.
    #[must_use]
    pub fn new(error: &'a (dyn std::error::Error + 'static)) -> Self {
        Self(error)
    }
}

impl std::fmt::Display for ErrorChain<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        let mut next = self.0.source();
        while let Some(cause) = next {
            write!(f, ": {cause}")?;
            next = cause.source();
        }
        Ok(())
    }
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
            // `ServiceOverloaded` originates only at the storage bridge and
            // flows *up* to the wire as an `SmError` (→ `503`); it never
            // round-trips back into a `ServiceError`, so this defensive reverse
            // mapping degrades to a server fault too.
            S::NotImplemented | S::ServiceOverloaded => ServiceError::Internal(m),
        }
    }

    /// A server-side fault (`500`) that carries the failure which caused it.
    ///
    /// `context` names the step that failed and is a LOG detail: the client
    /// body is the fixed internal message on both bridges, never this text and never
    /// the cause. The cause is reachable through
    /// [`std::error::Error::source`], which is what lets an operator read the
    /// `sqlx`/codec diagnosis the flat [`ServiceError::Internal`] string
    /// destroys.
    #[must_use]
    pub fn internal(
        context: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ServiceError::Caused(SmError::exception(context).with_source(source))
    }

    /// A malformed-request refusal (`400`) that carries the failure which
    /// caused it.
    ///
    /// `detail` is the client-visible text, unchanged from what the flat
    /// [`ServiceError::BadRequest`] carried: a `4xx` describes the caller's own
    /// request, so naming the defect there is the contract. The cause rides the
    /// source chain for the log and for callers that branch on it.
    #[must_use]
    pub fn bad_request(
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ServiceError::Caused(SmError::precondition(detail).with_source(source))
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
    /// | `JsonRead`                | `PreconditionViolation`      | 400  |
    /// | `JsonWrite`/`Signing`/`Internal` | `Exception`           | 500  |
    /// | `Caused`                  | its carried status            |      |
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
            // A malformed payload the server was asked to read names its own
            // defect (`400`); a failure to serialize the server's own data is
            // a server fault whose diagnostic stays in the log (`500`).
            ServiceError::JsonRead(e) => SmError::new(S::PreconditionViolation, e.to_string()),
            ServiceError::JsonWrite(e) => internal_fault("serialize a JSON payload", &e),
            ServiceError::Signing(m) => internal_fault("sign or verify a version", &m),
            ServiceError::Internal(m) => internal_fault("complete the request", &m),
            ServiceError::Caused(sm) => caused_sm_error(sm),
        }
    }
}

/// The SM half of the [`ServiceError::Caused`] row: a `500`-class status
/// answers the curated opaque message (with the whole cause chain traced),
/// every other status IS already the [`SmError`] the wire needs — cause
/// included, message untouched.
fn caused_sm_error(sm: SmError) -> SmError {
    if is_server_fault(sm.status) {
        internal_fault_caused("complete the request", &sm)
    } else {
        sm
    }
}

/// Whether an SM status is one [`ServiceError::sm`] routes to
/// [`ServiceError::Internal`] — the `500`-class rows whose body is the curated
/// opaque message, never the carried detail.
///
/// The single authority both [`ServiceError::Caused`] bridges consult, so a
/// cause-carrying failure lands on exactly the wire outcome its flat twin does.
fn is_server_fault(status: CallStatusType) -> bool {
    use super::status::CallStatusType as S;
    matches!(
        status,
        S::Success
            | S::Exception
            | S::FileNotWritable
            | S::AuthFailure
            | S::NotImplemented
            | S::ServiceOverloaded
    )
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
            // The ONE place the RM violation data becomes the protocol's
            // `validationErrors[]` shape (the two carry the same `{path,
            // message}` facts).
            ServiceError::ValidationFailed(v) => ApiError::ValidationFailed(
                v.into_iter()
                    .map(|e| openehr_its::rest::runtime::ValidationError {
                        path: e.path,
                        message: e.message,
                    })
                    .collect(),
            ),
            // Storage/DB failures carry SQLSTATE/constraint detail: classify
            // them (integrity/serialization conflict → 409, pool exhaustion →
            // 503) rather than blanket-500. A genuine fault stays 500. This
            // path is secondary to the SM `SmError` bridge, but must stay
            // consistent with it.
            ServiceError::Storage(e) => sqlx_conflict_api_error(SmError::from(e)),
            ServiceError::Database(e) => {
                sqlx_conflict_api_error(crate::storage::error::classify_sqlx(&e))
            }
            // A malformed payload the server was asked to read is a client
            // defect → `400`, and the body names it (that IS the strict
            // reader's contract). A failure to serialize the server's own data
            // is a server fault → `500` with the curated opaque message; the
            // serde diagnostic rides the trace record only.
            ServiceError::JsonRead(e) => ApiError::BadRequest(e.to_string()),
            ServiceError::JsonWrite(e) => {
                ApiError::Internal(internal_fault("serialize a JSON payload", &e).message)
            }
            // Signing/integrity failures and generic faults are server-side
            // (`500`): the carried text is the log detail, the body is the
            // curated message.
            ServiceError::Signing(m) => {
                ApiError::Internal(internal_fault("sign or verify a version", &m).message)
            }
            ServiceError::Internal(m) => {
                ApiError::Internal(internal_fault("complete the request", &m).message)
            }
            // The cause-carrying row: routed by its SM status through exactly
            // the table above, so the wire outcome equals the flat variant's.
            ServiceError::Caused(sm) if is_server_fault(sm.status) => {
                ApiError::Internal(internal_fault_caused("complete the request", &sm).message)
            }
            ServiceError::Caused(sm) => ApiError::from(ServiceError::sm(sm.status, sm.message)),
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
                InvariantViolation::at("/content[0]/data", "missing mandatory attribute"),
                InvariantViolation::at("/context/start_time", "not a valid DV_DATE_TIME"),
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

    /// The JSON twins: a payload the server was asked to READ names its own
    /// defect on a `400` (the strict reader's contract — the client can only
    /// fix what it is told), while a failure to serialize the server's OWN
    /// data is a `500` whose body says nothing about serde's diagnosis. Both
    /// bridges (`SmError` and `ApiError`) must agree, on both twins.
    #[test]
    fn json_read_names_its_defect_and_json_write_stays_opaque() {
        use http::StatusCode as C;

        // A serde diagnostic names Rust field/type names and the byte offset —
        // the "internal markers" that must not reach a 500-class body.
        let markers = ["invalid type", "line", "column"];
        let parse_failure = || {
            let failure: Result<std::collections::BTreeMap<String, i32>, _> =
                serde_json::from_str("{\"committer\":\"not an integer\"}");
            failure.expect_err("the fixture should fail to deserialize")
        };
        let rendered = parse_failure().to_string();
        assert!(
            markers.iter().any(|m| rendered.contains(m)),
            "the fixture must actually produce a serde diagnostic, got {rendered:?}"
        );

        // READ → 400 on both bridges, naming the defect.
        let read = ServiceError::JsonRead(parse_failure());
        assert_eq!(
            crate::service::status::SmError::from(ServiceError::JsonRead(parse_failure())).status,
            S::PreconditionViolation
        );
        let api = ApiError::from(read);
        assert_eq!(api.status(), C::BAD_REQUEST);
        assert!(
            markers.iter().any(|m| api.to_string().contains(m)),
            "a parse refusal must name its defect, got {:?}",
            api.to_string()
        );

        // WRITE → 500 on both bridges, carrying nothing of the diagnostic.
        let sm = crate::service::status::SmError::from(ServiceError::JsonWrite(parse_failure()));
        assert_eq!(sm.status, S::Exception);
        let api = ApiError::from(ServiceError::JsonWrite(parse_failure()));
        assert_eq!(api.status(), C::INTERNAL_SERVER_ERROR);
        for leaked in markers {
            assert!(
                !sm.message.contains(leaked),
                "the SM 500 message leaked {leaked:?}: {:?}",
                sm.message
            );
            assert!(
                !api.to_string().contains(leaked),
                "the wire 500 body leaked {leaked:?}: {:?}",
                api.to_string()
            );
        }
    }

    /// A `500`-class `ServiceError` carries its diagnosis to the LOG, never to
    /// the client: whatever the fault site wrote into `Internal`/`Signing`
    /// (a codec message, an unexpected stored shape, a failed conversion) stays
    /// out of the body on BOTH bridges.
    #[test]
    fn internal_faults_never_render_their_diagnosis_on_the_wire() {
        use http::StatusCode as C;

        let detail = "typing the ORIGINAL_VERSION: unknown field `_kind` at line 3 column 12";
        let markers = ["ORIGINAL_VERSION", "unknown field", "_kind", "line 3"];
        let variants: [fn(String) -> ServiceError; 2] =
            [ServiceError::Internal, ServiceError::Signing];
        for make in variants {
            let sm = crate::service::status::SmError::from(make(detail.to_owned()));
            assert_eq!(sm.status, S::Exception);
            let api = ApiError::from(make(detail.to_owned()));
            assert_eq!(api.status(), C::INTERNAL_SERVER_ERROR);
            for leaked in markers {
                assert!(
                    !sm.message.contains(leaked),
                    "the SM 500 message leaked {leaked:?}: {:?}",
                    sm.message
                );
                assert!(
                    !api.to_string().contains(leaked),
                    "the wire 500 body leaked {leaked:?}: {:?}",
                    api.to_string()
                );
            }
        }
    }

    /// Every SM status a [`ServiceError::Caused`] can carry lands on exactly
    /// the wire outcome — status AND body text — its cause-less twin produces.
    ///
    /// This is the whole safety property of carrying a cause: the chain is a
    /// new fact for the operator, never a change to what the client is told.
    /// Both bridges are checked, over every `CallStatusType`.
    #[test]
    fn a_carried_cause_changes_no_wire_outcome() {
        for status in every_status() {
            let flat_api = ApiError::from(ServiceError::sm(status, "m"));
            let caused_api = ApiError::from(ServiceError::Caused(
                crate::service::status::SmError::new(status, "m").with_source(cause()),
            ));
            assert_eq!(
                caused_api.status(),
                flat_api.status(),
                "row {} diverged on status",
                status.sm_name()
            );
            assert_eq!(
                caused_api.to_string(),
                flat_api.to_string(),
                "row {} diverged on body text",
                status.sm_name()
            );

            let flat_sm = crate::service::status::SmError::from(ServiceError::sm(status, "m"));
            let caused_sm = crate::service::status::SmError::from(ServiceError::Caused(
                crate::service::status::SmError::new(status, "m").with_source(cause()),
            ));
            assert_eq!(
                caused_sm.message,
                flat_sm.message,
                "row {} diverged on the SM message",
                status.sm_name()
            );
            // The two SM statuses need not be the same NAME: a flat variant with
            // a bare-string payload (`Conflict`, `Unprocessable`) has no slot for
            // WHICH conflict or WHICH 422 it was, so its bridge substitutes a
            // representative status (see the conversion table above) where the
            // cause-carrying row keeps the one it was built with. What must hold
            // is that both land on the same wire outcome.
            assert_eq!(
                ApiError::from(ServiceError::sm(caused_sm.status, "m")).status(),
                ApiError::from(ServiceError::sm(flat_sm.status, "m")).status(),
                "row {} routed the two SM statuses to different wire codes",
                status.sm_name()
            );
        }
    }

    /// A carried cause stays WALKABLE — [`std::error::Error::source`] reaches
    /// the concrete underlying error type, which is the whole point of RFC 0201
    /// chaining — while never appearing in the `500`-class body.
    #[test]
    fn a_carried_cause_is_walkable_and_never_on_the_wire() {
        use std::error::Error;

        let err = ServiceError::internal("read a stored version row", cause());
        let first = Error::source(&err).expect("a carried cause must be reachable");

        let mut hops = Vec::new();
        let mut found = None;
        let mut next = Some(first);
        while let Some(step) = next {
            hops.push(step.to_string());
            if let Some(io) = step.downcast_ref::<std::io::Error>() {
                found = Some(io.kind());
            }
            next = step.source();
        }
        assert_eq!(
            found,
            Some(std::io::ErrorKind::PermissionDenied),
            "walking the chain must reach the underlying std::io::Error, got {hops:?}"
        );

        // The same chain must be invisible to the client on both bridges.
        let leaked = "the node table is not readable";
        let api = ApiError::from(ServiceError::internal("read a stored version row", cause()));
        assert_eq!(api.status(), http::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !api.to_string().contains(leaked),
            "the wire 500 body leaked the cause: {:?}",
            api.to_string()
        );
        let sm = crate::service::status::SmError::from(ServiceError::internal(
            "read a stored version row",
            cause(),
        ));
        assert!(
            !sm.message.contains(leaked) && !sm.message.contains("read a stored version row"),
            "the SM 500 message leaked the fault detail: {:?}",
            sm.message
        );
    }

    /// A [`Violation`] with a carried cause renders EXACTLY as one without —
    /// the 422 body is the refusal's own facts, and the cause is chain-only.
    #[test]
    fn a_violation_cause_is_chain_only() {
        use std::error::Error;

        let plain =
            Violation::new("is not a valid PARTY_PROXY").with_path("AUDIT_DETAILS.committer");
        let carried = Violation::new("is not a valid PARTY_PROXY")
            .with_path("AUDIT_DETAILS.committer")
            .with_source(cause());
        assert_eq!(carried.to_string(), plain.to_string());
        assert_eq!(carried, plain, "equality is over the facts, not the cause");
        assert!(Error::source(&plain).is_none());
        // The hop must be the CONCRETE cause, not a smart-pointer wrapper
        // around it — otherwise no caller can branch on what actually failed.
        assert_eq!(
            Error::source(&carried)
                .and_then(|c| c.downcast_ref::<std::io::Error>())
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::PermissionDenied)
        );

        let api = ApiError::from(ServiceError::Unprocessable(carried));
        assert_eq!(
            api.to_string(),
            ApiError::from(ServiceError::Unprocessable(plain)).to_string()
        );
    }

    /// A stand-in underlying failure whose `Display` is exactly the kind of
    /// internal detail a `500` body must never carry.
    fn cause() -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "the node table is not readable",
        )
    }

    /// Every `CallStatusType` variant, so the parity check above cannot miss a
    /// row (the enum is deliberately exhaustive — see its type docs).
    fn every_status() -> [S; 31] {
        [
            S::Success,
            S::AuthFailure,
            S::PreconditionViolation,
            S::ObjectVersionDoesNotExist,
            S::VersionedObjectDoesNotExist,
            S::Exception,
            S::EhrIdDoesNotExist,
            S::PartyIdDoesNotExist,
            S::FileNotWritable,
            S::VersionMismatch,
            S::NotImplemented,
            S::Conflict,
            S::ServiceOverloaded,
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
        ]
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
