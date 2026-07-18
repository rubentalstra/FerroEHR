//! The non-blocking audit sender: a bounded `tokio::mpsc` drained by a
//! background task that renders each record once and fans it out to the
//! enabled sinks.
//!
//! The request path only [`AuditSender::emit`]s (a `try_send`, never blocking
//! or awaiting). The drain task optionally enriches the patient subject
//! (background only — never on the request path), renders the record — the
//! FHIR R4 `AuditEvent` (IHE BALP, [`super::fhir`]) and/or the DICOM PS3.15
//! §A.5 XML ([`super::message`]) — and delivers it:
//!
//! - **store** (`[audit.store]`, the durability anchor): the record is
//!   `INSERTed` into the local Audit Record Repository first, with bounded
//!   retries; a persistent write failure marks the store unhealthy, which
//!   under `fail_mode = closed` makes every subsequent auditable request
//!   answer `503` until a write succeeds again (prospective "no un-audited
//!   PHI access" — the emit path is non-blocking, so the failing record's
//!   own request has already been answered).
//! - **syslog** (`[audit.syslog]`): the classic ITI-20 feed, sent in-drain;
//!   with the store on, a successful send stamps `delivered_syslog_at`.
//! - **`fhir_feed`** (`[audit.fhir_feed]`, ITI-20 ATX:FHIR Feed): with the
//!   store on, a dedicated outbox worker polls undelivered rows
//!   (`delivered_fhir_feed_at IS NULL`) and POSTs them — a down ARR loses
//!   nothing; with the store off it ships in-drain with bounded retries.
//!
//! Failures are fail-open (drop + `tracing` + a `metrics` counter) or
//! fail-closed (the REST layer returns `503`); see [`EmitOutcome`].

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use sqlx::PgPool;
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinHandle;

use super::event::{AuditEvent, EmitOutcome};

use crate::system_log::AuditError;
use crate::system_log::config::{AuditConfig, FailMode, FhirFeedConfig};
use crate::system_log::fhir;
use crate::system_log::message::{AuditContext, AuditMessage};
use crate::system_log::store::AuditStore;
use crate::system_log::syslog::{Transport, assemble_syslog};

/// A background-only, indexed lookup of the EHR's patient subject id
/// (`ehr.subject_id`). Supplied by the binary (keeps the request path
/// DB-free); given an `ehr_id`, resolves the subject id or `None`.
pub type SubjectResolver =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>;

/// `metrics` counter names.
pub const METRIC_EMITTED: &str = "atna_audit_emitted_total";
/// Records dropped because the queue was full or the drain stopped.
pub const METRIC_DROPPED: &str = "atna_audit_dropped_total";
/// Auditable operations rejected under `fail_mode = closed` (queue full, or
/// the local store is unhealthy). Each rejection is a `503` at the REST layer.
pub const METRIC_REJECTED: &str = "atna_audit_rejected_total";
/// Records dropped in the drain because a rendering failed. SM master02
/// §Component table names the System Log as an "IHE ATNA-compliant system
/// log"; silent audit loss would undermine that, so every serialize-drop is
/// metered.
pub const METRIC_SERIALIZE_FAILED: &str = "atna_audit_serialize_failed_total";
/// Records successfully written to a sink (label `sink`:
/// `store`/`syslog`/`fhir_feed`).
pub const METRIC_SENT: &str = "atna_audit_sent_total";
/// Sink delivery failures (label `sink`, post-enqueue).
pub const METRIC_SEND_FAILED: &str = "atna_audit_send_failed_total";
/// Rows reaped by the retention job.
pub const METRIC_REAPED: &str = "atna_audit_reaped_total";

/// How often the retention reaper runs.
const REAP_INTERVAL: Duration = Duration::from_hours(1);

/// The cheaply-cloneable handle the REST layer emits through.
#[derive(Debug, Clone)]
pub struct AuditSender {
    inner: Arc<SenderInner>,
}

