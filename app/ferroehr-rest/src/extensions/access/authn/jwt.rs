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
// implementation (§5.1).
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
    /// Build a validator with explicit RBAC role-claim paths (§5.1 —
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
            KeySource::Remote(RemoteJwks::new(cfg.issuer.clone()))
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

        let subject = claims
            .get("sub")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
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
        // `scope`), normalized to upper-case (§5.1 / v1 authority converter).
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

/// A JWKS fetched from an issuer's OIDC discovery document, cached briefly.
struct RemoteJwks {
    issuer: String,
    cache: moka::future::Cache<&'static str, Arc<JwkSet>>,
}

impl RemoteJwks {
    fn new(issuer: String) -> Self {
        Self {
            issuer,
            cache: moka::future::Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_mins(5))
                .build(),
        }
    }

    async fn jwks(&self) -> Result<Arc<JwkSet>, AuthError> {
        self.cache
            .try_get_with("jwks", Self::fetch(self.issuer.clone()))
            .await
            .map_err(|e: Arc<AuthError>| (*e).clone())
    }

    async fn fetch(issuer: String) -> Result<Arc<JwkSet>, AuthError> {
        use openidconnect::core::CoreProviderMetadata;
        use openidconnect::{IssuerUrl, reqwest};

        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none()) // SSRF hardening
            .build()
            .map_err(|e| AuthError::KeyResolution(format!("HTTP client: {e}")))?;
        let issuer_url = IssuerUrl::new(issuer)
            .map_err(|e| AuthError::KeyResolution(format!("issuer URL: {e}")))?;
        let metadata = CoreProviderMetadata::discover_async(issuer_url, &http)
            .await
            .map_err(|e| AuthError::KeyResolution(format!("OIDC discovery: {e}")))?;
        let jwks_uri = metadata.jwks_uri().url().clone();
        let body = http
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
        // The validated claim set is retained on the Principal (§5.1).
        assert_eq!(p.claims.get("sub").and_then(Value::as_str), Some("alice"));
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
        // upper-cased; the `scope` claim also contributes roles (§5.1 / §9.2).
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
}
