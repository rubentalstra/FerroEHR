#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::expect_used,
    clippy::unwrap_used,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end test of the RESTful-ATNA feed (IHE ITI-20 **ATX: FHIR Feed**):
//! events emitted through the sender land in the local store, and the outbox
//! worker POSTs each stored FHIR R4 `AuditEvent` to the ARR
//! (`{url}/AuditEvent`, `application/fhir+json`) and stamps
//! `delivered_fhir_feed_at` — a real `PostgreSQL` 18 (testcontainers) plus a
//! `wiremock` ARR.

use std::time::Duration;

use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ehrbase::db::{self, DbConfig};
use ehrbase::system_log::config::{AuditConfig, FhirFeedConfig, StoreConfig};
use ehrbase::system_log::event::{
    AuditEvent, EventActionCode, EventOutcome, EventType, ObjectClass,
};
use ehrbase::system_log::sender;

struct Pg {
    _container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    async fn start() -> Self {
        let container = Postgres::default()
            .with_tag("18")
            .start()
            .await
            .expect("start postgres:18 (is Docker running?)");
        let host = container.get_host().await.expect("host").to_string();
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        Self {
            _container: container,
            host,
            port,
        }
    }

    async fn migrated_pool(&self, name: &str) -> PgPool {
        let admin = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            self.host, self.port
        );
        let mut conn = PgConnection::connect(&admin).await.expect("admin connect");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("create db");
        let settings = DbConfig::new(format!(
            "postgres://postgres:postgres@{}:{}/{name}",
            self.host, self.port
        ));
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrate");
        pool
    }
}

fn config(arr_url: &str) -> AuditConfig {
    AuditConfig {
        enabled: true,
        store: StoreConfig {
            enabled: true,
            retention_days: 0,
        },
        fhir_feed: FhirFeedConfig {
            enabled: true,
            url: ehrbase::config::secret::SecretUrl::new(arr_url),
            batch_size: 16,
            poll_interval_ms: 100,
            max_retries: 1,
        },
        ..AuditConfig::default()
    }
}

fn event() -> AuditEvent {
    let mut e = AuditEvent::new(
        EventActionCode::Read,
        ObjectClass::Composition,
        EventOutcome::Success,
    );
    "alice".clone_into(&mut e.user_id);
    e.object_id = Some("8fa1::ehrbase::1".to_owned());
    e.event_type = Some(EventType::RestOperation("composition_get"));
    e
}

/// Poll until `condition` returns true or the deadline passes.
async fn eventually<F, Fut>(mut condition: F, deadline: Duration) -> bool
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if condition().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn stored_records_are_fed_to_the_fhir_arr_and_stamped() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("audit_feed_ok").await;

    let arr = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/fhir/AuditEvent"))
        .and(header("content-type", "application/fhir+json"))
        .respond_with(ResponseTemplate::new(201))
        .expect(2..)
        .mount(&arr)
        .await;

    let cfg = config(&format!("{}/fhir", arr.uri()));
    let (sender, handle) = sender::start(cfg, None, Some(pool.clone()))
        .await
        .expect("start");

    for _ in 0..2 {
        let _ = sender.emit(event());
    }

    // Both rows are stored and, via the outbox worker, delivered + stamped.
    let delivered = eventually(
        || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM audit.audit_event WHERE delivered_fhir_feed_at IS NOT NULL",
            )
            .fetch_one(&pool)
            .await
            .unwrap_or(0)
                == 2
        },
        Duration::from_secs(10),
    )
    .await;
    assert!(delivered, "both records delivered to the ARR and stamped");

    drop(sender);
    handle.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn arr_outage_leaves_rows_pending_then_delivers_on_recovery() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("audit_feed_outage").await;

    let arr = MockServer::start().await;
    // Outage first: the ARR answers 503; the row must stay pending (durable).
    Mock::given(method("POST"))
        .and(path("/fhir/AuditEvent"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(4)
        .mount(&arr)
        .await;
    Mock::given(method("POST"))
        .and(path("/fhir/AuditEvent"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&arr)
        .await;

    let cfg = config(&format!("{}/fhir", arr.uri()));
    let (sender, handle) = sender::start(cfg, None, Some(pool.clone()))
        .await
        .expect("start");
    let _ = sender.emit(event());

    // Stored immediately (durability anchor) even while the ARR is down…
    let stored = eventually(
        || async {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit.audit_event")
                .fetch_one(&pool)
                .await
                .unwrap_or(0)
                == 1
        },
        Duration::from_secs(10),
    )
    .await;
    assert!(stored, "the record is stored while the ARR is down");

    // …and delivered + stamped once the ARR recovers (the 503 mock expires).
    let delivered = eventually(
        || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM audit.audit_event WHERE delivered_fhir_feed_at IS NOT NULL",
            )
            .fetch_one(&pool)
            .await
            .unwrap_or(0)
                == 1
        },
        Duration::from_secs(15),
    )
    .await;
    assert!(delivered, "the pending record ships after ARR recovery");

    drop(sender);
    handle.shutdown(Duration::from_secs(2)).await;
}
