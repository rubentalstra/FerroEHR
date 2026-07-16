//! The non-blocking audit sender: a bounded `tokio::mpsc` drained by a
//! background task that renders + ships each record.
//!
//! The request path only [`AuditSender::emit`]s (a `try_send`, never blocking
//! or awaiting). The drain task owns the socket, optionally enriches the patient
//! subject (background only — never on the request path), renders the DICOM
//! `AuditMessage`, frames it (RFC 5424 / 5425) and sends it. Failures are
//! fail-open (drop + `tracing` + a `metrics` counter) or fail-closed (the REST
//! layer returns `503`); see [`EmitOutcome`].

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::event::{AuditEvent, EmitOutcome};

use crate::system_log::AuditError;
use crate::system_log::config::{AuditConfig, FailMode};
use crate::system_log::message::{AuditContext, AuditMessage};
use crate::system_log::syslog::{Transport, assemble_syslog};

/// A background-only, indexed lookup of the EHR's patient subject id
/// (`ehr.subject_id`). Supplied by the binary (keeps this crate DB-free); given
/// an `ehr_id`, resolves the subject id or `None`.
pub type SubjectResolver =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>;

/// `metrics` counter names.
pub const METRIC_EMITTED: &str = "atna_audit_emitted_total";
/// Records dropped because the queue was full or the drain stopped.
pub const METRIC_DROPPED: &str = "atna_audit_dropped_total";
/// Records dropped in the drain because the ATNA message failed to serialize to
/// XML. SM master02 §Component table names the System Log as an "IHE
/// ATNA-compliant system log"; silent audit loss would undermine that, so every
/// serialize-drop is metered (W-14 F-20).
pub const METRIC_SERIALIZE_FAILED: &str = "atna_audit_serialize_failed_total";
/// Records successfully written to the transport.
pub const METRIC_SENT: &str = "atna_audit_sent_total";
/// Transport send failures (post-enqueue).
pub const METRIC_SEND_FAILED: &str = "atna_audit_send_failed_total";

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
}

impl AuditSender {
    /// Whether auditing is on (the master switch).
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.inner.enabled
    }

    /// Whether login / application-activity events are suppressed.
    #[must_use]
    pub fn suppress_login_events(&self) -> bool {
        self.inner.suppress_login_events
    }

    /// Enqueue an event (non-blocking). Never awaits, never blocks the request.
    /// A full queue (or a stopped drain) is metered and mapped through the
    /// configured [`FailMode`]: `open` → [`EmitOutcome::Dropped`] (the request
    /// proceeds), `closed` → [`EmitOutcome::Rejected`] (the REST layer returns
    /// `503`).
    pub fn emit(&self, event: AuditEvent) -> EmitOutcome {
        metrics::counter!(METRIC_EMITTED).increment(1);
        if let Ok(()) = self.inner.tx.try_send(event) {
            EmitOutcome::Enqueued
        } else {
            metrics::counter!(METRIC_DROPPED).increment(1);
            tracing::warn!(
                "ATNA audit record dropped (queue full or drain stopped); fail_mode={:?}",
                self.inner.fail_mode
            );
            match self.inner.fail_mode {
                FailMode::Open => EmitOutcome::Dropped,
                FailMode::Closed => EmitOutcome::Rejected,
            }
        }
    }
}

/// Owns the drain task; the binary keeps it and drains it on shutdown.
#[derive(Debug)]
pub struct AuditHandle {
    join: JoinHandle<()>,
}

impl AuditHandle {
    /// Await the drain task, bounded by `timeout`. All [`AuditSender`] clones
    /// must be dropped first so the channel closes and the drain flushes then
    /// exits.
    pub async fn shutdown(self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.join).await {
            Ok(Ok(())) => tracing::debug!("ATNA audit drain flushed and exited"),
            Ok(Err(e)) => tracing::warn!("ATNA audit drain task panicked: {e}"),
            Err(_) => tracing::warn!("ATNA audit drain did not flush within {timeout:?}"),
        }
    }
}

