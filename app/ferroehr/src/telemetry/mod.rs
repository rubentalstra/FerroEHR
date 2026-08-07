//! Telemetry initialization.
//!
//! The single `tracing` instrumentation API fanned out to stdout logs +
//! (opt-in) OTLP spans, the `metrics` facade pulled via a Prometheus recorder,
//! and (opt-in) an OTLP metrics push path.
//!
//! **No openEHR spec governs this — our own design.** This is operational
//! observability (spans, gauges, Prometheus scrape), categorically distinct
//! from the ATNA *audit* trail (`crate::system_log`), which is a
//! security/medico-legal record. The two are deliberately separate siblings.
//!
//! [`init`] installs the Prometheus recorder, builds the `OTel` providers when an
//! endpoint is configured (absent ⇒ nothing installed, zero overhead), sets the
//! W3C propagator, and installs the subscriber. It returns a [`TelemetryGuard`]
//! carrying the reload handle, the Prometheus render handle, and the `OTel`
//! providers; the guard flushes the batch exporter and stops the samplers on
//! shutdown (explicit async [`TelemetryGuard::shutdown`] + a `Drop` backstop).

pub mod config;

use crate::telemetry::config::{OtelConfig, TelemetryConfig};
pub mod indicators;
mod layers;
mod log_sanitize;
pub mod metrics;
pub mod prometheus;
pub mod samplers;

pub mod build_info;
pub mod health;
pub mod log_reload;
pub mod provenance;

use build_info::BuildInfo;
use log_reload::LogReload;
use metrics_exporter_prometheus::PrometheusHandle;
use opentelemetry::KeyValue;
use opentelemetry::metrics::Meter;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use sqlx::PgPool;
use tokio::task::JoinHandle;

/// The `OTel` instrumentation scope name for this service.
const SCOPE: &str = "ferroehr";

/// Telemetry setup failure.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// The Prometheus recorder could not be installed.
    #[error("prometheus recorder: {0}")]
    Prometheus(#[from] metrics_exporter_prometheus::BuildError),
    /// The global subscriber could not be installed.
    #[error("subscriber init failed")]
    Subscriber(#[from] tracing_subscriber::util::TryInitError),
    /// An OTLP exporter could not be built.
    #[error("otlp exporter could not be built")]
    Exporter(#[from] opentelemetry_otlp::ExporterBuildError),
    /// The span-flamegraph capture file could not be created.
    #[error("flame capture file could not be created")]
    Flame(#[from] tracing_flame::Error),
}

/// Initialize telemetry from configuration. Call once, early in `main`.
///
/// # Errors
/// Returns [`TelemetryError`] if the recorder, an OTLP exporter, or the
/// subscriber cannot be installed.
pub fn init(cfg: &TelemetryConfig, build: &BuildInfo) -> Result<TelemetryGuard, TelemetryError> {
    // 1) Metrics: the Prometheus recorder is always installed (pull default).
    let prometheus = prometheus::install(build)?;

    // 2) Traces + optional metrics push: only when an OTLP endpoint is set.
    let mut tracer_provider = None;
    let mut meter_provider = None;
    let mut meter = None;
    let mut tracer = None;

    if cfg.otel.export_enabled() {
        let endpoint = cfg.otel.otlp_endpoint.clone().unwrap_or_default();
        let resource = resource(&cfg.otel, build);

        let provider = build_tracer_provider(&endpoint, &cfg.otel, resource.clone())?;
        tracer = Some(provider.tracer(SCOPE));
        tracer_provider = Some(provider);

        if cfg.otel.metrics_push {
            let mp = build_meter_provider(&endpoint, resource)?;
            opentelemetry::global::set_meter_provider(mp.clone());
            meter = Some(opentelemetry::global::meter(SCOPE));
            meter_provider = Some(mp);
            warn_metrics_push_is_partial();
        }

        // W3C context propagation for ingress/egress correlation.
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    }

    // 3) Logs + the (optional) OTel span layer + the (optional) span-flame
    //    capture layer.
    let (reload, flame) = layers::init_subscriber(
        cfg.log.format,
        &cfg.log.filter,
        tracer,
        cfg.otel.flame_file.as_deref(),
    )?;

    Ok(TelemetryGuard {
        reload,
        prometheus,
        tracer_provider,
        meter_provider,
        meter,
        flame,
        sampler: None,
        shut: false,
    })
}

/// Announce, at the moment `metrics_push` is enabled, which families it does
/// NOT carry.
///
/// The server keeps two metric systems: the `OpenTelemetry` SDK's own
/// instruments, which the OTLP push exports, and the `metrics`-crate recorder
/// behind `/management/prometheus`, which it does not (#2175). An operator who
/// enables the push sees metrics appear in their collector and reasonably
/// concludes the wiring is complete — while the build identity, the request
/// latency histogram and the audit counters are silently absent. A partial
/// export that looks complete is worse than one that fails, so it says so.
///
/// The list is derived from the catalogue rather than written out, so a metric
/// added later cannot quietly fall out of this warning.
fn warn_metrics_push_is_partial() {
    let scrape_only: Vec<&str> = prometheus::catalog().iter().map(|spec| spec.name).collect();
    tracing::warn!(
        families = %scrape_only.join(", "),
        "telemetry.metrics_push exports the OpenTelemetry SDK instruments ONLY. The families \
         listed here are recorded through the metrics-crate recorder and reach /management/\
         prometheus by scrape alone — they will NOT appear in your OTLP collector. Scrape \
         /management/prometheus as well if you need them (see #2175)."
    );
}

/// The `OTel` resource attributes (`service.name`/`service.version` + git sha,
/// `deployment.environment`).
fn resource(otel: &OtelConfig, build: &BuildInfo) -> Resource {
    Resource::builder()
        .with_service_name(otel.service_name.clone())
        .with_attributes([
            KeyValue::new(
                "service.version",
                format!("{}+{}", build.version, build.git_sha),
            ),
            KeyValue::new("deployment.environment", otel.environment.clone()),
            KeyValue::new("deployment.environment.name", otel.environment.clone()),
        ])
        .build()
}

/// Build the batch OTLP/gRPC tracer provider with `parentbased_traceidratio`
/// sampling.
fn build_tracer_provider(
    endpoint: &str,
    otel: &OtelConfig,
    resource: Resource,
) -> Result<SdkTracerProvider, TelemetryError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            otel.traces_sample_ratio,
        ))))
        .build())
}

