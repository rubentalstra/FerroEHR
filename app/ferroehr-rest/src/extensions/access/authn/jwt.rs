//! OAuth2/OIDC bearer-token validation (resource-server role).
//!
//! The CDR validates access tokens presented as `Authorization: Bearer <jwt>`:
//! it verifies the signature against a key source and checks the `iss`/`aud`/
//! `exp` claims via `jsonwebtoken`. Three key sources are supported, in
//! precedence order: a symmetric HMAC secret, a static JWKS document, or a JWKS
//! discovered from the issuer's OIDC metadata (`openidconnect`) and cached.
//!
//! This is the SMART **resource-server** duty (the `org.openehr.rest` CDR):
//! validate a presented access token, never issue one. Obtaining tokens (the
//! authorization-code / client-credentials / JWT-bearer grants) is an
//! Authorization-Server/client concern — out of scope for a CDR
//! (`docs/specs/openehr/ITS-REST/docs/smart_app_launch/master06-authentication.adoc`
//! §Supported Authentication Flows; the deprecated Implicit and
//! Resource-Owner-Password grants that a CDR must never advertise are rejected
//! by `ferroehr::config::smart::SmartConfig::validate`).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 7): RFC 7519 leaves the claim set open; \
              decided-on claims lift into typed fields"
)]

use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};

use super::{AuthError, AuthMethod, Principal};
use ferroehr::config::auth::OidcConfig;

/// A configured bearer-token validator.
pub(super) struct JwtValidator {
    issuer: String,
    audiences: Vec<String>,
    algorithms: Vec<Algorithm>,
    keys: KeySource,
    /// Dotted JWT claim paths mined for RBAC roles (default
    /// `["realm_access.roles", "scope"]`); configurable via `authz.rbac.role_claims`.
    role_claims: Vec<String>,
}

// The role model (default claim paths + extraction algorithm) lives in the leaf
// `access::authz` module so the REST layer and the RBAC gate share one
// implementation.
use crate::extensions::access::authz::roles::extract_roles;

enum KeySource {
    /// Symmetric HS256 secret.
    Hmac(DecodingKey),
    /// A fixed JWKS document.
    Static(JwkSet),
    /// A JWKS discovered from the issuer and cached with a short TTL.
    Remote(RemoteJwks),
}

