//! Stored AQL query CRUD (ITS-REST DEFINITION `query` group), on the
//! `stored_query` table. A query is addressed by its qualified name
//! (`reverse_domain_name::semantic_id`) and a semantic version.

use serde_json::{Value, json};
use sqlx::Row;

use super::{EhrbaseService, ServiceError};

/// The SEMVER a no-version store assigns a query. The no-version store path is
/// defined as "stores a new query, or updates an existing query"
/// (`definition_query_store.yaml`), so it upserts at this default version.
//
// PORT NOTE: a coherent auto-increment scheme for repeated no-version stores of
// *different* query text (1.0.0 → 1.0.1 → …) is deferred (finding 03 hygiene
// note); today a no-version store always targets this version.
const DEFAULT_QUERY_VERSION: &str = "1.0.0";

impl EhrbaseService {
    /// Store a stored query, returning the effective SEMVER it is stored at.
    ///
    /// - **With an explicit `version`**, the `(name, version)` pair is
    ///   **immutable**: an already-existing pair is a **`Conflict`** (→ ITS-REST
    ///   `409`), never an overwrite — per
    ///   `docs/specs/openehr/ITS-REST/specifications/responses/409_StoredQuery_version.yaml`
    ///   ("409 Conflict … when a query with the given `qualified_query_name` and
    ///   `version` already exists"). The `definition_query_version_store`
    ///   operation lists `200/400/409`.
    /// - **Without a version**, the query is stored (or updated) at
    ///   [`DEFAULT_QUERY_VERSION`]. The `definition_query_store` operation lists
    ///   only `200/400` (no `409`) and is documented as
    ///   "stores a new query, or updates an existing query", so this path upserts.
    pub(super) async fn store_query(
        &self,
        qualified_name: &str,
        version: Option<&str>,
        query_text: String,
    ) -> Result<String, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified_name);
        let Some(v) = version else {
            // No-version store: upsert at the default version (spec-permitted
            // "stores a new query, or updates an existing query").
            sqlx::query(
                "INSERT INTO stored_query \
                 (reverse_domain_name, semantic_id, semver, query_type, query_text) \
                 VALUES ($1, $2, $3, 'AQL', $4) \
                 ON CONFLICT (reverse_domain_name, semantic_id, semver) \
                 DO UPDATE SET query_text = EXCLUDED.query_text, created_at = now()",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(DEFAULT_QUERY_VERSION)
            .bind(query_text)
            .execute(&self.pool)
            .await?;
            return Ok(DEFAULT_QUERY_VERSION.to_owned());
        };

        // Versioned store: insert-only. `DO NOTHING` on the (rdn, semantic,
        // semver) PK makes the duplicate case race-free; 0 affected rows means
        // the version already exists → 409 Conflict.
        let inserted = sqlx::query(
            "INSERT INTO stored_query \
             (reverse_domain_name, semantic_id, semver, query_type, query_text) \
             VALUES ($1, $2, $3, 'AQL', $4) \
             ON CONFLICT (reverse_domain_name, semantic_id, semver) DO NOTHING",
        )
        .bind(rdn)
        .bind(semantic)
        .bind(v)
        .bind(query_text)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if inserted == 0 {
            return Err(ServiceError::Conflict(format!(
                "a stored query '{qualified_name}' at version '{v}' already exists"
            )));
        }
        Ok(v.to_owned())
    }

    /// Retrieve a stored query — a specific `version`, or the latest.
    pub(super) async fn get_stored_query(
        &self,
        qualified_name: &str,
        version: Option<&str>,
    ) -> Result<Value, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified_name);
        let row = match version {
            Some(v) => sqlx::query(
                "SELECT semver, query_type, query_text, created_at FROM stored_query \
                 WHERE reverse_domain_name = $1 AND semantic_id = $2 AND semver = $3",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(v),
            None => sqlx::query(
                "SELECT semver, query_type, query_text, created_at FROM stored_query \
                 WHERE reverse_domain_name = $1 AND semantic_id = $2 \
                 ORDER BY string_to_array(semver, '.')::int[] DESC LIMIT 1",
            )
            .bind(rdn)
            .bind(semantic),
        }
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("stored query {qualified_name}")))?;

        Ok(Self::stored_query_json(qualified_name, &row))
    }

    /// List all stored versions of a qualified query name.
    pub(super) async fn list_stored_queries(
        &self,
        qualified_name: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified_name);
        let rows = sqlx::query(
            "SELECT semver, query_type, query_text, created_at FROM stored_query \
             WHERE reverse_domain_name = $1 AND semantic_id = $2 \
             ORDER BY string_to_array(semver, '.')::int[]",
        )
        .bind(rdn)
        .bind(semantic)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| Ok(Self::stored_query_json(qualified_name, row)))
            .collect()
    }

    /// The openEHR stored-query descriptor for one row.
    fn stored_query_json(qualified_name: &str, row: &sqlx::postgres::PgRow) -> Value {
        let saved = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")
            .map(|t| t.to_jiff().to_string())
            .unwrap_or_default();
        json!({
            "name": qualified_name,
            "type": row.try_get::<String, _>("query_type").unwrap_or_else(|_| "AQL".to_owned()),
            "version": row.try_get::<String, _>("semver").unwrap_or_default(),
            "saved": saved,
            "q": row.try_get::<String, _>("query_text").unwrap_or_default(),
        })
    }
}

/// Split a qualified query name `reverse_domain_name::semantic_id`. A name with
/// no `::` is treated as a bare semantic id (empty domain).
fn split_qualified(qualified_name: &str) -> (&str, &str) {
    qualified_name
        .split_once("::")
        .unwrap_or(("", qualified_name))
}
