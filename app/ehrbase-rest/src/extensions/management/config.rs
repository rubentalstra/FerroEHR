//! Management-surface configuration (binding doc §3).
//!
//! Loaded with `figment` from `EHRBASE_MANAGEMENT_*` environment variables (and
//! an optional TOML file), following the same layering the rest of the server
//! uses. **Everything is off by default**: `management.enabled = false`, and
//! every individual endpoint defaults to [`AccessLevel::Off`] (→ `404`). The
//! surface only exists once a deployment opts each piece in explicitly.

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

/// The access level of a single management endpoint. Maps onto the existing
/// authentication layer (binding doc §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// Not mounted at all — the route returns `404` (the default for every
    /// endpoint).
    #[default]
    Off,
    /// Requires an authenticated principal carrying the admin scope (the P11
    /// admin-scope gate). `401` unauthenticated, `403` authenticated-but-not-admin.
    AdminOnly,
    /// Requires any authenticated principal. `401` unauthenticated.
    Private,
    /// No authentication required.
    Public,
}

impl AccessLevel {
    /// Whether the endpoint is mounted (anything other than [`AccessLevel::Off`]).
    #[must_use]
    pub const fn is_mounted(self) -> bool {
        !matches!(self, AccessLevel::Off)
    }
}

/// Per-endpoint access levels. Each defaults to [`AccessLevel::Off`].
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EndpointLevels {
    /// `/management/health` (aggregate).
    #[serde(default)]
    pub health: AccessLevel,
    /// `/management/info`.
    #[serde(default)]
    pub info: AccessLevel,
    /// `/management/metrics` (+ `/{name}`) — the JSON registry view.
    #[serde(default)]
    pub metrics: AccessLevel,
    /// `/management/prometheus` — the text exposition scraped by Prometheus.
    #[serde(default)]
    pub prometheus: AccessLevel,
    /// `/management/env` — the redacted effective configuration.
    #[serde(default)]
    pub env: AccessLevel,
    /// `/management/loggers` — the runtime `EnvFilter` control.
    #[serde(default)]
    pub loggers: AccessLevel,
}

/// The management surface configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementConfig {
    /// Master switch. When `false`, no management routes are mounted at all.
    #[serde(default)]
    pub enabled: bool,
    /// The base path the management endpoints hang off.
    #[serde(default = "defaults::base_path")]
    pub base_path: String,
    /// When set, the management surface is served from its **own** listener on
    /// this port (a separate axum server task in the binary) instead of the main
    /// API listener — so production can keep it off the public listener entirely.
    #[serde(default)]
    pub port: Option<u16>,
    /// The global default access level (documented fallback; the concrete
    /// per-endpoint level in [`Self::endpoints`] wins). Also the access level of
    /// the aggregate `/health` when its endpoint level is left unset.
    #[serde(default = "defaults::access_default")]
    pub access_default: AccessLevel,
    /// Per-endpoint access levels.
    #[serde(default)]
    pub endpoints: EndpointLevels,
    /// When `true`, the K8s-style `/health/liveness` and `/health/readiness`
    /// probes are mounted as [`AccessLevel::Public`] (unauthenticated), so
    /// orchestrator probes need no credentials.
    #[serde(default)]
    pub probes_enabled: bool,
}

impl Default for ManagementConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_path: defaults::base_path(),
            port: None,
            access_default: defaults::access_default(),
            endpoints: EndpointLevels::default(),
            probes_enabled: false,
        }
    }
}

impl ManagementConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_MANAGEMENT_CONFIG`), then `EHRBASE_MANAGEMENT_`-prefixed
    /// environment variables. Nested endpoint levels use
    /// `EHRBASE_MANAGEMENT_ENDPOINTS_<EP>` (e.g. `..._ENDPOINTS_PROMETHEUS`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(ManagementConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_MANAGEMENT_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        // No `.split`: multi-word scalar keys (`base_path`, `access_default`,
        // `probes_enabled`) map 1:1 after lower-casing; only the `endpoints_*`
        // family needs to become the nested `endpoints.<ep>` key.
        // Env keys arrive upper-cased; lower-case before matching, and turn the
        // `endpoints_<ep>` family into the nested `endpoints.<ep>` key (figment
        // nests on `.`). Scalar keys (`base_path`, `access_default`, …) pass
        // through and match their fields case-insensitively.
        fig.merge(Env::prefixed("EHRBASE_MANAGEMENT_").map(|key| {
            let lower = key.as_str().to_ascii_lowercase();
            match lower.strip_prefix("endpoints_") {
                Some(ep) => format!("endpoints.{ep}").into(),
                None => lower.into(),
            }
        }))
        .extract()
    }

    /// The access level for the aggregate `/health` endpoint: its explicit
    /// per-endpoint level, or [`Self::access_default`] when left `Off` but the
    /// surface is otherwise enabled with probes.
    #[must_use]
    pub fn health_level(&self) -> AccessLevel {
        self.endpoints.health
    }
}

mod defaults {
    use super::AccessLevel;

    pub(super) fn base_path() -> String {
        "/management".to_owned()
    }
    pub(super) const fn access_default() -> AccessLevel {
        AccessLevel::AdminOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off() {
        let c = ManagementConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.base_path, "/management");
        assert!(c.port.is_none());
        assert_eq!(c.access_default, AccessLevel::AdminOnly);
        assert_eq!(c.endpoints.prometheus, AccessLevel::Off);
        assert!(!c.probes_enabled);
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn env_overrides_including_nested_endpoints() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_MANAGEMENT_ENABLED", "true");
            jail.set_env("EHRBASE_MANAGEMENT_BASE_PATH", "/mgmt");
            jail.set_env("EHRBASE_MANAGEMENT_PORT", "9100");
            jail.set_env("EHRBASE_MANAGEMENT_ACCESS_DEFAULT", "private");
            jail.set_env("EHRBASE_MANAGEMENT_PROBES_ENABLED", "true");
            jail.set_env("EHRBASE_MANAGEMENT_ENDPOINTS_PROMETHEUS", "public");
            jail.set_env("EHRBASE_MANAGEMENT_ENDPOINTS_LOGGERS", "admin_only");
            let c = ManagementConfig::load().expect("load");
            assert!(c.enabled);
            assert_eq!(c.base_path, "/mgmt");
            assert_eq!(c.port, Some(9100));
            assert_eq!(c.access_default, AccessLevel::Private);
            assert!(c.probes_enabled);
            assert_eq!(c.endpoints.prometheus, AccessLevel::Public);
            assert_eq!(c.endpoints.loggers, AccessLevel::AdminOnly);
            assert_eq!(c.endpoints.env, AccessLevel::Off);
            Ok(())
        });
    }
}
