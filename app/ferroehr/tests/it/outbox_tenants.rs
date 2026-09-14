// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The outbox readers drain every tenant, with a cursor per tenant (#3355).
//!
//! `event_outbox` is tenant-scoped by row policy. The drainer used to poll it
//! with no tenant in scope, so under multi-tenancy it saw the reserved default
//! tenant's rows alone and every other tenant's events were never published.
//! Proven here against a real database with two registered tenants: the
//! drainer publishes both tenants' rows, and the prune's floor is the cursor of
//! the tenant it runs for.

#![expect(
    clippy::expect_used,
    reason = "integration tests fail loudly on harness errors"
)]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;

use ferroehr::db::{self, DbConfig};
use ferroehr::extensions::events::config::EventsConfig;
use ferroehr::extensions::events::publisher::start_with_publisher;
use ferroehr::extensions::outbox::{OutboxReader, advance, prune, reconcile};
use ferroehr::extensions::tenancy::TenantDefinition;
use ferroehr::extensions::tenant_context::{TenantContext, scope};
use ferroehr::service::FerroEhrService;
use ferroehr_ext::events::{EventError, EventPublisher};

use crate::fixtures::dsn_as;

/// Records every envelope it is handed.
#[derive(Default)]
struct Capturing {
    published: Mutex<Vec<Value>>,
}

impl Capturing {
    fn ehr_ids(&self) -> Vec<String> {
        self.published
            .lock()
            .expect("mutex")
            .iter()
            .map(|env| env["ehr_id"].as_str().expect("ehr_id").to_owned())
            .collect()
    }
}

#[async_trait]
impl EventPublisher for Capturing {
    async fn publish(&self, _routing_key: &str, payload: &[u8]) -> Result<(), EventError> {
        let env: Value = serde_json::from_slice(payload).expect("payload json");
        self.published.lock().expect("mutex").push(env);
        Ok(())
    }
}

