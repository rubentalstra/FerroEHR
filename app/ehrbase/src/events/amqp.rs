//! The AMQP 0.9.1 (`RabbitMQ`) [`EventPublisher`] via `lapin` (ADR-014 §4).
//!
//! Publishes to a **durable topic exchange** with **publisher confirms**: a
//! publish resolves only after the broker acknowledges, so the drainer marks a
//! row published exactly when delivery is guaranteed (at-least-once). The
//! connection + channel are established lazily and re-established on loss, so a
//! broker that is down at start (or restarts) is tolerated — the outbox simply
//! stays pending until the broker returns (ADR-014 §3).

use async_trait::async_trait;
use lapin::options::{
    BasicPublishOptions, ConfirmSelectOptions, ExchangeDeclareOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind};
use tokio::sync::Mutex;

use super::{EventError, EventPublisher};

/// Persistent AMQP delivery mode (survives a broker restart on a durable queue).
const DELIVERY_MODE_PERSISTENT: u8 = 2;

/// A lazily-connecting `RabbitMQ` publisher. Cheap to construct (no I/O); the
/// first [`publish`](AmqpPublisher::publish) opens the connection + channel and
/// declares the exchange, and any later loss triggers a transparent reconnect.
#[derive(Debug)]
pub struct AmqpPublisher {
    url: String,
    exchange: String,
    /// The live connection + channel, or `None` until first use / after a loss.
    /// The [`Connection`] is retained so its background I/O task stays alive for
    /// as long as the channel is in use.
    conn: Mutex<Option<(Connection, Channel)>>,
}

impl AmqpPublisher {
    /// Construct over the effective (TLS-resolved) broker URL and exchange.
    /// Performs no I/O.
    #[must_use]
    pub fn new(url: impl Into<String>, exchange: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            exchange: exchange.into(),
            conn: Mutex::new(None),
        }
    }

    /// Return a connected channel, (re)connecting + (re)declaring the exchange
    /// when there is none or the current one has dropped.
    async fn channel(&self) -> Result<Channel, EventError> {
        let mut guard = self.conn.lock().await;
        if let Some((_conn, channel)) = guard.as_ref()
            && channel.status().connected()
        {
            return Ok(channel.clone());
        }
        // Reconnect: drop any stale handle first, then establish fresh.
        *guard = None;
        let conn = Connection::connect(&self.url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await?;
        channel
            .exchange_declare(
                self.exchange.as_str().into(),
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;
        let handle = channel.clone();
        *guard = Some((conn, channel));
        Ok(handle)
    }
}

#[async_trait]
impl EventPublisher for AmqpPublisher {
    async fn publish(&self, routing_key: &str, payload: &[u8]) -> Result<(), EventError> {
        let channel = self.channel().await?;
        let confirm = channel
            .basic_publish(
                self.exchange.as_str().into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(DELIVERY_MODE_PERSISTENT),
            )
            .await?
            .await?;
        if confirm.is_nack() {
            return Err(EventError::Nack(routing_key.to_owned()));
        }
        Ok(())
    }

    async fn declare_subscription(&self, queue: &str, binding_key: &str) -> Result<(), EventError> {
        // Idempotent: a durable queue survives a broker restart (matching the
        // persistent delivery mode), and re-declaring an existing queue/binding
        // with the same arguments is a no-op (ADR-014 §5).
        let channel = self.channel().await?;
        channel
            .queue_declare(
                queue.into(),
                QueueDeclareOptions {
                    durable: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;
        channel
            .queue_bind(
                queue.into(),
                self.exchange.as_str().into(),
                binding_key.into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(())
    }
}
