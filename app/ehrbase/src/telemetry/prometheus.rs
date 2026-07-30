//! The Prometheus recorder: install, bucket ladders, metric catalog, and the
//! `build_info` / `process_start_time` gauges.
//!
//! The [`catalog`] is the single source of truth for every application metric
//! — its names, kinds, units, and (for histograms) explicit bucket ladders. It
//! drives both the recorder's bucket configuration and the `describe_*`
//! registrations, and a snapshot test pins it so a rename is always deliberate.
//! That holds across crates: a metric the protocol adapter emits (the `http_*`,
//! `auth_*`, `authz_*` families) declares its NAME here too, so an emitted
//! series is never missing its `# HELP`/`# TYPE` registration — and, in the
//! other direction, a catalog entry no code emits would render a permanently
//! dead series, so every entry has at least one emitting call site.
//!
//! The `metrics` facade emits at the call sites; the recorder renders the
//! exposition text served at `/management/prometheus`.

use crate::system_log::sender::{
    METRIC_DROPPED, METRIC_EMITTED, METRIC_REAPED, METRIC_REJECTED, METRIC_SEND_FAILED,
    METRIC_SENT, METRIC_SERIALIZE_FAILED,
};
use crate::telemetry::build_info::BuildInfo;
use metrics_exporter_prometheus::{BuildError, Matcher, PrometheusBuilder, PrometheusHandle};

// Metric names the protocol adapter (`ehrbase-rest`) records — the HTTP
// surface and the auth/authz decisions — declared here so the exporter and the
// recorder share one vocabulary.
/// HTTP request-duration histogram (`http_route`, `http_request_method`,
/// `status_class`).
pub const HTTP_REQUEST_DURATION: &str = "http_server_request_duration_seconds";

/// In-flight requests gauge (`http_route`).
pub const HTTP_ACTIVE_REQUESTS: &str = "http_server_active_requests";

/// Request body size histogram (`http_route`).
pub const HTTP_REQUEST_BODY_SIZE: &str = "http_server_request_body_size_bytes";

/// Response body size histogram (`http_route`).
pub const HTTP_RESPONSE_BODY_SIZE: &str = "http_server_response_body_size_bytes";

/// Authentication-failure counter (`mechanism`, `status`), emitted by the auth
/// middleware.
pub const AUTH_FAILURES: &str = "auth_failures_total";

/// Cedar authorization-decision counter (`result` = permit/deny), emitted by
/// the protocol adapter's `access::authz` Cedar engine.
pub const AUTHZ_CEDAR_DECISIONS: &str = "authz_cedar_decisions_total";

/// Remote-PDP authorization-call counter (`result` = permit/deny), emitted by
/// the protocol adapter's `access::authz` remote decision point.
pub const AUTHZ_REMOTE_PDP_CALLS: &str = "authz_remote_pdp_calls_total";

// ── Metric names emitted from this crate (the http_* / auth_* / authz_* names
//    above are emitted by `ehrbase-rest`; the atna_* names below by
//    `crate::system_log::sender`). ────────────────────────────────────────────

/// DB pool connection gauge (`state` = `idle/in_use`).
pub const DB_POOL_CONNECTIONS: &str = "db_pool_connections";
/// DB connection-acquire latency histogram.
pub const DB_POOL_ACQUIRE_DURATION: &str = "db_pool_acquire_duration_seconds";
/// Service transaction outcome counter (`outcome` = commit/rollback).
pub const DB_TRANSACTIONS: &str = "db_transactions_total";
/// AQL query outcome counter (`outcome` = `ok/feature_rejected/analysis_error/exec_error`).
pub const AQL_QUERIES: &str = "aql_queries_total";
/// AQL query phase-latency histogram (`phase` = plan/sql/execute/assemble).
pub const AQL_QUERY_DURATION: &str = "aql_query_duration_seconds";
/// AQL plan-cache event counter (`event` = hit/miss): the bounded cache of
/// lowered query plans keyed on the query text (no openEHR spec governs
/// this, our own performance design).
pub const AQL_PLAN_CACHE_EVENTS: &str = "aql_plan_cache_events_total";
/// Committed-composition counter (`change_type` = the numeric openEHR
/// `audit_change_type` group code — `249`/`251`/`523`/…), incremented by
/// `crate::versioning::change::meter_committed` once per COMPOSITION version
/// that a commit route actually committed (the direct create/update/delete
/// routes and the CONTRIBUTION commit).
pub const COMPOSITIONS_COMMITTED: &str = "compositions_committed_total";
/// Validation-failure counter (`pass` =
/// `rm_terminology`/`template`/`constraint_binding` — the last being an
/// archetype ac-code value-set binding the routed terminology server refused
/// or could not resolve under `fail_on_error`).
pub const VALIDATION_FAILURES: &str = "validation_failures_total";
/// Version-signature verification-failure counter (`verdict` =
/// `digest_mismatch/pgp_invalid`), incremented under `verify_on_read`
/// (RM common §"Digital Signature").
pub const VERSION_SIGNATURE_INVALID: &str = "version_signature_invalid_total";
/// `WebTemplate` cache event counter (`event` = hit/miss/eviction).
pub const WEBTEMPLATE_CACHE_EVENTS: &str = "webtemplate_cache_events_total";
/// Contribution-outbox events published to the broker. Incremented
/// per event only after the broker confirm (`crate::events`).
pub const EVENTS_PUBLISHED: &str = "events_published_total";
/// Process start time gauge (unix seconds).
pub const PROCESS_START_TIME: &str = "process_start_time_seconds";
/// Build-info gauge (always `1`; `version/git_sha/rm_version` labels).
pub const BUILD_INFO: &str = "ehrbase_build_info";
/// Tokio runtime worker-thread count gauge.
pub const TOKIO_WORKERS: &str = "tokio_workers";
/// Tokio runtime global-queue depth gauge.
pub const TOKIO_GLOBAL_QUEUE_DEPTH: &str = "tokio_global_queue_depth";
/// Tokio runtime alive-task count gauge.
pub const TOKIO_ALIVE_TASKS: &str = "tokio_alive_tasks";

