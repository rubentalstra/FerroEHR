use sqlx::PgPool;
use sqlx::migrate::Migrator;

use crate::db::DbError;

/// The `ext`-schema baseline (AQL aggregate functions and the `en_US` ICU
/// collation), squashed from the `EHRbase` v2 Flyway chain (ADR-007). Must run
/// before the `ehr` set, whose DDL references that collation.
static EXT_MIGRATOR: Migrator = sqlx::migrate!("migrations/ext");

/// The `ehr`-schema baseline — the `EHRbase` v2 CDR schema itself, squashed
/// from the Flyway chain (ADR-007). A schema-equality test proves it matches
/// the legacy chain's end state (`tests/resources/legacy_schema`/).
static EHR_MIGRATOR: Migrator = sqlx::migrate!("migrations/ehr");

/// What Flyway/EHRbase provisioning did outside the migrations: the two
/// schemas and the required extensions (`uuid-ossp` is exercised by the DDL;
/// `pgcrypto`/`pg_trgm` complete the `EHRbase` baseline).
const BOOTSTRAP: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS ext",
    "CREATE SCHEMA IF NOT EXISTS ehr",
    r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA ext"#,
    "CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA ext",
    "CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA ext",
];

/// Bootstrap schemas/extensions and apply both migration sets.
///
/// Each migrator runs on a connection whose `search_path` starts with its
/// target schema, so the unqualified DDL and that set's `_sqlx_migrations`
/// bookkeeping table land in the right schema — mirroring Flyway's two
/// `flyway_schema_history` tables. Safe to call repeatedly (already-applied
/// migrations are skipped).
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
