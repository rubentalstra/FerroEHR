//! Management-surface configuration.
//!
//! The `[management]` section of the one server configuration tree; it carries **no loader of its own** — the
//! whole tree is assembled once by `ferroehr::config` and this struct is
//! deserialized as a field of it. **Everything is off by default**:
//! `management.enabled = false`, and every individual endpoint defaults to
//! [`AccessLevel::Off`] (→ `404`). The surface only exists once a deployment
//! opts each piece in explicitly.
//!
//! The surface is **ops introspection only** — build info, Prometheus, the
//! metric views, the redacted effective config, and the live log-filter
//! control. Health probes are NOT configured here: the `/health`,
//! `/health/liveness`, and `/health/readiness` endpoints are always-on and
//! public, so an orchestrator can probe a server that has this whole section
//! switched off.
//!
//! No openEHR spec governs configuration or the management surface — our own
//! design.

use serde::{Deserialize, Serialize};

/// The access level of a single management endpoint. Maps onto the existing
/// authentication layer (the same authenticator the API surface uses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// Not mounted at all — the route returns `404` (the default for every
    /// endpoint).
    #[default]
    Off,
    /// Requires an authenticated principal carrying the admin scope (the
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
#[serde(default, deny_unknown_fields)]
pub struct EndpointLevels {
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
#[serde(default, deny_unknown_fields)]
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
    /// per-endpoint level in [`Self::endpoints`] wins).
    #[serde(default = "defaults::access_default")]
    pub access_default: AccessLevel,
    /// Per-endpoint access levels.
    #[serde(default)]
    pub endpoints: EndpointLevels,
}

impl Default for ManagementConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_path: defaults::base_path(),
            port: None,
            access_default: defaults::access_default(),
            endpoints: EndpointLevels::default(),
        }
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
        assert_eq!(c.endpoints.info, AccessLevel::Off);
    }

    /// The health probes are not part of this section any more (they are the
    /// always-on public `/health` family), so a config still carrying the
    /// removed keys must fail loudly at boot rather than be silently ignored
    /// (`deny_unknown_fields`).
    #[test]
    fn removed_probe_keys_are_rejected() {
        let err = toml::from_str::<ManagementConfig>("probes_enabled = true")
            .expect_err("probes_enabled must be an unknown field");
        assert!(
            err.to_string().contains("probes_enabled"),
            "the error must name the offending key: {err}"
        );
        let err = toml::from_str::<EndpointLevels>("health = \"public\"")
            .expect_err("endpoints.health must be an unknown field");
        assert!(
            err.to_string().contains("health"),
            "the error must name the offending key: {err}"
        );
    }
}
