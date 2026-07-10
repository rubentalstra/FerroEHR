//! End-to-end contribution-outbox eventing against a **real broker**
//! (testcontainers `RabbitMQ`) + a real `PostgreSQL` 18 — the broker half of
//! ADR-014 (task 3/5b).
//!
//! Proves: (1) a committed contribution is published to the topic exchange and
//! is consumable from a bound queue, with the expected routing key and a
//! PHI-free envelope; (2) at-least-once with no loss under a broker outage —
//! while the broker is unreachable the outbox rows stay pending, and once a
//! publisher can reach the broker every row is delivered.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use std::time::Duration;

use futures::StreamExt as _;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties, ExchangeKind};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection as _, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::rabbitmq::RabbitMq;

use ehrbase::db::{self, DbSettings};
use ehrbase::events::{EventsConfig, start};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::types::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{EhrCompositionService, EhrService};
use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

const EXCHANGE: &str = "ehrbase.events";

// ── containers ───────────────────────────────────────────────────────────────

async fn migrated_pool(pg: &ContainerAsync<Postgres>, name: &str) -> PgPool {
    let host = pg.get_host().await.expect("pg host").to_string();
    let port = pg.get_host_port_ipv4(5432).await.expect("pg port");
    let admin = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let mut conn = PgConnection::connect(&admin).await.expect("admin connect");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut conn)
        .await
        .expect("create db");
    let settings = DbSettings::new(format!("postgres://postgres:postgres@{host}:{port}/{name}"));
    let pool = db::connect(&settings).await.expect("pool");
    db::run_migrations(&pool).await.expect("migrate");
    pool
}

async fn amqp_url(rmq: &ContainerAsync<RabbitMq>) -> String {
    let host = rmq.get_host().await.expect("rmq host").to_string();
    let port = rmq.get_host_port_ipv4(5672).await.expect("rmq amqp port");
    format!("amqp://guest:guest@{host}:{port}/%2f")
}

// ── fixtures ─────────────────────────────────────────────────────────────────

fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn uv(data: Value, change_code: &str) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: None,
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: None,
            committer: serde_json::from_value::<PartyProxy>(
                json!({ "_type": "PARTY_IDENTIFIED", "name": "event tester" }),
            )
            .expect("committer"),
        },
        signature: None,
    }
}

fn composition(name: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": name },
        "language": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" },
            "code_string": "en"
        },
        "territory": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" },
            "code_string": "NL"
        },
        "category": {
            "_type": "DV_CODED_TEXT", "value": "event",
            "defining_code": { "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "433" }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "secret clinician name" }
    })
}

async fn pending_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ehr.event_outbox WHERE published_at IS NULL")
        .fetch_one(pool)
        .await
        .expect("pending count")
}

fn events_config(url: String) -> EventsConfig {
    EventsConfig {
        enabled: true,
        url,
        exchange: EXCHANGE.to_owned(),
        poll_interval_ms: 50,
        publish_max_retries: 1,
        prune_interval_secs: 3_600,
        ..EventsConfig::default()
    }
}

