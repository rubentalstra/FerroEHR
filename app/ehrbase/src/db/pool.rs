//! The `sqlx` connection pool. No openEHR spec governs the persistence
//! mechanism — this is our own design (`docs/architecture.md` §Storage). Every
//! pooled connection is initialized with the application `search_path`; the
//! optional tenant-scoped variant stamps a session GUC for row-level security
//! (multi-tenancy is our own extension, spec-silent).

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::db::{DbConfig, DbError};

/// Search path applied to every pooled connection: the application tables live
/// in `ehr`, the AQL support functions and the `"C"`/`en_US` collations in
/// `ext`. Set on every physical connection so queries may use unqualified table
/// names.
pub(crate) const SET_SEARCH_PATH_SQL: &str = "SET search_path TO ehr, ext, public";

/// The pool options common to both the plain and the tenant-scoped pool: the
/// sizing/timeout from settings and the standard search path on every physical
/// connection (so queries can use unqualified table names).
fn base_options(settings: &DbConfig) -> PgPoolOptions {
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
/// Every connection is initialized with the standard search path
/// ([`SET_SEARCH_PATH_SQL`]) so queries can use unqualified table names, as
/// the schema expects. This is today's pool verbatim — no per-acquire hook, so
/// zero overhead and byte-identical behaviour when tenancy is off.
///
/// # Errors
///
/// Returns [`DbError`] if the database is unreachable or the URL is invalid.
pub async fn connect(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = base_options(settings)
        .connect(settings.url.expose())
        .await?;
    Ok(pool)
}

/// Create the **tenant-scoped** application pool: the same pool
/// plus a `before_acquire` hook that stamps `ehrbase.tenant_id` on every
/// checked-out connection from the current request's tenant context
/// ([`crate::extensions::current`]).
///
/// This is the [`SET LOCAL`-equivalent] seam that scopes **both** autocommit
/// reads and transactions: the service checks out a fresh connection per read
/// and one per write transaction, and each carries the session GUC the RLS
/// `tenant_isolation` policy (and the `tenant_id` column DEFAULT) read. A
/// connection returning to the pool keeps its (session-level) GUC, so every
/// acquire re-stamps it — to the request's tenant, or to `''` (⇒ the reserved
/// default tenant) when no tenant is in scope (a background worker, or a request
/// that resolved no tenant) so a reused connection never leaks the previous
/// request's tenant.
///
/// Only wired when tenancy is on (`main.rs`); the extra per-acquire statement is
/// the multi-tenant cost that is by design "paid only in
/// multi-tenant mode".
///
/// # Errors
///
/// Returns [`DbError`] if the database is unreachable or the URL is invalid.
pub async fn connect_tenant_scoped(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = base_options(settings)
        .before_acquire(|conn, _meta| {
            Box::pin(async move {
                let tenant = crate::extensions::current()
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
