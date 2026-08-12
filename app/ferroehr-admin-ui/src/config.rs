// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Console configuration: one TOML file (`ferroehr-admin-ui.toml`) with
//! `FERROEHR_ADMIN__…` environment overrides, mirroring the CDR's
//! one-file/strict/env-grammar convention.
//!
//! No openEHR spec governs configuration — our own design. The console is
//! stateless bar its in-process session store: no database, and no local store
//! of domain state either — every fact it shows lives in the CDR and is read
//! over ITS-REST.

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
}

/// Where and how to reach the CDR.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CdrConfig {
    /// Base URL of the CDR (scheme://host:port, no trailing path) — the
    /// ITS-REST base path `/ferroehr/rest/openehr/v1` is appended by the
    /// client.
    pub base_url: String,
    /// Base URL of the CDR's management surface **including its base path**
    /// (e.g. `http://cdr.internal:9464/management`). Empty = derive it from
    /// [`Self::base_url`] with the CDR's default `/management` base path.
    ///
    /// One knob covers both ways a deployment can move that surface: the CDR
    /// can serve it from its own internal listener (`management.port`) and can
    /// rename its base path (`management.base_path`), so the console takes the
    /// whole prefix rather than guessing a port and a path separately. No
    /// openEHR spec governs a management surface — our own operational
    /// extension.
    pub management_base_url: String,
    /// Per-request timeout in seconds.
    pub request_timeout_secs: u64,
}

impl Default for CdrConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_owned(),
            management_base_url: String::new(),
            request_timeout_secs: 30,
        }
    }
}

impl CdrConfig {
    /// The management-surface base URL with no trailing slash: the configured
    /// value, or `{base_url}/management` (the CDR's default base path) when it
    /// is empty.
    #[must_use]
    pub fn management_base(&self) -> String {
        let configured = self.management_base_url.trim_end_matches('/');
        if configured.is_empty() {
            format!("{}/management", self.base_url.trim_end_matches('/'))
        } else {
            configured.to_owned()
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
/// File discovery: `$FERROEHR_ADMIN_CONFIG` → `./ferroehr-admin-ui.toml` →
/// `/etc/ferroehr/admin-ui.toml` (search-order files are optional; an
/// explicitly pointed-at file must exist). Env overrides use the uniform
/// grammar `FERROEHR_ADMIN__<SECTION>__<KEY>` (e.g.
/// `FERROEHR_ADMIN__CDR__BASE_URL`).
///
/// # Errors
/// Returns a [`ConfigError`] on an unreadable/invalid file, an unknown key
/// (strict deserialization), a type mismatch, or an unresolvable
/// `client_secret_file`.
pub fn load() -> Result<AdminUiConfig, ConfigError> {
    let mut builder = config::Config::builder();

    // The one env read that cannot flow through the config tree: it is the
    // pointer AT the file the tree is assembled from, resolved before any
    // source exists.
    #[expect(
        clippy::disallowed_methods,
        reason = "this IS the console's config-tree loader; the config-file pointer is its bootstrap input and has no earlier source to come from"
    )]
    if let Ok(explicit) = std::env::var("FERROEHR_ADMIN_CONFIG") {
        builder = builder.add_source(config::File::new(&explicit, config::FileFormat::Toml));
    } else {
        builder = builder
            .add_source(
                config::File::new("ferroehr-admin-ui.toml", config::FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                config::File::new("/etc/ferroehr/admin-ui.toml", config::FileFormat::Toml)
                    .required(false),
            );
    }

    let assembled = builder
        .add_source(
            config::Environment::with_prefix("FERROEHR_ADMIN")
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

#[cfg(test)]
mod tests {
    use super::CdrConfig;

    #[test]
    fn management_base_defaults_to_the_cdr_origin() {
        let cfg = CdrConfig::default();
        assert_eq!(cfg.management_base(), "http://localhost:8080/management");
    }

    #[test]
    fn a_configured_management_base_wins_and_may_move_host_port_and_path() {
        // The whole prefix is configurable: a separate internal listener
        // (`management.port` CDR-side) and a renamed `management.base_path`
        // are the same one knob here.
        let cfg = CdrConfig {
            management_base_url: "http://cdr.internal:9464/ops/".to_owned(),
            ..CdrConfig::default()
        };
        assert_eq!(cfg.management_base(), "http://cdr.internal:9464/ops");
    }

    #[test]
    fn a_trailing_slash_on_the_cdr_base_url_does_not_double_up() {
        let cfg = CdrConfig {
            base_url: "http://cdr:8080/".to_owned(),
            ..CdrConfig::default()
        };
        assert_eq!(cfg.management_base(), "http://cdr:8080/management");
    }
}
