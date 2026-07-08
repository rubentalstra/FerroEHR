//! Telemetry configuration (binding doc §3): logging + OTLP export.
//!
//! Loaded with `figment` from `EHRBASE_LOG_*` / `EHRBASE_OTEL_*` environment
//! variables (and `RUST_LOG` as the log-filter fallback). Everything is off by
//! default: no OTLP endpoint means the `OTel` layer is **not installed at all**
//! (zero overhead); logging always runs (stdout).

use figment::Figment;
use figment::providers::{Env, Serialized};
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

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// The stdout rendering profile.
    #[serde(default)]
    pub format: LogFormat,
    /// The boot `EnvFilter` directives (also the `/management/loggers` reset
    /// target).
    #[serde(default = "defaults::filter")]
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

/// OpenTelemetry export configuration (traces always; metrics push opt-in).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelConfig {
    /// The OTLP/gRPC collector endpoint. **Unset ⇒ the `OTel` layer is not
    /// installed** (traces are not exported; zero overhead).
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    /// The `service.name` resource attribute.
    #[serde(default = "defaults::service_name")]
    pub service_name: String,
    /// The `deployment.environment` resource attribute.
    #[serde(default = "defaults::environment")]
    pub environment: String,
    /// Head-sampling ratio for `parentbased_traceidratio` (1.0 = sample all;
    /// 0.1 is the documented prod starting point).
    #[serde(default = "defaults::sample_ratio")]
    pub traces_sample_ratio: f64,
    /// Whether to also **push** metrics over OTLP (a periodic `OTel` meter
    /// provider alongside the Prometheus pull surface). Off by default.
    #[serde(default)]
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

/// The combined telemetry configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Logging configuration.
    #[serde(default)]
    pub log: LogConfig,
    /// OTLP export configuration.
    #[serde(default)]
    pub otel: OtelConfig,
}

impl TelemetryConfig {
    /// Load from `EHRBASE_LOG_*` / `EHRBASE_OTEL_*` (and `RUST_LOG` as the
    /// filter fallback when `EHRBASE_LOG_FILTER` is unset).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)]
    pub fn load() -> Result<Self, figment::Error> {
        // Env keys arrive upper-cased; figment nests on `.` and matches fields
        // case-insensitively, so `EHRBASE_OTEL_OTLP_ENDPOINT` → `otel.otlp_endpoint`.
        let fig = Figment::from(Serialized::defaults(TelemetryConfig::default()))
            .merge(
                Env::prefixed("EHRBASE_OTEL_")
                    .map(|k| format!("otel.{}", k.as_str().to_ascii_lowercase()).into()),
            )
            .merge(
                Env::prefixed("EHRBASE_LOG_")
                    .map(|k| format!("log.{}", k.as_str().to_ascii_lowercase()).into()),
            );
        let mut cfg: TelemetryConfig = fig.extract()?;

        // RUST_LOG is the fallback filter only when EHRBASE_LOG_FILTER is unset.
        if std::env::var_os("EHRBASE_LOG_FILTER").is_none()
            && let Ok(rust_log) = std::env::var("RUST_LOG")
            && !rust_log.trim().is_empty()
        {
            cfg.log.filter = rust_log;
        }
        Ok(cfg)
    }
}

/// Serialize the effective telemetry config to a JSON value for the
/// `/management/env` snapshot.
///
/// # Errors
/// Returns a [`serde_json::Error`] if serialization fails.
pub fn as_value(cfg: &TelemetryConfig) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(cfg)
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

    #[test]
    #[allow(clippy::result_large_err)]
    fn env_overrides() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_OTEL_OTLP_ENDPOINT", "http://collector:4317");
            jail.set_env("EHRBASE_OTEL_SERVICE_NAME", "ehrbase-prod");
            jail.set_env("EHRBASE_OTEL_TRACES_SAMPLE_RATIO", "0.1");
            jail.set_env("EHRBASE_OTEL_METRICS_PUSH", "true");
            jail.set_env("EHRBASE_LOG_FORMAT", "json");
            jail.set_env("EHRBASE_LOG_FILTER", "ehrbase=debug");
            let c = TelemetryConfig::load().expect("load");
            assert!(c.otel.export_enabled());
            assert_eq!(c.otel.service_name, "ehrbase-prod");
            assert!((c.otel.traces_sample_ratio - 0.1).abs() < f64::EPSILON);
            assert!(c.otel.metrics_push);
            assert_eq!(c.log.format, LogFormat::Json);
            assert_eq!(c.log.filter, "ehrbase=debug");
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn rust_log_is_the_filter_fallback() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("RUST_LOG", "warn,ehrbase=trace");
            let c = TelemetryConfig::load().expect("load");
            assert_eq!(c.log.filter, "warn,ehrbase=trace");
            Ok(())
        });
    }
}
