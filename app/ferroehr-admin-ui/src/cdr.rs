// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The BFF's CDR client.
//!
//! One `reqwest` client, the ITS-REST base path, credential injection, strict
//! `Accept`/`Content-Type` negotiation (`crate::format`), and error
//! normalization into [`crate::error::AdminUiError`]. The console reaches the
//! CDR ONLY here.

#![expect(
    clippy::disallowed_types,
    reason = "the console consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694)"
)]

use crate::error::AdminUiError;
use crate::session::Credential;

/// A normalized CDR response: status, declared content type, raw body.
#[derive(Debug, Clone)]
pub struct CdrResponse {
    /// The HTTP status the CDR answered with.
    pub status: http::StatusCode,
    /// The `Content-Type` the CDR declared, if any.
    pub content_type: Option<String>,
    /// The raw body bytes as text (all ITS-REST representations are text).
    pub body: String,
}

impl CdrResponse {
    /// Whether the response carries exactly this status.
    ///
    /// The status is typed all the way from `reqwest` to the branch that reads
    /// it (owner directive 2026-08-06), so callers name an
    /// [`http::StatusCode`] constant and a transposed code is a compile error
    /// rather than a silent wrong branch.
    #[must_use]
    pub fn is(&self, code: http::StatusCode) -> bool {
        self.status == code
    }
}

