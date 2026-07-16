//! `I_DEFINITION_QUERY` (`i_definition_query.adoc`; `master04` §Registered
//! Queries, §Query Formalism) + `QUERY_DESCRIPTOR` (`query_descriptor.adoc`),
//! and the stored-query CRUD it owns (on the `stored_query` table).
//!
//! DEFINITION owns query *registration* (`master04` §Registered Queries); the
//! Query service only *resolves + executes* a stored query, so the stored-query
//! store lives here, keyed by a qualified name (`reverse_domain_name ::
//! semantic_id`) and a semantic version.

use serde_json::{Value, json};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::service::status::{CallStatusType, SmError};
use crate::service::list::Page;
use crate::service::definition::types::QueryDescriptor;

use super::{compile_pattern, page_bounds, paginate, parse_qualified_name, split_qualified};
use crate::service::{EhrbaseService, ServiceError};

/// The SEMVER a no-version store assigns a query. The no-version store path is
/// "stores a new query, or updates an existing query"
/// (`definition_query_store.yaml`), so it upserts at this default version.
///
/// PORT NOTE (residue re-verified): a coherent auto-increment scheme for
/// repeated no-version stores of *different* text (1.0.0 → 1.0.1 → …) is not
/// attempted; the sanctioned "stores or updates" semantics is an upsert at this
/// version, which is what a no-version store does.
const DEFAULT_QUERY_VERSION: &str = "1.0.0";

impl EhrbaseService {
    // ── I_DEFINITION_QUERY domain logic ──────────────────────────────────────

