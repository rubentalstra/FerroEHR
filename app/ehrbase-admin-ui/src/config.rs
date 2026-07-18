//! Console configuration: one TOML file (`ehrbase-admin-ui.toml`) with
//! `EHRBASE_ADMIN__…` environment overrides, mirroring the CDR's
//! one-file/strict/env-grammar convention. No openEHR spec governs
//! configuration — our own design. The console is stateless bar its
//! in-process session store; there is no database.

use serde::Deserialize;

/// The single configuration root for the console.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AdminUiConfig {
    /// The CDR connection.
    pub cdr: CdrConfig,
    /// Console login modes.
    pub auth: AuthConfig,
    /// Session behaviour.
    pub session: SessionConfig,
    /// Path of the console-local query-groups JSON store (empty = the
    /// default `./admin-ui-groups.json`). No openEHR spec governs query
    /// groups — our own design/extension.
    pub groups_file: String,
}

impl AdminUiConfig {
    /// The query-groups store path (the configured value or the default).
    #[must_use]
    pub fn groups_file(&self) -> String {
        if self.groups_file.is_empty() {
            "admin-ui-groups.json".to_owned()
        } else {
            self.groups_file.clone()
        }
    }
}

/// Where and how to reach the CDR.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CdrConfig {
    /// Base URL of the CDR (scheme://host:port, no trailing path) — the
    /// ITS-REST base path `/ehrbase/rest/openehr/v1` is appended by the
    /// client.
    pub base_url: String,
    /// Per-request timeout in seconds.
    pub request_timeout_secs: u64,
}

impl Default for CdrConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_owned(),
            request_timeout_secs: 30,
        }
    }
}

/// Which login modes the console offers (design: both ship in v1).
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    /// Offer the Basic (username/password) form; credentials are validated
    /// against the CDR and held server-side in the session.
    pub basic_enabled: bool,
    /// OIDC authorization-code login.
    pub oidc: OidcConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            basic_enabled: true,
            oidc: OidcConfig::default(),
        }
    }
}

/// OIDC (Keycloak-style) settings for the console's own login.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct OidcConfig {
    /// Enable the OIDC login button + routes.
    pub enabled: bool,
    /// Issuer URL (discovery is derived from it).
    pub issuer: String,
    /// The `OAuth2` client id registered for the console.
    pub client_id: String,
    /// The `OAuth2` client secret (prefer `client_secret_file` in deployments).
    pub client_secret: String,
    /// Path to a file holding the client secret; wins over `client_secret`.
    pub client_secret_file: String,
    /// Externally visible base URL of the console (for the redirect URI),
    /// e.g. `http://localhost:3000`.
    pub public_base_url: String,
    /// Split-horizon DNS override for the issuer host, `host=ip:port`
    /// (e.g. `keycloak=127.0.0.1:8081`). Lets the console reach an issuer
    /// whose canonical hostname only resolves inside a container network
    /// while browsers and tokens keep the canonical URL. Empty = none.
    pub resolve: String,
    /// Additional scopes requested beyond `openid`.
    pub scopes: Vec<String>,
}

/// Session behaviour (in-process store; single-instance deployment).
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SessionConfig {
    /// Idle expiry, minutes.
    pub idle_minutes: u64,
    /// Set the `Secure` cookie flag (turn on behind TLS).
    pub cookie_secure: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_minutes: 60,
            cookie_secure: false,
        }
    }
}

/// One configuration error, rendered as a single human line.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ConfigError(String);

/// Load the console configuration: defaults < file < environment.
///
/// File discovery: `$EHRBASE_ADMIN_CONFIG` → `./ehrbase-admin-ui.toml` →
/// `/etc/ehrbase/admin-ui.toml` (search-order files are optional; an
/// explicitly pointed-at file must exist). Env overrides use the uniform
/// grammar `EHRBASE_ADMIN__<SECTION>__<KEY>` (e.g.
/// `EHRBASE_ADMIN__CDR__BASE_URL`).
///
/// # Errors
/// Returns a [`ConfigError`] on an unreadable/invalid file, an unknown key
/// (strict deserialization), a type mismatch, or an unresolvable
/// `client_secret_file`.
pub fn load() -> Result<AdminUiConfig, ConfigError> {
    let mut builder = config::Config::builder();

    if let Ok(explicit) = std::env::var("EHRBASE_ADMIN_CONFIG") {
        builder = builder.add_source(config::File::new(&explicit, config::FileFormat::Toml));
    } else {
        builder = builder
            .add_source(
                config::File::new("ehrbase-admin-ui.toml", config::FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                config::File::new("/etc/ehrbase/admin-ui.toml", config::FileFormat::Toml)
                    .required(false),
            );
    }

    let assembled = builder
        .add_source(
            config::Environment::with_prefix("EHRBASE_ADMIN")
                .prefix_separator("__")
                .separator("__"),
        )
        .build()
        .map_err(|e| ConfigError(format!("configuration assembly: {e}")))?;

    let mut cfg: AdminUiConfig = assembled
        .try_deserialize()
        .map_err(|e| ConfigError(format!("configuration: {e}")))?;

    if !cfg.auth.oidc.client_secret_file.is_empty() {
        let secret = std::fs::read_to_string(&cfg.auth.oidc.client_secret_file).map_err(|e| {
            ConfigError(format!(
                "auth.oidc.client_secret_file `{}`: {e}",
                cfg.auth.oidc.client_secret_file
            ))
        })?;
        secret.trim().clone_into(&mut cfg.auth.oidc.client_secret);
    }

    if cfg.auth.oidc.enabled {
        let mut missing = Vec::new();
        for (key, value) in [
            ("auth.oidc.issuer", &cfg.auth.oidc.issuer),
            ("auth.oidc.client_id", &cfg.auth.oidc.client_id),
            ("auth.oidc.public_base_url", &cfg.auth.oidc.public_base_url),
        ] {
            if value.is_empty() {
                missing.push(key);
            }
        }
        if !missing.is_empty() {
            return Err(ConfigError(format!(
                "auth.oidc.enabled = true requires: {}",
                missing.join(", ")
            )));
        }
    }

    Ok(cfg)
}
