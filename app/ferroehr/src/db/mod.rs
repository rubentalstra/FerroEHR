// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `PostgreSQL` bootstrap: connection settings, pool construction, and the
//! two-schema migration sequence.
//!
//! No openEHR spec governs the persistence mechanism — the storage substrate
//! is our own PG18-native design. This
//! module is the single place the rest of the crate obtains a database
//! handle: [`DbConfig`] (the `[db]` config section, a field of
//! [`crate::config::FerroEhrConfig`]) feeds [`connect`] /
//! [`connect_tenant_scoped`] (an `sqlx` [`PgPool`]), and [`run_migrations`]
//! bootstraps the `ext` + `ehr` schemas and applies both embedded migration
//! sets. The `sea-query` identifier vocabulary for the live schema lives in
//! [`iden`].
//!
//! This is the defining module for the whole bootstrap surface (no
//! re-exports): callers import `db::DbConfig`, `db::connect`,
//! `db::run_migrations`, `db::DEFAULT_URL`, `db::DbError` directly.

pub mod iden;

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool};

use crate::config::secret::SecretUrl;

// ---------------------------------------------------------------------------
// Settings — the `[db]` config section
// ---------------------------------------------------------------------------

/// The zero-config dev DSN (matches the compose dev stack). Production MUST
/// override it; the boot path warns prominently when
/// [`DbConfig::is_dev_default`] holds.
pub const DEFAULT_URL: &str = "postgres://ferroehr:ferroehr@localhost:5432/ferroehr";

/// What the server does about schema migrations when it boots.
///
/// Migrations are DDL, so a self-migrating server must authenticate as a role
/// that can execute DDL — and a role that can rewrite the schema is a role an
/// application-level SQL flaw can rewrite the schema with. This setting is how
/// a deployment opts out of that: the schema is applied out of band by the
/// migrator role, and the server connects as the DML-only `ferroehr_app` role.
///
/// No openEHR spec governs migration mechanics or database roles — our own
/// design/extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationMode {
    /// Apply the embedded migrations at boot, then verify (the default, and
    /// what makes an empty configuration boot against an empty database).
    #[default]
    Apply,
    /// Never issue DDL: verify that the database already carries exactly this
    /// build's migrations, and refuse to serve when it does not.
    Verify,
}

/// Connection settings for the application `PostgreSQL` database — the `[db]`
/// section of the one config tree ([`crate::config::FerroEhrConfig`]), with no
/// loader of its own.
///
/// No openEHR spec governs persistence — our own design. The DSN is a
/// [`SecretUrl`]: its embedded credentials are redacted from every rendering
/// (`Debug`, `/management/env`, `config check`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbConfig {
    /// `PostgreSQL` connection DSN (`postgres://user:pass@host:port/db`).
    /// Credentials are redacted from every rendering ([`SecretUrl`]).
    pub url: SecretUrl,
    /// Path to a file holding the DSN, read at boot in place of [`Self::url`].
    ///
    /// The route for a deployment that mounts its database credential as a file
    /// rather than passing it as an environment value, which is readable through
    /// `/proc/<pid>/environ` and inherited by every child process. Setting both
    /// this and a non-default `url` is a boot error.
    pub url_file: Option<PathBuf>,
    /// Upper bound of the connection pool.
    pub max_connections: u32,
    /// Connections the pool keeps open when idle (avoids cold reopen +
    /// `SET search_path` churn under variable load).
    pub min_connections: u32,
    /// Seconds to wait for a free connection before failing.
    pub acquire_timeout_secs: u64,
    /// `statement_timeout` applied to every pooled connection, in
    /// milliseconds; `0` leaves the server default (usually unlimited).
    ///
    /// This is the backstop the request timeout cannot be. A request that
    /// exceeds the HTTP timeout is answered `408` by dropping the handler
    /// future — which does not cancel the statement PostgreSQL is running
    /// (<https://www.postgresql.org/docs/18/runtime-config-client.html>). Without
    /// this, a handful of expensive queries can hold every pooled connection
    /// while every one of their clients has already been given up on: the
    /// clients see timeouts, the server looks idle, and the database is
    /// saturated by work nobody is waiting for.
    ///
    /// Set ABOVE the AQL engine's own budget
    /// ([`crate::service::query::config::QueryConfig::timeout_ms`]) so the
    /// engine's typed refusal fires first and this only catches what the engine
    /// does not govern. No openEHR spec governs it — our own design.
    pub statement_timeout_ms: u64,
    /// Whether the server applies its embedded migrations at boot
    /// ([`MigrationMode`]).
    ///
    /// `apply` (the default) keeps a fresh checkout and a fresh database
    /// working with no configuration at all. `verify` is the least-privilege
    /// production posture: the DSN may then authenticate as a role with no DDL
    /// rights, and the server refuses to boot against a database that is not
    /// already migrated to exactly this build.
    pub migrate: MigrationMode,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: SecretUrl::new(DEFAULT_URL),
            url_file: None,
            // Deliberate defaults: 20 max (10 hard-capped realistic write
            // concurrency ×2), 2 min (no cold reopen churn at idle).
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 30,
            // Twice the engine's own 30 s budget, so the engine refuses first
            // and this remains a backstop rather than the primary control.
            statement_timeout_ms: 60_000,
            migrate: MigrationMode::Apply,
        }
    }
}