#[derive(Debug)]
struct SenderInner {
    tx: mpsc::Sender<AuditEvent>,
    enabled: bool,
    suppress_login_events: bool,
    fail_mode: FailMode,
    /// Whether the local store is currently accepting writes (`true` when the
    /// store is disabled — health then rides on the queue alone). Written by
    /// the drain, read by [`AuditSender::emit`] under `fail_mode = closed`.
    store_healthy: Arc<AtomicBool>,
}

impl AuditSender {
    /// Whether auditing is on (the master switch).
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.inner.enabled
    }

    /// Whether login / user-authentication success events are suppressed.
    #[must_use]
    pub fn suppress_login_events(&self) -> bool {
        self.inner.suppress_login_events
    }

    /// Enqueue an event (non-blocking). Never awaits, never blocks the request.
    ///
    /// A full queue (or a stopped drain) is metered and mapped through the
    /// configured [`FailMode`]: `open` → [`EmitOutcome::Dropped`] (the request
    /// proceeds), `closed` → [`EmitOutcome::Rejected`] (the REST layer returns
    /// `503`). Under `fail_mode = closed` an unhealthy local store also
    /// rejects — the event is still enqueued (best-effort: it delivers when
    /// the store recovers), but the operation must not be reported as having
    /// been audited.
    pub fn emit(&self, event: AuditEvent) -> EmitOutcome {
        metrics::counter!(METRIC_EMITTED).increment(1);
        let enqueued = self.inner.tx.try_send(event).is_ok();
        if !enqueued {
            metrics::counter!(METRIC_DROPPED).increment(1);
            tracing::warn!(
                "ATNA audit record dropped (queue full or drain stopped); fail_mode={:?}",
                self.inner.fail_mode
            );
        }
        match self.inner.fail_mode {
            FailMode::Open => {
                if enqueued {
                    EmitOutcome::Enqueued
                } else {
                    EmitOutcome::Dropped
                }
            }
            FailMode::Closed => {
                if !enqueued {
                    metrics::counter!(METRIC_REJECTED).increment(1);
                    return EmitOutcome::Rejected;
                }
                if !self.inner.store_healthy.load(Ordering::Relaxed) {
                    // No un-audited PHI access: the store stopped accepting
                    // writes, so auditable operations 503 until it recovers
                    // (the enqueued record delivers on recovery).
                    metrics::counter!(METRIC_REJECTED).increment(1);
                    tracing::warn!(
                        "ATNA fail-closed: local audit store unhealthy — rejecting the operation"
                    );
                    return EmitOutcome::Rejected;
                }
                EmitOutcome::Enqueued
            }
        }
    }
}

/// Owns the background tasks; the binary keeps it and drains it on shutdown.
#[derive(Debug)]
pub struct AuditHandle {
    join: JoinHandle<()>,
    /// The forwarding/retention workers (outbox poller, reaper). Aborted on
    /// shutdown after the drain flushes: their state is durable (the store),
    /// so undelivered rows simply ship on the next boot.
    workers: Vec<JoinHandle<()>>,
}

impl AuditHandle {
    /// Await the drain task, bounded by `timeout`, then stop the background
    /// workers. All [`AuditSender`] clones must be dropped first so the
    /// channel closes and the drain flushes then exits.
    pub async fn shutdown(self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.join).await {
            Ok(Ok(())) => tracing::debug!("ATNA audit drain flushed and exited"),
            Ok(Err(e)) => tracing::warn!("ATNA audit drain task panicked: {e}"),
            Err(_) => tracing::warn!("ATNA audit drain did not flush within {timeout:?}"),
        }
        for worker in self.workers {
            worker.abort();
        }
    }
}

/// The resolved sink set the drain fans out to.
struct Sinks {
    store: Option<AuditStore>,
    syslog: Option<Transport>,
    /// The in-drain FHIR feed (store OFF only; with the store on the outbox
    /// worker owns delivery).
    direct_feed: Option<FeedClient>,
    /// Wakes the outbox worker after a store insert.
    feed_notify: Option<Arc<Notify>>,
}

/// The ITI-20 ATX:FHIR Feed HTTP client.
#[derive(Clone)]
struct FeedClient {
    client: reqwest::Client,
    endpoint: String,
    max_retries: usize,
}

