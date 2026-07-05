//! P09 integration tests: the baseline migrations apply cleanly on a real
//! `PostgreSQL` 18, a `sea-query` round-trip works, and — the load-bearing
//! gate — the squashed baseline produces a schema identical to the legacy
//! `EHRbase` Flyway chain (ADR-007).
//!
//! Requires Docker. Each test owns its container, so `Drop` removes it when
//! the test finishes — nothing is left running afterwards.

// Test code: expect/unwrap are fine here (see .claude/rules/testing.md).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use ehrbase::db::{self, DbSettings, iden::Ehr};
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, Row};
use testcontainers::core::{CmdWaitFor, ExecCommand, Mount};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

/// Debian-based image: ICU-enabled and ships the contrib extensions.
const PG_TAG: &str = "18";

/// A test-owned `PostgreSQL` 18 server. The container is removed on `Drop`
/// (end of the owning test), so no containers outlive the test run.
struct Pg {
    container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    /// Start a fresh server with the legacy Flyway chain bind-mounted
    /// read-only (used by the schema-equality gate).
    async fn start() -> Self {
        let legacy = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/resources/legacy_schema");
        let container = Postgres::default()
            .with_tag(PG_TAG)
            .with_mount(Mount::bind_mount(
                legacy.to_string_lossy().into_owned(),
                "/legacy_schema",
            ))
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
            container,
            host,
            port,
        }
    }

    /// Create a database on this server and return settings for it.
    async fn create_database(&self, name: &str) -> DbSettings {
        let Self { host, port, .. } = self;
        let admin_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let mut conn = PgConnection::connect(&admin_url)
            .await
            .expect("admin connect");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("create database");
        DbSettings::new(format!("postgres://postgres:postgres@{host}:{port}/{name}"))
    }
}

