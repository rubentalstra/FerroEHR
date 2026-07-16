//! The `[events]` section — contribution-outbox eventing configuration.
//!
//! **No openEHR spec governs this — our own design/extension.** A field of the
//! one config tree ([`crate::config::EhrbaseConfig`],
//! `docs/design/configuration.md` §3.13); no loader of its own. Publishing is
//! **off by default**: with [`EventsConfig::enabled`] `false` the binary never
//! spawns the publisher.
//!
//! The commit path only writes `event_outbox` rows when an outbox consumer is
//! configured on (this publisher OR the FHIR outbound emitter), gated in
//! `main.rs` from `events.enabled || fhir.outbound.enabled`.

use crate::config::SecretUrl;
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
/// Default per-row publish retry count before the drainer backs off.
const DEFAULT_PUBLISH_MAX_RETRIES: usize = 3;

/// Contribution-outbox eventing configuration (`[events]`) — our own extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EventsConfig {
    /// Master switch. Off by default; also (with `fhir.outbound.enabled`) gates
    /// the per-commit outbox INSERT.
    pub enabled: bool,
    /// AMQP broker URL (credentials redacted from every rendering).
    pub url: SecretUrl,
    /// Topic exchange to publish to (the PHI-free envelope stream).
    pub exchange: String,
    /// Use TLS: when `true` an `amqp://` URL is upgraded to `amqps://` (an
    /// already-`amqps://` URL is TLS regardless).
    pub tls: bool,
    /// Rows drained per poll.
    pub batch_size: i64,
    /// Poll interval when idle (ms).
    pub poll_interval_ms: u64,
    /// Published-row retention window in days.
    pub retention_days: i64,
    /// Retention-prune cadence in seconds.
    pub prune_interval_secs: u64,
    /// Per-row publish retry count before backing off.
    pub publish_max_retries: usize,
    /// Mount the `/admin/event_subscription` CRUD routes (was the REST
    /// `EventSubscriptionConfig` toggle; regrouped here per P-8).
    pub admin_api: bool,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: SecretUrl::new(DEFAULT_URL),
            exchange: DEFAULT_EXCHANGE.to_owned(),
            tls: false,
            batch_size: DEFAULT_BATCH_SIZE,
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            retention_days: DEFAULT_RETENTION_DAYS,
            prune_interval_secs: DEFAULT_PRUNE_INTERVAL_SECS,
            publish_max_retries: DEFAULT_PUBLISH_MAX_RETRIES,
            admin_api: false,
        }
    }
}

impl EventsConfig {
    /// The effective broker URL, upgraded to `amqps://` when [`Self::tls`] is set
    /// and the URL is a plain `amqp://`.
    #[must_use]
    pub fn effective_url(&self) -> String {
        let url = self.url.expose();
        if self.tls && url.starts_with("amqp://") {
            url.replacen("amqp://", "amqps://", 1)
        } else {
            url.to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_sane() {
        let c = EventsConfig::default();
        assert!(!c.enabled);
        assert!(!c.admin_api);
        assert_eq!(c.exchange, "ehrbase.events");
        assert_eq!(c.retention_days, 7);
        assert!(c.url.expose().starts_with("amqp://"));
    }

    #[test]
    fn tls_upgrades_amqp_scheme() {
        let c = EventsConfig {
            tls: true,
            url: SecretUrl::new("amqp://guest:guest@host:5672/%2f"),
            ..EventsConfig::default()
        };
        assert_eq!(c.effective_url(), "amqps://guest:guest@host:5672/%2f");
    }

    #[test]
    fn tls_leaves_amqps_untouched() {
        let c = EventsConfig {
            tls: true,
            url: SecretUrl::new("amqps://host:5671/%2f"),
            ..EventsConfig::default()
        };
        assert_eq!(c.effective_url(), "amqps://host:5671/%2f");
    }
}