/// Build the periodic OTLP/gRPC meter provider (metrics push).
fn build_meter_provider(
    endpoint: &str,
    resource: Resource,
) -> Result<SdkMeterProvider, TelemetryError> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    Ok(SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource)
        .build())
}

/// Owns the telemetry runtime handles and shuts them down on the same path the
/// ATNA sender drains on.
pub struct TelemetryGuard {
    reload: LogReload,
    prometheus: PrometheusHandle,
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
    meter: Option<Meter>,
    flame: Option<layers::FlameFlush>,
    sampler: Option<JoinHandle<()>>,
    shut: bool,
}

impl std::fmt::Debug for TelemetryGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryGuard")
            .field("otel_traces", &self.tracer_provider.is_some())
            .field("otel_metrics_push", &self.meter_provider.is_some())
            .field("flame_capture", &self.flame.is_some())
            .field("sampler_running", &self.sampler.is_some())
            .finish_non_exhaustive()
    }
}

impl TelemetryGuard {
    /// The runtime log-filter control handle (for `/management/loggers`).
    #[must_use]
    pub fn log_reload(&self) -> LogReload {
        self.reload.clone()
    }

    /// The Prometheus render handle (for `/management/prometheus`).
    #[must_use]
    pub fn prometheus_handle(&self) -> PrometheusHandle {
        self.prometheus.clone()
    }

    /// Start the background gauge sampler over the connection pool. Call once
    /// the pool is connected; the handle is aborted on [`Self::shutdown`].
    pub fn start_samplers(&mut self, pool: PgPool) {
        if self.sampler.is_none() {
            self.sampler = Some(samplers::spawn(
                pool,
                self.prometheus.clone(),
                self.meter.clone(),
            ));
        }
    }

    /// Stop the samplers and flush the `OTel` batch exporters. Idempotent-safe:
    /// after this the `Drop` backstop does nothing.
    pub async fn shutdown(mut self) {
        if let Some(sampler) = self.sampler.take() {
            sampler.abort();
        }
        // Flush the span-flame capture first (a buffered local file; dropping
        // the guard also flushes, but an explicit failure is logged, not lost).
        if let Some(flame) = self.flame.take()
            && let Err(e) = flame.flush()
        {
            tracing::warn!(error = %e, "span-flame flush at shutdown failed");
        }
        let tracer = self.tracer_provider.take();
        let meter = self.meter_provider.take();
        self.shut = true;
        // Provider shutdown is a blocking flush; keep it off the async worker.
        // A flush failure at shutdown is logged — telemetry loss is
        // acceptable, silent loss is not.
        let flushed = tokio::task::spawn_blocking(move || {
            if let Some(t) = tracer
                && let Err(e) = t.shutdown()
            {
                tracing::warn!(error = %e, "tracer flush at shutdown failed");
            }
            if let Some(m) = meter
                && let Err(e) = m.shutdown()
            {
                tracing::warn!(error = %e, "meter flush at shutdown failed");
            }
        })
        .await;
        if let Err(e) = flushed {
            tracing::warn!(error = %e, "telemetry shutdown flush task failed");
        }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(sampler) = self.sampler.take() {
            sampler.abort();
        }
        if !self.shut {
            tracing::warn!(
                "TelemetryGuard dropped without shutdown(); flushing exporters best-effort"
            );
            if let Some(flame) = self.flame.take()
                && let Err(e) = flame.flush()
            {
                tracing::warn!(error = %e, "span-flame flush in Drop failed");
            }
            if let Some(t) = self.tracer_provider.take()
                && let Err(e) = t.shutdown()
            {
                tracing::warn!(error = %e, "tracer flush in Drop failed");
            }
            if let Some(m) = self.meter_provider.take()
                && let Err(e) = m.shutdown()
            {
                tracing::warn!(error = %e, "meter flush in Drop failed");
            }
        }
    }
}