impl JwtValidator {
    /// Build a validator with explicit RBAC role-claim paths (from
    /// `authz.rbac.role_claims`).
    ///
    /// # Errors
    /// Returns a message when the algorithm list or key material is invalid.
    pub(super) fn with_role_claims(
        cfg: &OidcConfig,
        role_claims: Vec<String>,
    ) -> Result<Self, String> {
        #[expect(
            clippy::map_err_ignore,
            reason = "`jsonwebtoken`'s algorithm parse error carries no detail beyond \
                      \"unrecognised name\", and the message already echoes the name"
        )]
        let algorithms = cfg
            .algorithms
            .iter()
            .map(|a| {
                a.parse::<Algorithm>()
                    .map_err(|_| format!("unknown JWT algorithm: {a}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if algorithms.is_empty() {
            return Err("oidc.algorithms must not be empty".to_owned());
        }

        let keys = if let Some(secret) = &cfg.hmac_secret {
            KeySource::Hmac(DecodingKey::from_secret(secret.expose().as_bytes()))
        } else if let Some(jwks_json) = &cfg.jwks_json {
            let set: JwkSet = serde_json::from_str(jwks_json)
                .map_err(|e| format!("invalid oidc.jwks_json: {e}"))?;
            KeySource::Static(set)
        } else {
            KeySource::Remote(RemoteJwks::new(cfg)?)
        };

        Ok(Self {
            issuer: cfg.issuer.clone(),
            audiences: cfg.audiences.clone(),
            algorithms,
            keys,
            role_claims,
        })
    }

    /// Validate a raw bearer token (the value after `Bearer `).
    ///
    /// # Errors
    /// [`AuthError::InvalidToken`] for any signature/claim/format failure.
    pub(super) async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        let header = decode_header(token).map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        if !self.algorithms.contains(&header.alg) {
            return Err(AuthError::InvalidToken(format!(
                "token algorithm {:?} not accepted",
                header.alg
            )));
        }

        let mut validation = Validation::new(header.alg);
        validation.algorithms = self.algorithms.clone();
        validation.set_issuer(&[&self.issuer]);
        if self.audiences.is_empty() {
            validation.validate_aud = false;
        } else {
            validation.set_audience(&self.audiences);
        }

        let key = self.decoding_key(header.kid.as_deref()).await?;
        // Decode into the full claim map so the validated claim set is retained
        // for the RBAC role extraction and the Stage-2 ABAC attribute resolution;
        // `jsonwebtoken` still validates `exp`/`iss`/`aud` from the raw payload
        // independent of the deserialize target.
        let data = decode::<serde_json::Map<String, serde_json::Value>>(token, &key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let claims = data.claims;

        // RFC 7519 §4.1.2 makes `sub` optional, but this principal is stamped
        // into the ATNA audit trail — an unattributable caller is refused, never
        // recorded under a fabricated identity.
        let subject = claims
            .get("sub")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AuthError::InvalidToken(
                    "token carries no usable `sub` claim; an audit-attributable subject is \
                     required"
                        .to_owned(),
                )
            })?
            .to_owned();

        // Scopes: the space-delimited `scope` string plus the `scp` array,
        // preserved verbatim on the principal for ABAC/diagnostic consumers.
        let mut scopes: Vec<String> = claims
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .map(|s| s.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        if let Some(scp) = claims.get("scp").and_then(serde_json::Value::as_array) {
            scopes.extend(scp.iter().filter_map(|v| v.as_str().map(str::to_owned)));
        }

        // Roles from the configured claim paths (default `realm_access.roles` +
        // `scope`), normalized to upper-case (mirrors EHRbase v1's authority converter).
        let roles = extract_roles(&claims, &self.role_claims);

        Ok(Principal {
            subject,
            scopes,
            roles,
            claims,
            method: AuthMethod::Bearer,
        })
    }

    async fn decoding_key(&self, kid: Option<&str>) -> Result<DecodingKey, AuthError> {
        match &self.keys {
            KeySource::Hmac(key) => Ok(key.clone()),
            KeySource::Static(set) => jwk_to_key(set, kid),
            KeySource::Remote(remote) => {
                let set = remote.jwks().await?;
                jwk_to_key(&set, kid)
            }
        }
    }
}

fn jwk_to_key(set: &JwkSet, kid: Option<&str>) -> Result<DecodingKey, AuthError> {
    let jwk = match kid {
        Some(kid) => set.find(kid),
        // No `kid`: only unambiguous when the set has exactly one key.
        None => set.keys.first(),
    }
    .ok_or_else(|| AuthError::InvalidToken("no matching signing key for token".to_owned()))?;
    DecodingKey::from_jwk(jwk).map_err(|e| AuthError::InvalidToken(format!("invalid JWK: {e}")))
}

/// How long successfully fetched key material stays usable without a refetch.
const JWKS_TTL: Duration = Duration::from_mins(5);

/// The single cache slot: one issuer per validator, so the key is a constant.
const CACHE_KEY: &str = "jwks";

/// A JWKS fetched from an issuer's OIDC discovery document, cached briefly.
///
/// Both outcomes are cached, with different lifetimes ([`JwksExpiry`]): success
/// for [`JWKS_TTL`], failure for `oidc.negative_cache_ttl_seconds`. Caching the
/// failure is what keeps an issuer outage from turning every bearer request into
/// a fresh discovery attempt. No openEHR spec governs this — our own design.
struct RemoteJwks {
    issuer: String,
    /// Built once, so the timeouts are a configuration-time property and the
    /// connection pool survives across fetches.
    http: openidconnect::reqwest::Client,
    cache: moka::future::Cache<&'static str, Result<Arc<JwkSet>, AuthError>>,
}

/// Per-entry lifetimes for the [`RemoteJwks`] cache.
struct JwksExpiry {
    negative_ttl: Duration,
}

impl moka::Expiry<&'static str, Result<Arc<JwkSet>, AuthError>> for JwksExpiry {
    fn expire_after_create(
        &self,
        _key: &&'static str,
        value: &Result<Arc<JwkSet>, AuthError>,
        _created_at: std::time::Instant,
    ) -> Option<Duration> {
        match value {
            Ok(_) => Some(JWKS_TTL),
            // A zero negative TTL expires the entry on the next read, which is
            // the documented "negative caching off" setting.
            Err(_) => Some(self.negative_ttl),
        }
    }
}

