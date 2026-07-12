//! Eventing configuration ([`EventsConfig`]) — a `figment`-loaded serde struct,
//! matching the `AuditConfig`/`ExternalTerminologyConfig` pattern.
//!
//! **No openEHR spec governs this — our own design/extension** (`crate::extensions`,
//! G-12-01). Gate: `EHRBASE_EVENTS_ENABLED` (default off).
//!
//! Loading: defaults ← optional TOML file (`EHRBASE_EVENTS_CONFIG`) ←
//! `EHRBASE_EVENTS_`-prefixed environment (nested keys use `__`). Publishing is
//! **off by default**: with [`EventsConfig::enabled`] `false` the
//! binary never spawns the publisher and the outbox simply accumulates (and is
//! not drained) — the commit path always writes the rows regardless, so turning
//! eventing on later loses nothing already committed.

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

/// The default AMQP broker URL (`RabbitMQ`, vhost `/`).
const DEFAULT_URL: &str = "amqp://guest:guest@localhost:5672/%2f";
/// The default topic exchange.
const DEFAULT_EXCHANGE: &str = "ehrbase.events";
/// Default rows drained per poll.
const DEFAULT_BATCH_SIZE: i64 = 128;
/// Default poll interval when the outbox is idle (ms).
const DEFAULT_POLL_INTERVAL_MS: u64 = 1_000;
/// Default published-row retention window (days)
const DEFAULT_RETENTION_DAYS: i64 = 7;
/// Default retention-prune cadence (seconds).
const DEFAULT_PRUNE_INTERVAL_SECS: u64 = 3_600;
/// Default per-row publish retry count (on top of the first attempt) before the
/// drainer backs off and leaves the row pending.
const DEFAULT_PUBLISH_MAX_RETRIES: usize = 3;

/// Contribution-outbox eventing configuration (`[events]`) — our own extension. Every
/// field has a default, so an all-defaults [`EventsConfig`] is valid (eventing
/// is off unless `enabled`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsConfig {
    /// Master switch (`EHRBASE_EVENTS_ENABLED`). Off by default.
    #[serde(default)]
    pub enabled: bool,
    /// AMQP broker URL (`EHRBASE_EVENTS_URL`), e.g.
    /// `amqp://user:pass@host:5672/%2f` (or `amqps://…` for TLS).
    #[serde(default = "defaults::url")]
    pub url: String,
    /// Topic exchange to publish to (`EHRBASE_EVENTS_EXCHANGE`) — the eventing extension's own setting.
    #[serde(default = "defaults::exchange")]
    pub exchange: String,
    /// Use TLS (`EHRBASE_EVENTS_TLS`): when `true` an `amqp://` URL is upgraded
    /// to `amqps://` (an already-`amqps://` URL is TLS regardless).
    #[serde(default)]
    pub tls: bool,
    /// Rows drained per poll (`EHRBASE_EVENTS_BATCH_SIZE`).
    #[serde(default = "defaults::batch_size")]
    pub batch_size: i64,
    /// Poll interval when idle (`EHRBASE_EVENTS_POLL_INTERVAL_MS`).
    #[serde(default = "defaults::poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Published-row retention window in days (`EHRBASE_EVENTS_RETENTION_DAYS`);
    /// The eventing extension's own retention setting.
    #[serde(default = "defaults::retention_days")]
    pub retention_days: i64,
    /// Retention-prune cadence in seconds (`EHRBASE_EVENTS_PRUNE_INTERVAL_SECS`).
    #[serde(default = "defaults::prune_interval_secs")]
    pub prune_interval_secs: u64,
    /// Per-row publish retry count before backing off
    /// (`EHRBASE_EVENTS_PUBLISH_MAX_RETRIES`).
    #[serde(default = "defaults::publish_max_retries")]
    pub publish_max_retries: usize,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: defaults::url(),
            exchange: defaults::exchange(),
            tls: false,
            batch_size: defaults::batch_size(),
            poll_interval_ms: defaults::poll_interval_ms(),
            retention_days: defaults::retention_days(),
            prune_interval_secs: defaults::prune_interval_secs(),
            publish_max_retries: defaults::publish_max_retries(),
        }
    }
}

impl EventsConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_EVENTS_CONFIG`), then `EHRBASE_EVENTS_`-prefixed environment
    /// variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(EventsConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_EVENTS_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_EVENTS_").split("__"))
            .extract()
    }

    /// The effective broker URL, upgraded to `amqps://` when [`Self::tls`] is set
    /// and the URL is a plain `amqp://`.
    #[must_use]
    pub fn effective_url(&self) -> String {
        if self.tls && self.url.starts_with("amqp://") {
            self.url.replacen("amqp://", "amqps://", 1)
        } else {
            self.url.clone()
        }
    }
}

mod defaults {
    use super::{
        DEFAULT_BATCH_SIZE, DEFAULT_EXCHANGE, DEFAULT_POLL_INTERVAL_MS,
        DEFAULT_PRUNE_INTERVAL_SECS, DEFAULT_PUBLISH_MAX_RETRIES, DEFAULT_RETENTION_DAYS,
        DEFAULT_URL,
    };

    pub(super) fn url() -> String {
        DEFAULT_URL.to_owned()
    }
    pub(super) fn exchange() -> String {
        DEFAULT_EXCHANGE.to_owned()
    }
    pub(super) const fn batch_size() -> i64 {
        DEFAULT_BATCH_SIZE
    }
    pub(super) const fn poll_interval_ms() -> u64 {
        DEFAULT_POLL_INTERVAL_MS
    }
    pub(super) const fn retention_days() -> i64 {
        DEFAULT_RETENTION_DAYS
    }
    pub(super) const fn prune_interval_secs() -> u64 {
        DEFAULT_PRUNE_INTERVAL_SECS
    }
    pub(super) const fn publish_max_retries() -> usize {
        DEFAULT_PUBLISH_MAX_RETRIES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_sane() {
        let c = EventsConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.exchange, "ehrbase.events");
        assert_eq!(c.retention_days, 7);
        assert!(c.url.starts_with("amqp://"));
    }

    #[test]
    fn tls_upgrades_amqp_scheme() {
        let c = EventsConfig {
            tls: true,
            url: "amqp://guest:guest@host:5672/%2f".to_owned(),
            ..EventsConfig::default()
        };
        assert_eq!(c.effective_url(), "amqps://guest:guest@host:5672/%2f");
    }

    #[test]
    fn tls_leaves_amqps_untouched() {
        let c = EventsConfig {
            tls: true,
            url: "amqps://host:5671/%2f".to_owned(),
            ..EventsConfig::default()
        };
        assert_eq!(c.effective_url(), "amqps://host:5671/%2f");
    }
}