    /// `has_query` — true if a query with the qualified name `a_query_name` is
    /// registered (the `"misc"` namespace is assumed when none is supplied; a
    /// three-part name's formalism segment is lifted out per `master04`
    /// §Registered Queries, G-05-04). Identity is case-insensitive (BASE
    /// master05 §Composite Identifiers and Case, G-05-14).
    pub(super) async fn query_exists(&self, a_query_name: &str) -> Result<bool, ServiceError> {
        let qualified = parse_qualified_name(a_query_name).qualified();
        let (rdn, semantic) = split_qualified(&qualified);
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM stored_query \
             WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2))",
        )
        .bind(rdn)
        .bind(semantic)
        .fetch_one(&self.pool)
        .await?)
    }

    /// `store_query` (Pre `valid_query`) — register a query, returning its
    /// [`QueryDescriptor`].
    ///
    /// The name is qualified (`"misc"` default) or generated (`misc::q_<uuid>`)
    /// when absent. A three-part `<ns>::<formalism>::<name>` name lifts its
    /// formalism out (G-05-04): the store key is the two-part `<ns>::<name>` and
    /// the name-borne formalism, when present, is the effective formalism —
    /// otherwise `a_type` is (`master04` §Query Formalism).
    ///
    /// PORT NOTE (G-05-09, spec naming): the SM precondition names
    /// `is_valid_query` though the function is `valid_query`; we enforce
    /// `valid_query` and reject an invalid query as `invalid_query` (`422`).
    pub(super) async fn query_store_sm(
        &self,
        text: String,
        a_type: &str,
        name: Option<String>,
    ) -> Result<QueryDescriptor, ServiceError> {
        let (qualified, formalism) = match name {
            Some(n) => {
                let parsed = parse_qualified_name(&n);
                let formalism = parsed.formalism.clone();
                (parsed.qualified(), formalism)
            }
            None => (format!("misc::q_{}", Uuid::now_v7().simple()), None),
        };
        // The effective formalism is the name-borne one when a three-part name
        // supplied it, else the `a_type` parameter.
        let effective = formalism.as_deref().unwrap_or(a_type);
        if !valid_query_text(&text, effective) {
            return Err(ServiceError::sm(
                CallStatusType::InvalidQuery,
                "query text is not a valid instance of its formalism",
            ));
        }
        let version = self.store_query_response(&qualified, None, text).await?;
        self.query_descriptor(&qualified, &version).await
    }

    /// The descriptor of a single stored `(qualified name, version)` row.
    async fn query_descriptor(
        &self,
        qualified: &str,
        version: &str,
    ) -> Result<QueryDescriptor, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified);
        let row = sqlx::query(
            "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, created_at \
             FROM stored_query \
             WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2) \
             AND semver = $3",
        )
        .bind(rdn)
        .bind(semantic)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("stored query {qualified}")))?;
        descriptor_from_row(&row)
    }

    /// `list_queries` — all registered queries, as descriptors.
    pub(super) async fn query_list_response(
        &self,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        let rows = sqlx::query(
            "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, created_at \
             FROM stored_query \
             ORDER BY reverse_domain_name, semantic_id, string_to_array(semver, '.')::int[] \
             OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(descriptor_from_row).collect()
    }

    /// `list_matching_queries` — registered queries whose qualified name matches
    /// `id_pattern` (regex) and whose referenced artefacts match
    /// `artefact_id_pattern` (regex, `None` = match any). Uncompilable pattern →
    /// `invalid_id_pattern` (`400`).
    ///
    /// PORT NOTE (register 05 G-05-05): `artefact_id_pattern` is spec'd against
    /// "archetype / template identifiers referenced in the query". Until the AQL
    /// engine exposes a query's analysed FROM/CONTAINS artefact-id set
    /// (`openehr_query`), we approximate by regex-scanning the stored source
    /// text — a query matches when its source contains a substring matching the
    /// artefact pattern. Replacing the raw-text scan with that analysed set is
    /// the future AQL-surface work.
    pub(super) async fn query_list_matching(
        &self,
        id_pattern: &str,
        artefact_id_pattern: Option<&str>,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, ServiceError> {
        let id_re = compile_pattern(id_pattern)?;
        let artefact_re = artefact_id_pattern.map(compile_pattern).transpose()?;
        let rows = sqlx::query(
            "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, created_at \
             FROM stored_query \
             ORDER BY reverse_domain_name, semantic_id, string_to_array(semver, '.')::int[]",
        )
        .fetch_all(&self.pool)
        .await?;
        // Decode every row up front so a decode failure surfaces (500) rather
        // than silently dropping a query from the match set (W-14 F-29).
        let descriptors = rows
            .iter()
            .map(descriptor_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let matched = descriptors.into_iter().filter(|d| {
            id_re.is_match(&d.qualified_query_name)
                && artefact_re
                    .as_ref()
                    .is_none_or(|re| d.source.as_deref().is_some_and(|src| re.is_match(src)))
        });
        Ok(paginate(matched, page))
    }

    /// `delete_query` (Pre `has_query` / Post `query_deleted`) — delete every
    /// version of the query with qualified name `a_query_name` (the SM keys
    /// deletion by *name*); absent → `404`. Identity is case-insensitive.
    pub(super) async fn query_delete(&self, a_query_name: &str) -> Result<(), ServiceError> {
        let qualified = parse_qualified_name(a_query_name).qualified();
        let (rdn, semantic) = split_qualified(&qualified);
        let deleted = sqlx::query(
            "DELETE FROM stored_query \
             WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2)",
        )
        .bind(rdn)
        .bind(semantic)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::sm(
                CallStatusType::ArtefactDoesNotExist,
                format!("stored query {a_query_name}"),
            ));
        }
        Ok(())
    }

    /// `queries_count` — total count of queries.
    ///
    /// PORT NOTE: counts distinct *qualified names* (a query with N stored
    /// versions counts once) — the natural reading of "total count of queries".
    pub(super) async fn query_count(&self) -> Result<i64, ServiceError> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM \
             (SELECT DISTINCT reverse_domain_name, semantic_id FROM stored_query) t",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// `valid_query` — `a_query_text` is a valid instance of formalism `a_type`.
    #[must_use]
    pub(super) fn valid_query_source(a_query_text: &str, a_type: &str) -> bool {
        valid_query_text(a_query_text, a_type)
    }

    // ── stored-query store (CRUD used by the DEFINITION wire + Query service) ─

    /// Store a stored query, returning the effective SEMVER it is stored at.
    ///
    /// - **With an explicit `version`**, the `(name, version)` pair is
    ///   **immutable**: an already-existing pair (case-insensitive) is a
    ///   `Conflict` (→ ITS-REST `409`), never an overwrite
    ///   (`409_StoredQuery_version.yaml`).
    /// - **Without a version**, the query is stored (or updated) at
    ///   [`DEFAULT_QUERY_VERSION`] (`definition_query_store`: "stores a new
    ///   query, or updates an existing query"; response set `200/400`).
    ///
    /// Identity is case-insensitive but storage case-preserving (BASE master05
    /// §Composite Identifiers and Case, G-05-14).
    pub(in crate::service) async fn store_query_response(
        &self,
        qualified_name: &str,
        version: Option<&str>,
        query_text: String,
    ) -> Result<String, ServiceError> {
        // Store-time AQL validation via the `openehr_query` parser: the
        // stored-query table only holds AQL (`query_type = 'AQL'`), so a non-AQL
        // or syntactically-invalid body is a `400 Bad Request`
        // (`definition_query_store` lists only `200`/`400`).
        if let Err(err) = openehr_query::parser::parse_str(&query_text) {
            return Err(ServiceError::BadRequest(format!(
                "stored query text is not valid AQL: {err}"
            )));
        }

        let (rdn, semantic) = split_qualified(qualified_name);
        let Some(v) = version else {
            // No-version store: case-insensitive upsert at the default version.
            let mut tx = self.pool.begin().await?;
            sqlx::query(
                "DELETE FROM stored_query \
                 WHERE lower(reverse_domain_name) = lower($1) \
                 AND lower(semantic_id) = lower($2) AND semver = $3",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(DEFAULT_QUERY_VERSION)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO stored_query \
                 (reverse_domain_name, semantic_id, semver, query_type, query_text) \
                 VALUES ($1, $2, $3, 'AQL', $4)",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(DEFAULT_QUERY_VERSION)
            .bind(query_text)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(DEFAULT_QUERY_VERSION.to_owned());
        };

        // Versioned store: insert-only, immutable pair. A case-insensitive match
        // at this version is a 409 (BASE master05 §Composite Identifiers and
        // Case). The exact-case insert stays race-safe via `ON CONFLICT (rdn,
        // semantic, semver) DO NOTHING` — 0 affected rows also means the pair
        // already exists → 409.
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM stored_query \
             WHERE lower(reverse_domain_name) = lower($1) \
             AND lower(semantic_id) = lower($2) AND semver = $3)",
        )
        .bind(rdn)
        .bind(semantic)
        .bind(v)
        .fetch_one(&self.pool)
        .await?;
        if exists {
            return Err(ServiceError::Conflict(format!(
                "a stored query '{qualified_name}' at version '{v}' already exists"
            )));
        }
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
    /// (`{major}` or `{major}.{minor}` → the *highest* matching stored version,
    /// `parameters/path/version.yaml`, G-05-13), or the latest when no version
    /// is given. Identity is case-insensitive (G-05-14).
    pub(in crate::service) async fn get_stored_query(
        &self,
        qualified_name: &str,
        version: Option<&str>,
    ) -> Result<Value, ServiceError> {
        let (rdn, semantic) = split_qualified(qualified_name);
        let row = match version {
            Some(v) if is_partial_semver(v) => sqlx::query(
                // Prefix match on a dot boundary (`1` → `1.x.y`, `1.0` →
                // `1.0.x`), highest version first. `left(...)` (not LIKE) avoids
                // pattern-metacharacter escaping.
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2) \
                 AND (semver = $3 OR left(semver, length($3) + 1) = $3 || '.') \
                 ORDER BY string_to_array(semver, '.')::int[] DESC LIMIT 1",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(v),
            Some(v) => sqlx::query(
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2) \
                 AND semver = $3",
            )
            .bind(rdn)
            .bind(semantic)
            .bind(v),
            None => sqlx::query(
                "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, \
                 created_at FROM stored_query \
                 WHERE lower(reverse_domain_name) = lower($1) AND lower(semantic_id) = lower($2) \
                 ORDER BY string_to_array(semver, '.')::int[] DESC LIMIT 1",
            )
            .bind(rdn)
            .bind(semantic),
        }
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("stored query {qualified_name}")))?;

        Self::stored_query_json(&row)
    }

    /// List stored queries whose qualified name starts with `name_pattern`
    /// (`definition_query_list.yaml`: the name is a **prefix** — a bare
    /// `org.openehr` prefix "will list all versions of all queries with names
    /// starting with `org.openehr`"; empty ⇒ wildcard). All stored versions of
    /// every matching name are returned. The prefix match is case-insensitive
    /// (G-05-14).
    pub(in crate::service) async fn list_stored_queries(
        &self,
        name_pattern: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT reverse_domain_name, semantic_id, semver, query_type, query_text, created_at \
             FROM stored_query \
             WHERE left(lower(CASE WHEN reverse_domain_name = '' THEN semantic_id \
                             ELSE reverse_domain_name || '::' || semantic_id END), \
                        length($1)) = lower($1) \
             ORDER BY reverse_domain_name, semantic_id, string_to_array(semver, '.')::int[]",
        )
        .bind(name_pattern)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::stored_query_json).collect()
    }

    /// The openEHR stored-query descriptor for one row (the ITS-REST wire shape).
    ///
    /// Every projected column is `NOT NULL` (`0001_baseline.sql` §`stored_query`),
    /// so a decode failure is a genuine server fault, not an empty field:
    /// surface it (`?` → `500`) rather than silently blanking the value
    /// (W-14 F-29).
    fn stored_query_json(row: &PgRow) -> Result<Value, ServiceError> {
        let rdn = row.try_get::<String, _>("reverse_domain_name")?;
        let semantic = row.try_get::<String, _>("semantic_id")?;
        let name = if rdn.is_empty() {
            semantic
        } else {
            format!("{rdn}::{semantic}")
        };
        let saved = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
            .to_jiff()
            .to_string();
        Ok(json!({
            "name": name,
            "type": row.try_get::<String, _>("query_type")?,
            "version": row.try_get::<String, _>("semver")?,
            "saved": saved,
            "q": row.try_get::<String, _>("query_text")?,
        }))
    }
}