impl RemoteJwks {
    /// Builds the discovery client and its cache from the `[auth.oidc]` section.
    ///
    /// # Errors
    /// Returns a message when `reqwest` cannot build a client with the
    /// configured timeouts.
    fn new(cfg: &OidcConfig) -> Result<Self, String> {
        let http = openidconnect::reqwest::ClientBuilder::new()
            // SSRF hardening: an issuer must not redirect us anywhere.
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_millis(cfg.connect_timeout_ms))
            .timeout(Duration::from_millis(cfg.request_timeout_ms))
            .build()
            .map_err(|e| format!("oidc discovery HTTP client: {e}"))?;
        let expiry = JwksExpiry {
            negative_ttl: Duration::from_secs(cfg.negative_cache_ttl_seconds),
        };
        Ok(Self {
            issuer: cfg.issuer.clone(),
            http,
            cache: moka::future::Cache::builder()
                .max_capacity(1)
                .expire_after(expiry)
                .build(),
        })
    }

    async fn jwks(&self) -> Result<Arc<JwkSet>, AuthError> {
        // `get_with` coalesces concurrent misses onto one fetch AND stores the
        // outcome whichever way it went; `try_get_with` would discard the error.
        self.cache.get_with(CACHE_KEY, self.fetch()).await
    }

    async fn fetch(&self) -> Result<Arc<JwkSet>, AuthError> {
        let issuer_url = openidconnect::IssuerUrl::new(self.issuer.clone())
            .map_err(|e| AuthError::KeyResolution(format!("issuer URL: {e}")))?;
        let metadata =
            openidconnect::core::CoreProviderMetadata::discover_async(issuer_url, &self.http)
                .await
                .map_err(|e| AuthError::KeyResolution(format!("OIDC discovery: {e}")))?;
        let jwks_uri = metadata.jwks_uri().url().clone();
        let body = self
            .http
            .get(jwks_uri)
            .send()
            .await
            .map_err(|e| AuthError::KeyResolution(format!("JWKS fetch: {e}")))?
            .text()
            .await
            .map_err(|e| AuthError::KeyResolution(format!("JWKS body: {e}")))?;
        let set: JwkSet = serde_json::from_str(&body)
            .map_err(|e| AuthError::KeyResolution(format!("JWKS parse: {e}")))?;
        Ok(Arc::new(set))
    }
}

#[cfg(test)]
mod tests {
    use crate::extensions::access::authz::roles::default_role_claims;
    use jsonwebtoken::{EncodingKey, Header, encode};

    use super::*;
    use serde_json::{Value, json};

    const SECRET: &str = "test-signing-secret";
    const ISSUER: &str = "https://issuer.example";

    fn now() -> u64 {
        // Test-only wall clock; jiff is a workspace dep.
        u64::try_from(jiff::Timestamp::now().as_second()).unwrap()
    }

    fn validator(audiences: &[&str]) -> JwtValidator {
        JwtValidator::with_role_claims(
            &OidcConfig {
                issuer: ISSUER.to_owned(),
                audiences: audiences.iter().map(|s| (*s).to_owned()).collect(),
                algorithms: vec!["HS256".to_owned()],
                hmac_secret: Some(ferroehr::config::secret::Secret::new(SECRET.to_owned())),
                jwks_json: None,
                ..OidcConfig::default()
            },
            default_role_claims(),
        )
        .expect("validator")
    }

