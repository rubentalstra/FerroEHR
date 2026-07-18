//! The BFF's CDR client: one `reqwest` client, the ITS-REST base path,
//! credential injection, strict `Accept`/`Content-Type` negotiation
//! (`crate::format`), and error normalization into
//! [`crate::error::AdminUiError`]. The console reaches the CDR ONLY here.

use crate::error::AdminUiError;
use crate::session::Credential;

/// A normalized CDR response: status, declared content type, raw body.
#[derive(Debug, Clone)]
pub struct CdrResponse {
    /// HTTP status code.
    pub status: u16,
    /// The `Content-Type` the CDR declared, if any.
    pub content_type: Option<String>,
    /// The raw body bytes as text (all ITS-REST representations are text).
    pub body: String,
}

/// The one outbound HTTP client to the CDR.
#[derive(Debug, Clone)]
pub struct CdrClient {
    http: reqwest::Client,
    /// scheme://host:port — no trailing slash.
    origin: String,
}

impl CdrClient {
    /// Build the client from configuration.
    ///
    /// # Errors
    /// [`AdminUiError::Internal`] when the underlying client cannot be
    /// constructed.
    pub fn new(cfg: &crate::config::CdrConfig) -> Result<Self, AdminUiError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(cfg.request_timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AdminUiError::Internal(format!("CDR HTTP client: {e}")))?;
        Ok(Self {
            http,
            origin: cfg.base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// The ITS-REST v1 base (`{origin}/ehrbase/rest/openehr/v1`). Endpoint
    /// paths appended to this come from the generated `openehr-its` REST
    /// contract; the served `openapi.json` is the cross-check.
    #[must_use]
    pub fn rest_v1(&self, path: &str) -> String {
        format!(
            "{}/ehrbase/rest/openehr/v1/{}",
            self.origin,
            path.trim_start_matches('/')
        )
    }

    /// A URL directly under the CDR origin (`/ehrbase/rest/status`,
    /// `/.well-known/smart-configuration`, the served `openapi.json`).
    #[must_use]
    pub fn origin_url(&self, path: &str) -> String {
        format!("{}/{}", self.origin, path.trim_start_matches('/'))
    }

    /// GET `url` as `credential`, asking for `accept`.
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure; the response
    /// itself (any status) is returned for the caller to interpret.
    pub async fn get(
        &self,
        credential: &Credential,
        url: &str,
        accept: &str,
    ) -> Result<CdrResponse, AdminUiError> {
        let request = self.http.get(url).header(http::header::ACCEPT, accept);
        Self::finish(Self::authorize(request, credential)).await
    }

    /// GET `url` without any credential (pre-auth surfaces: `/ehrbase/rest/status`,
    /// the SMART discovery document).
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn get_public(&self, url: &str, accept: &str) -> Result<CdrResponse, AdminUiError> {
        Self::finish(self.http.get(url).header(http::header::ACCEPT, accept)).await
    }

    /// POST `body` to `url` as `credential` with the given `Content-Type` +
    /// `Accept` and extra headers (e.g. `openehr-template-id` on simplified
    /// commits, `Prefer: return=representation`).
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn post(
        &self,
        credential: &Credential,
        url: &str,
        content_type: &str,
        accept: &str,
        headers: &[(&str, &str)],
        body: String,
    ) -> Result<CdrResponse, AdminUiError> {
        let mut request = self
            .http
            .post(url)
            .header(http::header::CONTENT_TYPE, content_type)
            .header(http::header::ACCEPT, accept)
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        Self::finish(Self::authorize(request, credential)).await
    }

    /// PUT a text body to `url` as `credential` (stored-query writes use
    /// `text/plain` AQL per the Definition API).
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn put_text(
        &self,
        credential: &Credential,
        url: &str,
        content_type: &str,
        body: String,
    ) -> Result<CdrResponse, AdminUiError> {
        let request = self
            .http
            .put(url)
            .header(http::header::CONTENT_TYPE, content_type)
            .header(http::header::ACCEPT, "application/json")
            .body(body);
        Self::finish(Self::authorize(request, credential)).await
    }

    /// Map a non-2xx [`CdrResponse`] into the normalized error, extracting
    /// the diagnostic from the openEHR error body when present. 2xx passes
    /// through.
    ///
    /// # Errors
    /// [`AdminUiError::Forbidden`] on 401/403, [`AdminUiError::Cdr`] on any
    /// other non-2xx status.
    pub fn expect_success(response: CdrResponse) -> Result<CdrResponse, AdminUiError> {
        if (200..300).contains(&response.status) {
            return Ok(response);
        }
        let message = diagnostic_of(&response);
        match response.status {
            401 | 403 => Err(AdminUiError::Forbidden(message)),
            status => Err(AdminUiError::Cdr { status, message }),
        }
    }

    fn authorize(
        request: reqwest::RequestBuilder,
        credential: &Credential,
    ) -> reqwest::RequestBuilder {
        match credential {
            Credential::Basic { username, password } => {
                request.basic_auth(username, Some(password))
            }
            Credential::Bearer { access_token } => request.bearer_auth(access_token),
        }
    }

    async fn finish(request: reqwest::RequestBuilder) -> Result<CdrResponse, AdminUiError> {
        let response = request
            .send()
            .await
            .map_err(|e| AdminUiError::CdrUnreachable(e.to_string()))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let body = response
            .text()
            .await
            .map_err(|e| AdminUiError::CdrUnreachable(format!("reading body: {e}")))?;
        Ok(CdrResponse {
            status,
            content_type,
            body,
        })
    }
}

/// Pull the human diagnostic out of an openEHR error body
/// (`{"error": …, "message": …}` per ITS-REST), falling back to the raw
/// body or the bare status.
fn diagnostic_of(response: &CdrResponse) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.body) {
        for key in ["message", "error"] {
            if let Some(text) = value.get(key).and_then(serde_json::Value::as_str) {
                return text.to_owned();
            }
        }
    }
    if response.body.is_empty() {
        format!("HTTP {}", response.status)
    } else {
        let mut text = response.body.clone();
        text.truncate(500);
        text
    }
}