/// Build a [`QueryDescriptor`] from a `stored_query` row (per
/// `query_descriptor.adoc`). The qualified name is `rdn::semantic` (or the bare
/// `semantic` when the domain is empty); `formalism` is the `query_type`
/// lowercased (`QUERY_DESCRIPTOR` spells AQL `"aql"`).
///
/// Every projected column is `NOT NULL` (`0001_baseline.sql` §`stored_query`),
/// so a decode failure is a genuine server fault: surface it (`?` → `500`)
/// rather than silently blanking the descriptor field (W-14 F-29).
fn descriptor_from_row(row: &PgRow) -> Result<QueryDescriptor, ServiceError> {
    let rdn: String = row.try_get("reverse_domain_name")?;
    let semantic: String = row.try_get("semantic_id")?;
    let name = if rdn.is_empty() {
        semantic
    } else {
        format!("{rdn}::{semantic}")
    };
    let version: String = row.try_get("semver")?;
    let formalism: String = row.try_get::<String, _>("query_type")?.to_ascii_lowercase();
    let source: String = row.try_get("query_text")?;
    let registration_time = row
        .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
        .to_jiff()
        .to_string();
    Ok(QueryDescriptor {
        qualified_query_name: name,
        version: Some(version),
        registration_time,
        formalism,
        source: Some(source),
    })
}

