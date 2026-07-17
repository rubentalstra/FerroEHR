//! The external SUT client (design §4.3): a pure `reqwest` API client with the
//! two credential slots (regular clinical user + ADMIN-role user). It never
//! reaches into the SUT's database — cases are self-contained through the API
//! (unlike `EHRbase`'s own Robot harness).

use async_trait::async_trait;

use crate::engine::harness::{
    AuthSlot, HttpRequest, HttpResponse, Method, Transport, TransportError,
};

/// A SUT credential.
#[derive(Debug, Clone)]
pub enum Credential {
    /// HTTP Basic (`user:pass`).
    Basic {
        /// The username.
        user: String,
        /// The password.
        pass: String,
    },
    /// `OAuth2` bearer token.
    Bearer(String),
}

impl Credential {
    /// Parse an auth spec: `basic:<user>:<pass>` or `bearer:<token>`.
    ///
    /// # Errors
    /// Returns an error string if the scheme is unknown or the spec is malformed.
    pub fn parse(spec: &str) -> Result<Credential, String> {
        match spec.split_once(':') {
            Some(("basic", rest)) => {
                let (user, pass) = rest
                    .split_once(':')
                    .ok_or_else(|| "basic auth expects basic:<user>:<pass>".to_owned())?;
                Ok(Credential::Basic {
                    user: user.to_owned(),
                    pass: pass.to_owned(),
                })
            }
            Some(("bearer", token)) => Ok(Credential::Bearer(token.to_owned())),
            _ => Err(format!(
                "unknown auth spec {spec:?} (expected basic:… or bearer:…)"
            )),
        }
    }
}

/// A reqwest-backed SUT client.
#[derive(Debug, Clone)]
pub struct SutClient {
    http: reqwest::Client,
    /// The ITS-REST base URL, e.g. `http://host:8080/ehrbase/rest/openehr/v1`.
    base_url: String,
    /// An optional **sibling** base URL for the admin API, e.g.
    /// `http://host:8091/ehrbase/rest/admin`. When set, a request whose path
    /// starts with `/admin` is routed here (with the `/admin` prefix stripped)
    /// instead of nested under `base_url`. Upstream `EHRbase` serves admin at a
    /// sibling mount of `/rest/openehr`; our own server nests it under the
    /// openEHR base, so this stays `None` for our runs (§3a.5) — zero drift.
    admin_base_url: Option<String>,
    regular: Option<Credential>,
    admin: Option<Credential>,
}

impl SutClient {
    /// Build a client against `base_url` (trailing slash trimmed) with the given
    /// credential slots.
    ///
    /// # Errors
    /// [`TransportError`] if the HTTP client cannot be constructed.
    pub fn new(
        base_url: impl Into<String>,
        regular: Option<Credential>,
        admin: Option<Credential>,
    ) -> Result<Self, TransportError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| TransportError::Http(e.to_string()))?;
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        Ok(Self {
            http,
            base_url,
            admin_base_url: None,
            regular,
            admin,
        })
    }

    /// Route `/admin/*` requests to a sibling admin base URL (trailing slash
    /// trimmed) instead of nesting them under [`base_url`](Self::base_url).
    /// `None` (the default) keeps every request nested under the openEHR base —
    /// the behaviour of every existing run (§3a.5).
    #[must_use]
    pub fn with_admin_base_url(mut self, admin_base_url: Option<String>) -> Self {
        self.admin_base_url = admin_base_url.map(|u| u.trim_end_matches('/').to_owned());
        self
    }

    /// Resolve a request path to a full URL. A path under `/admin` goes to the
    /// sibling admin base when one is configured (the `/admin` prefix stripped),
    /// else everything nests under `base_url`.
    fn resolve_url(&self, path: &str) -> String {
        if let Some(admin) = &self.admin_base_url
            && let Some(rest) = path.strip_prefix("/admin")
        {
            return format!("{admin}{rest}");
        }
        format!("{}{}", self.base_url, path)
    }

    fn credential(&self, slot: AuthSlot) -> Option<&Credential> {
        match slot {
            AuthSlot::None => None,
            AuthSlot::Regular => self.regular.as_ref(),
            // An admin case with no admin slot configured falls back to the
            // regular credential (external runs may share one account).
            AuthSlot::Admin => self.admin.as_ref().or(self.regular.as_ref()),
        }
    }
}

fn reqwest_method(method: Method) -> reqwest::Method {
    match method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Delete => reqwest::Method::DELETE,
        Method::Options => reqwest::Method::OPTIONS,
    }
}

#[async_trait]
impl Transport for SutClient {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let url = self.resolve_url(&request.path);
        let mut builder = self.http.request(reqwest_method(request.method), &url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(cred) = self.credential(request.auth) {
            builder = match cred {
                Credential::Basic { user, pass } => builder.basic_auth(user, Some(pass)),
                Credential::Bearer(token) => builder.bearer_auth(token),
            };
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder
            .send()
            .await
            .map_err(|e| TransportError::Http(e.to_string()))?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|e| TransportError::Http(e.to_string()))?
            .to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    fn describe(&self) -> String {
        self.base_url.clone()
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
    use super::*;

    #[test]
    fn parses_basic_and_bearer() {
        match Credential::parse("basic:ehrbase:ehrbase").expect("basic") {
            Credential::Basic { user, pass } => {
                assert_eq!(user, "ehrbase");
                assert_eq!(pass, "ehrbase");
            }
            Credential::Bearer(_) => panic!("expected basic"),
        }
        match Credential::parse("bearer:abc.def.ghi").expect("bearer") {
            Credential::Bearer(t) => assert_eq!(t, "abc.def.ghi"),
            Credential::Basic { .. } => panic!("expected bearer"),
        }
        assert!(Credential::parse("digest:x").is_err());
        assert!(Credential::parse("basic:noPassword").is_err());
    }

    #[test]
    fn trims_trailing_slash_in_base_url() {
        let c = SutClient::new("http://host/rest/", None, None).expect("client");
        assert_eq!(c.describe(), "http://host/rest");
    }

    #[test]
    fn default_nests_admin_under_the_base() {
        let c =
            SutClient::new("http://host:8080/ehrbase/rest/openehr/v1", None, None).expect("client");
        assert_eq!(
            c.resolve_url("/admin/ehr/abc"),
            "http://host:8080/ehrbase/rest/openehr/v1/admin/ehr/abc"
        );
        assert_eq!(
            c.resolve_url("/ehr/abc"),
            "http://host:8080/ehrbase/rest/openehr/v1/ehr/abc"
        );
    }

    #[test]
    fn admin_base_url_routes_admin_paths_to_the_sibling_mount() {
        let c = SutClient::new("http://host:8091/ehrbase/rest/openehr/v1", None, None)
            .expect("client")
            .with_admin_base_url(Some("http://host:8091/ehrbase/rest/admin/".to_owned()));
        // `/admin/ehr/abc` → the sibling admin mount, `/admin` prefix stripped.
        assert_eq!(
            c.resolve_url("/admin/ehr/abc"),
            "http://host:8091/ehrbase/rest/admin/ehr/abc"
        );
        assert_eq!(
            c.resolve_url("/admin/ehr/all?ehr_id=a&ehr_id=b"),
            "http://host:8091/ehrbase/rest/admin/ehr/all?ehr_id=a&ehr_id=b"
        );
        // Non-admin paths still nest under the openEHR base.
        assert_eq!(
            c.resolve_url("/ehr/abc"),
            "http://host:8091/ehrbase/rest/openehr/v1/ehr/abc"
        );
    }
}
