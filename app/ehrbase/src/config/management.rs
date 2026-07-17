//! Management-surface configuration (binding doc §3).
//!
//! The `[management]` section of the one server configuration tree
//! (`docs/design/configuration.md`); it carries **no loader of its own** — the
//! whole tree is assembled once by `ehrbase::config` and this struct is
//! deserialized as a field of it. **Everything is off by default**:
//! `management.enabled = false`, and every individual endpoint defaults to
//! [`AccessLevel::Off`] (→ `404`). The surface only exists once a deployment
//! opts each piece in explicitly.
//!
//! No openEHR spec governs configuration or the management surface — our own
//! design.

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
#[serde(default, deny_unknown_fields)]
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
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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
}