impl DbConfig {
    /// Settings for `url` with defaults for everything else.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: SecretUrl::new(url.into()),
            ..Self::default()
        }
    }

    /// Whether the DSN is the built-in dev default (no operator override). The
    /// boot path logs a prominent warning in this case so a production
    /// deployment never silently runs against the dev database.
    #[must_use]
    pub fn is_dev_default(&self) -> bool {
        self.url.expose() == DEFAULT_URL
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors produced by the persistence foundation.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A driver/pool/query error from `sqlx`.
    #[error("database: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A schema migration failed to apply.
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// The database does not carry exactly this build's migrations, and
    /// [`MigrationMode::Verify`] forbids applying them.
    #[error(
        "the database is not migrated to this build ({0}). \
         [db].migrate is `verify`, so this server will not issue DDL: apply the \
         migrations out of band with the migrator role, then start it again"
    )]
    SchemaNotReady(#[source] SchemaMismatch),

    /// The cold archival tier outlived the primary tier it mirrors.
    #[error(
        "the cold archival tier (schema `cold`) is present but the primary tier \
         (`ehr.vo_version`) is not: the two are one repository and have been wiped \
         apart. The cold tables still hold clinical content, and their column shape \
         was copied from the primary tables as they stood before the wipe — so this \
         server will not adopt them: a re-adopted mirror can differ in shape from the \
         tier it mirrors, and the rows belong to a repository that no longer exists. \
         Restore the whole database from backup (both schemas together), or, if the \
         wipe was intended, `DROP SCHEMA cold CASCADE` and start again"
    )]
    OrphanedArchiveTier,
}

/// How a database's recorded migration state differs from the one this binary
/// embeds.
///
/// Each variant names a distinct operational situation, so a caller can branch
/// on it rather than match a message: an unmigrated database, a database behind
/// the binary, a database ahead of it, and a database whose bookkeeping is
/// damaged.
#[derive(Debug, thiserror::Error)]
pub enum SchemaMismatch {
    /// The schema carries no `_sqlx_migrations` table at all.
    #[error("schema `{schema}` has never been migrated")]
    NeverMigrated {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
    },

    /// Migrations this binary embeds are absent from the database.
    #[error(
        "schema `{schema}` is missing migration(s) {versions:?} — the database is older than this build"
    )]
    Missing {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The absent migration versions, ascending.
        versions: Vec<i64>,
    },

    /// A migration is recorded as having failed partway through.
    #[error("schema `{schema}` records migration {version} as failed")]
    Failed {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The failed migration's version.
        version: i64,
    },

    /// A migration was applied from source text this binary does not carry.
    #[error("schema `{schema}` applied migration {version} from different source text")]
    ChecksumMismatch {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The diverging migration's version.
        version: i64,
    },

    /// The database carries migrations this binary does not know about.
    #[error(
        "schema `{schema}` carries migration(s) {versions:?} — the database is newer than this build"
    )]
    Ahead {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The unknown migration versions, ascending.
        versions: Vec<i64>,
    },
}

// ---------------------------------------------------------------------------
// Pool
// ---------------------------------------------------------------------------