async fn wait_until<F>(mut f: F)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if f() {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "condition not met before deadline"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_until_async<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if f().await {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "condition not met before deadline"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn tenant(svc: &FerroEhrService, name: &str) -> TenantContext {
    let record = svc
        .tenant_create(TenantDefinition {
            name: name.to_owned(),
            system_id: format!("{name}.example.invalid"),
        })
        .await
        .expect("tenant_create");
    TenantContext {
        tenant_id: record.id,
        system_id: record.system_id,
    }
}

async fn scoped_pending(pool: &PgPool, ctx: &TenantContext) -> i64 {
    scope(
        ctx.clone(),
        sqlx::query_scalar("SELECT count(*) FROM ehr.event_outbox WHERE published_at IS NULL")
            .fetch_one(pool),
    )
    .await
    .expect("count pending")
}

async fn scoped_seqs(pool: &PgPool, ctx: &TenantContext) -> Vec<i64> {
    scope(
        ctx.clone(),
        sqlx::query_scalar("SELECT seq FROM ehr.event_outbox ORDER BY seq").fetch_all(pool),
    )
    .await
    .expect("read outbox seqs")
}

#[tokio::test]
async fn the_drainer_publishes_every_tenants_rows() {
    let testdb = testkit::db().await.expect("testkit database");
    // A non-superuser app role: a superuser bypasses row-level security, and
    // the tenant scoping under test IS that policy.
    let settings = DbConfig::new(dsn_as(&testdb, "otb", "ferroehr_app").await);
    let pool = db::connect_tenant_scoped(&settings)
        .await
        .expect("tenant-scoped pool");
    let svc = FerroEhrService::new(pool.clone());

    let a = tenant(&svc, "clinic-a").await;
    let b = tenant(&svc, "clinic-b").await;
    let ehr_a = scope(a.clone(), svc.create_ehr(None))
        .await
        .expect("create_ehr in A");
    let ehr_b1 = scope(b.clone(), svc.create_ehr(None))
        .await
        .expect("create_ehr in B");
    let ehr_b2 = scope(b.clone(), svc.create_ehr(None))
        .await
        .expect("create_ehr in B");
    assert_eq!(scoped_seqs(&pool, &a).await.len(), 1);
    assert_eq!(scoped_seqs(&pool, &b).await.len(), 2);

    let publisher = Arc::new(Capturing::default());
    let dyn_publisher: Arc<dyn EventPublisher> = Arc::<Capturing>::clone(&publisher);
    let handle = start_with_publisher(
        EventsConfig {
            enabled: true,
            poll_interval_ms: 50,
            publish_max_retries: 0,
            prune_interval_secs: 3_600,
            ..EventsConfig::default()
        },
        pool.clone(),
        db::demographic_pool_from(&pool),
        dyn_publisher,
    );
    // Wait for the marks, not the publishes: the drainer marks a row after
    // publishing it, and the shutdown flush re-reads unmarked rows
    // (at-least-once), which would count a row twice here.
    wait_until(|| publisher.ehr_ids().len() >= 3).await;
    wait_until_async(|| async {
        scoped_pending(&pool, &a).await + scoped_pending(&pool, &b).await == 0
    })
    .await;
    handle.shutdown(Duration::from_secs(5)).await;

    // One message per version: an EHR creation commits EHR_STATUS and
    // EHR_ACCESS, so a row yields two envelopes for the same EHR.
    let mut seen = publisher.ehr_ids();
    seen.sort();
    seen.dedup();
    let mut expected = vec![ehr_a.to_string(), ehr_b1.to_string(), ehr_b2.to_string()];
    expected.sort();
    assert_eq!(
        seen, expected,
        "every tenant's rows are published, none twice"
    );
}

#[tokio::test]
async fn the_prune_floor_is_the_cursor_of_the_tenant_it_runs_for() {
    let testdb = testkit::db().await.expect("testkit database");
    // A non-superuser app role: a superuser bypasses row-level security, and
    // the tenant scoping under test IS that policy.
    let settings = DbConfig::new(dsn_as(&testdb, "otb", "ferroehr_app").await);
    let pool = db::connect_tenant_scoped(&settings)
        .await
        .expect("tenant-scoped pool");
    let svc = FerroEhrService::new(pool.clone());

    let a = tenant(&svc, "clinic-a").await;
    let b = tenant(&svc, "clinic-b").await;
    for _ in 0..2 {
        scope(a.clone(), svc.create_ehr(None))
            .await
            .expect("create_ehr in A");
        scope(b.clone(), svc.create_ehr(None))
            .await
            .expect("create_ehr in B");
    }
    let a_seqs = scoped_seqs(&pool, &a).await;
    let b_seqs = scoped_seqs(&pool, &b).await;
    assert_eq!((a_seqs.len(), b_seqs.len()), (2, 2));

    // Every row published long ago, in each tenant's scope (the row policy
    // bounds the update to the scoped tenant).
    for ctx in [&a, &b] {
        scope(
            ctx.clone(),
            sqlx::query("UPDATE ehr.event_outbox SET published_at = now() - interval '30 days'")
                .execute(&pool),
        )
        .await
        .expect("back-date");
    }

    // The reader is active in both tenants; in A it has passed the first row,
    // in B nothing.
    for ctx in [&a, &b] {
        scope(
            ctx.clone(),
            reconcile(&pool, OutboxReader::FHIR_OUTBOUND, true),
        )
        .await
        .expect("reconcile");
    }
    scope(
        a.clone(),
        advance(&pool, OutboxReader::FHIR_OUTBOUND, a_seqs[0]),
    )
    .await
    .expect("advance in A");

    let pruned_a = scope(a.clone(), prune(&pool, 7)).await.expect("prune A");
    assert_eq!(
        pruned_a, 1,
        "A loses the row its reader passed, keeps the rest"
    );
    assert_eq!(scoped_seqs(&pool, &a).await, a_seqs[1..]);
    assert_eq!(
        scoped_seqs(&pool, &b).await,
        b_seqs,
        "B's rows are untouched by A's prune"
    );

    // B's reader never advanced: its cursor is zero, nothing may go.
    let pruned_b = scope(b.clone(), prune(&pool, 7)).await.expect("prune B");
    assert_eq!(pruned_b, 0);

    // A reader switched off in B holds no floor there; A's row is untouched.
    scope(
        b.clone(),
        reconcile(&pool, OutboxReader::FHIR_OUTBOUND, false),
    )
    .await
    .expect("reconcile inactive in B");
    assert_eq!(scope(b.clone(), prune(&pool, 7)).await.expect("prune B"), 2);
    assert!(scoped_seqs(&pool, &b).await.is_empty());
    let a_active: bool = scope(
        a.clone(),
        sqlx::query_scalar(
            "SELECT active FROM ehr.event_outbox_reader WHERE reader = 'fhir-outbound'",
        )
        .fetch_one(&pool),
    )
    .await
    .expect("A's registry row");
    assert!(
        a_active,
        "the registry is scoped per tenant: B's switch-off leaves A active"
    );
}
