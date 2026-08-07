//! The metrics system: ONE `OpenTelemetry` `MeterProvider` feeding both
//! surfaces.
//!
//! **No openEHR spec governs metrics — our own design/extension.**
//!
//! # Why one provider
//!
//! The server previously ran two metric systems: the `metrics` crate facade
//! behind `/management/prometheus`, and the `OpenTelemetry` SDK's own
//! instruments behind the OTLP push. A family could therefore exist on one
//! surface and not the other, and did — the OTLP push carried four of ten
//! families, missing the build identity, the request histogram and the audit
//! counters (#2175). That is a defect generator, not a one-off.
//!
//! Now there is one [`SdkMeterProvider`] with up to two readers: a Prometheus
//! reader serving the pull surface, and — when `telemetry.metrics_push` is on —
//! a periodic OTLP reader. Every instrument reaches both by construction, so
//! the class of defect is gone rather than fixed.
//!
//! # Naming
//!
//! Instrument names carry NO Prometheus suffix, and units are declared instead:
//! the exporter derives `_total` for a counter and `_seconds`/`_bytes` from the
//! unit, per the `OpenTelemetry`-to-Prometheus conventions. Writing
//! `auth_failures_total` here would render `auth_failures_total_total`.
//!
//! The `OpenTelemetry` metrics API and SDK are Stable upstream — a firmer
//! footing than the trace path this server already depends on, which is Beta
//! (<https://github.com/open-telemetry/opentelemetry-rust>).

use std::sync::OnceLock;

use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Histogram, Meter, UpDownCounter};
use opentelemetry_sdk::metrics::reader::MetricReader;
use opentelemetry_sdk::metrics::{Instrument, SdkMeterProvider, Stream, Temporality};

use crate::telemetry::build_info::BuildInfo;

/// The instrumentation scope every instrument is created under.
pub const SCOPE: &str = "ferroehr";

// ── Instrument names (no Prometheus suffix — the exporter adds it) ───────────

/// HTTP request duration (`http_route`, `http_request_method`, `status_class`).
pub const HTTP_REQUEST_DURATION: &str = "http_server_request_duration";
/// In-flight HTTP requests (`http_route`).
pub const HTTP_ACTIVE_REQUESTS: &str = "http_server_active_requests";
/// HTTP request body size (`http_route`).
pub const HTTP_REQUEST_BODY_SIZE: &str = "http_server_request_body_size";
/// HTTP response body size (`http_route`).
pub const HTTP_RESPONSE_BODY_SIZE: &str = "http_server_response_body_size";
/// Authentication failures (`mechanism`, `reason`).
pub const AUTH_FAILURES: &str = "auth_failures";
/// Cedar authorization decisions (`result`).
pub const AUTHZ_CEDAR_DECISIONS: &str = "authz_cedar_decisions";
/// Remote-PDP authorization calls (`result`).
pub const AUTHZ_REMOTE_PDP_CALLS: &str = "authz_remote_pdp_calls";
/// Database pool connections (`state`).
pub const DB_POOL_CONNECTIONS: &str = "db_pool_connections";
/// Database connection-acquire latency.
pub const DB_POOL_ACQUIRE_DURATION: &str = "db_pool_acquire_duration";
/// Service transactions (`outcome`).
pub const DB_TRANSACTIONS: &str = "db_transactions";
/// AQL queries (`outcome`).
pub const AQL_QUERIES: &str = "aql_queries";
/// AQL query phase latency (`phase`).
pub const AQL_QUERY_DURATION: &str = "aql_query_duration";
/// AQL plan-cache events (`event`).
pub const AQL_PLAN_CACHE_EVENTS: &str = "aql_plan_cache_events";
/// Committed compositions (`change_type`, the numeric openEHR audit group code).
pub const COMPOSITIONS_COMMITTED: &str = "compositions_committed";
/// Validation failures (`pass`).
pub const VALIDATION_FAILURES: &str = "validation_failures";
/// Version-signature verification failures (`verdict`).
pub const VERSION_SIGNATURE_INVALID: &str = "version_signature_invalid";
/// `WebTemplate` cache events (`event`).
pub const WEBTEMPLATE_CACHE_EVENTS: &str = "webtemplate_cache_events";
/// Contribution-outbox events published to the broker.
pub const EVENTS_PUBLISHED: &str = "events_published";
/// ATNA audit records emitted.
pub const ATNA_AUDIT_EMITTED: &str = "atna_audit_emitted";
/// ATNA audit records delivered to a sink.
pub const ATNA_AUDIT_SENT: &str = "atna_audit_sent";
/// Process start time, unix seconds.
pub const PROCESS_START_TIME: &str = "process_start_time";
/// Build identity, always `1`, carrying `version`/`git_sha`/`rm_version`.
pub const BUILD_INFO: &str = "ferroehr_build_info";
/// Tokio worker threads.
pub const TOKIO_WORKERS: &str = "tokio_workers";
/// Tokio global queue depth.
pub const TOKIO_GLOBAL_QUEUE_DEPTH: &str = "tokio_global_queue_depth";
/// Tokio alive tasks.
pub const TOKIO_ALIVE_TASKS: &str = "tokio_alive_tasks";