/// Search path applied to every pooled connection: the application tables live
/// in `ehr`, the AQL support functions and the `"C"`/`en_US` collations in
/// `ext`. Set once per physical connection (`after_connect`) so queries may
/// use unqualified table names.
const SET_SEARCH_PATH_SQL: &str = "SET search_path TO ehr, ext, public";

/// The pool options common to the plain and the tenant-scoped pool: sizing +
/// acquire timeout from settings, the standard search path on every physical
/// connection, and no per-checkout liveness ping. Connection retirement stays
/// on the `sqlx` defaults (idle reap + bounded lifetime — infinite-lived
/// connections are discouraged by the driver, so we do not disable them).
fn pool_options(settings: &DbConfig) -> PgPoolOptions {
    // Rendered once here rather than per connection. The value is an integer
    // from our own configuration, never client input, and it is bound as a
    // literal because PostgreSQL's `SET` takes no parameter placeholder.
    let statement_timeout = (settings.statement_timeout_ms > 0)
        .then(|| format!("SET statement_timeout = {}", settings.statement_timeout_ms));
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(Duration::from_secs(settings.acquire_timeout_secs))
        // No liveness ping per checkout: the default `test_before_acquire`
        // adds one round trip to EVERY acquisition; a broken connection is
        // detected by its first real statement and retried by the pool.
        .test_before_acquire(false)
        .after_connect(move |conn, _meta| {
            // Cloned per call: `after_connect` takes an `Fn`, so the captured
            // value cannot be moved out of it.
            let statement_timeout = statement_timeout.clone();
            Box::pin(async move {
                sqlx::query(SET_SEARCH_PATH_SQL).execute(&mut *conn).await?;
                if let Some(statement_timeout) = statement_timeout {
                    // `SET LOCAL` would last only the current transaction, so
                    // this is a session-level SET on the physical connection —
                    // it survives every checkout of that connection.
                    // `AssertSqlSafe` because PostgreSQL's `SET` takes no bind
                    // placeholder, so the value has to be rendered into the
                    // statement. Audited: it is a `u64` from our own
                    // configuration formatted with `{}`, never client input, so
                    // no string can reach here that is not decimal digits.
                    sqlx::query(sqlx::AssertSqlSafe(statement_timeout))
                        .execute(&mut *conn)
                        .await?;
                }
                Ok(())
            })
        })
}

/// Create the application connection pool (single-tenant / tenancy-off).
///
/// Every physical connection is initialized with the standard search path
/// (`ehr, ext, public`) so queries can use unqualified table names, as the
/// schema expects. There is no per-acquire hook: zero checkout overhead when
/// tenancy is off.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when the DSN does not parse as a `PostgreSQL`
/// URL, the initial connection fails (unreachable host, refused
/// authentication, unknown database), or the search-path initialization
/// statement fails on that first connection.
pub async fn connect(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings)
        .connect(settings.url.expose())
        .await?;
    Ok(pool)
}

