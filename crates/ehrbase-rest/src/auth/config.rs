//! Authentication configuration (Stage 1: Basic + OAuth2/OIDC bearer).
//!
//! Loaded via `figment` alongside the rest of [`crate::config::RestConfig`].
//! Fine-grained RBAC is Stage 2 (ADR-006); the only authorization here is the
//! optional coarse [`AuthConfig::admin_scope`] gate — a seam, off by default.

use serde::{Deserialize, Serialize};

/// A string that never reveals itself in `Debug` output (secrets in config).
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Redacted(pub String);

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"***\"")
    }
}

/// Top-level authentication settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Master switch. When `false`, all requests pass unauthenticated (dev only).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Basic-auth user store (username → Argon2 PHC hash). Absent → Basic disabled.
    #[serde(default)]
    pub basic: Option<BasicConfig>,
    /// OAuth2/OIDC bearer validation. Absent → bearer disabled.
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
    /// When set, requests to `/admin/*` must carry this `OAuth2` scope, else 403.
    /// Stage-1 seam for Stage-2 RBAC; unset by default (authentication-only).
    // TODO(port): Stage 2 RBAC — replace this coarse scope gate with real authz.
    #[serde(default)]
    pub admin_scope: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            basic: None,
            oidc: None,
            admin_scope: None,
        }
    }
}

impl AuthConfig {
    /// Whether at least one authentication mechanism is configured.
    #[must_use]
    pub fn has_mechanism(&self) -> bool {
        self.basic.is_some() || self.oidc.is_some()
    }
}

/// Basic-auth user store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicConfig {
    /// The configured users. Passwords are stored only as Argon2 PHC hashes.
    #[serde(default)]
    pub users: Vec<BasicUser>,
}

/// One Basic-auth principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicUser {
    pub username: String,
    /// Argon2 PHC hash string (`$argon2id$v=19$...`).
    pub password_hash: Redacted,
}

/// OAuth2/OIDC bearer configuration. Validation happens as a resource server:
/// the token's signature is checked against a key source and the `iss`/`aud`
/// claims validated. The authorization-code client flow (the `oauth2` crate) is
/// a client concern, not a CDR's, so it is out of scope here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Expected token issuer (`iss`). Also the OIDC discovery base when no
    /// static key material is supplied.
    pub issuer: String,
    /// Accepted audiences (`aud`). Empty → audience not checked.
    #[serde(default)]
    pub audiences: Vec<String>,
    /// Accepted signature algorithms (e.g. `["RS256"]`). Defaults to `RS256`.
    #[serde(default = "default_algorithms")]
    pub algorithms: Vec<String>,
    /// A symmetric HS256 secret — the simplest key source (tests/dev).
    #[serde(default)]
    pub hmac_secret: Option<Redacted>,
    /// A static JWKS document (JSON). Preferred over discovery when present.
    #[serde(default)]
    pub jwks_json: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_algorithms() -> Vec<String> {
    vec!["RS256".to_owned()]
}
