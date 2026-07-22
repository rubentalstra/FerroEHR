//! The request/response types the benchmark driver builds and asserts on, plus
//! the [`Transport`] abstraction it dispatches through. A caller builds an
//! [`HttpRequest`], sends it through a [`Transport`] (the concrete
//! [`SutClient`](crate::sutclient::transport::SutClient) reqwest client), and
//! reads the [`HttpResponse`]. The driver never reaches into the SUT's database
//! — every op is exercised through the SUT's public ITS-REST API only, so the
//! request path is identical for every target
//! (`CNF/docs/guide/master05-assessment.adoc` §Test Environment: the SUT is
//! exercised through its public API only). Absorbed from the retired ECC
//! harness so the benchmark keeps the same provably-identical client for both
//! SUTs (the core fairness guarantee).

/// An HTTP method a request can invoke.
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
    /// `OPTIONS`.
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
/// user, the ADMIN-role user, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSlot {
    /// Send no `Authorization` header.
    None,
    /// The regular clinical-user credential.
    Regular,
    /// The ADMIN-role credential.
    Admin,
}

/// A request made against the SUT. `path` is relative to the SUT's ITS-REST
/// base path (e.g. `"/ehr"`).
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
    /// A `serde_json::Error` if the body is not valid JSON.
    pub fn json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

/// Errors from the transport layer (network, TLS, connection).
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The underlying HTTP client failed.
    #[error("transport: {0}")]
    Http(String),
}

/// The transport a request is dispatched through: send a request, get a
/// response.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Send `request` and return the response.
    ///
    /// # Errors
    /// [`TransportError`] on a network/protocol failure (never for a non-2xx
    /// status — that is returned as an [`HttpResponse`] for the caller to
    /// assert on).
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;

    /// A human-readable description of the SUT this transport reaches
    /// (base URL), recorded in the results.
    fn describe(&self) -> String;
}