/// Stamp the `ferroehr.tenant_id` session GUC on a connection from the
/// current task's tenant context ([`crate::extensions::tenant_context::current`])
/// — `''` (⇒ the reserved default tenant) when no tenant is in scope (a
/// background worker, or a request that resolved no tenant).
async fn stamp_tenant_guc(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    let tenant = crate::extensions::tenant_context::current()
        .map_or_else(String::new, |t| t.tenant_id.to_string());
    sqlx::query("SELECT set_config('ferroehr.tenant_id', $1, false)")
        .bind(tenant)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Creates the **tenant-scoped** application pool.
///
/// Wraps [`connect`] with hooks that stamp the `ferroehr.tenant_id` session GUC
/// on every checked-out connection from the current request's tenant context
/// ([`crate::extensions::tenant_context::current`]). Multi-tenancy is our own
/// deployment extension — no openEHR spec governs it.
///
/// This is the seam that scopes **both** autocommit reads and transactions:
/// the service checks out a fresh connection per read and one per write
/// transaction, and each carries the session GUC the RLS `tenant_isolation`
/// policy (and the `tenant_id` column DEFAULT) read. A connection returning
/// to the pool keeps its session-level GUC, so every acquire re-stamps it —
/// to the request's tenant, or to `''` (⇒ the reserved default tenant) when
/// no tenant is in scope — so a reused connection never leaks the previous
/// request's tenant.
///
/// The GUC is stamped in **both** pool hooks, and both are required
/// (docs.rs, `sqlx::pool::PoolOptions::before_acquire`: "This is _not_
/// invoked for new connections. Use `after_connect` for those."):
///
/// * `after_connect` — covers a connection freshly opened by `acquire`
///   itself (pool growth under load). Without it, that acquire would run
///   with the GUC unset, i.e. as the reserved default tenant. A connection
///   opened by the background `min_connections` maintainer has no request
///   context and is stamped `''`; it is re-stamped on checkout.
/// * `before_acquire` — re-stamps a previously idle connection on every
///   checkout, replacing whatever tenant its session carried before.
///
/// Only intended to be wired when tenancy is on; the extra per-acquire
/// statement is the multi-tenant cost, paid only in multi-tenant mode.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when the DSN does not parse as a `PostgreSQL`
/// URL, the initial connection fails (unreachable host, refused
/// authentication, unknown database), or the search-path initialization
/// statement fails on that first connection.
pub async fn connect_tenant_scoped(settings: &DbConfig) -> Result<PgPool, DbError> {
    let statement_timeout = (settings.statement_timeout_ms > 0)
        .then(|| format!("SET statement_timeout = {}", settings.statement_timeout_ms));
    let pool = pool_options(settings)
        // Replaces the base `after_connect` (the setter overwrites), so EVERY
        // base session setting is re-applied here — the search path AND the
        // statement timeout (dropping the timeout silently disarms the
        // DB-side runaway-query guard; reliability.md — a broken control must
        // never look like a policy outcome).
        .after_connect(move |conn, _meta| {
            let statement_timeout = statement_timeout.clone();
            Box::pin(async move {
                sqlx::query(SET_SEARCH_PATH_SQL).execute(&mut *conn).await?;
                if let Some(statement_timeout) = statement_timeout {
                    // Session-level SET; the value is our own u64 rendered
                    // with `{}` (see `pool_options`), never client input.
                    sqlx::query(sqlx::AssertSqlSafe(statement_timeout))
                        .execute(&mut *conn)
                        .await?;
                }
                stamp_tenant_guc(conn).await
            })
        })
        .before_acquire(|conn, _meta| {
            Box::pin(async move {
                stamp_tenant_guc(conn).await?;
                Ok(true)
            })
        })
        .connect(settings.url.expose())
        .await?;
    Ok(pool)
}

// ---------------------------------------------------------------------------
// Migrations
// ---------------------------------------------------------------------------

/// The `ext` schema: our openEHR support functions (`openehr_magnitude` and
/// its ISO-8601 helpers). Runs before `ehr`.
static EXT_MIGRATOR: Migrator = sqlx::migrate!("migrations/ext");

/// The `ehr` schema — the greenfield PG18-native CDR schema (no openEHR spec
/// governs the physical schema, spike-validated): the unified
/// per-version `node` table, the temporal `vo_version` table, and the
/// supporting tables.
static EHR_MIGRATOR: Migrator = sqlx::migrate!("migrations/ehr");

/// The `audit` schema — the local IHE ATNA Audit Record Repository (the
/// `audit_event` table). Strictly outside the EHR content (BASE
/// `architecture_overview/master07-security.adoc` §Access logging: in-system
/// access logs, never part of the EHR proper); runs after `ehr`.
static AUDIT_MIGRATOR: Migrator = sqlx::migrate!("migrations/audit");

/// The three migration sets in application order, each paired with the schema
/// that carries its `_sqlx_migrations` bookkeeping table.
const MIGRATION_SETS: &[(&str, &Migrator)] = &[
    ("ext", &EXT_MIGRATOR),
    ("ehr", &EHR_MIGRATOR),
    ("audit", &AUDIT_MIGRATOR),
];

/// Bootstrap done outside the migrations: the three schemas and `btree_gist`
/// (required by the temporal `WITHOUT OVERLAPS` primary key).
const BOOTSTRAP: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS ext",
    "CREATE SCHEMA IF NOT EXISTS ehr",
    "CREATE SCHEMA IF NOT EXISTS audit",
    "CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext",
];