/// True if `a_query_text` is a valid instance of the formalism `a_type`.
///
/// Only AQL major-version 1 is a known formalism (`valid_query`; `master04`
/// §Query Formalism); any other formalism → `false` (G-05-06 — the SM sanctions
/// "any other string value", which we reject typed since the store only holds
/// AQL). AQL validity is a successful `openehr_query` parse.
fn valid_query_text(text: &str, a_type: &str) -> bool {
    // AQL validity is a successful `openehr_query` parse.
    is_aql_v1(a_type) && openehr_query::parser::parse_str(text).is_ok()
}

/// Parse the `a_type` formalism per `master04` §Query Formalism (case-insensitive
/// name, optional `::version`, major `"1"` when absent) and report whether it
/// names AQL major-version 1.
fn is_aql_v1(a_type: &str) -> bool {
    let (name, version) = match a_type.split_once("::") {
        Some((n, v)) => (n, Some(v)),
        None => (a_type, None),
    };
    if !name.trim().eq_ignore_ascii_case("aql") {
        return false;
    }
    let major = version
        .and_then(|v| v.trim().split('.').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("1");
    major == "1"
}

/// Whether a `version` path parameter is a SEMVER *prefix* (`{major}` or
/// `{major}.{minor}`) rather than an exact `{major}.{minor}.{patch}` triple
/// (`parameters/path/version.yaml`, G-05-13).
fn is_partial_semver(version: &str) -> bool {
    let segments: Vec<&str> = version.split('.').collect();
    segments.len() < 3
        && segments
            .iter()
            .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

// ── SM Definitions native API (I_DEFINITION_QUERY) ───────────────────────────

impl EhrbaseService {
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn has_query(&self, a_query_name: String) -> Result<bool, SmError> {
        Ok(self.query_exists(&a_query_name).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub fn valid_query(&self, a_query_text: &str, a_type: &str) -> Result<bool, SmError> {
        Ok(Self::valid_query_source(a_query_text, a_type))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn store_query(
        &self,
        a_query_text: String,
        a_type: String,
        a_query_name: Option<String>,
    ) -> Result<QueryDescriptor, SmError> {
        Ok(self
            .query_store_sm(a_query_text, &a_type, a_query_name)
            .await?)
    }

    /// `store_query_set (a_query_set_name: String [0..1]): UUID` — "Register
    /// a query set. TODO: determine details."
    ///
    /// PORT NOTE: an explicit spec TODO with no defined semantics
    /// (`i_definition_query.adoc`) — `NotImplemented` (→ `501`) until the
    /// spec defines it.
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub fn store_query_set(&self, _a_query_set_name: Option<String>) -> Result<String, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "store_query_set is an SM TODO (i_definition_query.adoc): not implemented",
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_queries(&self, page: Page) -> Result<Vec<QueryDescriptor>, SmError> {
        Ok(self.query_list_response(page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_matching_queries(
        &self,
        id_pattern: String,
        artefact_id_pattern: Option<String>,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, SmError> {
        Ok(self
            .query_list_matching(&id_pattern, artefact_id_pattern.as_deref(), page)
            .await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn delete_query(&self, a_query_name: String) -> Result<(), SmError> {
        Ok(self.query_delete(&a_query_name).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn queries_count(&self) -> Result<i64, SmError> {
        Ok(self.query_count().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formalism_case_and_version_equivalence() {
        // "AQL" ≡ "aql" ≡ "AQL::1" (master04 §Query Formalism).
        assert!(is_aql_v1("AQL"));
        assert!(is_aql_v1("aql"));
        assert!(is_aql_v1("AQL::1"));
        assert!(is_aql_v1("aql::1.0.3"));
        // Other formalisms / other major versions are not known here (G-05-06).
        assert!(!is_aql_v1("AQL::2"));
        assert!(!is_aql_v1("cql"));
        assert!(!is_aql_v1("sparql::1"));
    }

    #[test]
    fn valid_query_needs_aql_and_a_parse() {
        assert!(valid_query_text("SELECT c FROM COMPOSITION c", "aql"));
        // Wrong formalism short-circuits to false without parsing.
        assert!(!valid_query_text("SELECT c FROM COMPOSITION c", "cql"));
        // AQL but unparseable.
        assert!(!valid_query_text("this is not aql", "AQL"));
    }

    #[test]
    fn partial_semver_detection() {
        // `{major}` / `{major}.{minor}` prefixes (version.yaml, G-05-13).
        assert!(is_partial_semver("1"));
        assert!(is_partial_semver("1.0"));
        assert!(is_partial_semver("12.34"));
        // Exact triples and malformed tokens are not prefixes.
        assert!(!is_partial_semver("1.0.0"));
        assert!(!is_partial_semver("1..0"));
        assert!(!is_partial_semver("1.a"));
        assert!(!is_partial_semver(""));
    }

    /// The store path rejects a body that is not valid AQL (ECC-SQR-006/007):
    /// `store_query` guards on `openehr_query::parser::parse_str`, so a non-AQL
    /// or malformed body must fail that parse (→ `BadRequest` → `400`).
    #[test]
    fn store_time_aql_validation() {
        assert!(
            openehr_query::parser::parse_str("SELECT * FROM patients; -- SQL, not AQL").is_err()
        );
        assert!(openehr_query::parser::parse_str("SELECT FROM WHERE {{{ not valid aql").is_err());
        assert!(openehr_query::parser::parse_str("SELECT c FROM COMPOSITION c").is_ok());
    }
}
