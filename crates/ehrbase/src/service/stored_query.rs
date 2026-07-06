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

    /// Retrieve a stored query — an exact `version`, a SEMVER **prefix**
    /// (`{major}` or `{major}.{minor}` resolve to the *highest* matching stored
    /// version — `parameters/path/version.yaml`; finding F-03-07), or the
    /// latest when no version is given.
    pub(super) async fn get_stored_query(
        &self,
        qualified_name: &str,
        version: Option<&str>,
    ) -> Result<Value, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified_name);
        let row = match version {
            Some(v) if is_partial_semver(v) => sqlx::query(
                // Prefix match on a dot boundary (`1` → `1.x.y`, `1.0` →
                // `1.0.x`), highest version first. `left(...)` (not LIKE)
                // avoids pattern-metacharacter escaping.
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE reverse_domain_name = $1 AND semantic_id = $2 \
                 AND (semver = $3 OR left(semver, length($3) + 1) = $3 || '.') \
                 ORDER BY string_to_array(semver, '.')::int[] DESC LIMIT 1",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(v),
            Some(v) => sqlx::query(
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE reverse_domain_name = $1 AND semantic_id = $2 AND semver = $3",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(v),
            None => sqlx::query(
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE reverse_domain_name = $1 AND semantic_id = $2 \
                 ORDER BY string_to_array(semver, '.')::int[] DESC LIMIT 1",
            )
            .bind(rdn)
            .bind(semantic),
        }
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("stored query {qualified_name}")))?;

        Ok(Self::stored_query_json(&row))
    }

    /// List stored queries whose qualified name starts with `name_pattern`
    /// (`definition_query_list.yaml`: the name is a **pattern** — a bare
    /// `org.openehr` prefix "will list all versions of all queries with names
    /// starting with `org.openehr`"; empty ⇒ wildcard — finding F-03-08). All
    /// stored versions of every matching name are returned.
    pub(super) async fn list_stored_queries(
        &self,
        name_pattern: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            // Prefix over the full qualified name; a query stored with no
            // reverse domain has the bare semantic id as its full name.
            "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, created_at \
             FROM stored_query \
             WHERE left(CASE WHEN reverse_domain_name = '' THEN semantic_id \
                             ELSE reverse_domain_name || '::' || semantic_id END, \
                        length($1)) = $1 \
             ORDER BY reverse_domain_name, semantic_id, string_to_array(semver, '.')::int[]",
        )
        .bind(name_pattern)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| Ok(Self::stored_query_json(row)))
            .collect()
    }

    /// The openEHR stored-query descriptor for one row.
    fn stored_query_json(row: &sqlx::postgres::PgRow) -> Value {
        let rdn = row
            .try_get::<String, _>("reverse_domain_name")
            .unwrap_or_default();
        let semantic = row.try_get::<String, _>("semantic_id").unwrap_or_default();
        let name = if rdn.is_empty() {
            semantic
        } else {
            format!("{rdn}::{semantic}")
        };
        let saved = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")
            .map(|t| t.to_jiff().to_string())
            .unwrap_or_default();
        json!({
            "name": name,
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

/// Whether a `version` path parameter is a SEMVER *prefix* (`{major}` or
/// `{major}.{minor}`) rather than an exact `{major}.{minor}.{patch}` triple
/// (`parameters/path/version.yaml`).
fn is_partial_semver(version: &str) -> bool {
    let segments: Vec<&str> = version.split('.').collect();
    segments.len() < 3
        && segments
            .iter()
            .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_semver_detection() {
        // `{major}` / `{major}.{minor}` prefixes (version.yaml).
        assert!(is_partial_semver("1"));
        assert!(is_partial_semver("1.0"));
        assert!(is_partial_semver("12.34"));
        // Exact triples and malformed tokens are not prefixes.
        assert!(!is_partial_semver("1.0.0"));
        assert!(!is_partial_semver("1..0"));
        assert!(!is_partial_semver("1.a"));
        assert!(!is_partial_semver(""));
    }
}
