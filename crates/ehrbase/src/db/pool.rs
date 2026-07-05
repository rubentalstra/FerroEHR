use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::db::{DbError, DbSettings};

/// Search path applied to every pooled connection: the application tables
/// live in `ehr`, the AQL support functions and the `en_US` collation in
/// `ext` (matching how `EHRbase` configures its datasource).
pub(crate) const SET_SEARCH_PATH_SQL: &str = "SET search_path TO ehr, ext, public";

/// Create the application connection pool.
///
/// Every connection is initialized with the standard search path
/// ([`SET_SEARCH_PATH_SQL`]) so queries can use unqualified table names, as
/// the schema expects.
///
/// # Errors
///
/// Returns [`DbError`] if the database is unreachable or the URL is invalid.
pub async fn connect(settings: &DbSettings) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(Duration::from_secs(settings.acquire_timeout_secs))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query(SET_SEARCH_PATH_SQL).execute(&mut *conn).await?;
                Ok(())
            })
        })
        .connect(&settings.url)
        .await?;
    Ok(pool)
}