// ── Bucket ladders ───────────────────────────────────────────────────────────

/// HTTP request duration: 5 ms … 10 s.
pub const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
/// DB connection acquire: 100 µs … 1 s.
pub const DB_ACQUIRE_BUCKETS: &[f64] = &[0.0001, 0.000_5, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0];
/// AQL query phases: 1 ms … 30 s.
pub const AQL_DURATION_BUCKETS: &[f64] =
    &[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0];
/// Request/response bodies: 1 KiB … 64 MiB.
pub const BODY_SIZE_BUCKETS: &[f64] = &[
    1024.0,
    16_384.0,
    262_144.0,
    1_048_576.0,
    4_194_304.0,
    16_777_216.0,
    67_108_864.0,
];

/// Every histogram and its bucket ladder, in one place.
///
/// The ladders are attached as SDK views rather than left to the default
/// boundaries: `OTel`'s defaults are not ours, and a silently re-bucketed
/// latency histogram invalidates every dashboard and alert built on it.
#[must_use]
pub fn histogram_views() -> Vec<(&'static str, &'static [f64])> {
    vec![
        (HTTP_REQUEST_DURATION, HTTP_DURATION_BUCKETS),
        (HTTP_REQUEST_BODY_SIZE, BODY_SIZE_BUCKETS),
        (HTTP_RESPONSE_BODY_SIZE, BODY_SIZE_BUCKETS),
        (DB_POOL_ACQUIRE_DURATION, DB_ACQUIRE_BUCKETS),
        (AQL_QUERY_DURATION, AQL_DURATION_BUCKETS),
    ]
}

/// The instruments, built once and reachable from every emitting site.
///
/// A struct of typed handles rather than macro call sites: an instrument is
/// created once and recorded through a real type, so a misspelled metric name
/// is a compile error instead of a silently-new time series.
#[derive(Debug)]
pub struct Metrics {
    /// HTTP request duration, seconds.
    pub http_request_duration: Histogram<f64>,
    /// In-flight HTTP requests.
    pub http_active_requests: UpDownCounter<i64>,
    /// HTTP request body size, bytes.
    pub http_request_body_size: Histogram<u64>,
    /// HTTP response body size, bytes.
    pub http_response_body_size: Histogram<u64>,
    /// Authentication failures.
    pub auth_failures: Counter<u64>,
    /// Cedar authorization decisions.
    pub authz_cedar_decisions: Counter<u64>,
    /// Remote-PDP authorization calls.
    pub authz_remote_pdp_calls: Counter<u64>,
    /// Database pool connections by state.
    pub db_pool_connections: UpDownCounter<i64>,
    /// Database connection-acquire latency, seconds.
    pub db_pool_acquire_duration: Histogram<f64>,
    /// Service transactions by outcome.
    pub db_transactions: Counter<u64>,
    /// AQL queries by outcome.
    pub aql_queries: Counter<u64>,
    /// AQL query phase latency, seconds.
    pub aql_query_duration: Histogram<f64>,
    /// AQL plan-cache events.
    pub aql_plan_cache_events: Counter<u64>,
    /// Committed compositions.
    pub compositions_committed: Counter<u64>,
    /// Validation failures by pass.
    pub validation_failures: Counter<u64>,
    /// Version-signature verification failures.
    pub version_signature_invalid: Counter<u64>,
    /// `WebTemplate` cache events.
    pub webtemplate_cache_events: Counter<u64>,
    /// Events published to the broker.
    pub events_published: Counter<u64>,
    /// ATNA audit records emitted.
    pub atna_audit_emitted: Counter<u64>,
    /// ATNA audit records delivered.
    pub atna_audit_sent: Counter<u64>,
}

