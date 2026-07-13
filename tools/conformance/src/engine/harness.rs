//! Execution scaffolding shared by the runner and the case suites: the
//! transport abstraction ([`Transport`]) a case drives, the per-run context
//! ([`RunContext`]), and the case run-function type ([`CaseRun`]).
//!
//! A case is a plain async function over a [`RunContext`]; it builds
//! [`HttpRequest`]s, sends them through the context's [`Transport`], and
//! asserts on the [`HttpResponse`]. The concrete transport — the external
//! reqwest client — lives in [`crate::engine::transport`]; cases never depend
//! on it directly, so a case runs against any deployed SUT unchanged
//! (`CNF/docs/guide/master05-assessment.adoc` §Test Environment: the SUT is
//! exercised through its public API only).

use std::future::Future;
use std::pin::Pin;

use crate::edition::{EditionPolicy, EditionRecorder};
use crate::model::case::Format;
use crate::sut::descriptor::SutDescriptor;

/// An HTTP method a case can invoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `DELETE`.
    Delete,
    /// `OPTIONS` (the ITS-REST conformance endpoint + capability probes).
    Options,
}

impl Method {
    /// The uppercase HTTP method name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Options => "OPTIONS",
        }
    }
}

/// Which credential slot a request authenticates with: the regular clinical
/// user, the ADMIN-role user, or none (the SEC negative cases).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSlot {
    /// Send no `Authorization` header.
    None,
    /// The regular clinical-user credential.
    Regular,
    /// The ADMIN-role credential.
    Admin,
}

/// A request a case makes against the SUT. `path` is relative to the SUT's
/// ITS-REST base path (e.g. `"/ehr"`).
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// The HTTP method.
    pub method: Method,
    /// The request path, relative to the SUT base path.
    pub path: String,
    /// Extra request headers (name, value).
    pub headers: Vec<(String, String)>,
    /// The request body, if any.
    pub body: Option<Vec<u8>>,
    /// The credential slot to authenticate with.
    pub auth: AuthSlot,
}

impl HttpRequest {
    /// A request with the [`AuthSlot::Regular`] credential and no body/headers.
    #[must_use]
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: Vec::new(),
            body: None,
            auth: AuthSlot::Regular,
        }
    }

    /// A `GET` request.
    #[must_use]
    pub fn get(path: impl Into<String>) -> Self {
        Self::new(Method::Get, path)
    }

    /// A `POST` request.
    #[must_use]
    pub fn post(path: impl Into<String>) -> Self {
        Self::new(Method::Post, path)
    }

    /// A `PUT` request.
    #[must_use]
    pub fn put(path: impl Into<String>) -> Self {
        Self::new(Method::Put, path)
    }

    /// A `DELETE` request.
    #[must_use]
    pub fn delete(path: impl Into<String>) -> Self {
        Self::new(Method::Delete, path)
    }

    /// Set the credential slot.
    #[must_use]
    pub fn with_auth(mut self, auth: AuthSlot) -> Self {
        self.auth = auth;
        self
    }

    /// Add a header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Attach a canonical-JSON body, setting `Content-Type: application/json`.
    ///
    /// # Errors
    /// Propagates a `serde_json` serialization error.
    pub fn json_body(mut self, value: &serde_json::Value) -> Result<Self, CaseError> {
        let body = serde_json::to_vec(value).map_err(|e| CaseError::Codec(e.to_string()))?;
        self.body = Some(body);
        self.headers
            .push(("content-type".to_owned(), "application/json".to_owned()));
        Ok(self)
    }

    /// Attach a raw text body with an explicit content type (e.g. OPT 1.4
    /// XML for template upload, or an AQL string).
    #[must_use]
    pub fn text_body(mut self, body: impl Into<String>, content_type: &str) -> Self {
        self.body = Some(body.into().into_bytes());
        self.headers
            .push(("content-type".to_owned(), content_type.to_owned()));
        self
    }
}

/// A response from the SUT.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// The HTTP status code.
    pub status: u16,
    /// Response headers (name lowercased, value).
    pub headers: Vec<(String, String)>,
    /// The raw response body.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// The first value of the header named `name` (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&name))
            .map(|(_, v)| v.as_str())
    }

    /// The body decoded as UTF-8 (lossy).
    #[must_use]
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }

    /// The body parsed as JSON.
    ///
    /// # Errors
    /// [`CaseError::Codec`] if the body is not valid JSON.
    pub fn json(&self) -> Result<serde_json::Value, CaseError> {
        serde_json::from_slice(&self.body).map_err(|e| CaseError::Codec(e.to_string()))
    }
}