// ── Histogram bucket ladders, defined once. ─────────────────────────────────

/// HTTP request duration: 5ms … 10s log ladder.
pub const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
/// DB connection-acquire duration: 100µs … 1s.
pub const DB_ACQUIRE_BUCKETS: &[f64] = &[
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
];
/// AQL query duration: 1ms … 30s.
pub const AQL_DURATION_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];
/// HTTP body sizes: 64 B … 16 MiB.
pub const BODY_SIZE_BUCKETS: &[f64] = &[
    64.0,
    256.0,
    1024.0,
    4096.0,
    16384.0,
    65536.0,
    262_144.0,
    1_048_576.0,
    4_194_304.0,
    16_777_216.0,
];

/// A metric's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricKind {
    /// Monotonic counter.
    Counter,
    /// Instantaneous gauge.
    Gauge,
    /// Bucketed histogram.
    Histogram,
}

/// A metric's catalog entry: the single source of truth for name/kind/buckets.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricSpec {
    /// The metric name.
    pub name: &'static str,
    /// The metric kind.
    pub kind: MetricKind,
    /// A one-line description.
    pub description: &'static str,
    /// The explicit bucket ladder (histograms only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buckets: Option<&'static [f64]>,
}

const fn counter(name: &'static str, description: &'static str) -> MetricSpec {
    MetricSpec {
        name,
        kind: MetricKind::Counter,
        description,
        buckets: None,
    }
}

const fn gauge(name: &'static str, description: &'static str) -> MetricSpec {
    MetricSpec {
        name,
        kind: MetricKind::Gauge,
        description,
        buckets: None,
    }
}

const fn histogram(
    name: &'static str,
    description: &'static str,
    buckets: &'static [f64],
) -> MetricSpec {
    MetricSpec {
        name,
        kind: MetricKind::Histogram,
        description,
        buckets: Some(buckets),
    }
}