/// A stable fingerprint of the complete embedded migration state: the
/// bootstrap statements plus every migrator's (version, checksum) sequence,
/// in application order.
///
/// Two builds with identical migrations produce the same value; any migration
/// change produces a new one. Test infrastructure keys its migrated template
/// databases on this (no openEHR spec governs test infrastructure — our own
/// design).
#[must_use]
pub fn migration_fingerprint() -> String {
    // FNV-1a over the bootstrap text + each migration's version/checksum —
    // collision-resistant enough for a cache key with a handful of live
    // values, with no hashing dependency.
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(PRIME);
        }
    };
    for statement in BOOTSTRAP {
        eat(statement.as_bytes());
    }
    for (_, migrator) in MIGRATION_SETS {
        for migration in migrator.iter() {
            eat(&migration.version.to_le_bytes());
            eat(&migration.checksum);
        }
    }
    format!("{hash:016x}")
}

/// Bootstrap schemas/extensions and apply the migration sets, `ext` before
/// `ehr`.
///
/// Each migrator runs on a connection whose `search_path` starts with its
/// target schema, so the unqualified DDL and that set's `_sqlx_migrations`
/// bookkeeping table land in the right schema (two independent bookkeeping
/// tables, one per set). Safe to call repeatedly: an already-applied migration
/// is skipped, and its recorded checksum is validated against the embedded
/// source.
///
/// The sequence runs on a connection **detached** from the pool and closed
/// afterwards: the migrators mutate the session `search_path`, and a
/// mid-sequence failure must never return a connection with a non-standard
/// search path to the pool.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when no connection can be acquired within the
/// acquire timeout or a bootstrap/`search_path` statement fails (e.g. the
/// role lacks `CREATE` on the database, or `btree_gist` is unavailable), and
/// [`DbError::Migrate`] when a migration fails to apply or an
/// already-applied migration fails checksum validation against the embedded
/// source.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    let mut conn = pool.acquire().await?.detach();
    let outcome = apply_migrations(&mut conn).await;
    if let Err(error) = conn.close().await {
        tracing::debug!(%error, "closing the migration connection failed");
    }
    outcome
}

/// Brings the database to the state this build requires, as
/// [`DbConfig::migrate`] directs.
///
/// [`MigrationMode::Apply`] runs [`run_migrations`]; [`MigrationMode::Verify`]
/// issues no DDL and only checks, so the DSN may authenticate as a role that
/// holds no DDL rights at all. This is the boot-path entry point — call it
/// instead of [`run_migrations`] anywhere the operator's configuration should
/// decide.
///
/// # Errors
///
/// In `apply` mode, whatever [`run_migrations`] returns. In `verify` mode,
/// [`DbError::SchemaNotReady`] when the database does not carry exactly this
/// build's migrations, or [`DbError::Sqlx`] when the check itself cannot run.
pub async fn prepare(settings: &DbConfig, pool: &PgPool) -> Result<(), DbError> {
    match settings.migrate {
        MigrationMode::Apply => run_migrations(pool).await,
        MigrationMode::Verify => {
            tracing::info!(
                "[db].migrate is `verify`: this server issues no DDL and requires an \
                 already-migrated database"
            );
            verify_migrations(pool).await
        }
    }
}

/// Verifies, without issuing any DDL, that the database carries exactly the
/// migrations this binary embeds.
///
/// Read-only by construction: it reads each set's `_sqlx_migrations`
/// bookkeeping table and compares versions, success flags and checksums
/// against the embedded sources. That makes it usable both as the boot gate for
/// [`MigrationMode::Verify`] and as an operator check against a running
/// database.
///
/// # Errors
///
/// [`DbError::SchemaNotReady`] naming the first divergence found, or
/// [`DbError::Sqlx`] when a connection or the bookkeeping read fails.
pub async fn verify_migrations(pool: &PgPool) -> Result<(), DbError> {
    for (schema, migrator) in MIGRATION_SETS {
        verify_set(pool, schema, migrator).await?;
    }
    Ok(())
}