/// Declare the durable topic exchange + a queue bound to `binding_key`, and open
/// a consumer on it. Returns the live connection (kept alive by the caller) and
/// the consumer.
async fn bound_consumer(url: &str, binding_key: &str) -> (Connection, lapin::Consumer) {
    let conn = Connection::connect(url, ConnectionProperties::default())
        .await
        .expect("consumer connect");
    let channel = conn.create_channel().await.expect("consumer channel");
    channel
        .exchange_declare(
            EXCHANGE.into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..ExchangeDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("declare exchange");
    let queue = channel
        .queue_declare(
            "test-events".into(),
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("declare queue");
    channel
        .queue_bind(
            queue.name().as_str().into(),
            EXCHANGE.into(),
            binding_key.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("bind queue");
    let consumer = channel
        .basic_consume(
            queue.name().as_str().into(),
            "test-consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("consume");
    (conn, consumer)
}

/// Consume the next delivery within a timeout, returning (`routing_key`, body).
async fn next_delivery(consumer: &mut lapin::Consumer) -> (String, Value) {
    let delivery = tokio::time::timeout(Duration::from_secs(20), consumer.next())
        .await
        .expect("a delivery within the timeout")
        .expect("consumer stream not closed")
        .expect("delivery ok");
    let routing_key = delivery.routing_key.as_str().to_owned();
    let body: Value = serde_json::from_slice(&delivery.data).expect("json body");
    delivery.ack(BasicAckOptions::default()).await.expect("ack");
    (routing_key, body)
}

fn assert_phi_free(env: &Value) {
    let text = serde_json::to_string(env).expect("serialize");
    for forbidden in [
        "composer",
        "secret clinician name",
        "archetype_node_id",
        "DV_TEXT",
    ] {
        assert!(!text.contains(forbidden), "leaked clinical content: {text}");
    }
    assert!(env.get("versions").is_some(), "envelope carries versions");
    assert!(env.get("seq").is_some(), "published payload carries seq");
}

// ── tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn end_to_end_publish_and_consume() {
    let pg = Postgres::default()
        .with_tag("18")
        .start()
        .await
        .expect("start postgres:18 (is Docker running?)");
    let rmq = RabbitMq::default()
        .start()
        .await
        .expect("start rabbitmq (is Docker running?)");
    let pool = migrated_pool(&pg, "amqp_e2e").await;
    let url = amqp_url(&rmq).await;

    // Bind a consumer for COMPOSITION events BEFORE publishing (a topic exchange
    // drops unroutable messages), so the composition event lands in the queue.
    let (_conn, mut consumer) = bound_consumer(&url, "COMPOSITION.#").await;

    // Commit an EHR (→ EHR_STATUS event, not COMPOSITION) then a composition.
    let svc = EhrbaseService::new(pool.clone());
    let ehr = svc.create_ehr(None).await.expect("create_ehr");
    svc.create_composition(ehr, uv(composition("v1"), "249"))
        .await
        .expect("create_composition");

    // Start the real publisher; it drains + publishes to the broker.
    let handle = start(events_config(url), pool.clone());

    // The bound queue receives exactly the composition event.
    let (routing_key, body) = next_delivery(&mut consumer).await;
    assert_eq!(
        routing_key, "COMPOSITION.249.-",
        "routing key is <kind>.<change_type>.<template|-> (ADR-014 §5)"
    );
    assert_phi_free(&body);
    assert_eq!(body["versions"][0]["kind"], json!("COMPOSITION"));
    assert_eq!(body["ehr_id"], json!(ehr.to_string()));

    // Everything drained.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(pending_count(&pool).await, 0, "all rows drained");

    handle.shutdown(Duration::from_secs(3)).await;
}

#[tokio::test]
async fn broker_down_then_up_delivers_without_loss() {
    let pg = Postgres::default()
        .with_tag("18")
        .start()
        .await
        .expect("start postgres:18 (is Docker running?)");
    let rmq = RabbitMq::default()
        .start()
        .await
        .expect("start rabbitmq (is Docker running?)");
    let pool = migrated_pool(&pg, "amqp_resilience").await;
    let url = amqp_url(&rmq).await;

    // Bind a catch-all consumer before any publish.
    let (_conn, mut consumer) = bound_consumer(&url, "#").await;

    // Commit an EHR + a composition ⇒ two pending outbox rows.
    let svc = EhrbaseService::new(pool.clone());
    let ehr = svc.create_ehr(None).await.expect("create_ehr");
    svc.create_composition(ehr, uv(composition("v1"), "249"))
        .await
        .expect("create_composition");
    let committed = pending_count(&pool).await;
    assert_eq!(committed, 2, "EHR creation + composition ⇒ two rows");

    // Broker "down": a publisher pointed at a dead port cannot deliver; rows
    // stay pending (the outbox buffers — ADR-014 §3).
    let bad_url = "amqp://guest:guest@127.0.0.1:1/%2f".to_owned();
    let down = start(events_config(bad_url), pool.clone());
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        pending_count(&pool).await,
        committed,
        "no row drains while the broker is unreachable"
    );
    down.shutdown(Duration::from_secs(3)).await;

    // Broker up: a correctly-configured publisher drains every row (no loss).
    let up = start(events_config(url), pool.clone());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        if pending_count(&pool).await == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "did not drain in time"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Both events arrive on the queue (no loss).
    for _ in 0..committed {
        let (routing_key, body) = next_delivery(&mut consumer).await;
        assert!(routing_key.contains('.'), "topic-shaped key: {routing_key}");
        assert_phi_free(&body);
    }

    up.shutdown(Duration::from_secs(3)).await;
}