    fn token(claims: &Value) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("encode")
    }

    fn base_claims() -> Value {
        json!({
            "sub": "alice",
            "iss": ISSUER,
            "exp": now() + 3600,
            "scope": "openid profile",
        })
    }

    #[tokio::test]
    async fn valid_token_authenticates() {
        let p = validator(&[])
            .validate(&token(&base_claims()))
            .await
            .expect("ok");
        assert_eq!(p.subject, "alice");
        assert_eq!(p.method, AuthMethod::Bearer);
        assert!(p.scopes.contains(&"openid".to_owned()));
        // The validated claim set is retained on the Principal.
        assert_eq!(p.claims.get("sub").and_then(Value::as_str), Some("alice"));
    }

    #[tokio::test]
    async fn token_without_sub_rejected() {
        let mut c = base_claims();
        c.as_object_mut().expect("object").remove("sub");
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(
            matches!(&err, AuthError::InvalidToken(m) if m.contains("sub")),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn token_with_blank_sub_rejected() {
        let mut c = base_claims();
        c["sub"] = json!("   ");
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn expired_token_rejected() {
        let mut c = base_claims();
        c["exp"] = json!(now() - 3600); // beyond jsonwebtoken's default 60s leeway
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn wrong_issuer_rejected() {
        let mut c = base_claims();
        c["iss"] = json!("https://evil.example");
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn wrong_audience_rejected() {
        let mut c = base_claims();
        c["aud"] = json!("other-api");
        let err = validator(&["my-api"])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn correct_audience_accepted() {
        let mut c = base_claims();
        c["aud"] = json!("my-api");
        let p = validator(&["my-api"])
            .validate(&token(&c))
            .await
            .expect("ok");
        assert_eq!(p.subject, "alice");
    }

    #[tokio::test]
    async fn tampered_signature_rejected() {
        // Tamper with a char in the *middle* of the base64url signature, not
        // the last one: the final char of a 43-char HS256 signature encodes
        // only 4 meaningful bits, so an 'A'↔'B' flip there lands in the
        // ignored trailing bits ~1/16 of the time (decodes to the identical
        // signature → flaky false-accept).
        let mut t = token(&base_claims());
        let flip = t.len() - 10;
        let tampered = if t.as_bytes()[flip] == b'A' { 'B' } else { 'A' };
        t.replace_range(flip..=flip, &tampered.to_string());
        let err = validator(&[]).validate(&t).await.expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn keycloak_realm_access_roles_extracted() {
        // Keycloak-shaped token: roles live under `realm_access.roles` and are
        // upper-cased; the `scope` claim also contributes roles.
        let mut c = base_claims();
        c["realm_access"] = json!({ "roles": ["user", "ferroehr-admin"] });
        c["scope"] = json!("openid EHR_READ");
        let p = validator(&[]).validate(&token(&c)).await.expect("ok");
        assert!(p.roles.contains(&"USER".to_owned()));
        assert!(p.roles.contains(&"FERROEHR-ADMIN".to_owned()));
        // Scope tokens are mined into roles too (upper-cased).
        assert!(p.roles.contains(&"OPENID".to_owned()));
        assert!(p.roles.contains(&"EHR_READ".to_owned()));
        // Scopes stay verbatim (not upper-cased) for the deprecated seam.
        assert!(p.scopes.contains(&"openid".to_owned()));
    }

    #[test]
    fn extract_roles_upper_cases_and_dedups() {
        let claims = json!({
            "realm_access": { "roles": ["admin", "Admin", "user"] },
            "scope": "user OFFLINE_ACCESS",
        })
        .as_object()
        .cloned()
        .unwrap();
        let roles = extract_roles(&claims, &default_role_claims());
        assert_eq!(
            roles,
            vec![
                "ADMIN".to_owned(),
                "USER".to_owned(),
                "OFFLINE_ACCESS".to_owned()
            ]
        );
    }

    #[test]
    fn extract_roles_missing_paths_yields_empty() {
        let claims = json!({ "sub": "x" }).as_object().cloned().unwrap();
        assert!(extract_roles(&claims, &default_role_claims()).is_empty());
    }

    #[test]
    fn extract_roles_custom_nested_path() {
        let claims = json!({ "resource_access": { "client": { "roles": ["writer"] } } })
            .as_object()
            .cloned()
            .unwrap();
        let roles = extract_roles(&claims, &["resource_access.client.roles".to_owned()]);
        assert_eq!(roles, vec!["WRITER".to_owned()]);
    }

    /// The remote (OIDC-discovery) key source: timeout budget + negative
    /// caching. No openEHR spec governs auth transport hardening — our own
    /// design.
    mod remote_discovery {
        use std::time::Instant;

        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::*;

        const DISCOVERY_PATH: &str = "/.well-known/openid-configuration";
        const JWKS_PATH: &str = "/jwks";

        /// A discovery-key-source config: no HMAC secret, no static JWKS.
        fn oidc(
            issuer: &str,
            negative_cache_ttl_seconds: u64,
            request_timeout_ms: u64,
        ) -> OidcConfig {
            OidcConfig {
                issuer: issuer.to_owned(),
                algorithms: vec!["HS256".to_owned()],
                request_timeout_ms,
                negative_cache_ttl_seconds,
                ..OidcConfig::default()
            }
        }

        /// The minimum OIDC provider metadata `CoreProviderMetadata` accepts;
        /// `issuer` must echo the requested issuer exactly.
        fn metadata(issuer: &str) -> Value {
            json!({
                "issuer": issuer,
                "authorization_endpoint": format!("{issuer}/authorize"),
                "token_endpoint": format!("{issuer}/token"),
                "jwks_uri": format!("{issuer}{JWKS_PATH}"),
                "response_types_supported": ["code"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["HS256"],
            })
        }

        /// A JWKS carrying the same HS256 secret [`token`] signs with, so a
        /// recovered issuer can actually validate a token.
        fn jwks_body() -> Value {
            let jwk = jsonwebtoken::jwk::Jwk::from_encoding_key(
                &EncodingKey::from_secret(SECRET.as_bytes()),
                Algorithm::HS256,
            )
            .expect("jwk from secret");
            json!({ "keys": [jwk] })
        }

        async fn mount_healthy(server: &MockServer) {
            Mock::given(method("GET"))
                .and(path(DISCOVERY_PATH))
                .respond_with(ResponseTemplate::new(200).set_body_json(metadata(&server.uri())))
                .mount(server)
                .await;
            Mock::given(method("GET"))
                .and(path(JWKS_PATH))
                .respond_with(ResponseTemplate::new(200).set_body_json(jwks_body()))
                .mount(server)
                .await;
        }

        #[tokio::test]
        async fn stalled_issuer_fails_within_the_configured_request_timeout() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(DISCOVERY_PATH))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(metadata(&server.uri()))
                        .set_delay(Duration::from_secs(30)),
                )
                .mount(&server)
                .await;

            let remote = RemoteJwks::new(&oidc(&server.uri(), 10, 300)).expect("client");
            let started = Instant::now();
            let err = remote.jwks().await.expect_err("the timeout must fire");
            let elapsed = started.elapsed();

            assert!(matches!(err, AuthError::KeyResolution(_)), "got {err:?}");
            // The 300 ms budget, generously slacked; without it the request
            // would park for the mock's 30 s (or, on a blackhole, the OS TCP
            // timeout — 75 s+).
            assert!(
                elapsed < Duration::from_secs(5),
                "discovery took {elapsed:?}: the request timeout did not fire"
            );
        }

        #[tokio::test]
        async fn failed_fetch_is_cached_so_no_second_request_reaches_the_issuer() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(DISCOVERY_PATH))
                .respond_with(ResponseTemplate::new(500))
                .expect(1)
                .mount(&server)
                .await;

            let remote = RemoteJwks::new(&oidc(&server.uri(), 60, 5_000)).expect("client");
            assert!(remote.jwks().await.is_err(), "issuer is down");
            assert!(
                remote.jwks().await.is_err(),
                "still down, answered from cache"
            );

            // The mock's `.expect(1)` is verified when the server drops; the
            // explicit count names the failure if a second attempt goes out.
            let attempts = server.received_requests().await.expect("recorded requests");
            assert_eq!(
                attempts.len(),
                1,
                "the negative cache did not suppress the second discovery attempt"
            );
        }

        #[tokio::test]
        async fn negative_entry_expires_and_a_recovered_issuer_validates() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(DISCOVERY_PATH))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;

            let cfg = oidc(&server.uri(), 1, 5_000);
            let validator =
                JwtValidator::with_role_claims(&cfg, default_role_claims()).expect("validator");
            let mut claims = base_claims();
            claims["iss"] = json!(server.uri());
            let token = token(&claims);

            let err = validator
                .validate(&token)
                .await
                .expect_err("issuer is down");
            assert!(matches!(err, AuthError::KeyResolution(_)), "got {err:?}");

            server.reset().await;
            mount_healthy(&server).await;

            // Still inside the 1 s negative TTL: refused from cache, no wire I/O.
            assert!(validator.validate(&token).await.is_err());
            let during_ttl = server.received_requests().await.expect("recorded requests");
            assert!(
                during_ttl.is_empty(),
                "a request escaped during the negative TTL: {during_ttl:?}"
            );

            tokio::time::sleep(Duration::from_millis(1_200)).await;
            let principal = validator
                .validate(&token)
                .await
                .expect("the recovered issuer must validate");
            assert_eq!(principal.subject, "alice");
        }
    }
}