impl Metrics {
    /// Create every instrument on `meter`.
    #[must_use]
    pub fn new(meter: &Meter) -> Self {
        Self {
            http_request_duration: meter
                .f64_histogram(HTTP_REQUEST_DURATION)
                .with_description("HTTP server request duration")
                .with_unit("s")
                .build(),
            http_active_requests: meter
                .i64_up_down_counter(HTTP_ACTIVE_REQUESTS)
                .with_description("In-flight HTTP requests")
                .build(),
            http_request_body_size: meter
                .u64_histogram(HTTP_REQUEST_BODY_SIZE)
                .with_description("HTTP request body size")
                .with_unit("By")
                .build(),
            http_response_body_size: meter
                .u64_histogram(HTTP_RESPONSE_BODY_SIZE)
                .with_description("HTTP response body size")
                .with_unit("By")
                .build(),
            auth_failures: meter
                .u64_counter(AUTH_FAILURES)
                .with_description("Authentication failures by mechanism and reason")
                .build(),
            authz_cedar_decisions: meter
                .u64_counter(AUTHZ_CEDAR_DECISIONS)
                .with_description("Cedar authorization decisions by result")
                .build(),
            authz_remote_pdp_calls: meter
                .u64_counter(AUTHZ_REMOTE_PDP_CALLS)
                .with_description("Remote-PDP authorization calls by result")
                .build(),
            db_pool_connections: meter
                .i64_up_down_counter(DB_POOL_CONNECTIONS)
                .with_description("Connection pool connections by state")
                .build(),
            db_pool_acquire_duration: meter
                .f64_histogram(DB_POOL_ACQUIRE_DURATION)
                .with_description("Database connection acquire latency")
                .with_unit("s")
                .build(),
            db_transactions: meter
                .u64_counter(DB_TRANSACTIONS)
                .with_description("Service transactions by outcome")
                .build(),
            aql_queries: meter
                .u64_counter(AQL_QUERIES)
                .with_description("AQL queries by outcome")
                .build(),
            aql_query_duration: meter
                .f64_histogram(AQL_QUERY_DURATION)
                .with_description("AQL query phase latency")
                .with_unit("s")
                .build(),
            aql_plan_cache_events: meter
                .u64_counter(AQL_PLAN_CACHE_EVENTS)
                .with_description("AQL plan-cache events by result")
                .build(),
            compositions_committed: meter
                .u64_counter(COMPOSITIONS_COMMITTED)
                .with_description("Committed compositions by openEHR audit change type")
                .build(),
            validation_failures: meter
                .u64_counter(VALIDATION_FAILURES)
                .with_description("Validation failures by pass")
                .build(),
            version_signature_invalid: meter
                .u64_counter(VERSION_SIGNATURE_INVALID)
                .with_description("Version-signature verification failures by verdict")
                .build(),
            webtemplate_cache_events: meter
                .u64_counter(WEBTEMPLATE_CACHE_EVENTS)
                .with_description("WebTemplate cache events")
                .build(),
            events_published: meter
                .u64_counter(EVENTS_PUBLISHED)
                .with_description("Contribution-outbox events confirmed by the broker")
                .build(),
            atna_audit_emitted: meter
                .u64_counter(ATNA_AUDIT_EMITTED)
                .with_description("ATNA audit records emitted")
                .build(),
            atna_audit_sent: meter
                .u64_counter(ATNA_AUDIT_SENT)
                .with_description("ATNA audit records delivered to a sink")
                .build(),
        }
    }
}

