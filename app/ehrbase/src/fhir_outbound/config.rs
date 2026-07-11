//! FHIR **outbound** emitter configuration ([`FhirOutboundConfig`]) — a
//! `figment`-loaded serde struct, mirroring [`EventsConfig`](crate::events::EventsConfig).
//!
//! Loading: defaults ← optional TOML file (`EHRBASE_FHIR_OUTBOUND_CONFIG`) ←
//! `EHRBASE_FHIR_OUTBOUND_`-prefixed environment (nested keys use `__`).
//!
//! PORT NOTE (ADR-016 §Decision 4a): the outbound emitter lives in `ehrbase`
//! (the binary/platform crate), NOT in `ehrbase-rest`'s [`FhirConfig`] — it is a
//! broker + DB **background** concern (a drainer wired like the E1 outbox
//! publisher, ADR-014), and `ehrbase-rest` is the protocol adapter only
//! (ADR-011, no broker/DB work). So the REST `FhirConfig.enabled` gate (the
//! inbound/façade surface) and this emitter config are independent switches.
//!
//! PHI NOTE (ADR-016 §Decision 4a): unlike the E1 event envelopes — which are
//! PHI-free by design (ADR-014 §2) — the outbound emitter's payload IS the
//! mapped FHIR **resource**, so it carries clinical content by design. It is
//! therefore off by default behind its own explicit [`enabled`](Self::enabled)
//! flag, and publishes to a SEPARATE [`exchange`](Self::exchange) (default
//! `ehrbase.fhir`, distinct from the events exchange) so broker-level access
//! control can restrict the PHI-bearing stream independently of the PHI-free
//! envelope stream. Turning it on is an explicit, audited deployment decision.

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

/// The default AMQP broker URL (`RabbitMQ`, vhost `/`).
const DEFAULT_URL: &str = "amqp://guest:guest@localhost:5672/%2f";
/// The default topic exchange — SEPARATE from the events exchange (PHI note).
const DEFAULT_EXCHANGE: &str = "ehrbase.fhir";
/// Default rows drained per poll.
const DEFAULT_BATCH_SIZE: i64 = 128;
/// Default poll interval when the outbox is idle (ms).
const DEFAULT_POLL_INTERVAL_MS: u64 = 1_000;
/// Default per-message publish retry count before the emitter backs off.
const DEFAULT_PUBLISH_MAX_RETRIES: usize = 3;

/// FHIR outbound emitter configuration (`[fhir_outbound]`; ADR-016 §Decision
/// 4a). Every field has a default, so an all-defaults value is valid (the
/// emitter is off unless `enabled`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirOutboundConfig {
    /// Master switch (`EHRBASE_FHIR_OUTBOUND_ENABLED`). Off by default — this
    /// stream carries PHI (the mapped FHIR resource), so enabling it is an
    /// explicit deployment decision (see the module PHI note).
    #[serde(default)]
    pub enabled: bool,
    /// AMQP broker URL (`EHRBASE_FHIR_OUTBOUND_URL`).
    #[serde(default = "defaults::url")]
    pub url: String,
    /// Topic exchange to publish FHIR resources to
    /// (`EHRBASE_FHIR_OUTBOUND_EXCHANGE`); default `ehrbase.fhir`, distinct from
    /// the events exchange for PHI isolation (module PHI note).
    #[serde(default = "defaults::exchange")]
    pub exchange: String,
    /// Use TLS (`EHRBASE_FHIR_OUTBOUND_TLS`): upgrades an `amqp://` URL to
    /// `amqps://`.
    #[serde(default)]
    pub tls: bool,
    /// Outbox rows scanned per poll (`EHRBASE_FHIR_OUTBOUND_BATCH_SIZE`).
    #[serde(default = "defaults::batch_size")]
    pub batch_size: i64,
    /// Poll interval when idle (`EHRBASE_FHIR_OUTBOUND_POLL_INTERVAL_MS`).
    #[serde(default = "defaults::poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Per-message publish retry count before backing off
    /// (`EHRBASE_FHIR_OUTBOUND_PUBLISH_MAX_RETRIES`).
    #[serde(default = "defaults::publish_max_retries")]
    pub publish_max_retries: usize,
}

impl Default for FhirOutboundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: defaults::url(),
            exchange: defaults::exchange(),
            tls: false,
            batch_size: defaults::batch_size(),
            poll_interval_ms: defaults::poll_interval_ms(),
            publish_max_retries: defaults::publish_max_retries(),
        }
    }
}

impl FhirOutboundConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_FHIR_OUTBOUND_CONFIG`), then `EHRBASE_FHIR_OUTBOUND_`-prefixed
    /// environment variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(FhirOutboundConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_FHIR_OUTBOUND_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_FHIR_OUTBOUND_").split("__"))
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
        DEFAULT_PUBLISH_MAX_RETRIES, DEFAULT_URL,
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
    pub(super) const fn publish_max_retries() -> usize {
        DEFAULT_PUBLISH_MAX_RETRIES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_use_a_separate_exchange() {
        let c = FhirOutboundConfig::default();
        assert!(!c.enabled, "off by default (PHI stream)");
        assert_eq!(
            c.exchange, "ehrbase.fhir",
            "separate from the events exchange"
        );
        assert!(c.url.starts_with("amqp://"));
    }

    #[test]
    fn tls_upgrades_amqp_scheme() {
        let c = FhirOutboundConfig {
            tls: true,
            url: "amqp://guest:guest@host:5672/%2f".to_owned(),
            ..FhirOutboundConfig::default()
        };
        assert_eq!(c.effective_url(), "amqps://guest:guest@host:5672/%2f");
    }
}
