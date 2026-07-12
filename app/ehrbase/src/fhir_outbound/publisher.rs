//! The outbound emitter's background drainer — wired like
//! the E1 outbox publisher, but reading committed `event_outbox`
//! rows through its OWN persistent cursor (`fhir_outbound_cursor.last_seq`), so
//! it never touches the E1 drainer's `published_at` watermark.
//!
//! A single tokio task polls the outbox for rows past the cursor in `seq` order.
//! For each COMPOSITION version whose template matches an enabled `fhir_mapping`,
//! it loads the committed version through the versioned read seam, reverse-maps
//! it to a FHIR resource, and publishes it (with broker confirms) to the
//! configured PHI exchange. The cursor advances only over the fully-published
//! prefix, so a crash/retry re-emits from the unadvanced cursor (at-least-once;
//! downstream FHIR systems upsert by resource id).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use serde_json::Value;
use sqlx::{PgPool, Row};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::events::{AmqpPublisher, EventError, EventPublisher};
use crate::service::EhrbaseService;

use super::config::FhirOutboundConfig;

/// Owns the outbound-emitter task; the binary keeps it and shuts it down on exit.
#[derive(Debug)]
pub struct FhirOutboundHandle {
    shutdown: watch::Sender<bool>,
    join: JoinHandle<()>,
    /// Liveness of broker delivery: `true` while emitting succeeds, `false`
    /// after a publish/transport failure. Surfaced by a health indicator.
    healthy: Arc<AtomicBool>,
}

impl FhirOutboundHandle {
    /// A shared flag reporting whether outbound delivery is currently healthy.
    #[must_use]
    pub fn healthy(&self) -> Arc<AtomicBool> {
        self.healthy.clone()
    }

    /// Signal the emitter to stop and await it, bounded by `timeout`. Unprocessed
    /// outbox rows stay past the cursor and are emitted on next start
    /// (at-least-once).
    pub async fn shutdown(self, timeout: Duration) {
        let _ = self.shutdown.send(true);
        match tokio::time::timeout(timeout, self.join).await {
            Ok(Ok(())) => tracing::debug!("fhir outbound emitter stopped"),
            Ok(Err(e)) => tracing::warn!("fhir outbound emitter task panicked: {e}"),
            Err(_) => tracing::warn!("fhir outbound emitter did not stop within {timeout:?}"),
        }
    }
}

/// Start the outbound emitter over the real AMQP broker. Constructs the
/// lazily-connecting [`AmqpPublisher`] on the PHI exchange and spawns the
/// drainer (a broker that is down at start is tolerated — rows stay past the
/// cursor until it returns).
#[must_use]
pub fn start(
    config: FhirOutboundConfig,
    pool: PgPool,
    service: Arc<EhrbaseService>,
) -> FhirOutboundHandle {
    let publisher = Arc::new(AmqpPublisher::new(
        config.effective_url(),
        config.exchange.clone(),
    ));
    start_with_publisher(config, pool, service, publisher)
}

/// Start the emitter over an arbitrary [`EventPublisher`] (the seam the tests use
/// to drive the emitter without a real broker).
#[must_use]
pub fn start_with_publisher(
    config: FhirOutboundConfig,
    pool: PgPool,
    service: Arc<EhrbaseService>,
    publisher: Arc<dyn EventPublisher>,
) -> FhirOutboundHandle {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let healthy = Arc::new(AtomicBool::new(true));
    let join = tokio::spawn(run(
        config,
        pool,
        service,
        publisher,
        shutdown_rx,
        healthy.clone(),
    ));
    FhirOutboundHandle {
        shutdown: shutdown_tx,
        join,
        healthy,
    }
}

/// A failure during one process pass.
enum ProcessError {
    /// A DB error reading the outbox / cursor.
    Db(sqlx::Error),
    /// Reverse-mapping a COMPOSITION failed (a stored mapping/template problem);
    /// the row stays past the cursor for retry.
    Map(String),
    /// A broker publish failure; back off before retrying.
    Publish(EventError),
}

impl From<sqlx::Error> for ProcessError {
    fn from(e: sqlx::Error) -> Self {
        ProcessError::Db(e)
    }
}

/// The poll loop.
async fn run(
    config: FhirOutboundConfig,
    pool: PgPool,
    service: Arc<EhrbaseService>,
    publisher: Arc<dyn EventPublisher>,
    mut shutdown: watch::Receiver<bool>,
    healthy: Arc<AtomicBool>,
) {
    let poll_interval = Duration::from_millis(config.poll_interval_ms.max(1));
    tracing::info!(
        exchange = %config.exchange,
        batch_size = config.batch_size,
        "fhir outbound emitter started"
    );

    loop {
        if *shutdown.borrow() {
            break;
        }
        // Drain until the outbox is caught up or the broker/DB stalls.
        loop {
            if *shutdown.borrow() {
                break;
            }
            match process_batch(&pool, &service, publisher.as_ref(), &config).await {
                Ok(n) => {
                    healthy.store(true, Ordering::Relaxed);
                    // A short batch means the outbox is caught up.
                    if usize::try_from(config.batch_size).unwrap_or(usize::MAX) > n {
                        break;
                    }
                }
                Err(ProcessError::Publish(e)) => {
                    healthy.store(false, Ordering::Relaxed);
                    tracing::warn!("fhir outbound publish stalled (broker unavailable?): {e}");
                    break;
                }
                Err(ProcessError::Db(e)) => {
                    tracing::warn!("fhir outbound DB error: {e}");
                    break;
                }
                Err(ProcessError::Map(e)) => {
                    tracing::warn!("fhir outbound reverse-mapping error (row stays pending): {e}");
                    break;
                }
            }
        }

        tokio::select! {
            _ = shutdown.changed() => {}
            () = tokio::time::sleep(poll_interval) => {}
        }
    }
    tracing::debug!("fhir outbound emitter loop exited");
}