#[cfg(test)]
mod tests {
    use super::{CdrClient, CdrResponse, diagnostic_of};
    use crate::error::AdminUiError;

    fn response(status: u16, body: &str) -> CdrResponse {
        CdrResponse {
            status,
            content_type: Some("application/json".to_owned()),
            body: body.to_owned(),
        }
    }

    #[test]
    fn diagnostic_prefers_the_openehr_message_field() {
        let r = response(
            400,
            r#"{"error":"Bad Request","message":"AQL syntax error at line 1"}"#,
        );
        assert_eq!(diagnostic_of(&r), "AQL syntax error at line 1");
    }

    #[test]
    fn diagnostic_falls_back_to_error_then_raw_then_status() {
        assert_eq!(
            diagnostic_of(&response(409, r#"{"error":"duplicate"}"#)),
            "duplicate"
        );
        assert_eq!(diagnostic_of(&response(418, "teapot")), "teapot");
        assert_eq!(diagnostic_of(&response(502, "")), "HTTP 502");
    }

    #[test]
    fn expect_success_maps_auth_statuses_to_forbidden() {
        let err = CdrClient::expect_success(response(403, r#"{"message":"scope"}"#)).unwrap_err();
        assert_eq!(err, AdminUiError::Forbidden("scope".to_owned()));
        let err = CdrClient::expect_success(response(404, "")).unwrap_err();
        assert_eq!(
            err,
            AdminUiError::Cdr {
                status: 404,
                message: "HTTP 404".to_owned()
            }
        );
        assert!(CdrClient::expect_success(response(201, "ok")).is_ok());
    }
}