/// The process-wide instruments.
static METRICS: OnceLock<Metrics> = OnceLock::new();

/// The instruments, once [`init`] has run.
///
/// Returns `None` before initialisation — a caller that records early is a
/// no-op rather than a panic, because a metric is never worth a crash.
#[must_use]
pub fn metrics() -> Option<&'static Metrics> {
    METRICS.get()
}

/// Install the instruments for the process.
///
/// # Errors
/// Never fails; returns the existing set if called twice, so a second call in a
/// test binary is harmless.
pub fn init(meter: &Meter) -> &'static Metrics {
    METRICS.get_or_init(|| Metrics::new(meter))
}

/// A view applying `buckets` to the histogram named `name`.
///
/// `SdkMeterProvider::with_view` takes the closure directly
/// (`Fn(&Instrument) -> Option<Stream>`), so no trait import is involved.
fn bucket_view(
    name: &'static str,
    buckets: &'static [f64],
) -> impl Fn(&Instrument) -> Option<Stream> + Send + Sync + 'static {
    move |instrument: &Instrument| {
        (instrument.name() == name).then(|| {
            Stream::builder()
                .with_aggregation(
                    opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                        boundaries: buckets.to_vec(),
                        record_min_max: true,
                    },
                )
                .build()
                .ok()
        })?
    }
}

/// Build the meter provider with the Prometheus reader always attached, and the
/// OTLP periodic reader when `otlp_reader` is supplied.
///
/// Returns the provider and the Prometheus registry the pull surface renders.
///
/// # Errors
/// Returns the exporter's error if the Prometheus reader cannot be built.
pub fn build_provider<R: MetricReader>(
    resource: opentelemetry_sdk::Resource,
    otlp_reader: Option<R>,
) -> Result<(SdkMeterProvider, prometheus::Registry), Box<dyn std::error::Error + Send + Sync>> {
    let registry = prometheus::Registry::new();
    let prometheus_reader = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()?;

    let mut builder = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(prometheus_reader);
    if let Some(reader) = otlp_reader {
        builder = builder.with_reader(reader);
    }
    for (name, buckets) in histogram_views() {
        builder = builder.with_view(bucket_view(name, buckets));
    }
    Ok((builder.build(), registry))
}

/// Render the Prometheus text exposition for `/management/prometheus`.
///
/// # Errors
/// Returns the encoder's error if the gathered families cannot be encoded.
pub fn render(registry: &prometheus::Registry) -> Result<String, prometheus::Error> {
    use prometheus::Encoder as _;
    let encoder = prometheus::TextEncoder::new();
    let mut buf = Vec::new();
    encoder.encode(&registry.gather(), &mut buf)?;
    String::from_utf8(buf).map_err(|e| prometheus::Error::Msg(e.to_string()))
}

