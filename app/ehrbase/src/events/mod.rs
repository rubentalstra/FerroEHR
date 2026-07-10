//! Contribution-outbox eventing (ADR-014): drain the transactional
//! `event_outbox` (written by `service::vobject` in the same transaction as
//! every CONTRIBUTION commit) and publish each PHI-free event to a broker.
//!
//! ## Design (ADR-014)
//! - **At-least-once, per-EHR ordered.** A single background drainer reads
//!   pending rows in sequence order, publishes with broker confirms, and only
//!   then stamps `published_at`. A crash/retry may duplicate (consumers
//!   deduplicate on `contribution_id`), never lose. The outbox is the buffer
//!   when the broker is down — commits never block on the broker (§3).
//! - **PHI-free.** The payload is the stored envelope (contribution id,
//!   `ehr_id`, `committed_at`, per-version `(vo_id, kind, sys_version,
//!   change_type, template_id)`) plus the delivery `seq`. No clinical content
//!   (§2).
//! - **Broker abstraction, AMQP first.** [`EventPublisher`] is the seam;
//!   [`AmqpPublisher`] is the `RabbitMQ` (lapin) implementation. Publishing is
//!   **off by default** ([`EventsConfig::enabled`]).
//! - **Retention.** Published rows are pruned after a configurable window
//!   (default 7 days, §6).
//!
//! ## Module map
//! - [`config`] — the `figment` [`EventsConfig`].
//! - [`amqp`] — the lapin [`AmqpPublisher`].
//! - [`publisher`] — the drainer task + retention pruner + [`EventsHandle`].

pub mod config;

mod amqp;
mod publisher;

use async_trait::async_trait;

pub use amqp::AmqpPublisher;
pub use config::EventsConfig;
pub use publisher::{EventsHandle, start, start_with_publisher};

/// Placeholder routing-key segment for an absent value (ADR-014 §5).
const ABSENT: &str = "-";

/// An eventing failure — either the broker transport or a negative confirm.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    /// The AMQP transport failed (connect, channel, publish).
    #[error("amqp transport: {0}")]
    Amqp(#[from] lapin::Error),
    /// The broker negatively acknowledged the publish (could not route/store);
    /// the row stays pending for retry (at-least-once).
    #[error("broker nacked publish (routing key {0})")]
    Nack(String),
}

/// The broker-publish seam (ADR-014 §4). AMQP is the first implementation
/// ([`AmqpPublisher`]); Kafka would be another impl of this same trait.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish one event `payload` under `routing_key`, resolving only after the
    /// broker confirms. An `Err` leaves the outbox row pending for retry.
    async fn publish(&self, routing_key: &str, payload: &[u8]) -> Result<(), EventError>;
}

/// Build the topic routing key for an event's **primary** version (ADR-014 §5):
/// `<kind>.<change_type>.<template_id|->` on the topic exchange.
///
/// PORT NOTE (ADR-014 §5): AMQP topic keys use `.` as the word separator, so a
/// `template_id` containing dots (e.g. `openEHR-EHR-COMPOSITION.encounter.v1`)
/// is sanitised — every non-`[A-Za-z0-9_-]` char collapses to `_` — to keep the
/// key exactly three fields. An absent/empty `template_id` renders as `-`.
///
/// PORT NOTE (ADR-014 §5): a CONTRIBUTION may carry several versions of
/// differing kinds; the ADR's payload is one per-contribution message, so the
/// routing key is derived from the **first** version. Multi-version fan-out at
/// the routing layer is deferred with the subscription store (task 4).
#[must_use]
pub fn routing_key(kind: &str, change_type: &str, template_id: Option<&str>) -> String {
    let template = template_id
        .filter(|s| !s.is_empty())
        .map_or_else(|| ABSENT.to_owned(), sanitize_segment);
    format!("{kind}.{change_type}.{template}")
}

/// Derive the routing key from a stored envelope's first version entry.
fn routing_key_of(envelope: &serde_json::Value) -> String {
    let first = envelope
        .get("versions")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first());
    match first {
        Some(v) => {
            let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("UNKNOWN");
            let change = v
                .get("change_type")
                .and_then(|c| c.as_str())
                .unwrap_or(ABSENT);
            let template = v.get("template_id").and_then(|t| t.as_str());
            routing_key(kind, change, template)
        }
        None => routing_key("UNKNOWN", ABSENT, None),
    }
}

/// Collapse any AMQP-topic-hostile character (dots, spaces, …) to `_` so a
/// value stays a single routing-key word.
fn sanitize_segment(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn routing_key_shape() {
        assert_eq!(
            routing_key("COMPOSITION", "249", Some("template_x")),
            "COMPOSITION.249.template_x"
        );
    }

    #[test]
    fn routing_key_absent_template_is_dash() {
        assert_eq!(routing_key("EHR_STATUS", "251", None), "EHR_STATUS.251.-");
        assert_eq!(routing_key("FOLDER", "523", Some("")), "FOLDER.523.-");
    }

    #[test]
    fn routing_key_sanitises_dotted_template() {
        // A dotted OPT id must not split the three-field key.
        assert_eq!(
            routing_key(
                "COMPOSITION",
                "249",
                Some("openEHR-EHR-COMPOSITION.encounter.v1")
            ),
            "COMPOSITION.249.openEHR-EHR-COMPOSITION_encounter_v1"
        );
    }

    #[test]
    fn routing_key_of_reads_first_version() {
        let envelope = json!({
            "contribution_id": "c",
            "versions": [
                { "kind": "COMPOSITION", "change_type": "251", "template_id": "vitals.v2" },
                { "kind": "EHR_STATUS", "change_type": "251", "template_id": null }
            ]
        });
        assert_eq!(routing_key_of(&envelope), "COMPOSITION.251.vitals_v2");
    }

    #[test]
    fn routing_key_of_handles_null_template() {
        let envelope = json!({
            "versions": [ { "kind": "EHR_STATUS", "change_type": "249", "template_id": null } ]
        });
        assert_eq!(routing_key_of(&envelope), "EHR_STATUS.249.-");
    }
}
