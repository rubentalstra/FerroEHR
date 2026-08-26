// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The blocking SUT clients for the measurement machinery: base URL + auth
//! resolved once per ixit instance; one connection pool per client, shared
//! by every worker thread.
//!
//! A measured run drives ONE principal per [`crate::perf::Principal`] the
//! workload names — the party's default `sut` instance plus, when the ixit
//! declares them, the `unauthenticated` / `readonly` boundary principals and
//! the SMART Platform base. A principal the ixit does not declare is not a
//! runner guess: the journeys that need it are simply not scheduled.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::io::Read;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use base64::Engine;

use crate::ixit::{AuthMode, BearerMint, Instance, Ixit};
use crate::perf::Principal;

/// Per-request client timeout. A response slower than this is an error
/// arrival recorded at the timeout latency (the SLO is 1 s — a 60 s stall
/// is already a hard violation either way).
pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_mins(1);

/// Re-mint a lane token this many milliseconds before it expires, so no
/// arrival can ever present one that lapses in flight.
const MINT_REFRESH_MARGIN_MS: i64 = 30_000;

/// One minted lane token and the instant it stops being presentable.
#[derive(Debug)]
struct MintedToken {
    header: String,
    expires_at_ms: i64,
}

/// The standing SMART grant a `bearer_mint` principal holds for the whole
/// measured window: the ixit lane's static test issuer plus THIS
/// principal's subject/roles/`default_scopes` (ITS-REST
/// `docs/smart_app_launch/master08-scopes.adoc` §Resource Scopes). A
/// measured run declares no per-step scopes — there are no steps — so the
/// standing grant IS the scope claim, minted once and re-minted only when
/// the declared `ttl_seconds` is about to lapse (never per arrival: the
/// instrument measures the CDR, not token signing).
#[derive(Debug)]
struct MintedGrant {
    mint: BearerMint,
    subject: Option<String>,
    roles: Option<Vec<String>>,
    scopes: Vec<String>,
    current: RwLock<Arc<MintedToken>>,
}

/// How a client presents itself: a fixed header (none / Basic / a Bearer
/// token from the environment) or a standing minted SMART grant.
#[derive(Debug)]
enum Credential {
    Fixed(Option<String>),
    // Boxed: the grant carries the whole mint declaration, and a fixed
    // header is the common case.
    Minted(Box<MintedGrant>),
}