/// Errors from the transport layer (network, TLS, connection).
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The underlying HTTP client failed.
    #[error("transport: {0}")]
    Http(String),
}

/// A case failure or execution error.
#[derive(Debug, thiserror::Error)]
pub enum CaseError {
    /// An assertion did not hold — a genuine conformance finding. Where the
    /// assertion was edition-laddered, the message names the rungs tried.
    #[error("assertion failed: {0}")]
    Assertion(String),
    /// The transport failed (not a conformance finding — a runner/SUT
    /// availability error).
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// A payload could not be (de)serialized.
    #[error("codec: {0}")]
    Codec(String),
    /// The case was skipped for a stated reason (structural: no ITS-REST
    /// binding, native-API-only, SUT config unavailable, or an own-corpus
    /// adjudication).
    #[error("skipped: {0}")]
    Skipped(String),
}

/// The transport a case drives: send a request, get a response.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Send `request` and return the response.
    ///
    /// # Errors
    /// [`TransportError`] on a network/protocol failure (never for a non-2xx
    /// status — that is returned as an [`HttpResponse`] for the case to
    /// assert on).
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;

    /// A human-readable description of the SUT this transport reaches
    /// (base URL), recorded in the results.
    fn describe(&self) -> String;
}

/// The data-set accounting of one case execution — including the schedule
/// coverage bound (register 13 G-2: the report prints `driven/total` against
/// `schedule_rows`, so a bound is logged, never silent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataSetReport {
    /// Data sets that passed.
    pub passed: u32,
    /// Data sets attempted.
    pub total: u32,
    /// The number of data-set rows the governing schedule table defines for
    /// this case, where the schedule tabulates one (`None` for single-flow
    /// cases). `total < schedule_rows` is a logged coverage bound.
    pub schedule_rows: Option<u32>,
}

impl DataSetReport {
    /// A report where a single (whole-case) assertion passed.
    pub const SINGLE: DataSetReport = DataSetReport {
        passed: 1,
        total: 1,
        schedule_rows: None,
    };

    /// A report of `n`/`n` data sets passing.
    #[must_use]
    pub const fn all(n: u32) -> Self {
        Self {
            passed: n,
            total: n,
            schedule_rows: None,
        }
    }

    /// Attach the governing schedule table's row count (the coverage bound).
    #[must_use]
    pub const fn of_schedule_rows(mut self, rows: u32) -> Self {
        self.schedule_rows = Some(rows);
        self
    }
}

/// The per-run context handed to a case run function.
pub struct RunContext<'a> {
    /// The transport reaching the SUT.
    pub transport: &'a dyn Transport,
    /// The wire format this run is exercising.
    pub format: Format,
    /// The SUT descriptor: per-SUT wire facts (system id, template-id form,
    /// admin mount) come from here, never from literals (register 90 §8.4).
    pub sut: &'a SutDescriptor,
    /// The edition policy for this run (pinned for our CI; auto for BYO).
    pub edition_policy: EditionPolicy,
    /// The per-case edition recorder (drained by the executor).
    pub edition: &'a EditionRecorder,
    /// The terminology server available for this run (`TS` area): the
    /// hermetic wiremock fixture or a real server. `None` when none was
    /// established.
    pub tx: Option<&'a crate::ts::TxServer>,
}

impl std::fmt::Debug for RunContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunContext")
            .field("format", &self.format)
            .field("transport", &self.transport.describe())
            .field("sut", &self.sut.name)
            .field("edition_policy", &self.edition_policy)
            .finish_non_exhaustive()
    }
}

impl RunContext<'_> {
    /// Send a request through the SUT transport, mapping transport failures
    /// into [`CaseError`].
    ///
    /// # Errors
    /// [`CaseError::Transport`] on a network/protocol failure.
    pub async fn send(&self, request: HttpRequest) -> Result<HttpResponse, CaseError> {
        Ok(self.transport.send(request).await?)
    }
}

/// A boxed future returned by a case run function.
pub type CaseFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DataSetReport, CaseError>> + Send + 'a>>;

/// A case run function: execute the case against the context, returning the
/// data-set report or a [`CaseError`].
pub type CaseRun = for<'a> fn(&'a RunContext<'a>) -> CaseFuture<'a>;
