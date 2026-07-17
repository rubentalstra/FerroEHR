//! `PostgreSQL` bootstrap: connection settings, pool construction, and the
//! two-schema migration sequence.
//!
//! No openEHR spec governs the persistence mechanism — the storage substrate
//! is our own PG18-native design (`docs/architecture.md` §Storage). This
//! module is the single place the rest of the crate obtains a database
//! handle: [`DbConfig`] (the `[db]` config section, a field of
//! [`crate::config::EhrbaseConfig`]) feeds [`connect`] /
//! [`connect_tenant_scoped`] (an `sqlx` [`PgPool`]), and [`run_migrations`]
//! bootstraps the `ext` + `ehr` schemas and applies both embedded migration
//! sets. The `sea-query` identifier vocabulary for the live schema lives in
//! [`iden`].
//!
//! This is the defining module for the whole bootstrap surface (no
//! re-exports): callers import `db::DbConfig`, `db::connect`,
//! `db::run_migrations`, `db::DEFAULT_URL`, `db::DbError` directly.

pub mod iden;

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
/// override it (`docs/design/configuration.md` §3.16 checklist); the boot
/// path warns prominently when [`DbConfig::is_dev_default`] holds.
pub const DEFAULT_URL: &str = "postgres://ehrbase:ehrbase@localhost:5432/ehrbase";

/// Connection settings for the application `PostgreSQL` database — the `[db]`
/// section of the one config tree ([`crate::config::EhrbaseConfig`]), with no
/// loader of its own.
///
/// No openEHR spec governs persistence — our own design
/// (`docs/design/configuration.md` §3.2). The DSN is a [`SecretUrl`]: its
/// embedded credentials are redacted from every rendering (`Debug`,
/// `/management/env`, `config check`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbConfig {
    /// `PostgreSQL` connection DSN (`postgres://user:pass@host:port/db`).
    /// Credentials are redacted from every rendering ([`SecretUrl`]).
    pub url: SecretUrl,
    /// Upper bound of the connection pool.
    pub max_connections: u32,
    /// Connections the pool keeps open when idle (avoids cold reopen +
    /// `SET search_path` churn under variable load).
    pub min_connections: u32,
    /// Seconds to wait for a free connection before failing.
    pub acquire_timeout_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: SecretUrl::new(DEFAULT_URL),
            // Deliberate P20 defaults: 20 max (10 hard-capped realistic write
            // concurrency ×2), 2 min (no cold reopen churn at idle).
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 30,
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
    /// boot path logs a prominent warning in this case
    /// (`docs/design/configuration.md` §3.16) so a production deployment never
    /// silently runs against the dev database.
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
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(Duration::from_secs(settings.acquire_timeout_secs))
        // No liveness ping per checkout: the default `test_before_acquire`
        // adds one round trip to EVERY acquisition; a broken connection is
        // detected by its first real statement and retried by the pool.
        .test_before_acquire(false)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query(SET_SEARCH_PATH_SQL).execute(&mut *conn).await?;
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

/// Create the **tenant-scoped** application pool: [`connect`] plus a
/// `before_acquire` hook that stamps the `ehrbase.tenant_id` session GUC on
/// every checked-out connection from the current request's tenant context
/// ([`crate::extensions::tenant_context::current`]). Multi-tenancy is our own
/// deployment extension — no openEHR spec governs it.
///
/// This is the seam that scopes **both** autocommit reads and transactions:
/// the service checks out a fresh connection per read and one per write
/// transaction, and each carries the session GUC the RLS `tenant_isolation`
/// policy (and the `tenant_id` column DEFAULT) read. A connection returning
/// to the pool keeps its session-level GUC, so every acquire re-stamps it —
/// to the request's tenant, or to `''` (⇒ the reserved default tenant) when
/// no tenant is in scope (a background worker, or a request that resolved no
/// tenant) — so a reused connection never leaks the previous request's
/// tenant.
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
    let pool = pool_options(settings)
        .before_acquire(|conn, _meta| {
            Box::pin(async move {
                let tenant = crate::extensions::tenant_context::current()
                    .map_or_else(String::new, |t| t.tenant_id.to_string());
                sqlx::query("SELECT set_config('ehrbase.tenant_id', $1, false)")
                    .bind(tenant)
                    .execute(&mut *conn)
                    .await?;
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
/// governs the physical schema, spike-validated at P10): the unified
/// per-version `node` table, the temporal `vo_version` table, and the
/// supporting tables.
static EHR_MIGRATOR: Migrator = sqlx::migrate!("migrations/ehr");

/// Bootstrap done outside the migrations: the two schemas and `btree_gist`
/// (required by the temporal `WITHOUT OVERLAPS` primary key).
const BOOTSTRAP: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS ext",
    "CREATE SCHEMA IF NOT EXISTS ehr",
    "CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext",
];

/// Bootstrap schemas/extensions and apply both migration sets, `ext` before
/// `ehr`.
///
/// Each migrator runs on a connection whose `search_path` starts with its
/// target schema, so the unqualified DDL and that set's `_sqlx_migrations`
/// bookkeeping table land in the right schema (two independent bookkeeping
/// tables, one per set). Safe to call repeatedly: already-applied migrations
/// are skipped, and previously applied migrations are checksum-validated
/// against the embedded sources.
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

/// The bootstrap + two-migrator sequence on one dedicated connection.
async fn apply_migrations(conn: &mut PgConnection) -> Result<(), DbError> {
    for &statement in BOOTSTRAP {
        sqlx::query(statement).execute(&mut *conn).await?;
    }

    sqlx::query("SET search_path TO ext")
        .execute(&mut *conn)
        .await?;
    EXT_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO ehr, ext")
        .execute(&mut *conn)
        .await?;
    EHR_MIGRATOR.run(&mut *conn).await?;
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let s = DbConfig::new("postgres://localhost/ehrbase");
        assert_eq!(s.url.expose(), "postgres://localhost/ehrbase");
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