/// Process one batch of outbox rows past the cursor in `seq` order, publishing a
/// FHIR resource for every matching COMPOSITION version and advancing the cursor
/// over the fully-published prefix. Returns the number of rows fully processed
/// this pass, or a [`ProcessError`] (the committed prefix's cursor advance is
/// persisted first, preserving order).
async fn process_batch(
    pool: &PgPool,
    service: &EhrbaseService,
    publisher: &dyn EventPublisher,
    config: &FhirOutboundConfig,
) -> Result<usize, ProcessError> {
    let last_seq = read_cursor(pool).await?;
    let rows =
        sqlx::query("SELECT seq, envelope FROM event_outbox WHERE seq > $1 ORDER BY seq LIMIT $2")
            .bind(last_seq)
            .bind(config.batch_size)
            .fetch_all(pool)
            .await?;
    if rows.is_empty() {
        return Ok(0);
    }

    let mut processed = 0usize;
    let mut advanced = last_seq;
    let mut outcome: Result<usize, ProcessError> = Ok(0);
    'rows: for row in &rows {
        let seq: i64 = row.try_get("seq")?;
        let envelope: Value = row.try_get("envelope")?;
        let ehr_id = envelope
            .get("ehr_id")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok());

        for version in envelope
            .get("versions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            // Only COMPOSITION versions can map to a FHIR resource
            // (EHR_STATUS/FOLDER carry no mappable template). The template is
            // read from the COMPOSITION body by the service (the envelope's
            // template_id is currently NULL — see the service PORT NOTE).
            if version.get("kind").and_then(Value::as_str) != Some("COMPOSITION") {
                continue;
            }
            let (Some(vo_id), Some(sys_version)) = (
                version
                    .get("vo_id")
                    .and_then(Value::as_str)
                    .and_then(|s| Uuid::parse_str(s).ok()),
                version
                    .get("sys_version")
                    .and_then(Value::as_i64)
                    .and_then(|v| i32::try_from(v).ok()),
            ) else {
                continue;
            };

            let messages = match service
                .fhir_outbound_messages(ehr_id, vo_id, sys_version)
                .await
            {
                Ok(m) => m,
                Err(e) => {
                    outcome = Err(ProcessError::Map(e.to_string()));
                    break 'rows;
                }
            };
            for (resource_type, template_id, resource) in &messages {
                let routing_key = routing_key(resource_type, template_id);
                let payload = serde_json::to_vec(resource).unwrap_or_default();
                if let Err(e) = publish_with_retry(publisher, &routing_key, &payload, config).await
                {
                    outcome = Err(ProcessError::Publish(e));
                    break 'rows;
                }
            }
        }
        // The whole row published: it is safe to advance the cursor past it.
        advanced = seq;
        processed += 1;
    }

    // Persist the fully-published prefix's cursor even on a mid-batch failure, so
    // a re-run resumes from the last completed row (at-least-once).
    if advanced > last_seq {
        write_cursor(pool, advanced).await?;
    }
    match outcome {
        Err(e) => Err(e),
        Ok(_) => Ok(processed),
    }
}

/// Read the emitter's delivery cursor (`0` when unset).
async fn read_cursor(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let last: Option<i64> = sqlx::query_scalar("SELECT last_seq FROM fhir_outbound_cursor")
        .fetch_optional(pool)
        .await?;
    Ok(last.unwrap_or(0))
}

/// Advance the emitter's delivery cursor to `seq`.
async fn write_cursor(pool: &PgPool, seq: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE fhir_outbound_cursor SET last_seq = $1")
        .bind(seq)
        .execute(pool)
        .await?;
    Ok(())
}

/// The topic routing key for one emitted FHIR resource:
/// `<resource_type>.<template_id>` on the PHI exchange, with both segments
/// sanitised to single topic words (a dotted OPT id must not split the key).
fn routing_key(resource_type: &str, template_id: &str) -> String {
    format!("{}.{}", sanitize(resource_type), sanitize(template_id))
}

/// Collapse any AMQP-topic-hostile character (dots, spaces, …) to `_`.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Publish with exponential backoff, up to `publish_max_retries` extra attempts.
async fn publish_with_retry(
    publisher: &dyn EventPublisher,
    routing_key: &str,
    payload: &[u8],
    config: &FhirOutboundConfig,
) -> Result<(), EventError> {
    (|| async { publisher.publish(routing_key, payload).await })
        .retry(ExponentialBuilder::default().with_max_times(config.publish_max_retries))
        .notify(|e, d| tracing::warn!("fhir outbound publish retry in {d:?}: {e}"))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_key_sanitises_both_segments() {
        assert_eq!(
            routing_key("Observation", "minimal_evaluation.en.v1"),
            "Observation.minimal_evaluation_en_v1"
        );
    }
}
