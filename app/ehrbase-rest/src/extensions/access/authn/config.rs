//! Authentication configuration (Stage 1: Basic + OAuth2/OIDC bearer).
//!
//! Loaded via `figment` alongside the rest of [`crate::config::RestConfig`].
//! The coarse RBAC gate is configured separately (`crate::extensions::access::authz::AuthzConfig`);
//! [`AuthConfig::admin_scope`] here is a deprecated back-compat alias.

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
    /// Verified Basic-credential cache TTL in seconds (`0` disables the cache).
    /// Argon2 verification costs tens of milliseconds of CPU **per call by
    /// design**; re-running it on every request of a busy client turns the
    /// KDF's work factor into a self-inflicted throughput ceiling. A credential
    /// that has verified successfully is therefore remembered — as a SHA-256
    /// digest of the presented header, never plaintext — and re-verified only
    /// after the TTL (which bounds credential-revocation lag exactly like a
    /// session lifetime). No openEHR spec governs authentication mechanics
    /// (ITS-REST leaves the scheme open) — our own design.
    #[serde(default = "default_verified_cache_ttl")]
    pub verified_cache_ttl_seconds: u64,
    /// OAuth2/OIDC bearer validation. Absent → bearer disabled.
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
    /// **Deprecated alias**, retained for back-compat: a configured scope name
    /// surfaces as the identically-named (upper-cased) role via the scope→role
    /// extraction, so the RBAC `admin_role` gate subsumes it (a scope `ADMIN`
    /// becomes role `ADMIN`). Still consulted by the management surface's
    /// `AdminOnly` access level. Unset by default.
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
            verified_cache_ttl_seconds: default_verified_cache_ttl(),
        }
    }
}

/// 60 s: long enough that a busy client pays the KDF once a minute instead of
/// per request, short enough that a revoked credential dies within a minute.
fn default_verified_cache_ttl() -> u64 {
    60
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
    /// Roles granted to this user (normalized to upper-case when authenticated),
    /// feeding the RBAC gate (§5.2 of `docs/enterprise/access-control.md`).
    /// Defaults to `["USER"]` — the baseline clinical role — when unspecified;
    /// configure `["ADMIN"]` for an administrative account.
    #[serde(default = "default_basic_roles")]
    pub roles: Vec<String>,
}

fn default_basic_roles() -> Vec<String> {
    vec!["USER".to_owned()]
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
