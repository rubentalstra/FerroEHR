//! E2 multi-tenancy isolation integration tests, against a real
//! `PostgreSQL` 18 via testcontainers.
//!
//! Proves the engine-enforced tenant isolation the tenancy extension specifies:
//!   * RLS is ENABLED **and** FORCED on every scoping table (catalog assertion);
//!   * a non-superuser role scoped to tenant A cannot read/list/point-read
//!     tenant B's EHRs / templates / stored queries — and vice versa — while its
//!     own writes land in A only (RLS filters reads AND writes);
//!   * a raw, filter-less `SELECT` in-session-as-A still cannot see B's rows
//!     (FORCE proof — the app never adds a `tenant_id` predicate);
//!   * a session with no `ehrbase.tenant_id` set sees only the reserved
//!     default-tenant rows (the single-tenant / pre-tenancy behaviour).
//!
//! RLS is bypassed by superusers/BYPASSRLS roles unconditionally (a Postgres
//! invariant), so these tests deliberately connect as a dedicated *non*-super
//! login role that is a member of `ehrbase_app` — which is exactly how a
//! production deployment runs (never as superuser).
//!
//! Requires Docker. The container is dropped when the test finishes.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use ehrbase::db::{self, DbSettings};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, Row};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

const PG_TAG: &str = "18";

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

struct Pg {
    _container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    async fn start() -> Self {
        let container = Postgres::default()
            .with_tag(PG_TAG)
            .start()
            .await
            .expect("start postgres:18 container (is Docker running?)");
        let host = container
            .get_host()
            .await
            .expect("container host")
            .to_string();
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped 5432");
        Self {
            _container: container,
            host,
            port,
        }
    }

    async fn migrated_pool(&self, name: &str) -> PgPool {
        let admin_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            self.host, self.port
        );
        let mut conn = PgConnection::connect(&admin_url)
            .await
            .expect("admin connect");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("create database");
        let settings = DbSettings::new(format!(
            "postgres://postgres:postgres@{}:{}/{name}",
            self.host, self.port
        ));
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrations apply");
        pool
    }

    /// A fresh connection as the non-superuser `rls_tester` login role (created
    /// by the test), with the application search path — so RLS is in force.
    async fn tester_conn(&self, db: &str) -> PgConnection {
        let url = format!(
            "postgres://rls_tester:testpw@{}:{}/{db}",
            self.host, self.port
        );
        let mut conn = PgConnection::connect(&url)
            .await
            .expect("rls_tester connect");
        sqlx::query("SET search_path TO ehr, ext, public")
            .execute(&mut conn)
            .await
            .expect("search_path");
        conn
    }
}

/// Set the session tenant on a tester connection (empty string = unset ⇒ the
/// reserved default tenant, per `ext.current_tenant_id()`).
async fn set_tenant(conn: &mut PgConnection, tenant: &str) {
    sqlx::query("SELECT set_config('ehrbase.tenant_id', $1, false)")
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
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("rls_flags").await;

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
    let pg = Pg::start().await;
    let db = "rls_isolation";
    let pool = pg.migrated_pool(db).await;

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

    // A non-superuser login role that RLS actually applies to.
    sqlx::query("CREATE ROLE rls_tester LOGIN PASSWORD 'testpw' IN ROLE ehrbase_app")
        .execute(&pool)
        .await
        .expect("create rls_tester");

    let mut conn = pg.tester_conn(db).await;
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
