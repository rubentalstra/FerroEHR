//! The blocking SUT client for the measurement machinery: base URL + auth
//! resolved once from the ixit `sut` instance; one connection pool shared
//! by every worker thread.

use std::io::Read;
use std::time::Duration;

use base64::Engine;

use crate::ixit::{AuthMode, Instance};

/// Per-request client timeout. A response slower than this is an error
/// arrival recorded at the timeout latency (the SLO is 1 s — a 60 s stall
/// is already a hard violation either way).
pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_mins(1);

/// The blocking SUT client (see the module doc).
#[derive(Clone)]
pub struct PerfClient {
    client: reqwest::blocking::Client,
    base_url: String,
    authorization: Option<String>,
    extra_headers: Vec<(String, String)>,
}

impl std::fmt::Debug for PerfClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerfClient")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

/// One wire response the client observed (status + the two id-bearing
/// headers the bindings capture from).
#[derive(Debug)]
pub(crate) struct WireReply {
    pub(crate) status: u16,
    pub(crate) etag: Option<String>,
    pub(crate) location: Option<String>,
}

/// A request body: content type + bytes.
pub(crate) type RequestBody = Option<(&'static str, Vec<u8>)>;

impl PerfClient {
    /// Build the client from an ixit instance (credentials resolved from
    /// the referenced environment variables, exactly like the functional
    /// driver).
    ///
    /// # Errors
    /// A message when a credential env var is unset or the client cannot
    /// be built.
    pub fn from_instance(instance: &Instance) -> Result<Self, String> {
        let authorization = match &instance.auth {
            AuthMode::None => None,
            AuthMode::Basic {
                user_env,
                password_env,
            } => {
                let user = std::env::var(user_env)
                    .map_err(|_| format!("credential env {user_env} unset"))?;
                let pass = std::env::var(password_env)
                    .map_err(|_| format!("credential env {password_env} unset"))?;
                let token = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{pass}").as_bytes());
                Some(format!("Basic {token}"))
            }
            AuthMode::Bearer { token_env } => {
                let token = std::env::var(token_env)
                    .map_err(|_| format!("credential env {token_env} unset"))?;
                Some(format!("Bearer {token}"))
            }
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(CLIENT_TIMEOUT)
            .pool_max_idle_per_host(256)
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(Self {
            client,
            base_url: instance.base_url.trim_end_matches('/').to_owned(),
            authorization,
            extra_headers: instance.headers.clone().unwrap_or_default(),
        })
    }

    /// Issue one request; `if_match` adds the concurrency-control header
    /// (ITS-REST overview §Concurrency control).
    pub(crate) fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: RequestBody,
        prefer_minimal: bool,
        if_match: Option<&str>,
    ) -> Result<WireReply, String> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url))
            .header("Accept", "application/json");
        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }
        if prefer_minimal {
            request = request.header("Prefer", "return=minimal");
        }
        if let Some(preceding) = if_match {
            request = request.header("If-Match", format!("\"{preceding}\""));
        }
        if let Some((content_type, bytes)) = body {
            request = request.header("Content-Type", content_type).body(bytes);
        }
        let response = request.send().map_err(|e| format!("transport: {e}"))?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let reply = WireReply {
            status: response.status().as_u16(),
            etag: header("etag"),
            location: header("location"),
        };
        // Drain the body so the pooled connection is reusable.
        let mut sink = Vec::new();
        let mut reader = response;
        let _drained = reader.read_to_end(&mut sink);
        Ok(reply)
    }
}

/// `W/"uid"` / `"uid"` → `uid` (the bindings' `strip: weak-quotes`
/// capture).
pub(crate) fn strip_weak_quotes(etag: &str) -> String {
    etag.trim_start_matches("W/").trim_matches('"').to_owned()
}

/// The last path segment of a `Location` header (the bindings' fallback
/// id capture on a `return=minimal` create).
pub(crate) fn location_last_segment(location: &str) -> Option<String> {
    location
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The versioned-object part of an `OBJECT_VERSION_ID`
/// (`uid::system::1` → `uid`).
pub(crate) fn object_uid_of(version_uid: &str) -> String {
    version_uid
        .split("::")
        .next()
        .unwrap_or(version_uid)
        .to_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn header_captures_match_the_bindings() {
        assert_eq!(
            strip_weak_quotes("W/\"abc::sys::1\""),
            "abc::sys::1".to_owned()
        );
        assert_eq!(strip_weak_quotes("\"abc\""), "abc".to_owned());
        assert_eq!(
            location_last_segment("http://sut/ehr/42").as_deref(),
            Some("42")
        );
        assert_eq!(location_last_segment(""), None);
        assert_eq!(object_uid_of("abc::sys::3"), "abc");
        assert_eq!(object_uid_of("bare"), "bare");
    }
}
