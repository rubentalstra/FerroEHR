// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! E2 multi-tenancy isolation integration tests, against a real
//! `PostgreSQL` 18 via the shared testkit harness.
//!
//! Proves the engine-enforced tenant isolation the tenancy extension specifies:
//!   * RLS is ENABLED **and** FORCED on every scoping table (catalog assertion);
//!   * a non-superuser role scoped to tenant A cannot read/list/point-read
//!     tenant B's EHRs / templates / stored queries — and vice versa — while its
//!     own writes land in A only (RLS filters reads AND writes);
//!   * a raw, filter-less `SELECT` in-session-as-A still cannot see B's rows
//!     (FORCE proof — the app never adds a `tenant_id` predicate);
//!   * a session with no `ferroehr.tenant_id` set sees only the reserved
//!     default-tenant rows (the single-tenant / pre-tenancy behaviour).
//!
//! RLS is bypassed by superusers/BYPASSRLS roles unconditionally (a Postgres
//! invariant), so these tests deliberately connect as a dedicated *non*-super
//! login role that is a member of `ferroehr_app` — which is exactly how a
//! production deployment runs (never as superuser).
//!
//! Requires Docker (the shared testkit `PostgreSQL` server).

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]
#![expect(
    clippy::too_many_lines,
    reason = "an end-to-end suite drives one long lifecycle per test on purpose: \
              splitting a case would hide the order its assertions depend on"
)]

use ferroehr::db::{self, DbConfig};
use sqlx::{AssertSqlSafe, Connection, PgConnection, Row};
use uuid::Uuid;

/// Every table the tenancy extension puts under tenant scope + RLS FORCE.
const SCOPED_TABLES: &[&str] = &[
    "ehr",
    "contribution",
    "vo_version",
    "node",
    "item_tag",
    "audit",
    "template_store",
    "archetype_store",
    "adl2_artefact",
    "stored_query",
    "sp_subject",
    "sp_binding",
    "sp_data_frame",
    "sp_variable",
    "sp_data_set",
    "event_outbox",
    "event_subscription",
    "fhir_mapping",
];

/// Rewrite the userinfo of a testkit clone DSN so a test can connect to the
/// same database as a different login role (scheme/host/port/database
/// preserved) — the RLS tests must connect as a non-superuser role rather than
/// the harness's owner role.
fn with_role(base_url: &str, user: &str, password: &str) -> String {
    let (scheme, rest) = base_url.split_once("://").expect("dsn scheme");
    let host_and_path = rest.split_once('@').map_or(rest, |(_, tail)| tail);
    format!("{scheme}://{user}:{password}@{host_and_path}")
}

/// A fresh connection as a non-superuser login role (created by the test),
/// with the application search path — so RLS is in force. Roles are
/// cluster-global on the shared testkit server, so each test derives its
/// role name from its clone's database name (`<clone>_tester` — unique per
/// test, reaped by the testkit sweep).
async fn tester_conn(base_url: &str, role: &str) -> PgConnection {
    let mut conn = PgConnection::connect(&with_role(base_url, role, "testpw"))
        .await
        .expect("tester role connect");
    sqlx::query("SET search_path TO ehr, ext, public")
        .execute(&mut conn)
        .await
        .expect("search_path");
    conn
}

/// Set the session tenant on a tester connection (empty string = unset ⇒ the
/// reserved default tenant, per `ext.current_tenant_id()`).
async fn set_tenant(conn: &mut PgConnection, tenant: &str) {
    sqlx::query("SELECT set_config('ferroehr.tenant_id', $1, false)")
        .bind(tenant)
        .execute(conn)
        .await
        .expect("set tenant");
}

async fn count(conn: &mut PgConnection, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(AssertSqlSafe(sql.to_owned()))
        .fetch_one(conn)
        .await
        .expect("count")
}

#[tokio::test]
async fn rls_is_enabled_and_forced_on_every_scoping_table() {
    let testdb = testkit::db().await.expect("testkit database");
    let pool = testdb.pool();

    for table in SCOPED_TABLES {
        let row = sqlx::query(
            "SELECT c.relrowsecurity, c.relforcerowsecurity \
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = 'ehr' AND c.relname = $1",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("pg_class for {table}: {e}"));
        let enabled: bool = row.get("relrowsecurity");
        let forced: bool = row.get("relforcerowsecurity");
        assert!(enabled, "RLS must be ENABLED on {table}");
        assert!(forced, "RLS must be FORCED on {table}");
    }
}