impl FeedClient {
    fn new(config: &FhirFeedConfig) -> Self {
        FeedClient {
            client: reqwest::Client::new(),
            endpoint: format!("{}/AuditEvent", config.url.expose().trim_end_matches('/')),
            max_retries: config.max_retries,
        }
    }

    /// POST one FHIR `AuditEvent` (ITI-20 ATX:FHIR Feed) with bounded jittered
    /// retries.
    async fn post(&self, body: &serde_json::Value) -> Result<(), AuditError> {
        let send = || async {
            let resp = self
                .client
                .post(&self.endpoint)
                .header(http::header::CONTENT_TYPE, "application/fhir+json")
                .json(body)
                .send()
                .await
                .map_err(|e| AuditError::Transport(e.to_string()))?;
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(AuditError::Transport(format!(
                    "FHIR feed answered {}",
                    resp.status()
                )))
            }
        };
        send.retry(
            ExponentialBuilder::default()
                .with_jitter()
                .with_max_times(self.max_retries),
        )
        .await
    }
}

/// Start the audit subsystem: resolve the sinks and spawn the drain plus any
/// forwarding/retention workers. `pool` backs the local store (`None` in
/// store-less setups, e.g. transport-only tests).
///
/// # Errors
/// [`AuditError`] if the syslog transport cannot be established (UDP
/// bind/connect, or a TLS config/CA problem).
pub async fn start(
    config: AuditConfig,
    resolver: Option<SubjectResolver>,
    pool: Option<PgPool>,
) -> Result<(AuditSender, AuditHandle), AuditError> {
    let syslog = if config.syslog.enabled {
        Some(
            Transport::connect(&config.syslog)
                .await
                .map_err(|e| AuditError::Transport(e.to_string()))?,
        )
    } else {
        None
    };

    let store = if config.store.enabled {
        if let Some(pool) = pool {
            Some(AuditStore::new(pool))
        } else {
            tracing::warn!(
                "ATNA audit store enabled but no database pool supplied — store sink off"
            );
            None
        }
    } else {
        None
    };

    let mut workers = Vec::new();
    let mut feed_notify = None;
    let mut direct_feed = None;
    if config.fhir_feed.enabled {
        let feed = FeedClient::new(&config.fhir_feed);
        if let Some(store) = store.clone() {
            // Outbox-durable delivery: the worker polls undelivered rows.
            let notify = Arc::new(Notify::new());
            feed_notify = Some(Arc::clone(&notify));
            workers.push(tokio::spawn(feed_outbox(
                store,
                feed,
                config.fhir_feed.clone(),
                notify,
            )));
        } else {
            direct_feed = Some(feed);
        }
    }
    if let Some(store) = store.clone()
        && config.store.retention_days > 0
    {
        workers.push(tokio::spawn(reaper(store, config.store.retention_days)));
    }

    if store.is_none() && syslog.is_none() && !config.fhir_feed.enabled {
        tracing::warn!(
            "ATNA auditing is enabled with no active sink — records will be rendered and discarded"
        );
    }

    let ctx = AuditContext {
        source_id: config.source_id.clone(),
        enterprise_site_id: config.enterprise_site_id.clone().unwrap_or_default(),
        server_ip: config.server_host.clone().unwrap_or_default(),
        value_if_missing: config.value_if_missing.clone(),
    };

    let store_healthy = Arc::new(AtomicBool::new(true));
    let (tx, rx) = mpsc::channel(config.queue_capacity.max(1));
    let sinks = Sinks {
        store,
        syslog,
        direct_feed,
        feed_notify,
    };
    let join = tokio::spawn(drain(
        rx,
        sinks,
        ctx,
        resolver,
        config.resolve_subject,
        Arc::clone(&store_healthy),
    ));

    let sender = AuditSender {
        inner: Arc::new(SenderInner {
            tx,
            enabled: config.enabled,
            suppress_login_events: config.suppress_login_events,
            fail_mode: config.fail_mode,
            store_healthy,
        }),
    };
    Ok((sender, AuditHandle { join, workers }))
}

