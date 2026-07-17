//! Telemetry configuration: the `[log]` ([`LogConfig`]) and `[telemetry]`
//! ([`OtelConfig`]) sections of the one config tree
//! (`docs/design/configuration.md` §3.3–3.4).
//!
//! No loaders here — both are fields of [`crate::config::EhrbaseConfig`],
//! assembled once at boot. Everything is off by default: no OTLP endpoint means
//! the `OTel` layer is **not installed at all** (zero overhead); logging always
//! runs (stdout). [`TelemetryConfig`] is the runtime pair the binary hands to
//! [`super::init`]; the config tree stores `log`/`telemetry` as siblings.
//!
//! No openEHR spec governs telemetry — our own design.

use serde::{Deserialize, Serialize};

/// The stdout log rendering profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Pick `json` when stdout is not a TTY, `pretty` when it is.
    #[default]
    Auto,
    /// One JSON object per line (containers / log collectors).
    Json,
    /// Human-friendly multi-line (interactive dev).
    Pretty,
}

/// Logging configuration (`[log]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    /// The stdout rendering profile.
    pub format: LogFormat,
    /// The boot `EnvFilter` directives (also the `/management/loggers` reset
    /// target). `RUST_LOG` is a recognized lower-priority alias.
    pub filter: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Auto,
            filter: defaults::filter(),
        }
    }
}

/// OpenTelemetry export configuration (`[telemetry]`; traces always, metrics
/// push opt-in).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OtelConfig {
    /// The OTLP/gRPC collector endpoint. **Unset ⇒ the `OTel` layer is not
    /// installed** (traces are not exported; zero overhead).
    pub otlp_endpoint: Option<String>,
    /// The `service.name` resource attribute.
    pub service_name: String,
    /// The `deployment.environment` resource attribute.
    pub environment: String,
    /// Head-sampling ratio for `parentbased_traceidratio` (1.0 = sample all;
    /// 0.1 is the documented prod starting point).
    pub traces_sample_ratio: f64,
    /// Whether to also **push** metrics over OTLP (a periodic `OTel` meter
    /// provider alongside the Prometheus pull surface). Off by default.
    pub metrics_push: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            service_name: defaults::service_name(),
            environment: defaults::environment(),
            traces_sample_ratio: defaults::sample_ratio(),
            metrics_push: false,
        }
    }
}

impl OtelConfig {
    /// Whether the `OTel` export layer should be installed at all.
    #[must_use]
    pub fn export_enabled(&self) -> bool {
        self.otlp_endpoint
            .as_ref()
            .is_some_and(|e| !e.trim().is_empty())
    }
}

/// The runtime telemetry pair the binary hands to [`super::init`]: the `[log]`
/// and `[telemetry]` sections together.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Logging configuration.
    pub log: LogConfig,
    /// OTLP export configuration.
    pub otel: OtelConfig,
}

mod defaults {
    pub(super) fn filter() -> String {
        "info,ehrbase=info".to_owned()
    }
    pub(super) fn service_name() -> String {
        "ehrbase".to_owned()
    }
    pub(super) fn environment() -> String {
        "dev".to_owned()
    }
    pub(super) const fn sample_ratio() -> f64 {
        1.0
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
    fn defaults_disable_export() {
        let c = TelemetryConfig::default();
        assert!(!c.otel.export_enabled());
        assert_eq!(c.log.filter, "info,ehrbase=info");
        assert_eq!(c.otel.service_name, "ehrbase");
        assert_eq!(c.log.format, LogFormat::Auto);
        assert!(!c.otel.metrics_push);
    }
}
