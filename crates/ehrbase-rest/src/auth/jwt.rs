//! OAuth2/OIDC bearer-token validation (resource-server role).
//!
//! The CDR validates access tokens presented as `Authorization: Bearer <jwt>`:
//! it verifies the signature against a key source and checks the `iss`/`aud`/
//! `exp` claims via `jsonwebtoken`. Three key sources are supported, in
//! precedence order: a symmetric HMAC secret, a static JWKS document, or a JWKS
//! discovered from the issuer's OIDC metadata (`openidconnect`) and cached.
//!
//! Obtaining tokens (the `OAuth2` authorization-code/client-credentials flows, the
//! `oauth2` crate) is a *client* concern — a CDR only validates — so it is out
//! of scope here (recorded per ADR-006's Stage-1 auth scope).

use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

use super::config::OidcConfig;
use super::{AuthError, AuthMethod, Principal};

/// The subset of JWT claims the CDR reads. `iss`/`aud`/`exp` are validated by
/// `jsonwebtoken` from the raw token; `scope`/`scp` feed the coarse admin gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    #[serde(skip_serializing_if = "Option::is_none")]
    sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<u64>,
    /// `OAuth2` space-delimited scope string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    /// Alternative scope claim used by some `IdPs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scp: Vec<String>,
}

/// A configured bearer-token validator.
pub(super) struct JwtValidator {
    issuer: String,
    audiences: Vec<String>,
    algorithms: Vec<Algorithm>,
    keys: KeySource,
}

enum KeySource {
    /// Symmetric HS256 secret.
    Hmac(DecodingKey),
    /// A fixed JWKS document.
    Static(JwkSet),
    /// A JWKS discovered from the issuer and cached with a short TTL.
    Remote(RemoteJwks),
}

impl JwtValidator {
    /// Build a validator from configuration.
    ///
    /// # Errors
    /// Returns a message when the algorithm list or key material is invalid.
    pub(super) fn from_config(cfg: &OidcConfig) -> Result<Self, String> {
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
            KeySource::Hmac(DecodingKey::from_secret(secret.0.as_bytes()))
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
        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        let mut scopes: Vec<String> = data
            .claims
            .scope
            .as_deref()
            .map(|s| s.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        scopes.extend(data.claims.scp);

        Ok(Principal {
            subject: data.claims.sub.unwrap_or_else(|| "unknown".to_owned()),
            scopes,
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
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};

    const SECRET: &str = "test-signing-secret";
    const ISSUER: &str = "https://issuer.example";

    fn now() -> u64 {
        // Test-only wall clock; jiff is a workspace dep.
        u64::try_from(jiff::Timestamp::now().as_second()).unwrap()
    }

    fn validator(audiences: &[&str]) -> JwtValidator {
        JwtValidator::from_config(&OidcConfig {
            issuer: ISSUER.to_owned(),
            audiences: audiences.iter().map(|s| (*s).to_owned()).collect(),
            algorithms: vec!["HS256".to_owned()],
            hmac_secret: Some(super::super::config::Redacted(SECRET.to_owned())),
            jwks_json: None,
        })
        .expect("validator")
    }

    fn token(claims: &Claims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("encode")
    }

    fn base_claims() -> Claims {
        Claims {
            sub: Some("alice".to_owned()),
            iss: Some(ISSUER.to_owned()),
            aud: None,
            exp: Some(now() + 3600),
            scope: Some("openid profile".to_owned()),
            scp: Vec::new(),
        }
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
    }

    #[tokio::test]
    async fn expired_token_rejected() {
        let mut c = base_claims();
        c.exp = Some(now() - 3600); // beyond jsonwebtoken's default 60s leeway
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn wrong_issuer_rejected() {
        let mut c = base_claims();
        c.iss = Some("https://evil.example".to_owned());
        let err = validator(&[])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn wrong_audience_rejected() {
        let mut c = base_claims();
        c.aud = Some("other-api".to_owned());
        let err = validator(&["my-api"])
            .validate(&token(&c))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn correct_audience_accepted() {
        let mut c = base_claims();
        c.aud = Some("my-api".to_owned());
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
}