async fn drain(
    mut rx: mpsc::Receiver<AuditEvent>,
    mut sinks: Sinks,
    ctx: AuditContext,
    resolver: Option<SubjectResolver>,
    resolve_subject: bool,
    store_healthy: Arc<AtomicBool>,
) {
    while let Some(event) = rx.recv().await {
        // Optional subject enrichment — background only, never on the request path.
        let subject = match (resolve_subject, &resolver, &event.ehr_id) {
            (true, Some(resolve), Some(ehr_id)) => resolve(ehr_id.clone()).await,
            _ => None,
        };

        // Render once; both the store and the FHIR feed consume this document.
        let rendered = fhir::to_fhir(&event, &ctx, subject.as_deref());

        // 1) The store — the durability anchor, written first.
        let mut row_id = None;
        if let Some(store) = &sinks.store {
            let insert = || async { store.insert(&event, subject.as_deref(), &rendered).await };
            match insert
                .retry(
                    ExponentialBuilder::default()
                        .with_jitter()
                        .with_max_times(2),
                )
                .await
            {
                Ok(id) => {
                    row_id = Some(id);
                    store_healthy.store(true, Ordering::Relaxed);
                    metrics::counter!(METRIC_SENT, "sink" => "store").increment(1);
                    if let Some(notify) = &sinks.feed_notify {
                        notify.notify_one();
                    }
                }
                Err(e) => {
                    store_healthy.store(false, Ordering::Relaxed);
                    metrics::counter!(METRIC_SEND_FAILED, "sink" => "store").increment(1);
                    tracing::warn!("ATNA audit store write failed: {e}");
                }
            }
        }

        // 2) The classic syslog feed (DICOM PS3.15 §A.5 XML per ITI-20).
        if let Some(transport) = &mut sinks.syslog {
            let message = AuditMessage::build(&event, &ctx, subject.as_deref());
            match message.to_xml() {
                Ok(xml) => {
                    let syslog =
                        assemble_syslog(&ctx.server_ip, &ctx.source_id, &event.timestamp, &xml);
                    match transport.send(&syslog).await {
                        Ok(()) => {
                            metrics::counter!(METRIC_SENT, "sink" => "syslog").increment(1);
                            if let (Some(store), Some(id)) = (&sinks.store, row_id) {
                                store.mark_syslog_delivered(id).await;
                            }
                        }
                        Err(e) => {
                            metrics::counter!(METRIC_SEND_FAILED, "sink" => "syslog").increment(1);
                            tracing::warn!("ATNA audit syslog send failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    metrics::counter!(METRIC_SERIALIZE_FAILED).increment(1);
                    tracing::warn!("ATNA audit message serialization failed: {e}");
                }
            }
        }

        // 3) The FHIR feed, in-drain only when the store is off (otherwise the
        //    outbox worker owns delivery).
        if let Some(feed) = &sinks.direct_feed {
            match serde_json::to_value(&rendered) {
                Ok(body) => match feed.post(&body).await {
                    Ok(()) => {
                        metrics::counter!(METRIC_SENT, "sink" => "fhir_feed").increment(1);
                    }
                    Err(e) => {
                        metrics::counter!(METRIC_SEND_FAILED, "sink" => "fhir_feed").increment(1);
                        tracing::warn!("ATNA audit FHIR feed send failed: {e}");
                    }
                },
                Err(e) => {
                    metrics::counter!(METRIC_SERIALIZE_FAILED).increment(1);
                    tracing::warn!("ATNA audit FHIR serialization failed: {e}");
                }
            }
        }
    }
    tracing::debug!("ATNA audit drain: channel closed, task exiting");
}

/// The ITI-20 ATX:FHIR Feed outbox worker: ship undelivered rows oldest-first,
/// stamping `delivered_fhir_feed_at` per success. On a delivery failure the
/// batch stops (rows stay pending — durable) and the worker backs off one poll
/// interval.
async fn feed_outbox(
    store: AuditStore,
    feed: FeedClient,
    config: FhirFeedConfig,
    notify: Arc<Notify>,
) {
    let idle = Duration::from_millis(config.poll_interval_ms.max(100));
    loop {
        let batch = match store.pending_fhir_feed(config.batch_size).await {
            Ok(batch) => batch,
            Err(e) => {
                tracing::warn!("ATNA audit feed outbox poll failed: {e}");
                tokio::time::sleep(idle).await;
                continue;
            }
        };
        if batch.is_empty() {
            tokio::select! {
                () = notify.notified() => {}
                () = tokio::time::sleep(idle) => {}
            }
            continue;
        }
        for (id, body) in batch {
            match feed.post(&body).await {
                Ok(()) => {
                    store.mark_fhir_feed_delivered(id).await;
                    metrics::counter!(METRIC_SENT, "sink" => "fhir_feed").increment(1);
                }
                Err(e) => {
                    metrics::counter!(METRIC_SEND_FAILED, "sink" => "fhir_feed").increment(1);
                    tracing::warn!("ATNA audit FHIR feed delivery failed (row stays pending): {e}");
                    tokio::time::sleep(idle).await;
                    break;
                }
            }
        }
    }
}

/// The hourly retention reaper (`[audit.store] retention_days > 0`).
async fn reaper(store: AuditStore, retention_days: u32) {
    loop {
        tokio::time::sleep(REAP_INTERVAL).await;
        match store.reap(retention_days).await {
            Ok(0) => {}
            Ok(n) => {
                metrics::counter!(METRIC_REAPED).increment(n);
                tracing::debug!("ATNA audit retention reaped {n} records");
            }
            Err(e) => tracing::warn!("ATNA audit retention reap failed: {e}"),
        }
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
    use crate::system_log::config::{StoreConfig, SyslogConfig, Transport as ConfigTransport};
    use crate::system_log::event::{EventActionCode, EventOutcome, ObjectClass};

    fn udp_config() -> AuditConfig {
        AuditConfig {
            enabled: true,
            // No DB pool in these tests — exercise the transport path alone.
            store: StoreConfig {
                enabled: false,
                retention_days: 0,
            },
            syslog: SyslogConfig {
                enabled: true,
                transport: ConfigTransport::Udp,
                // A port nothing listens on is fine: connected UDP send does
                // not fail.
                host: "127.0.0.1".to_owned(),
                port: 1,
                ..SyslogConfig::default()
            },
            queue_capacity: 8,
            ..AuditConfig::default()
        }
    }

    fn event() -> AuditEvent {
        AuditEvent::new(
            EventActionCode::Create,
            ObjectClass::Ehr,
            EventOutcome::Success,
        )
    }

    fn bare_sender(fail_mode: FailMode, capacity: usize, store_healthy: bool) -> AuditSender {
        let (tx, rx) = mpsc::channel(capacity);
        // Keep the receiver alive but unread so the queue can fill.
        std::mem::forget(rx);
        AuditSender {
            inner: Arc::new(SenderInner {
                tx,
                enabled: true,
                suppress_login_events: true,
                fail_mode,
                store_healthy: Arc::new(AtomicBool::new(store_healthy)),
            }),
        }
    }

    #[tokio::test]
    async fn emit_enqueues_and_drains_on_shutdown() {
        let (sender, handle) = start(udp_config(), None, None).await.expect("start");
        assert!(sender.enabled());
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        drop(sender);
        handle.shutdown(Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn fail_open_drops_when_full() {
        let sender = bare_sender(FailMode::Open, 1, true);
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        assert_eq!(sender.emit(event()), EmitOutcome::Dropped);
    }

    #[tokio::test]
    async fn fail_closed_rejects_when_full() {
        let sender = bare_sender(FailMode::Closed, 1, true);
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        assert_eq!(sender.emit(event()), EmitOutcome::Rejected);
    }

    #[tokio::test]
    async fn fail_closed_rejects_while_store_unhealthy() {
        // The event is still enqueued (delivers on recovery) but the
        // operation is rejected: no un-audited PHI access.
        let sender = bare_sender(FailMode::Closed, 8, false);
        assert_eq!(sender.emit(event()), EmitOutcome::Rejected);
    }

    #[tokio::test]
    async fn fail_open_ignores_store_health() {
        let sender = bare_sender(FailMode::Open, 8, false);
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
    }
}