/// The full application metric catalog. Ordered for a stable snapshot.
#[must_use]
pub fn catalog() -> Vec<MetricSpec> {
    vec![
        // HTTP surface (emitted by ehrbase-rest).
        histogram(
            HTTP_REQUEST_DURATION,
            "HTTP server request duration in seconds",
            HTTP_DURATION_BUCKETS,
        ),
        gauge(HTTP_ACTIVE_REQUESTS, "In-flight HTTP requests"),
        histogram(
            HTTP_REQUEST_BODY_SIZE,
            "HTTP request body size in bytes",
            BODY_SIZE_BUCKETS,
        ),
        histogram(
            HTTP_RESPONSE_BODY_SIZE,
            "HTTP response body size in bytes",
            BODY_SIZE_BUCKETS,
        ),
        counter(
            AUTH_FAILURES,
            "Authentication failures by mechanism and status",
        ),
        counter(
            AUTHZ_CEDAR_DECISIONS,
            "Cedar authorization decisions by result",
        ),
        counter(
            AUTHZ_REMOTE_PDP_CALLS,
            "Remote-PDP authorization calls by result",
        ),
        // Database.
        gauge(DB_POOL_CONNECTIONS, "Connection pool connections by state"),
        histogram(
            DB_POOL_ACQUIRE_DURATION,
            "Connection acquire latency in seconds",
            DB_ACQUIRE_BUCKETS,
        ),
        counter(DB_TRANSACTIONS, "Service transactions by outcome"),
        // AQL engine.
        counter(AQL_QUERIES, "AQL queries by outcome"),
        histogram(
            AQL_QUERY_DURATION,
            "AQL query duration by phase in seconds",
            AQL_DURATION_BUCKETS,
        ),
        counter(AQL_PLAN_CACHE_EVENTS, "AQL plan-cache events by result"),
        // Service / validation / templates.
        counter(
            COMPOSITIONS_COMMITTED,
            "Committed compositions by change type",
        ),
        counter(VALIDATION_FAILURES, "Validation failures by pass"),
        counter(
            VERSION_SIGNATURE_INVALID,
            "Version signature verification failures by verdict",
        ),
        counter(WEBTEMPLATE_CACHE_EVENTS, "WebTemplate cache events"),
        // Contribution-outbox eventing.
        counter(
            EVENTS_PUBLISHED,
            "Contribution-outbox events published to the broker",
        ),
        // ATNA audit (emitted by crate::system_log::sender).
        counter(METRIC_EMITTED, "ATNA audit records enqueued"),
        counter(METRIC_DROPPED, "ATNA audit records dropped"),
        counter(
            METRIC_REJECTED,
            "Auditable operations rejected under fail_mode = closed",
        ),
        counter(METRIC_SENT, "ATNA audit records sent to transport"),
        counter(METRIC_SEND_FAILED, "ATNA audit transport send failures"),
        counter(
            METRIC_SERIALIZE_FAILED,
            "ATNA audit record serialization failures (record dropped)",
        ),
        counter(METRIC_REAPED, "ATNA audit rows reaped by the retention job"),
        // Process / build / runtime.
        gauge(PROCESS_START_TIME, "Process start time (unix seconds)"),
        gauge(
            BUILD_INFO,
            "Build info (always 1; labels carry version/git_sha)",
        ),
        gauge(TOKIO_WORKERS, "Tokio runtime worker threads"),
        gauge(TOKIO_GLOBAL_QUEUE_DEPTH, "Tokio runtime global queue depth"),
        gauge(TOKIO_ALIVE_TASKS, "Tokio runtime alive tasks"),
    ]
}

/// Install the global Prometheus recorder with the catalog bucket ladders, then
/// register descriptions and emit the `build_info` / `process_start_time`
/// gauges. Returns the render handle for `/management/prometheus`.
///
/// # Errors
/// Returns [`BuildError`] if a recorder is already installed or bucket
/// configuration is invalid.
pub fn install(build: &BuildInfo) -> Result<PrometheusHandle, BuildError> {
    let mut builder = PrometheusBuilder::new();
    for spec in catalog() {
        if let Some(buckets) = spec.buckets {
            builder =
                builder.set_buckets_for_metric(Matcher::Full(spec.name.to_owned()), buckets)?;
        }
    }
    let handle = builder.install_recorder()?;

    describe_all();
    emit_static_gauges(build);
    Ok(handle)
}

/// Register a human-readable description + kind for every catalog metric.
fn describe_all() {
    for spec in catalog() {
        match spec.kind {
            MetricKind::Counter => {
                metrics::describe_counter!(spec.name, spec.description);
            }
            MetricKind::Gauge => {
                metrics::describe_gauge!(spec.name, spec.description);
            }
            MetricKind::Histogram => {
                metrics::describe_histogram!(spec.name, spec.description);
            }
        }
    }
}

/// Emit the constant gauges: the build-info marker and the process start time.
fn emit_static_gauges(build: &BuildInfo) {
    metrics::gauge!(
        BUILD_INFO,
        "version" => build.version,
        "git_sha" => build.git_sha,
        "rm_version" => build.spec.rm,
    )
    .set(1.0);

    // Wall clock comes from jiff (the pinned time library, docs/VERSIONS.md);
    // `as_duration` is the signed span since the Unix epoch, so no fallible
    // `duration_since` and no lossy cast
    // (https://docs.rs/jiff/latest/jiff/struct.Timestamp.html#method.as_duration).
    let start = jiff::Timestamp::now().as_duration().as_secs_f64();
    metrics::gauge!(PROCESS_START_TIME).set(start);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_names_are_unique() {
        let mut names: Vec<&str> = catalog().iter().map(|s| s.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate metric name in catalog");
    }

    #[test]
    fn every_histogram_has_buckets() {
        for spec in catalog() {
            if spec.kind == MetricKind::Histogram {
                assert!(spec.buckets.is_some(), "{} lacks buckets", spec.name);
            }
        }
    }

    #[test]
    fn catalog_covers_the_binding_doc_families() {
        let names: std::collections::BTreeSet<&str> = catalog().iter().map(|s| s.name).collect();
        for expected in [
            HTTP_REQUEST_DURATION,
            HTTP_ACTIVE_REQUESTS,
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
            METRIC_SENT,
            METRIC_REJECTED,
            METRIC_REAPED,
            BUILD_INFO,
            PROCESS_START_TIME,
            TOKIO_WORKERS,
        ] {
            assert!(names.contains(expected), "catalog missing {expected}");
        }
    }
}
