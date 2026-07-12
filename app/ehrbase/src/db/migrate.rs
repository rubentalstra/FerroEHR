//! Schema bootstrap + migration runner. No openEHR spec governs the SQL schema
//! (`docs/architecture.md` §Storage) — two migrators (`ext` then `ehr`), each
//! with its own `_sqlx_migrations` bookkeeping, apply our own PG18-native DDL.

use sqlx::PgPool;
use sqlx::migrate::Migrator;

use crate::db::DbError;

/// The `ext` schema: our openEHR support functions (`openehr_magnitude` and
/// its ISO-8601 helpers). Runs before `ehr`.
static EXT_MIGRATOR: Migrator = sqlx::migrate!("migrations/ext");

/// The `ehr` schema — the greenfield PG18-native CDR schema (no openEHR spec
/// governs the physical schema,
/// spike-validated at P10): the unified per-version `node` table, the
/// temporal `vo_version` table, and the supporting tables.
static EHR_MIGRATOR: Migrator = sqlx::migrate!("migrations/ehr");

/// Bootstrap done outside the migrations: the two schemas and `btree_gist`
/// (required by the temporal `WITHOUT OVERLAPS` primary key).
const BOOTSTRAP: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS ext",
    "CREATE SCHEMA IF NOT EXISTS ehr",
    "CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext",
];

/// Bootstrap schemas/extensions and apply both migration sets.
///
/// Each migrator runs on a connection whose `search_path` starts with its
/// target schema, so the unqualified DDL and that set's `_sqlx_migrations`
/// bookkeeping table land in the right schema. Safe to call repeatedly
/// (already-applied migrations are skipped).
///
/// # Errors
///
/// Returns [`DbError`] if a connection cannot be acquired, a bootstrap
/// statement fails, or a migration fails to apply or fails its checksum.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    let mut conn = pool.acquire().await?;
    for stmt in BOOTSTRAP {
        sqlx::query(*stmt).execute(&mut *conn).await?;
    }

    sqlx::query("SET search_path TO ext")
        .execute(&mut *conn)
        .await?;
    EXT_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO ehr, ext")
        .execute(&mut *conn)
        .await?;
    EHR_MIGRATOR.run(&mut *conn).await?;

    // Return the pooled connection on the standard application search path.
    sqlx::query(super::pool::SET_SEARCH_PATH_SQL)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