/// The blocking SUT client (see the module doc).
#[derive(Clone)]
pub struct PerfClient {
    client: reqwest::blocking::Client,
    base_url: String,
    credential: Arc<Credential>,
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
    pub(crate) status: reqwest::StatusCode,
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
    /// A message when a credential env var is unset, a `bearer_mint`
    /// instance has no declared SMART lane to mint against, or the client
    /// cannot be built.
    #[expect(
        clippy::disallowed_methods,
        reason = "credentials are read from the environment BY DESIGN: the ixit \
                  declares only the variable NAME so no secret ever enters the \
                  catalogue; cnf-runner is a standalone instrument with no access \
                  to the server's config tree, which is what that ban protects"
    )]
    pub fn from_instance(instance: &Instance, ixit: &Ixit) -> Result<Self, String> {
        let credential = match &instance.auth {
            AuthMode::None => Credential::Fixed(None),
            AuthMode::Basic {
                user_env,
                password_env,
            } => {
                let user = std::env::var(user_env)
                    .map_err(|error| format!("credential env {user_env}: {error}"))?;
                let pass = std::env::var(password_env)
                    .map_err(|error| format!("credential env {password_env}: {error}"))?;
                let token = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{pass}").as_bytes());
                Credential::Fixed(Some(format!("Basic {token}")))
            }
            AuthMode::Bearer { token_env } => {
                let token = std::env::var(token_env)
                    .map_err(|error| format!("credential env {token_env}: {error}"))?;
                Credential::Fixed(Some(format!("Bearer {token}")))
            }
            // The SMART resource-server posture IS this product's standard
            // conformance posture, so the measured lane drives it too: the
            // principal's STANDING grant (`default_scopes`) is minted once
            // against the ixit's declared static test issuer and re-minted
            // only near expiry. Every measured arrival therefore rides a
            // scope-limited Bearer token, which is what the SMART scope
            // grammar is enforced against under load.
            AuthMode::BearerMint {
                subject,
                roles,
                default_scopes,
            } => {
                let lane = ixit.smart.as_ref().ok_or_else(|| {
                    "instance declares auth mode `bearer_mint` but the ixit declares no `smart` \
                     lane to mint against"
                        .to_owned()
                })?;
                let grant = MintedGrant {
                    mint: lane.mint.clone(),
                    subject: subject.clone(),
                    roles: roles.clone(),
                    scopes: default_scopes.clone(),
                    current: RwLock::new(Arc::new(MintedToken {
                        header: String::new(),
                        expires_at_ms: i64::MIN,
                    })),
                };
                // Mint eagerly so a credential defect fails the run before
                // any window opens, never as error arrivals inside one.
                let token = grant.freshly_minted()?;
                grant
                    .current
                    .write()
                    .map_err(|error| format!("minted-token lock poisoned: {error}"))?
                    .clone_from(&token);
                Credential::Minted(Box::new(grant))
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
            credential: Arc::new(credential),
            extra_headers: instance.headers.clone().unwrap_or_default(),
        })
    }

    /// The `Authorization` header this arrival presents.
    fn authorization(&self) -> Result<Option<String>, String> {
        match self.credential.as_ref() {
            Credential::Fixed(header) => Ok(header.clone()),
            Credential::Minted(grant) => grant.presentable().map(Some),
        }
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
        self.request_negotiated(method, path, body, prefer_minimal, if_match, None, &[])
    }

    /// The ONE request-construction path: every measured arrival — and
    /// every seeding call — builds its headers here, so an operation can
    /// never go on the wire two ways by code path. `accept` overrides the
    /// exchange's default representation (the Simplified-Format reads);
    /// `extra` carries the operation's own declared headers (e.g.
    /// `openehr-template-id` on a FLAT commit).
    #[expect(
        clippy::too_many_arguments,
        reason = "the single request-construction seam"
    )]
    pub(crate) fn request_negotiated(
        &self,
        method: reqwest::Method,
        path: &str,
        body: RequestBody,
        prefer_minimal: bool,
        if_match: Option<&str>,
        accept: Option<&str>,
        extra: &[(&str, String)],
    ) -> Result<WireReply, String> {
        // Accept follows the exchange's native representation (ITS-REST
        // overview §Requests and responses): canonical JSON everywhere
        // except the ADL 1.4 template surface, whose native form is the
        // OPT XML — a JSON-only Accept there draws a 406 from SUTs that
        // honour strict negotiation on the returned template.
        let accept = accept.unwrap_or(match &body {
            Some((content_type, _)) if content_type.contains("xml") => "application/xml",
            _ => "application/json",
        });
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url))
            .header("Accept", accept);
        if let Some(auth) = self.authorization()? {
            request = request.header("Authorization", auth);
        }
        for (name, value) in extra {
            request = request.header(*name, value);
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
            status: response.status(),
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

impl MintedGrant {
    /// Sign one token for this principal's standing grant.
    fn freshly_minted(&self) -> Result<Arc<MintedToken>, String> {
        let token = crate::exec::driver::mint_access_token(
            &self.mint,
            self.subject.as_deref(),
            self.roles.as_deref(),
            &self.scopes,
        )?;
        let ttl_ms = i64::try_from(self.mint.ttl_seconds.saturating_mul(1_000)).unwrap_or(i64::MAX);
        Ok(Arc::new(MintedToken {
            header: format!("Bearer {token}"),
            expires_at_ms: crate::exec::driver::now_ms().saturating_add(ttl_ms),
        }))
    }

    /// The token to present now, re-minting once when the standing one is
    /// within the refresh margin of its declared expiry.
    fn presentable(&self) -> Result<String, String> {
        let now = crate::exec::driver::now_ms();
        {
            let current = self
                .current
                .read()
                .map_err(|error| format!("minted-token lock poisoned: {error}"))?;
            if now.saturating_add(MINT_REFRESH_MARGIN_MS) < current.expires_at_ms {
                return Ok(current.header.clone());
            }
        }
        let mut current = self
            .current
            .write()
            .map_err(|error| format!("minted-token lock poisoned: {error}"))?;
        // Re-check: another arrival may have re-minted while this one
        // waited for the write lock.
        if now.saturating_add(MINT_REFRESH_MARGIN_MS) < current.expires_at_ms {
            return Ok(current.header.clone());
        }
        let fresh = self.freshly_minted()?;
        let header = fresh.header.clone();
        *current = fresh;
        Ok(header)
    }
}

/// The ixit instance names the measured lane's non-default principals are
/// declared under. They are ixit DECLARATIONS, never runner guesses — a
/// party that declares none runs the workload without the journeys that
/// address them.
const UNAUTHENTICATED_INSTANCE: &str = "unauthenticated";
const READONLY_INSTANCE: &str = "readonly";
const ADMIN_INSTANCE: &str = "admin";

/// Every principal a measured window drives: the party's default `sut`
/// instance plus the optional boundary/platform principals.
#[derive(Debug, Clone)]
pub struct PerfPrincipals {
    primary: PerfClient,
    unauthenticated: Option<PerfClient>,
    readonly: Option<PerfClient>,
    admin: Option<PerfClient>,
    smart_platform: Option<PerfClient>,
}

impl PerfPrincipals {
    /// Resolve every principal the ixit declares. The default instance is
    /// mandatory (there is no measured run without a SUT); each optional
    /// principal is built only when its instance is declared, and a
    /// declared-but-unusable one (an unset credential env) fails loudly
    /// rather than silently degrading the workload.
    ///
    /// # Errors
    /// A message from the default instance's resolution, or from a declared
    /// optional instance that cannot be built.
    pub fn from_ixit(ixit: &Ixit) -> Result<Self, String> {
        let primary = PerfClient::from_instance(ixit.default_instance()?, ixit)?;
        let optional = |name: &crate::ids::InstanceName| -> Result<Option<PerfClient>, String> {
            match ixit.instance(name) {
                None => Ok(None),
                Some(instance) => PerfClient::from_instance(instance, ixit)
                    .map(Some)
                    .map_err(|e| format!("ixit instance {name}: {e}")),
            }
        };
        let named = |token: &str| -> Result<Option<PerfClient>, String> {
            match crate::ids::InstanceName::parse(token) {
                Ok(name) => optional(&name),
                Err(e) => Err(e.to_string()),
            }
        };
        let smart_platform = match ixit.smart.as_ref() {
            None => None,
            Some(lane) => optional(&lane.platform_instance)?,
        };
        Ok(Self {
            primary,
            unauthenticated: named(UNAUTHENTICATED_INSTANCE)?,
            readonly: named(READONLY_INSTANCE)?,
            admin: named(ADMIN_INSTANCE)?,
            smart_platform,
        })
    }

    /// A single-principal set (seeding, the offline harnesses).
    #[must_use]
    pub fn single(primary: PerfClient) -> Self {
        Self {
            primary,
            unauthenticated: None,
            readonly: None,
            admin: None,
            smart_platform: None,
        }
    }

    /// Add the credential-less boundary principal.
    #[must_use]
    pub fn with_unauthenticated(mut self, client: PerfClient) -> Self {
        self.unauthenticated = Some(client);
        self
    }

    /// Add the read-only boundary principal.
    #[must_use]
    pub fn with_readonly(mut self, client: PerfClient) -> Self {
        self.readonly = Some(client);
        self
    }

    /// Add the ADMIN-role principal.
    #[must_use]
    pub fn with_admin(mut self, client: PerfClient) -> Self {
        self.admin = Some(client);
        self
    }

    /// Add the SMART Platform-base principal.
    #[must_use]
    pub fn with_smart_platform(mut self, client: PerfClient) -> Self {
        self.smart_platform = Some(client);
        self
    }

    /// The default `sut` principal — the one every provisioning call and
    /// every ordinary journey stage drives.
    #[must_use]
    pub fn primary(&self) -> &PerfClient {
        &self.primary
    }

    /// The client for one principal, or `None` when the party's ixit does
    /// not declare its instance.
    #[must_use]
    pub fn client(&self, principal: Principal) -> Option<&PerfClient> {
        match principal {
            Principal::Primary => Some(&self.primary),
            Principal::Unauthenticated => self.unauthenticated.as_ref(),
            Principal::ReadOnly => self.readonly.as_ref(),
            Principal::SmartPlatform => self.smart_platform.as_ref(),
            Principal::Admin => self.admin.as_ref(),
        }
    }

    /// Whether the party declares the instance a principal needs.
    #[must_use]
    pub fn declares(&self, principal: Principal) -> bool {
        self.client(principal).is_some()
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
mod tests {
    use super::*;

    /// The SMART posture's standing grant is minted ONCE and cached: a
    /// measured run must never pay token-signing cost per arrival, and every
    /// arrival must present a scope-limited Bearer token.
    #[test]
    fn a_bearer_mint_principal_presents_one_cached_standing_grant() {
        let key = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("party/smart/cnf-smart-test.key.pem");
        assert!(key.is_file(), "committed test issuer key is missing");
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://stub", "auth": {
                "mode": "bearer_mint", "subject": "cnf-user", "roles": ["USER"],
                "default_scopes": ["user/aql-*.r"] } } },
            "smart": { "platform_instance": "sut", "mint": {
                "issuer": "https://as.cnf.test", "subject": "cnf-smart-app",
                "roles": ["USER"], "key_file": key, "kid": "cnf-smart-test",
                "ttl_seconds": 3600 } }
        }))
        .unwrap();
        let client = PerfClient::from_instance(ixit.default_instance().unwrap(), &ixit).unwrap();
        let first = client.authorization().unwrap().unwrap();
        let second = client.authorization().unwrap().unwrap();
        assert!(first.starts_with("Bearer "));
        assert_eq!(first.split('.').count(), 3, "not a JWS compact token");
        assert_eq!(first, second, "the standing grant was re-minted per call");
    }

    /// A `bearer_mint` principal with no declared SMART lane is an authoring
    /// defect the run refuses before any window opens.
    #[test]
    fn a_bearer_mint_principal_without_a_lane_is_refused() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://stub",
                                     "auth": { "mode": "bearer_mint" } } }
        }))
        .unwrap();
        let error = PerfClient::from_instance(ixit.default_instance().unwrap(), &ixit).unwrap_err();
        assert!(error.contains("no `smart` lane"), "{error}");
    }

    /// Only the principals the ixit declares are available; the rest cost
    /// coverage, never correctness.
    #[test]
    fn principals_resolve_from_the_ixit_declarations_only() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://stub", "auth": { "mode": "none" } },
                "unauthenticated": { "base_url": "http://stub", "auth": { "mode": "none" } }
            }
        }))
        .unwrap();
        let principals = PerfPrincipals::from_ixit(&ixit).unwrap();
        assert!(principals.declares(Principal::Primary));
        assert!(principals.declares(Principal::Unauthenticated));
        assert!(!principals.declares(Principal::ReadOnly));
        assert!(!principals.declares(Principal::SmartPlatform));
        assert!(principals.client(Principal::ReadOnly).is_none());
    }

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
