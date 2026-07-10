//! The background drainer + retention pruner + [`EventsHandle`] (ADR-014 §3/§6).
//!
//! A **single** tokio task polls the outbox, publishes pending rows in `seq`
//! order (a global order that trivially preserves per-EHR order), and marks
//! each published only after the broker confirms. On a publish failure it stops
//! the batch — never skipping ahead — so an EHR's events keep their order, and
//! backs off before retrying (the outbox buffers while the broker is down). A
//! periodic pass prunes published rows older than the retention window.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use sqlx::{PgPool, Row};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::amqp::AmqpPublisher;
use super::config::EventsConfig;
use super::{EventError, EventPublisher};
use crate::telemetry::prometheus::EVENTS_PUBLISHED;

/// Owns the drainer task; the binary keeps it and shuts it down on exit.
#[derive(Debug)]
pub struct EventsHandle {
    shutdown: watch::Sender<bool>,
    join: JoinHandle<()>,
    /// Liveness of broker delivery: `true` while draining succeeds, `false`
    /// after a publish/transport failure. Surfaced by the `events` health
    /// indicator (never blocks readiness — degraded-tolerable).
    healthy: Arc<AtomicBool>,
}

impl EventsHandle {
    /// A shared flag reporting whether broker delivery is currently healthy,
    /// for the `events` health indicator.
    #[must_use]
    pub fn healthy(&self) -> Arc<AtomicBool> {
        self.healthy.clone()
    }

    /// Signal the drainer to stop and await it, bounded by `timeout`. Pending
    /// (unpublished) rows are left in the outbox and drained on next start
    /// (at-least-once).
    pub async fn shutdown(self, timeout: Duration) {
        let _ = self.shutdown.send(true);
        match tokio::time::timeout(timeout, self.join).await {
            Ok(Ok(())) => tracing::debug!("event publisher stopped"),
            Ok(Err(e)) => tracing::warn!("event publisher task panicked: {e}"),
            Err(_) => tracing::warn!("event publisher did not stop within {timeout:?}"),
        }
    }
}

/// Start the publisher over the real AMQP broker (ADR-014 §4). Constructs the
/// lazily-connecting [`AmqpPublisher`] and spawns the drainer — a broker that is
/// down at start is tolerated (rows stay pending until it returns).
#[must_use]
pub fn start(config: EventsConfig, pool: PgPool) -> EventsHandle {
    let publisher = Arc::new(AmqpPublisher::new(
        config.effective_url(),
        config.exchange.clone(),
    ));
    start_with_publisher(config, pool, publisher)
}

/// Start the publisher over an arbitrary [`EventPublisher`] (the seam the tests
/// use to drive the drainer without a real broker).
#[must_use]
pub fn start_with_publisher(
    config: EventsConfig,
    pool: PgPool,
    publisher: Arc<dyn EventPublisher>,
) -> EventsHandle {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let healthy = Arc::new(AtomicBool::new(true));
    let join = tokio::spawn(run(config, pool, publisher, shutdown_rx, healthy.clone()));
    EventsHandle {
        shutdown: shutdown_tx,
        join,
        healthy,
    }
}

/// A failure during one drain pass.
enum DrainError {
    /// A DB error reading/marking the outbox.
    Db(sqlx::Error),
    /// A broker publish failure; the drainer should back off before retrying.
    Publish(EventError),
}