#[tokio::test]
async fn tenants_are_isolated_end_to_end() {
    let testdb = testkit::db().await.expect("testkit database");
    let pool = testdb.pool();

    // Two tenants (as the superuser — the tenant registry is not RLS-scoped).
    let (tenant_a, tenant_b) = (Uuid::now_v7(), Uuid::now_v7());
    sqlx::query(
        "INSERT INTO tenant (id, name, system_id) VALUES ($1, 'tenant-a', 'sys-a'), ($2, 'tenant-b', 'sys-b')",
    )
    .bind(tenant_a)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed tenants");

    // A non-superuser login role that RLS actually applies to (per-clone
    // name: roles are cluster-global on the shared testkit server).
    let tester = format!("{}_tester", testdb.name());
    sqlx::query(AssertSqlSafe(format!(
        "CREATE ROLE {tester} LOGIN PASSWORD 'testpw' IN ROLE ferroehr_app"
    )))
    .execute(&pool)
    .await
    .expect("create tester role");

    let mut conn = tester_conn(testdb.url(), &tester).await;
    let (a, b) = (tenant_a.to_string(), tenant_b.to_string());

    // ── As tenant A: write EHR + template + stored query. ────────────────────
    let ehr_a = Uuid::now_v7();
    set_tenant(&mut conn, &a).await;
    sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, 'sys-a')")
        .bind(ehr_a)
        .execute(&mut conn)
        .await
        .expect("A inserts ehr");
    sqlx::query("INSERT INTO template_store (template_id, content) VALUES ('tmpl-a', '<t/>')")
        .execute(&mut conn)
        .await
        .expect("A inserts template");
    sqlx::query(
        "INSERT INTO stored_query (reverse_domain_name, semantic_id, query_text) \
         VALUES ('org.a', 'q', 'SELECT 1')",
    )
    .execute(&mut conn)
    .await
    .expect("A inserts stored_query");
    assert_eq!(count(&mut conn, "SELECT count(*) FROM ehr").await, 1);
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM template_store").await,
        1
    );
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM stored_query").await,
        1
    );

    // ── As tenant B: A's rows are invisible; B writes its own. ────────────────
    let ehr_b = Uuid::now_v7();
    set_tenant(&mut conn, &b).await;
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM ehr").await,
        0,
        "B must not see A's EHR"
    );
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM template_store").await,
        0
    );
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM stored_query").await,
        0
    );
    // A point read of A's EHR by its id returns nothing (no existence leak).
    assert_eq!(
        count(
            &mut conn,
            &format!("SELECT count(*) FROM ehr WHERE id = '{ehr_a}'"),
        )
        .await,
        0,
        "B must not point-read A's EHR"
    );
    sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, 'sys-b')")
        .bind(ehr_b)
        .execute(&mut conn)
        .await
        .expect("B inserts ehr");
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM ehr").await,
        1,
        "B sees only its own"
    );

    // ── Back as tenant A: sees only A's row; B's is invisible (FORCE proof). ──
    set_tenant(&mut conn, &a).await;
    // Raw, filter-less SELECT — the app adds no tenant predicate — still isolated.
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM ehr").await,
        1,
        "A sees only its own"
    );
    let visible: Uuid = sqlx::query_scalar("SELECT id FROM ehr")
        .fetch_one(&mut conn)
        .await
        .expect("A's single row");
    assert_eq!(visible, ehr_a);
    assert_eq!(
        count(
            &mut conn,
            &format!("SELECT count(*) FROM ehr WHERE id = '{ehr_b}'"),
        )
        .await,
        0,
        "A must not see B's EHR"
    );

    // ── An unset session sees only the reserved default-tenant rows. ──────────
    set_tenant(&mut conn, "").await;
    // Neither A's nor B's rows are default-tenant.
    assert_eq!(
        count(&mut conn, "SELECT count(*) FROM ehr").await,
        0,
        "an unset session sees no tenant-scoped rows"
    );
    // A default-tenant row (unset session ⇒ nil default via the column DEFAULT)
    // is visible to the unset session — the single-tenant / pre-tenancy path.
    let ehr_default = Uuid::now_v7();
    sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, 'sys-default')")
        .bind(ehr_default)
        .execute(&mut conn)
        .await
        .expect("default-tenant insert");
    assert_eq!(count(&mut conn, "SELECT count(*) FROM ehr").await, 1);
    let default_row: Uuid = sqlx::query_scalar("SELECT id FROM ehr")
        .fetch_one(&mut conn)
        .await
        .expect("default row");
    assert_eq!(default_row, ehr_default);
    let default_tenant: Uuid = sqlx::query_scalar("SELECT tenant_id FROM ehr WHERE id = $1")
        .bind(ehr_default)
        .fetch_one(&mut conn)
        .await
        .expect("default tenant_id");
    assert_eq!(
        default_tenant,
        Uuid::nil(),
        "unset ⇒ the reserved default tenant"
    );
}

