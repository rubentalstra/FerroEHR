//! Background metric samplers: the DB pool gauges, the tokio runtime gauges
//! (stable `Handle::metrics()` subset), and the periodic Prometheus recorder
//! upkeep.
//!
//! When OTLP metrics push is enabled the same gauge values are additionally
//! recorded through the `OTel` meter (dual path).
//!
//! A single background task samples on a fixed interval and is aborted by the
//! [`TelemetryGuard`](super::TelemetryGuard) on shutdown, on the same path the
//! ATNA sender drains on.

use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::metrics::Meter;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use sqlx::postgres::Postgres;
use tokio::task::JoinHandle;

use super::metrics::{
    DB_POOL_CONNECTIONS, TOKIO_ALIVE_TASKS, TOKIO_GLOBAL_QUEUE_DEPTH, TOKIO_WORKERS,
};

/// How often the sampler reads gauges + runs recorder upkeep.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// Acquire a pooled connection, recording the wait on
/// [`DB_POOL_ACQUIRE_DURATION`](crate::telemetry::metrics::DB_POOL_ACQUIRE_DURATION). Use in place of `pool.acquire()` on measured
/// hot paths.
///
/// # Errors
/// Propagates the underlying [`sqlx::Error`] from `pool.acquire()`.
pub async fn acquire(pool: &PgPool) -> Result<PoolConnection<Postgres>, sqlx::Error> {
    let started = std::time::Instant::now();
    let result = pool.acquire().await;
    crate::telemetry::metrics::metrics()
        .db_pool_acquire_duration
        .record(started.elapsed().as_secs_f64(), &[]);
    result
}

/// Spawn the background sampler.
///
/// Returns the task handle so the telemetry guard can abort it on shutdown.
/// When `meter` is `Some`, the pool/runtime gauges are mirrored through the
/// `OTel` meter for OTLP push.
#[must_use]
pub fn spawn(pool: PgPool, meter: Meter) -> JoinHandle<()> {
    tokio::spawn(async move {
        let gauges = OtelGauges::new(&meter);
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            gauges.record(&sample(&pool));
        }
    })
}

/// One sampling snapshot (all counts, so the only casts are `u64` → `f64` when
/// feeding the gauge facade).
struct Sample {
    idle: u64,
    in_use: u64,
    workers: u64,
    global_queue_depth: u64,
    alive_tasks: u64,
}

/// Read the pool + runtime gauges.
#[expect(
    clippy::as_conversions,
    reason = "pool and runtime counts widen exactly: usize is at most 64 bits on \
              every supported target"
)]
fn sample(pool: &PgPool) -> Sample {
    let size = u64::from(pool.size());
    let idle = pool.num_idle() as u64;
    let in_use = size.saturating_sub(idle);

    // `Handle::metrics()` — the stable subset only (unstable-gated counters are
    // omitted, not deferred).
    let rt = tokio::runtime::Handle::current().metrics();
    Sample {
        idle,
        in_use,
        workers: rt.num_workers() as u64,
        global_queue_depth: rt.global_queue_depth() as u64,
        alive_tasks: rt.num_alive_tasks() as u64,
    }
}

/// The pool + runtime gauges, on the one meter provider.
struct OtelGauges {
    pool: opentelemetry::metrics::Gauge<u64>,
    workers: opentelemetry::metrics::Gauge<u64>,
    global_queue_depth: opentelemetry::metrics::Gauge<u64>,
    alive_tasks: opentelemetry::metrics::Gauge<u64>,
}

impl OtelGauges {
    fn new(meter: &Meter) -> Self {
        Self {
            pool: meter.u64_gauge(DB_POOL_CONNECTIONS).build(),
            workers: meter.u64_gauge(TOKIO_WORKERS).build(),
            global_queue_depth: meter.u64_gauge(TOKIO_GLOBAL_QUEUE_DEPTH).build(),
            alive_tasks: meter.u64_gauge(TOKIO_ALIVE_TASKS).build(),
        }
    }

    fn record(&self, s: &Sample) {
        self.pool.record(s.idle, &[KeyValue::new("state", "idle")]);
        self.pool
            .record(s.in_use, &[KeyValue::new("state", "in_use")]);
        self.workers.record(s.workers, &[]);
        self.global_queue_depth.record(s.global_queue_depth, &[]);
        self.alive_tasks.record(s.alive_tasks, &[]);
    }
}