/// Start the audit subsystem: connect the transport and spawn the drain task.
///
/// # Errors
/// [`AuditError`] if the transport cannot be established (UDP bind/connect, or a
/// TLS config/CA problem).
pub async fn start(
    config: AuditConfig,
    resolver: Option<SubjectResolver>,
) -> Result<(AuditSender, AuditHandle), AuditError> {
    let transport = Transport::connect(&config)
        .await
        .map_err(|e| AuditError::Transport(e.to_string()))?;

    let ctx = AuditContext {
        source_id: config.source_id.clone(),
        enterprise_site_id: config.enterprise_site_id.clone().unwrap_or_default(),
        server_ip: config.server_host.clone().unwrap_or_default(),
        value_if_missing: config.value_if_missing.clone(),
    };

    let (tx, rx) = mpsc::channel(config.queue_capacity.max(1));
    let resolve_subject = config.resolve_subject;
    let join = tokio::spawn(drain(rx, transport, ctx, resolver, resolve_subject));

    let sender = AuditSender {
        inner: Arc::new(SenderInner {
            tx,
            enabled: config.enabled,
            suppress_login_events: config.suppress_login_events,
            fail_mode: config.fail_mode,
        }),
    };
    Ok((sender, AuditHandle { join }))
}

async fn drain(
    mut rx: mpsc::Receiver<AuditEvent>,
    mut transport: Transport,
    ctx: AuditContext,
    resolver: Option<SubjectResolver>,
    resolve_subject: bool,
) {
    while let Some(event) = rx.recv().await {
        // Optional subject enrichment — background only, never on the request path.
        let subject = match (resolve_subject, &resolver, &event.ehr_id) {
            (true, Some(resolve), Some(ehr_id)) => resolve(ehr_id.clone()).await,
            _ => None,
        };

        let message = AuditMessage::build(&event, &ctx, subject.as_deref());
        let xml = match message.to_xml() {
            Ok(xml) => xml,
            Err(e) => {
                // SM master02 §Component table ("IHE ATNA-compliant system
                // log"): a dropped audit record must never be silent — meter it
                // (W-14 F-20).
                metrics::counter!(METRIC_SERIALIZE_FAILED).increment(1);
                tracing::warn!("ATNA audit message serialization failed: {e}");
                continue;
            }
        };
        let syslog = assemble_syslog(&ctx.server_ip, &ctx.source_id, &event.timestamp, &xml);

        match transport.send(&syslog).await {
            Ok(()) => metrics::counter!(METRIC_SENT).increment(1),
            Err(e) => {
                metrics::counter!(METRIC_SEND_FAILED).increment(1);
                tracing::warn!("ATNA audit transport send failed: {e}");
            }
        }
    }
    tracing::debug!("ATNA audit drain: channel closed, task exiting");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_log::config::Transport as ConfigTransport;
    use crate::system_log::event::{EventActionCode, EventOutcome, ObjectClass};

    fn udp_config() -> AuditConfig {
        AuditConfig {
            enabled: true,
            transport: ConfigTransport::Udp,
            // A port nothing listens on is fine: connected UDP send does not fail.
            repository_host: "127.0.0.1".to_owned(),
            repository_port: 1,
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

    #[tokio::test]
    async fn emit_enqueues_and_drains_on_shutdown() {
        let (sender, handle) = start(udp_config(), None).await.expect("start");
        assert!(sender.enabled());
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        drop(sender);
        handle.shutdown(Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn fail_open_drops_when_full() {
        // Capacity 1, no drain consuming (we hold the receiver): the 2nd emit
        // finds the queue full. Build the sender directly to keep rx unread.
        let (tx, _rx) = mpsc::channel(1);
        let sender = AuditSender {
            inner: Arc::new(SenderInner {
                tx,
                enabled: true,
                suppress_login_events: true,
                fail_mode: FailMode::Open,
            }),
        };
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        assert_eq!(sender.emit(event()), EmitOutcome::Dropped);
    }

    #[tokio::test]
    async fn fail_closed_rejects_when_full() {
        let (tx, _rx) = mpsc::channel(1);
        let sender = AuditSender {
            inner: Arc::new(SenderInner {
                tx,
                enabled: true,
                suppress_login_events: true,
                fail_mode: FailMode::Closed,
            }),
        };
        assert_eq!(sender.emit(event()), EmitOutcome::Enqueued);
        assert_eq!(sender.emit(event()), EmitOutcome::Rejected);
    }
}