/// The PRODUCTION seam, end to end: [`db::connect_tenant_scoped`]'s
/// `before_acquire` hook stamps every checked-out connection with the tenant
/// from the request task-local ([`tenant_context::scope`]) — no manual
/// `set_config` anywhere — and the service layer's reads/writes come back
/// tenant-filtered by the same RLS policies proven above. This is the wiring
/// the binary uses when `tenancy.enabled = true`.
#[tokio::test]
async fn scoped_pool_isolates_tenants_through_the_service() {
    use ferroehr::extensions::tenant_context::{self, TenantContext};
    use ferroehr::service::FerroEhrService;

    let testdb = testkit::db().await.expect("testkit database");
    let pool = testdb.pool();

    let (tenant_a, tenant_b) = (Uuid::now_v7(), Uuid::now_v7());
    sqlx::query(
        "INSERT INTO tenant (id, name, system_id) VALUES ($1, 'seam-a', 'sys-a'), ($2, 'seam-b', 'sys-b')",
    )
    .bind(tenant_a)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("seed tenants");
    let app_role = format!("{}_app", testdb.name());
    sqlx::query(AssertSqlSafe(format!(
        "CREATE ROLE {app_role} LOGIN PASSWORD 'testpw' IN ROLE ferroehr_app"
    )))
    .execute(&pool)
    .await
    .expect("create app role");

    let scoped = DbConfig::new(with_role(testdb.url(), &app_role, "testpw"));
    let service = FerroEhrService::new(
        db::connect_tenant_scoped(&scoped)
            .await
            .expect("tenant-scoped pool"),
    );
    let ctx = |id: Uuid, sys: &str| TenantContext {
        tenant_id: id,
        system_id: sys.to_owned(),
    };

    // Tenant A creates an EHR through the service and sees it.
    let ehr_a = tenant_context::scope(ctx(tenant_a, "sys-a"), service.create_ehr(None))
        .await
        .expect("create EHR under tenant a");
    assert!(
        tenant_context::scope(ctx(tenant_a, "sys-a"), service.has_ehr(ehr_a))
            .await
            .expect("has_ehr a/a"),
        "tenant a sees its own EHR"
    );

    // Tenant B and the unscoped (default-tenant) caller must not.
    assert!(
        !tenant_context::scope(ctx(tenant_b, "sys-b"), service.has_ehr(ehr_a))
            .await
            .expect("has_ehr b/a"),
        "tenant b must NOT see tenant a's EHR"
    );
    assert!(
        !service.has_ehr(ehr_a).await.expect("has_ehr unscoped"),
        "the reserved default tenant must NOT see tenant a's EHR"
    );

    // And the reverse direction holds for tenant B's own data.
    let ehr_b = tenant_context::scope(ctx(tenant_b, "sys-b"), service.create_ehr(None))
        .await
        .expect("create EHR under tenant b");
    assert!(
        tenant_context::scope(ctx(tenant_b, "sys-b"), service.has_ehr(ehr_b))
            .await
            .expect("has_ehr b/b")
    );
    assert!(
        !tenant_context::scope(ctx(tenant_a, "sys-a"), service.has_ehr(ehr_b))
            .await
            .expect("has_ehr a/b"),
        "tenant a must NOT see tenant b's EHR"
    );
}

/// A connection opened by `acquire` itself (pool growth) gets only the
/// `after_connect` hook — `before_acquire` fires solely for previously idle
/// connections (docs.rs, `sqlx::pool::PoolOptions::before_acquire`: "This is
/// _not_ invoked for new connections. Use `after_connect` for those."). The
/// scoped pool must therefore stamp `ferroehr.tenant_id` in BOTH hooks;
/// stamping only `before_acquire` lets a pool-growth acquire run as the
/// reserved default tenant. This test forces pool growth inside a tenant
/// scope by holding more connections than physically exist, then asserts the
/// GUC on every one.
#[tokio::test]
async fn scoped_pool_stamps_connections_opened_during_acquire() {
    use ferroehr::extensions::tenant_context::{self, TenantContext};

    let testdb = testkit::db().await.expect("testkit database");
    // GUC stamping is a pure pool-hook property; the shared testkit database serves.
    let scoped = DbConfig::new(testdb.url().to_owned());
    let pool = db::connect_tenant_scoped(&scoped)
        .await
        .expect("tenant-scoped pool");

    let tenant = Uuid::now_v7();
    let ctx = TenantContext {
        tenant_id: tenant,
        system_id: "sys-fresh".to_owned(),
    };
    tenant_context::scope(ctx, async {
        // Hold one connection more than the pool currently has, without
        // releasing any: at least one acquire must open a NEW connection
        // inside this tenant scope.
        let target = usize::try_from(pool.size()).expect("pool size should fit usize") + 1;
        let mut held = Vec::with_capacity(target);
        for _ in 0..target {
            held.push(pool.acquire().await.expect("acquire under scope"));
        }
        for conn in &mut held {
            let guc: Option<String> =
                sqlx::query_scalar("SELECT current_setting('ferroehr.tenant_id', true)")
                    .fetch_one(&mut **conn)
                    .await
                    .expect("read tenant GUC");
            assert_eq!(
                guc.as_deref(),
                Some(tenant.to_string().as_str()),
                "every checked-out connection must carry the scoped tenant GUC \
                 (a fresh in-acquire connection missing it would run as the \
                 reserved default tenant)"
            );
        }
    })
    .await;
}