#[tokio::test]
async fn migrations_apply_cleanly_and_idempotently() {
    let pg = Pg::start().await;
    let settings = pg.create_database("mig_apply").await;
    let pool = db::connect(&settings).await.expect("pool");
    db::run_migrations(&pool).await.expect("migrations apply");
    // Running again must be a no-op, not an error.
    db::run_migrations(&pool)
        .await
        .expect("migrations idempotent");

    let applied_ext: i64 = sqlx::query_scalar("SELECT count(*) FROM ext._sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("ext bookkeeping");
    let applied_ehr: i64 = sqlx::query_scalar("SELECT count(*) FROM ehr._sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("ehr bookkeeping");
    assert_eq!(
        (applied_ext, applied_ehr),
        (1, 1),
        "one baseline per schema"
    );

    let tables: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables \
         WHERE table_schema = 'ehr' AND table_name <> '_sqlx_migrations'",
    )
    .fetch_one(&pool)
    .await
    .expect("table count");
    assert_eq!(tables, 17, "the EHRbase v2 schema has 17 tables");
}

#[tokio::test]
async fn sea_query_round_trip_through_ehr_table() {
    let pg = Pg::start().await;
    let settings = pg.create_database("smoke_roundtrip").await;
    let pool = db::connect(&settings).await.expect("pool");
    db::run_migrations(&pool).await.expect("migrations");

    let id = uuid::Uuid::new_v4();
    let (sql, values) = Query::insert()
        .into_table(Ehr::Table)
        .columns([Ehr::Id, Ehr::CreationDate])
        .values([Expr::val(id), Expr::cust("now()")])
        .expect("insert values")
        .build_sqlx(PostgresQueryBuilder);
    sqlx::query_with(AssertSqlSafe(sql), values)
        .execute(&pool)
        .await
        .expect("insert via sea-query");

    let row = sqlx::query("SELECT id, creation_date FROM ehr WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("select back");
    let read_id: uuid::Uuid = row.get("id");
    let created: jiff_sqlx::Timestamp = row.get("creation_date");
    assert_eq!(read_id, id);
    let age = jiff::Timestamp::now() - created.to_jiff();
    assert!(age.abs().get_hours() < 1, "creation_date is recent");
}

// ─── The schema-equality gate (ADR-007) ─────────────────────────────────────
//
// The shipped baseline migrations were squashed from the legacy EHRbase v2
// Flyway chain. This test applies the ORIGINAL chain (bind-mounted from
// tests/resources/legacy_schema/) to one database via psql inside the
// container, the baseline to another via `db::run_migrations`, and asserts
// the resulting schemas are identical at the pg_catalog level.
//
// Documented exceptions (ADR-007): the orphaned `tenant_id_seq` (legacy
// leftover, deliberately not recreated) and sqlx's `_sqlx_migrations`
// bookkeeping tables (baseline side only).

async fn apply_legacy_chain(pg: &Pg, dbname: &str) {
    let script = format!(
        r#"set -e
psql -U postgres -d {dbname} -v ON_ERROR_STOP=1 -q \
  -c 'CREATE SCHEMA ext' -c 'CREATE SCHEMA ehr' \
  -c 'CREATE EXTENSION "uuid-ossp" WITH SCHEMA ext' \
  -c 'CREATE EXTENSION pgcrypto WITH SCHEMA ext' \
  -c 'CREATE EXTENSION pg_trgm WITH SCHEMA ext'
for f in /legacy_schema/ext/*.sql; do
  PGOPTIONS='-c search_path=ext' psql -U postgres -d {dbname} -v ON_ERROR_STOP=1 -q -f "$f"
done
for f in /legacy_schema/ehr/*.sql; do
  PGOPTIONS='-c search_path=ehr,ext' psql -U postgres -d {dbname} -v ON_ERROR_STOP=1 -q -f "$f"
done
"#
    );
    let mut result = pg
        .container
        .exec(
            ExecCommand::new(["bash", "-c", &script]).with_cmd_ready_condition(CmdWaitFor::exit()),
        )
        .await
        .expect("exec legacy chain");
    let exit = result.exit_code().await.expect("exit code");
    if exit != Some(0) {
        let stderr =
            String::from_utf8_lossy(&result.stderr_to_vec().await.unwrap_or_default()).into_owned();
        panic!("legacy chain failed (exit {exit:?}):\n{stderr}");
    }
}

/// A canonical, ordered description of everything schema-relevant in the
/// `ehr` + `ext` schemas: columns, constraints, indexes, functions,
/// aggregates, enum types, collations, sequences, storage options, comments.
#[allow(clippy::too_many_lines)] // a flat list of catalog queries
async fn schema_fingerprint(pool: &PgPool) -> String {
    const SECTIONS: &[(&str, &str)] = &[
        (
            // Column position is compared as a dense rank, not the raw
            // attnum: dropped columns in the legacy chain leave attnum gaps
            // that carry no schema semantics; relative order must still match.
            "columns",
            "SELECT table_schema||'.'||table_name||'.'||column_name||'|'|| \
                    (row_number() OVER (PARTITION BY table_schema, table_name \
                                        ORDER BY ordinal_position))||'|'|| \
                    COALESCE(data_type,'')||'|'||COALESCE(udt_name,'')||'|'||is_nullable||'|'|| \
                    COALESCE(column_default,'')||'|'||COALESCE(collation_name,'') \
             FROM information_schema.columns \
             WHERE table_schema IN ('ehr','ext') AND table_name <> '_sqlx_migrations' \
             ORDER BY 1",
        ),
        (
            "constraints",
            "SELECT n.nspname||'.'||cl.relname||'.'||c.conname||'|'||pg_get_constraintdef(c.oid) \
             FROM pg_constraint c \
             JOIN pg_class cl ON cl.oid = c.conrelid \
             JOIN pg_namespace n ON n.oid = cl.relnamespace \
             WHERE n.nspname IN ('ehr','ext') AND cl.relname <> '_sqlx_migrations' \
             ORDER BY 1",
        ),
        (
            "indexes",
            "SELECT schemaname||'.'||tablename||'.'||indexname||'|'||indexdef \
             FROM pg_indexes \
             WHERE schemaname IN ('ehr','ext') AND tablename <> '_sqlx_migrations' \
             ORDER BY 1",
        ),
        (
            "functions",
            "SELECT n.nspname||'.'||p.proname||'('||pg_get_function_identity_arguments(p.oid)||')|'|| \
                    md5(pg_get_functiondef(p.oid)) \
             FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
             WHERE n.nspname IN ('ehr','ext') AND p.prokind = 'f' \
             ORDER BY 1",
        ),
        (
            "aggregates",
            "SELECT n.nspname||'.'||p.proname||'|'||a.aggtransfn::text||'|'||a.aggfinalfn::text||'|'|| \
                    a.aggcombinefn::text||'|'||COALESCE(a.agginitval,'')||'|'|| \
                    a.aggsortop::regoperator::text||'|'||format_type(a.aggtranstype, NULL) \
             FROM pg_aggregate a \
             JOIN pg_proc p ON p.oid = a.aggfnoid \
             JOIN pg_namespace n ON n.oid = p.pronamespace \
             WHERE n.nspname IN ('ehr','ext') \
             ORDER BY 1",
        ),
        (
            "enum-types",
            "SELECT n.nspname||'.'||t.typname||'|'||string_agg(e.enumlabel, ',' ORDER BY e.enumsortorder) \
             FROM pg_type t \
             JOIN pg_enum e ON e.enumtypid = t.oid \
             JOIN pg_namespace n ON n.oid = t.typnamespace \
             WHERE n.nspname IN ('ehr','ext') \
             GROUP BY n.nspname, t.typname \
             ORDER BY 1",
        ),
        (
            "collations",
            "SELECT n.nspname||'.'||c.collname||'|'||c.collprovider::text||'|'|| \
                    COALESCE(c.colllocale, c.collcollate, '') \
             FROM pg_collation c JOIN pg_namespace n ON n.oid = c.collnamespace \
             WHERE n.nspname IN ('ehr','ext') \
             ORDER BY 1",
        ),
        (
            // ADR-007 exception: the orphaned legacy `tenant_id_seq` is
            // deliberately absent from the baseline.
            "sequences",
            "SELECT sequence_schema||'.'||sequence_name \
             FROM information_schema.sequences \
             WHERE sequence_schema IN ('ehr','ext') AND sequence_name <> 'tenant_id_seq' \
             ORDER BY 1",
        ),
        (
            "reloptions",
            "SELECT n.nspname||'.'||c.relname||'|'||COALESCE(array_to_string(c.reloptions, ','), '') \
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname IN ('ehr','ext') AND c.relkind = 'r' \
               AND c.relname <> '_sqlx_migrations' \
             ORDER BY 1",
        ),
        (
            "attstorage",
            "SELECT n.nspname||'.'||c.relname||'.'||a.attname||'|'||a.attstorage::text \
             FROM pg_attribute a \
             JOIN pg_class c ON c.oid = a.attrelid \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname IN ('ehr','ext') AND c.relkind = 'r' \
               AND a.attnum > 0 AND NOT a.attisdropped \
               AND c.relname <> '_sqlx_migrations' \
             ORDER BY 1",
        ),
        (
            "comments",
            "SELECT n.nspname||'.'||c.relname||'|'||d.description \
             FROM pg_description d \
             JOIN pg_class c ON c.oid = d.objoid AND d.objsubid = 0 \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname IN ('ehr','ext') \
             ORDER BY 1",
        ),
    ];

    let mut fingerprint = String::new();
    for (section, query) in SECTIONS {
        let lines: Vec<String> = sqlx::query_scalar(*query)
            .fetch_all(pool)
            .await
            .unwrap_or_else(|e| panic!("fingerprint section {section}: {e}"));
        fingerprint.push_str("## ");
        fingerprint.push_str(section);
        fingerprint.push('\n');
        fingerprint.push_str(&lines.join("\n"));
        fingerprint.push('\n');
    }
    fingerprint
}

#[tokio::test]
async fn baseline_schema_is_identical_to_legacy_flyway_chain() {
    let pg = Pg::start().await;
    let legacy = pg.create_database("schema_legacy").await;
    let baseline = pg.create_database("schema_baseline").await;

    apply_legacy_chain(&pg, "schema_legacy").await;
    let baseline_pool = db::connect(&baseline).await.expect("baseline pool");
    db::run_migrations(&baseline_pool)
        .await
        .expect("baseline migrations");

    let legacy_pool = db::connect(&legacy).await.expect("legacy pool");
    let legacy_fp = schema_fingerprint(&legacy_pool).await;
    let baseline_fp = schema_fingerprint(&baseline_pool).await;

    assert!(!legacy_fp.is_empty() && legacy_fp.contains("ehr.comp_data"));
    if legacy_fp != baseline_fp {
        let diff: Vec<String> = diff_lines(&legacy_fp, &baseline_fp);
        panic!(
            "baseline schema diverges from the legacy Flyway chain ({} differing lines):\n{}",
            diff.len(),
            diff.join("\n"),
        );
    }
}

/// Line-level diff of the two fingerprints (legacy = `-`, baseline = `+`).
fn diff_lines(legacy: &str, baseline: &str) -> Vec<String> {
    use std::collections::BTreeSet;
    let legacy_set: BTreeSet<&str> = legacy.lines().collect();
    let baseline_set: BTreeSet<&str> = baseline.lines().collect();
    legacy_set
        .difference(&baseline_set)
        .map(|l| format!("- {l}"))
        .chain(
            baseline_set
                .difference(&legacy_set)
                .map(|l| format!("+ {l}")),
        )
        .collect()
}