/// The one outbound HTTP client to the CDR.
#[derive(Debug, Clone)]
pub struct CdrClient {
    http: reqwest::Client,
    /// scheme://host:port — no trailing slash.
    origin: String,
    /// The management surface's base URL including its base path — no trailing
    /// slash. Derived from the configuration
    /// ([`CdrConfig::management_base`](crate::config::CdrConfig::management_base)),
    /// so it may point at a different host/port than [`Self::origin`].
    management_base: String,
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
            management_base: cfg.management_base(),
        })
    }

    /// The ITS-REST v1 base (`{origin}/ferroehr/rest/openehr/v1`). Endpoint
    /// paths appended to this come from the generated `openehr-its` REST
    /// contract; the served `openapi.json` is the cross-check.
    #[must_use]
    pub fn rest_v1(&self, path: &str) -> String {
        format!(
            "{}/ferroehr/rest/openehr/v1/{}",
            self.origin,
            path.trim_start_matches('/')
        )
    }

    /// The ITS-REST v1 base path itself, with NO trailing slash — the System
    /// API's conformance-manifest resource (`OPTIONS {base_path}`). The server
    /// mounts that route at the exact base path above its normalization layer,
    /// so the trailing slash `rest_v1("")` would add must not be sent.
    #[must_use]
    pub fn rest_v1_root(&self) -> String {
        format!("{}/ferroehr/rest/openehr/v1", self.origin)
    }

    /// A URL directly under the CDR origin (`/ferroehr/rest/status`,
    /// `/.well-known/smart-configuration`, the served `openapi.json`).
    #[must_use]
    pub fn origin_url(&self, path: &str) -> String {
        format!("{}/{}", self.origin, path.trim_start_matches('/'))
    }

    /// A URL under the CDR's **management** surface (`info`, `metrics`,
    /// `metrics/{name}`, `env`, `loggers`).
    ///
    /// NOTE: no openEHR spec governs a management surface — our own operational
    /// extension. It is a separate base URL because the CDR may serve it from
    /// its own internal listener; `path` is appended to the configured prefix,
    /// which already carries the CDR's `management.base_path`.
    #[must_use]
    pub fn management_url(&self, path: &str) -> String {
        format!("{}/{}", self.management_base, path.trim_start_matches('/'))
    }

    /// `OPTIONS url` with no credential, asking for `accept`.
    ///
    /// The System API's conformance manifest is served above the CORS layer and
    /// outside authentication (ITS-REST System API — `security: []`), so this
    /// probe carries no credential.
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure; the response
    /// itself (any status) is returned for the caller to interpret.
    pub async fn options_public(
        &self,
        url: &str,
        accept: &str,
    ) -> Result<CdrResponse, AdminUiError> {
        Self::finish(
            self.http
                .request(http::Method::OPTIONS, url)
                .header(http::header::ACCEPT, accept),
        )
        .await
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

    /// GET `url` without any credential (pre-auth surfaces: `/ferroehr/rest/status`,
    /// the SMART discovery document).
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn get_public(&self, url: &str, accept: &str) -> Result<CdrResponse, AdminUiError> {
        Self::finish(self.http.get(url).header(http::header::ACCEPT, accept)).await
    }

    /// The auth schemes the CDR advertises, as `(basic, bearer)`.
    ///
    /// An unauthenticated request to a protected endpoint is answered `401`
    /// with a `WWW-Authenticate` challenge listing exactly the enabled
    /// mechanisms (RFC 9110 §11.6.1; the ITS-REST overview requires the
    /// challenge). A non-401 answer means the CDR runs with auth disabled —
    /// every mechanism the console offers can then "succeed", so both count
    /// as advertised.
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn advertised_schemes(&self) -> Result<(bool, bool), AdminUiError> {
        let url = self.rest_v1("definition/template/adl1.4");
        let response = self
            .http
            .get(&url)
            .header(http::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| AdminUiError::CdrUnreachable(e.to_string()))?;
        if response.status() != http::StatusCode::UNAUTHORIZED {
            return Ok((true, true));
        }
        let challenge = response
            .headers()
            .get(http::header::WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        Ok((challenge.contains("Basic"), challenge.contains("Bearer")))
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

    /// PUT `body` to `url` as `credential` with the given `Content-Type` +
    /// `Accept` and extra headers (e.g. `If-Match` = the preceding
    /// `version_uid` on a versioned COMPOSITION update, `openehr-template-id`
    /// on a simplified commit). Mirrors [`Self::post`] for the write paths
    /// that need conditional/representation headers on a PUT.
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn put(
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
            .put(url)
            .header(http::header::CONTENT_TYPE, content_type)
            .header(http::header::ACCEPT, accept)
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        Self::finish(Self::authorize(request, credential)).await
    }

    /// DELETE `url` as `credential` with the given extra headers (e.g.
    /// `If-Match` = the preceding `version_uid` on a versioned directory
    /// delete). No request body; asks for `application/json` so an error body
    /// (the openEHR diagnostic) is returned in the negotiated format.
    ///
    /// # Errors
    /// [`AdminUiError::CdrUnreachable`] on transport failure.
    pub async fn delete(
        &self,
        credential: &Credential,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<CdrResponse, AdminUiError> {
        let mut request = self
            .http
            .delete(url)
            .header(http::header::ACCEPT, "application/json");
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
        if response.status.is_success() {
            return Ok(response);
        }
        let message = diagnostic_of(&response);
        match response.status {
            http::StatusCode::UNAUTHORIZED | http::StatusCode::FORBIDDEN => {
                Err(AdminUiError::Forbidden(message))
            }
            status => Err(AdminUiError::Cdr {
                status: status.as_u16(),
                message,
            }),
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
        let status = response.status();
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

/// The `validationErrors` entries of an openEHR error body, one string each.
///
/// The released error shape declares the member as required and carries one
/// string per violation
/// (`docs/specs/openehr/ITS-REST/specifications/schemas/others/Error.yaml`),
/// so a `422` can name every offending path instead of one generic line.
fn validation_errors(value: &serde_json::Value) -> Vec<String> {
    value
        .get("validationErrors")
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    entry
                        .as_str()
                        .map_or_else(|| entry.to_string(), str::to_owned)
                })
                .filter(|text| !text.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Pull the human diagnostic out of a refused body, whatever its vocabulary:
/// the openEHR error shape (`{"message": …, "validationErrors": […]}` per
/// ITS-REST — the message first, then one line per violation) or a FHIR
/// `OperationOutcome` (the connector authors its refusals as outcomes — their
/// `issue[].diagnostics` join into one line), falling back to the raw body or
/// the bare status. ONE reader for both vocabularies, so no screen ever hands
/// raw JSON to a toast (#2581).
fn diagnostic_of(response: &CdrResponse) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.body) {
        if value
            .get("resourceType")
            .and_then(serde_json::Value::as_str)
            == Some("OperationOutcome")
        {
            let joined = value
                .get("issue")
                .and_then(serde_json::Value::as_array)
                .map(|issues| {
                    issues
                        .iter()
                        .filter_map(|issue| {
                            issue
                                .get("diagnostics")
                                .or_else(|| issue.get("code"))
                                .and_then(serde_json::Value::as_str)
                                .filter(|text| !text.is_empty())
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_default();
            if !joined.is_empty() {
                return joined;
            }
        }
        let details = validation_errors(&value);
        for key in ["message", "error"] {
            if let Some(text) = value.get(key).and_then(serde_json::Value::as_str) {
                return std::iter::once(text.to_owned())
                    .chain(details)
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        if !details.is_empty() {
            return details.join("\n");
        }
    }
    if response.body.is_empty() {
        format!("HTTP {}", response.status.as_u16())
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

    fn response(status: http::StatusCode, body: &str) -> CdrResponse {
        CdrResponse {
            status,
            content_type: Some("application/json".to_owned()),
            body: body.to_owned(),
        }
    }

    #[test]
    fn an_operation_outcome_refusal_yields_its_diagnostics_never_raw_json() {
        // #2581: the shared reader speaks the FHIR refusal vocabulary too.
        let response = CdrResponse {
            status: http::StatusCode::BAD_REQUEST,
            content_type: Some("application/fhir+json".to_owned()),
            body: r#"{"resourceType":"OperationOutcome","issue":[
                {"severity":"error","code":"invalid","diagnostics":"missing field `template_id`"},
                {"severity":"error","code":"structure"}]}"#
                .to_owned(),
        };
        let text = diagnostic_of(&response);
        assert_eq!(text, "missing field `template_id`; structure");
        assert!(!text.contains("resourceType"), "never raw JSON: {text}");
    }

    #[test]
    fn diagnostic_prefers_the_openehr_message_field() {
        let r = response(
            http::StatusCode::BAD_REQUEST,
            r#"{"error":"Bad Request","message":"AQL syntax error at line 1"}"#,
        );
        assert_eq!(diagnostic_of(&r), "AQL syntax error at line 1");
    }

    #[test]
    fn diagnostic_falls_back_to_error_then_raw_then_status() {
        assert_eq!(
            diagnostic_of(&response(
                http::StatusCode::CONFLICT,
                r#"{"error":"duplicate"}"#
            )),
            "duplicate"
        );
        assert_eq!(
            diagnostic_of(&response(http::StatusCode::IM_A_TEAPOT, "teapot")),
            "teapot"
        );
        assert_eq!(
            diagnostic_of(&response(http::StatusCode::BAD_GATEWAY, "")),
            "HTTP 502"
        );
    }

    #[test]
    fn every_validation_error_gets_its_own_line_under_the_message() {
        // `Error.yaml` declares `validationErrors` as a required member: a 422
        // names each offending path, so the reader renders all of them.
        let r = response(
            http::StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"message":"Validation failed","validationErrors":[
                "/content[0]/data: missing mandatory element",
                "/content[0]/items[1]: cardinality 1..1 violated",
                "/context/start_time: value is not a DV_DATE_TIME"]}"#,
        );
        assert_eq!(
            diagnostic_of(&r),
            "Validation failed\n/content[0]/data: missing mandatory element\n/content[0]/items[1]: \
             cardinality 1..1 violated\n/context/start_time: value is not a DV_DATE_TIME"
        );
    }

    #[test]
    fn validation_errors_render_even_without_a_message_member() {
        let r = response(
            http::StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"validationErrors":["/content: missing"]}"#,
        );
        assert_eq!(diagnostic_of(&r), "/content: missing");
        // An empty list leaves the message exactly as it was.
        let plain = response(
            http::StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"message":"Validation failed","validationErrors":[]}"#,
        );
        assert_eq!(diagnostic_of(&plain), "Validation failed");
    }

    #[test]
    fn management_urls_hang_off_their_own_configurable_prefix() {
        let default = CdrClient::new(&crate::config::CdrConfig::default()).expect("client");
        assert_eq!(
            default.management_url("info"),
            "http://localhost:8080/management/info"
        );
        // A leading slash on the path is not a second separator.
        assert_eq!(
            default.management_url("/metrics/aql_queries_total"),
            "http://localhost:8080/management/metrics/aql_queries_total"
        );
        // The surface may live on its own internal listener, unrelated to the
        // API origin the rest of the console talks to.
        let split = CdrClient::new(&crate::config::CdrConfig {
            base_url: "http://cdr:8080".to_owned(),
            management_base_url: "http://cdr:9464/management".to_owned(),
            ..crate::config::CdrConfig::default()
        })
        .expect("client");
        assert_eq!(
            split.management_url("loggers"),
            "http://cdr:9464/management/loggers"
        );
        assert_eq!(
            split.rest_v1("query/aql"),
            "http://cdr:8080/ferroehr/rest/openehr/v1/query/aql"
        );
    }

    #[test]
    fn expect_success_maps_auth_statuses_to_forbidden() {
        let err = CdrClient::expect_success(response(
            http::StatusCode::FORBIDDEN,
            r#"{"message":"scope"}"#,
        ))
        .unwrap_err();
        assert_eq!(err, AdminUiError::Forbidden("scope".to_owned()));
        let err = CdrClient::expect_success(response(http::StatusCode::NOT_FOUND, "")).unwrap_err();
        assert_eq!(
            err,
            AdminUiError::Cdr {
                status: 404,
                message: "HTTP 404".to_owned()
            }
        );
        // The typed status survives the round trip into the error's own reader.
        assert_eq!(err.status_code(), Some(http::StatusCode::NOT_FOUND));
        assert!(CdrClient::expect_success(response(http::StatusCode::CREATED, "ok")).is_ok());
    }
}