/// The drain + prune loop.
async fn run(
    config: EventsConfig,
    pool: PgPool,
    publisher: Arc<dyn EventPublisher>,
    mut shutdown: watch::Receiver<bool>,
    healthy: Arc<AtomicBool>,
) {
    let poll_interval = Duration::from_millis(config.poll_interval_ms.max(1));
    let prune_every = Duration::from_secs(config.prune_interval_secs.max(1));
    let mut last_prune = tokio::time::Instant::now();
    tracing::info!(
        exchange = %config.exchange,
        batch_size = config.batch_size,
        retention_days = config.retention_days,
        "event publisher started"
    );

    loop {
        if *shutdown.borrow() {
            break;
        }
        // (Re)declare + bind the queues for the enabled subscriptions (ADR-014
        // §5) at the top of each cycle. This covers drainer startup AND picks up
        // subscription CRUD (a config-gated admin surface that has no broker
        // access of its own — ADR-011 keeps the service protocol-free); queue
        // declaration is idempotent, and doing it before this cycle's publishes
        // guarantees a just-created subscription's queue is bound before any
        // matching event is routed (a topic exchange drops unroutable messages).
        // Best-effort: a failure (broker down) is logged; rows stay pending and
        // the queues are declared on the next cycle the broker is reachable.
        if let Err(e) = sync_subscriptions(&pool, publisher.as_ref(), &config.exchange).await {
            tracing::debug!("event subscription sync deferred: {e}");
        }
        // Drain until the outbox is empty or the broker/DB stalls.
        loop {
            if *shutdown.borrow() {
                break;
            }
            match drain_batch(&pool, publisher.as_ref(), &config).await {
                Ok(n) => {
                    healthy.store(true, Ordering::Relaxed);
                    // A full batch means more may remain — keep draining;
                    // a short batch means the outbox is now empty.
                    if usize::try_from(config.batch_size).unwrap_or(usize::MAX) > n {
                        break;
                    }
                }
                Err(DrainError::Publish(e)) => {
                    healthy.store(false, Ordering::Relaxed);
                    tracing::warn!("event publish stalled (broker unavailable?): {e}");
                    break;
                }
                Err(DrainError::Db(e)) => {
                    tracing::warn!("event outbox drain DB error: {e}");
                    break;
                }
            }
        }

        // Retention prune (best-effort), on its own cadence.
        if last_prune.elapsed() >= prune_every {
            if let Err(e) = prune(&pool, config.retention_days).await {
                tracing::warn!("event outbox retention prune failed: {e}");
            }
            last_prune = tokio::time::Instant::now();
        }

        // Wait for the next poll tick or a shutdown signal.
        tokio::select! {
            _ = shutdown.changed() => {}
            () = tokio::time::sleep(poll_interval) => {}
        }
    }

    // Best-effort final drain so a clean shutdown flushes what the broker will
    // still take; anything left stays pending for next start (at-least-once).
    if let Ok(n) = drain_batch(&pool, publisher.as_ref(), &config).await
        && n > 0
    {
        tracing::debug!("event publisher flushed {n} events on shutdown");
    }
    tracing::debug!("event publisher loop exited");
}

/// Publish one batch of pending rows in `seq` order, marking each published only
/// after the broker confirms. Returns the count published this pass, or a
/// [`DrainError`] (the published prefix is committed first, preserving per-EHR
/// order). `FOR UPDATE SKIP LOCKED` makes concurrent instances safe.
async fn drain_batch(
    pool: &PgPool,
    publisher: &dyn EventPublisher,
    config: &EventsConfig,
) -> Result<usize, DrainError> {
    let mut tx = pool.begin().await.map_err(DrainError::Db)?;
    let rows = sqlx::query(
        "SELECT seq, envelope FROM event_outbox \
         WHERE published_at IS NULL ORDER BY seq LIMIT $1 FOR UPDATE SKIP LOCKED",
    )
    .bind(config.batch_size)
    .fetch_all(&mut *tx)
    .await
    .map_err(DrainError::Db)?;
    if rows.is_empty() {
        return Ok(0);
    }

    let mut sent = 0usize;
    let mut publish_err = None;
    'rows: for row in &rows {
        let seq: i64 = row.try_get("seq").map_err(DrainError::Db)?;
        let envelope: serde_json::Value = row.try_get("envelope").map_err(DrainError::Db)?;
        // Per-version fan-out (ADR-014 §5): one message per version entry, each
        // under its own routing key, carrying the shared envelope + seq +
        // version_index. All of a row's messages must confirm before the row is
        // marked published; a failure part-way leaves the whole row pending, so
        // the retry re-publishes every message (at-least-once — consumers
        // deduplicate on (contribution_id, version_index)).
        for (version_index, routing_key) in version_routing_keys(&envelope) {
            let payload = build_payload(seq, version_index, &envelope);
            if let Err(e) = publish_with_retry(publisher, &routing_key, &payload, config).await {
                // Stop at the first failure: never publish a later event for an
                // EHR before an earlier one (per-EHR ordering, ADR-014 §3).
                publish_err = Some(e);
                break 'rows;
            }
        }
        sqlx::query("UPDATE event_outbox SET published_at = now() WHERE seq = $1")
            .bind(seq)
            .execute(&mut *tx)
            .await
            .map_err(DrainError::Db)?;
        sent += 1;
    }
    tx.commit().await.map_err(DrainError::Db)?;
    if sent > 0 {
        metrics::counter!(EVENTS_PUBLISHED).increment(sent as u64);
    }
    match publish_err {
        Some(e) => Err(DrainError::Publish(e)),
        None => Ok(sent),
    }
}