/// Compare one migration set's bookkeeping table against its embedded source.
async fn verify_set(pool: &PgPool, schema: &str, migrator: &Migrator) -> Result<(), DbError> {
    // The schema name is one of the three literals in `MIGRATION_SETS`, never
    // input: `to_regclass` answers NULL for a missing relation rather than
    // failing (PostgreSQL 18 docs, "System Information Functions").
    let bookkeeping = format!("{schema}._sqlx_migrations");
    let present: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(&bookkeeping)
        .fetch_one(pool)
        .await?;
    if !present {
        return Err(DbError::SchemaNotReady(SchemaMismatch::NeverMigrated {
            schema: schema.to_owned(),
        }));
    }

    let query = format!("SELECT version, success, checksum FROM {bookkeeping} ORDER BY version");
    let applied: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(query))
        .fetch_all(pool)
        .await?;

    let mut unknown: Vec<i64> = applied.iter().map(|(version, _, _)| *version).collect();
    let mut missing: Vec<i64> = Vec::new();
    for embedded in migrator.iter() {
        let Some((version, success, checksum)) = applied
            .iter()
            .find(|(version, _, _)| *version == embedded.version)
        else {
            missing.push(embedded.version);
            continue;
        };
        unknown.retain(|known| known != version);
        if !success {
            return Err(DbError::SchemaNotReady(SchemaMismatch::Failed {
                schema: schema.to_owned(),
                version: *version,
            }));
        }
        if checksum.as_slice() != embedded.checksum.as_ref() {
            return Err(DbError::SchemaNotReady(SchemaMismatch::ChecksumMismatch {
                schema: schema.to_owned(),
                version: *version,
            }));
        }
    }
    if !missing.is_empty() {
        return Err(DbError::SchemaNotReady(SchemaMismatch::Missing {
            schema: schema.to_owned(),
            versions: missing,
        }));
    }
    if !unknown.is_empty() {
        return Err(DbError::SchemaNotReady(SchemaMismatch::Ahead {
            schema: schema.to_owned(),
            versions: unknown,
        }));
    }
    Ok(())
}

/// The bootstrap + two-migrator sequence on one dedicated connection.
async fn apply_migrations(conn: &mut PgConnection) -> Result<(), DbError> {
    for &statement in BOOTSTRAP {
        sqlx::query(statement).execute(&mut *conn).await?;
    }

    sqlx::query("SET search_path TO ext")
        .execute(&mut *conn)
        .await?;
    EXT_MIGRATOR.run(&mut *conn).await?;

    guard_orphaned_archive_tier(&mut *conn).await?;

    sqlx::query("SET search_path TO ehr, ext")
        .execute(&mut *conn)
        .await?;
    EHR_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO audit, ext")
        .execute(&mut *conn)
        .await?;
    AUDIT_MIGRATOR.run(&mut *conn).await?;
    Ok(())
}

/// Refuse to migrate a database whose cold archival tier outlived its primary
/// tier.
///
/// `0007_cold_archive_tier` is the only migration in the `ehr` set whose objects
/// live outside the `ehr` schema, so a `DROP SCHEMA ehr CASCADE` — a restore gone
/// wrong, a recreated volume, a wiped test database — leaves the `cold` tables
/// standing while the bookkeeping that records them goes away. Re-applying then
/// hits `relation "vo_version" already exists`, which is a permanent boot loop
/// with no error naming the cause.
///
/// Making the migration re-runnable is the wrong repair: those mirrors were built
/// with `CREATE TABLE … (LIKE …)` against the primary tables as they stood at the
/// time, so adopting a surviving one silently accepts a mirror that may no longer
/// match the tier it mirrors — and it re-attaches clinical rows to a repository
/// that no longer exists. The refusal is the answer, with the remedy in the
/// message.
///
/// `to_regclass` is used rather than a catalog join because it answers `NULL` for
/// a missing relation instead of failing
/// (<https://www.postgresql.org/docs/18/functions-info.html>), so one statement
/// covers both a fresh database and a healthy one.
async fn guard_orphaned_archive_tier(conn: &mut PgConnection) -> Result<(), DbError> {
    let orphaned: bool = sqlx::query_scalar(
        "SELECT to_regclass('cold.vo_version') IS NOT NULL
            AND to_regclass('ehr.vo_version') IS NULL",
    )
    .fetch_one(&mut *conn)
    .await?;
    if orphaned {
        return Err(DbError::OrphanedArchiveTier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let s = DbConfig::new("postgres://localhost/ferroehr");
        assert_eq!(s.url.expose(), "postgres://localhost/ferroehr");
        assert_eq!(s.max_connections, 20);
        assert_eq!(s.min_connections, 2);
        assert_eq!(s.acquire_timeout_secs, 30);
        assert!(!s.is_dev_default());
    }

    #[test]
    fn default_url_is_the_dev_dsn() {
        assert!(DbConfig::default().is_dev_default());
    }
}
