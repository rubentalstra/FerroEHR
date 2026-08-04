//! Authentication configuration (Stage 1: Basic + OAuth2/OIDC bearer).
//!
//! No openEHR spec governs authentication mechanics (ITS-REST leaves the scheme
//! open) — our own design. This is the `[auth]` section of the one server
//! configuration tree; it carries **no
//! loader of its own** — the whole tree is assembled once by `ferroehr::config`
//! and this struct is deserialized as a field of it.
//!
//! Secrets use the shared [`crate::config::secret::Secret`] newtype (P-6): a password hash
//! or HMAC secret deserializes from a plain string but never renders itself
//! (`Debug`, `/management/env`, `ferroehr config check` all show `***`). Each
//! secret key has a `*_file` sibling for file-based indirection
//! (Kubernetes/Docker secrets), resolved by the loader.

use std::path::PathBuf;

use crate::config::secret::Secret;
use serde::{Deserialize, Serialize};

/// Top-level authentication settings (`[auth]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    /// Master switch. When `false`, all requests pass unauthenticated (dev only).
    pub enabled: bool,
    /// Basic-auth user store (username → Argon2 PHC hash). Absent → Basic disabled.
    pub basic: Option<BasicConfig>,
    /// Verified Basic-credential cache TTL in seconds (`0` disables the cache).
    /// Argon2 verification costs tens of milliseconds of CPU **per call by
    /// design**; re-running it on every request of a busy client turns the
    /// KDF's work factor into a self-inflicted throughput ceiling. A credential
    /// that has verified successfully is therefore remembered — as a SHA-256
    /// digest of the presented header, never plaintext — and re-verified only
    /// after the TTL (which bounds credential-revocation lag exactly like a
    /// session lifetime).
    pub verified_cache_ttl_seconds: u64,
    /// OAuth2/OIDC bearer validation. Absent → bearer disabled.
    pub oidc: Option<OidcConfig>,
    /// **Deprecated back-compat alias**: a configured scope name surfaces as the
    /// identically-named (upper-cased) role via scope→role extraction, so the
    /// RBAC `admin_role` gate subsumes it. Still consulted by the management
    /// surface's `AdminOnly` access level. Unset by default.
    // NOTE: the configuration design retires `admin_scope`; it is
    // kept for one transition while the management AdminOnly gate still reads it
    // — retiring it fully is a follow-up that rewires that gate to the RBAC
    // admin role. No openEHR spec governs authorization (SM places it out of
    // band) — our own design.
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BasicConfig {
    /// The configured users. Passwords are stored only as Argon2 PHC hashes.
    pub users: Vec<BasicUser>,
}

/// One Basic-auth principal (`[[auth.basic.users]]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasicUser {
    /// The login name presented in the `Authorization: Basic` header.
    pub username: String,
    /// Argon2 PHC hash string (`$argon2id$v=19$...`). Never a plaintext password.
    pub password_hash: Secret,
    /// Roles granted to this user (normalized to upper-case when authenticated),
    /// feeding the RBAC gate. Defaults to `["USER"]` — the baseline clinical
    /// role — when unspecified; configure `["ADMIN"]` for an administrative
    /// account.
    #[serde(default = "default_basic_roles")]
    pub roles: Vec<String>,
}

fn default_basic_roles() -> Vec<String> {
    vec!["USER".to_owned()]
}

/// OAuth2/OIDC bearer configuration.
///
/// Validation happens as a resource server: the token's signature is checked
/// against a key source and the `iss`/`aud` claims validated. The
/// authorization-code client flow (the `oauth2` crate) is a client concern,
/// not a CDR's, so it is out of scope here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OidcConfig {
    /// Expected token issuer (`iss`). Also the OIDC discovery base when no
    /// static key material is supplied. Required when the table is present.
    pub issuer: String,
    /// Accepted audiences (`aud`). Empty → audience not checked.
    pub audiences: Vec<String>,
    /// Accepted signature algorithms (e.g. `["RS256"]`). Defaults to `RS256`.
    pub algorithms: Vec<String>,
    /// A symmetric HS256 secret — the simplest key source (tests/dev).
    pub hmac_secret: Option<Secret>,
    /// File-based indirection for [`Self::hmac_secret`] (K8s/Docker secrets).
    /// Exactly one of the pair may be set; the loader reads and trims the file.
    pub hmac_secret_file: Option<PathBuf>,
    /// A static JWKS document (JSON). Preferred over discovery when present.
    pub jwks_json: Option<String>,
    /// File-based indirection for [`Self::jwks_json`] — a JWKS is a file-shaped
    /// blob that never belonged in an env var. The loader reads the file.
    pub jwks_json_file: Option<PathBuf>,
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            issuer: String::new(),
            audiences: Vec::new(),
            algorithms: default_algorithms(),
            hmac_secret: None,
            hmac_secret_file: None,
            jwks_json: None,
            jwks_json_file: None,
        }
    }
}

fn default_algorithms() -> Vec<String> {
    vec!["RS256".to_owned()]
}