/// Record the always-`1` build-identity gauge and the process start time.
///
/// Both are observable gauges: their value is a fact about the process rather
/// than an event, so they are read at collection time instead of pushed.
pub fn register_static_gauges(meter: &Meter, build: &BuildInfo) {
    let labels = [
        KeyValue::new("version", build.version),
        KeyValue::new("git_sha", build.git_sha),
        KeyValue::new("rm_version", build.spec.rm),
    ];
    let _build_info = meter
        .u64_observable_gauge(BUILD_INFO)
        .with_description("Build identity; always 1, carrying the identity as labels")
        .with_callback(move |observer| observer.observe(1, &labels))
        .build();

    // Wall clock comes from jiff, the workspace's one pinned time library
    // (`std::time::SystemTime::now` is a disallowed method); `as_duration` is
    // the signed span since the Unix epoch, so no fallible `duration_since`.
    let start = jiff::Timestamp::now().as_duration().as_secs_f64();
    let _start_time = meter
        .f64_observable_gauge(PROCESS_START_TIME)
        .with_description("Process start time")
        .with_unit("s")
        .with_callback(move |observer| observer.observe(start, &[]))
        .build();
}

/// The temporality the OTLP push uses.
///
/// Cumulative, matching what the Prometheus surface exposes, so the two
/// surfaces cannot disagree about the same counter.
#[must_use]
pub const fn otlp_temporality() -> Temporality {
    Temporality::Cumulative
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No instrument name may carry a Prometheus suffix: the exporter derives
    /// `_total` from the instrument kind and `_seconds`/`_bytes` from the unit,
    /// so a manual suffix here renders `auth_failures_total_total`.
    #[test]
    fn instrument_names_carry_no_prometheus_suffix() {
        let names = [
            HTTP_REQUEST_DURATION,
            HTTP_ACTIVE_REQUESTS,
            HTTP_REQUEST_BODY_SIZE,
            HTTP_RESPONSE_BODY_SIZE,
            AUTH_FAILURES,
            AUTHZ_CEDAR_DECISIONS,
            AUTHZ_REMOTE_PDP_CALLS,
            DB_POOL_CONNECTIONS,
            DB_POOL_ACQUIRE_DURATION,
            DB_TRANSACTIONS,
            AQL_QUERIES,
            AQL_QUERY_DURATION,
            AQL_PLAN_CACHE_EVENTS,
            COMPOSITIONS_COMMITTED,
            VALIDATION_FAILURES,
            VERSION_SIGNATURE_INVALID,
            WEBTEMPLATE_CACHE_EVENTS,
            EVENTS_PUBLISHED,
            ATNA_AUDIT_EMITTED,
            ATNA_AUDIT_SENT,
            PROCESS_START_TIME,
            TOKIO_WORKERS,
            TOKIO_GLOBAL_QUEUE_DEPTH,
            TOKIO_ALIVE_TASKS,
        ];
        for name in names {
            for suffix in ["_total", "_seconds", "_bytes"] {
                assert!(
                    !name.ends_with(suffix),
                    "{name} ends with {suffix}; the exporter adds it — drop it and set a unit"
                );
            }
        }
    }

    /// Every histogram instrument has an explicit bucket ladder. `OTel`'s default
    /// boundaries are not ours, and a silently re-bucketed latency histogram
    /// invalidates every dashboard built on it.
    #[test]
    fn every_histogram_has_an_explicit_bucket_ladder() {
        let with_views: Vec<&str> = histogram_views().iter().map(|(name, _)| *name).collect();
        for name in [
            HTTP_REQUEST_DURATION,
            HTTP_REQUEST_BODY_SIZE,
            HTTP_RESPONSE_BODY_SIZE,
            DB_POOL_ACQUIRE_DURATION,
            AQL_QUERY_DURATION,
        ] {
            assert!(
                with_views.contains(&name),
                "histogram {name} has no bucket view — it would take OTel's defaults"
            );
        }
    }

    /// The bucket ladders must be sorted and free of duplicates, or the
    /// exporter's cumulative buckets are nonsense.
    #[test]
    fn bucket_ladders_are_sorted_and_unique() {
        for (name, buckets) in histogram_views() {
            let mut sorted = buckets.to_vec();
            sorted.sort_by(f64::total_cmp);
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                buckets.len(),
                "{name} has duplicate bucket boundaries"
            );
            assert!(
                sorted.iter().eq(buckets.iter()),
                "{name} bucket boundaries are not ascending"
            );
        }
    }
}