/// The per-version routing keys for one envelope (ADR-014 §5): `(version_index,
/// routing_key)` for each entry in `versions`. A well-formed outbox row always
/// carries ≥1 version; a defensively-empty envelope yields a single message at
/// `version_index` 0 with the fallback key so nothing is silently dropped.
fn version_routing_keys(envelope: &serde_json::Value) -> Vec<(usize, String)> {
    match envelope.get("versions").and_then(|v| v.as_array()) {
        Some(versions) if !versions.is_empty() => versions
            .iter()
            .enumerate()
            .map(|(i, v)| (i, super::routing_key_of_version(v)))
            .collect(),
        _ => vec![(0, super::routing_key("UNKNOWN", super::ABSENT, None))],
    }
}

/// The published payload for one version: the stored PHI-free envelope with the
/// delivery `seq` and the `version_index` injected. Consumers order by `seq`,
/// and deduplicate on `(contribution_id, version_index)` (at-least-once).
fn build_payload(seq: i64, version_index: usize, envelope: &serde_json::Value) -> Vec<u8> {
    let mut payload = envelope.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("seq".to_owned(), serde_json::json!(seq));
        obj.insert("version_index".to_owned(), serde_json::json!(version_index));
    }
    serde_json::to_vec(&payload).unwrap_or_default()
}

/// Declare + bind the queue for every **enabled** subscription (ADR-014 §5). One
/// idempotent `queue_declare` + `queue_bind` per row, keyed by
/// [`super::subscription_binding_key`]. Best-effort at the call site: an error
/// (broker unreachable) is propagated for the caller to log and retry next
/// cycle.
async fn sync_subscriptions(
    pool: &PgPool,
    publisher: &dyn EventPublisher,
    exchange: &str,
) -> Result<(), SyncError> {
    let rows = sqlx::query(
        "SELECT name, kind, change_type, template_id FROM event_subscription WHERE enabled",
    )
    .fetch_all(pool)
    .await
    .map_err(SyncError::Db)?;
    for row in &rows {
        let name: String = row.try_get("name").map_err(SyncError::Db)?;
        let kind: Option<String> = row.try_get("kind").map_err(SyncError::Db)?;
        let change_type: Option<String> = row.try_get("change_type").map_err(SyncError::Db)?;
        let template_id: Option<String> = row.try_get("template_id").map_err(SyncError::Db)?;
        let binding_key = super::subscription_binding_key(
            kind.as_deref(),
            change_type.as_deref(),
            template_id.as_deref(),
        );
        let queue = subscription_queue_name(exchange, &name);
        publisher
            .declare_subscription(&queue, &binding_key)
            .await
            .map_err(SyncError::Broker)?;
    }
    Ok(())
}

/// The broker queue name for a subscription (ADR-014 §5): `<exchange>.<name>`
/// (`ehrbase.events.<name>` for the default exchange) — the configured exchange
/// prefix + the subscription name. Exposed so a consumer knows the queue to
/// consume from.
#[must_use]
pub fn subscription_queue_name(exchange: &str, name: &str) -> String {
    format!("{exchange}.{name}")
}

/// A failure while syncing subscription queues.
enum SyncError {
    /// Reading the subscription rows failed.
    Db(sqlx::Error),
    /// Declaring/binding a queue on the broker failed.
    Broker(EventError),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Db(e) => write!(f, "subscription store: {e}"),
            SyncError::Broker(e) => write!(f, "broker declare: {e}"),
        }
    }
}

/// Publish with exponential backoff (ADR-014 §3), up to `publish_max_retries`
/// extra attempts, before giving up for this pass.
async fn publish_with_retry(
    publisher: &dyn EventPublisher,
    routing_key: &str,
    payload: &[u8],
    config: &EventsConfig,
) -> Result<(), EventError> {
    (|| async { publisher.publish(routing_key, payload).await })
        .retry(ExponentialBuilder::default().with_max_times(config.publish_max_retries))
        .notify(|e, d| tracing::warn!("event publish retry in {d:?}: {e}"))
        .await
}

/// Delete published rows older than the retention window (ADR-014 §6). Returns
/// the number pruned.
async fn prune(pool: &PgPool, retention_days: i64) -> Result<u64, sqlx::Error> {
    let cutoff = format!("{retention_days} days");
    let result = sqlx::query(
        "DELETE FROM event_outbox \
         WHERE published_at IS NOT NULL AND published_at < now() - $1::interval",
    )
    .bind(cutoff)
    .execute(pool)
    .await?;
    let pruned = result.rows_affected();
    if pruned > 0 {
        tracing::debug!("pruned {pruned} published event rows past retention");
    }
    Ok(pruned)
}
