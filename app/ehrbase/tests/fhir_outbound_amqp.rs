//! End-to-end FHIR **outbound** emitter against a real broker (testcontainers
//! `RabbitMQ`) + a real `PostgreSQL` 18
//!
//! Proves: an inbound FHIR commit (→ a COMPOSITION + an `event_outbox` row) is
//! picked up by the outbound emitter, reverse-mapped, and published to the
//! **separate** PHI exchange (`ehrbase.fhir`) as a FHIR resource — with the
//! `<resource_type>.<template>` routing key and the mapped clinical value
//! present (unlike the PHI-free E1 envelope stream). Requires Docker.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use std::sync::Arc;
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

use ehrbase::db::{self, DbConfig};
use ehrbase::extensions::fhir::{FhirOutboundConfig, start};
use ehrbase::service::EhrbaseService;

/// The separate PHI exchange the outbound emitter publishes to.
const EXCHANGE: &str = "ehrbase.fhir";
const OPT_REL: &str = "tests/resources/service/knowledge/opt/minimal_evaluation.opt";
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";
const PROFILE: &str = "http://example.org/StructureDefinition/bp";

async fn migrated_pool(pg: &ContainerAsync<Postgres>, name: &str) -> PgPool {
    let host = pg.get_host().await.expect("pg host").to_string();
    let port = pg.get_host_port_ipv4(5432).await.expect("pg port");
    let admin = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let mut conn = PgConnection::connect(&admin).await.expect("admin connect");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut conn)
        .await
        .expect("create db");
    let settings = DbConfig::new(format!("postgres://postgres:postgres@{host}:{port}/{name}"));
    let pool = db::connect(&settings).await.expect("pool");
    db::run_migrations(&pool).await.expect("migrate");
    pool
}

async fn amqp_url(rmq: &ContainerAsync<RabbitMq>) -> String {
    let host = rmq.get_host().await.expect("rmq host").to_string();
    let port = rmq.get_host_port_ipv4(5672).await.expect("rmq amqp port");
    format!("amqp://guest:guest@{host}:{port}/%2f")
}

fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn mapping_body() -> Value {
    json!({
        "name": "bp",
        "definition": {
            "resource_type": "Observation",
            "profile_url": PROFILE,
            "template_id": TEMPLATE_ID,
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "context": {
                "ctx/language": "en", "ctx/territory": "US",
                "ctx/composer_name": "fhir-connector", "ctx/time": "2026-02-03T04:05:06Z"
            },
            "entries": [
                { "openehr_path": "minimal/minimal:0/quantity",
                  "fhir_path": "valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "valueQuantity.unit" } }
            ]
        }
    })
}

fn observation() -> Value {
    json!({
        "resourceType": "Observation",
        "id": "bp-obs-1",
        "meta": { "versionId": "1", "profile": [PROFILE] },
        "status": "final",
        "subject": { "reference": "Patient/p-42" },
        "valueQuantity": { "value": 118, "unit": "kg" }
    })
}

fn outbound_config(url: String) -> FhirOutboundConfig {
    FhirOutboundConfig {
        enabled: true,
        url: ehrbase::config::secret::SecretUrl::new(url),
        exchange: EXCHANGE.to_owned(),
        poll_interval_ms: 50,
        publish_max_retries: 1,
        ..FhirOutboundConfig::default()
    }
}

/// Declare the durable topic PHI exchange + a queue bound to `binding_key`, open
/// a consumer. Returns the live connection (kept alive by the caller).
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
            "test-fhir-out".into(),
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
            "test-fhir-consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("consume");
    (conn, consumer)
}

async fn next_delivery(consumer: &mut lapin::Consumer) -> (String, Value) {
    let delivery = tokio::time::timeout(Duration::from_secs(90), consumer.next())
        .await
        .expect("a delivery within the timeout")
        .expect("consumer stream not closed")
        .expect("delivery ok");
    let routing_key = delivery.routing_key.as_str().to_owned();
    let body: Value = serde_json::from_slice(&delivery.data).expect("json body");
    delivery.ack(BasicAckOptions::default()).await.expect("ack");
    (routing_key, body)
}

#[tokio::test]
async fn commit_emits_reverse_mapped_fhir_resource() {
    let pg = Postgres::default()
        .with_tag("18")
        .start()
        .await
        .expect("start postgres:18 (is Docker running?)");
    let rmq = RabbitMq::default()
        .start()
        .await
        .expect("start rabbitmq (is Docker running?)");
    let pool = migrated_pool(&pg, "fhir_out_e2e").await;
    let url = amqp_url(&rmq).await;

    // Ingest the OPT + create the mapping.
    let svc = Arc::new(EhrbaseService::new(pool.clone()));
    svc.template_adl14_upload(fixture(OPT_REL))
        .await
        .expect("ingest OPT");
    svc.fhir_mapping_create(mapping_body())
        .await
        .expect("create mapping");

    // Bind a consumer on the PHI exchange BEFORE the emitter publishes (a topic
    // exchange drops unroutable messages).
    let (_conn, mut consumer) = bound_consumer(&url, "Observation.#").await;

    // Commit an inbound FHIR Observation → EHR + COMPOSITION + outbox rows.
    svc.fhir_ingest(
        "Observation".to_owned(),
        Some(PROFILE.to_owned()),
        observation(),
    )
    .await
    .expect("fhir_ingest commits");

    // Start the outbound emitter; it walks the outbox, reverse-maps the
    // composition, and publishes the FHIR resource.
    let handle = start(outbound_config(url), pool.clone(), svc.clone());

    let (routing_key, body) = next_delivery(&mut consumer).await;
    assert_eq!(
        routing_key, "Observation.minimal_evaluation_en_v1",
        "routing key is <resource_type>.<sanitised template>"
    );
    // The payload IS the mapped FHIR resource (clinical content, by design).
    assert_eq!(body["resourceType"], "Observation", "body: {body}");
    assert_eq!(
        body["valueQuantity"]["value"].as_f64(),
        Some(118.0),
        "mapped magnitude reverse-mapped: {body}"
    );
    assert_eq!(body["valueQuantity"]["unit"], "kg");
    assert_eq!(
        body["subject"]["reference"], "Patient/p-42",
        "subject reconstructed with strip_prefix re-applied"
    );

    // The emitter advances its cursor AFTER a successful publish (at-least-once
    // ordering: a crash in between redelivers, never loses). Delivery of the
    // message therefore precedes the cursor write — poll with a bounded
    // deadline instead of reading immediately (the instrumented coverage build
    // on shared CI runners widened that gap enough to flake a one-shot read).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let cursor: i64 = loop {
        let cursor: i64 = sqlx::query_scalar("SELECT last_seq FROM ehr.fhir_outbound_cursor")
            .fetch_one(&pool)
            .await
            .expect("cursor read");
        if cursor > 0 || tokio::time::Instant::now() >= deadline {
            break cursor;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    assert!(cursor > 0, "cursor advanced past the emitted rows");

    handle.shutdown(Duration::from_secs(3)).await;
}
