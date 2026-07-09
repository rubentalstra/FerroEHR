//! SM Definitions service logic (SM-2): ADL 1.4 source archetypes (on the
//! `archetype_store` table), ADL 1.4 OPTs (delegated to the existing
//! `template_store` machinery), and registered queries (on the existing
//! `stored_query` table).
//!
//! Realizes `I_DEFINITION_ADL14` + `I_DEFINITION_QUERY`
//! (`docs/specs/openehr/SM/docs/UML/classes/{i_definition_adl14,i_definition_query}.adoc`;
//! `master04-definition_package.adoc`). The SM pre/post-conditions are the test
//! oracle; the ITS-REST DEFINITION wire is unchanged (SM-2 is native-API only).
//!
//! PORT NOTE (no AOM source parser): openEHR has no BMM meta-model for AOM, and
//! the tree has no ADL 1.4 *source* parser (OPT XML is ingested at P13; ADL2 is
//! `501`). Archetype "validity" is therefore a lightweight structural check —
//! the source must begin with the ADL 1.4 `archetype` keyword line and carry a
//! well-formed `ARCHETYPE_ID` on the next line (parsed via
//! [`openehr_base::prelude::ArchetypeId`]). Full AOM validation lands when the
//! ADL 1.4 source parser does.

use std::str::FromStr;

use regex::Regex;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use ehrbase_sm::{CallStatusType, Page, QueryDescriptor};
use openehr_base::prelude::ArchetypeId;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    // ── ADL 1.4 archetypes (keyed by ARCHETYPE_ID) ───────────────────────────

    /// True if an ADL 1.4 archetype with id `an_id` is stored
    /// (`I_DEFINITION_ADL14.has_archetype`).
    pub(super) async fn archetype_exists(&self, an_id: &str) -> Result<bool, ServiceError> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM archetype_store WHERE archetype_id = $1)",
        )
        .bind(an_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Upload a valid ADL 1.4 archetype, replacing any existing one with the
    /// same id (`I_DEFINITION_ADL14.upload_archetype`,
    /// `Post_has_archetype`). Invalid source → `invalid_archetype` (`422`).
    pub(super) async fn archetype_upload(&self, adl: &str) -> Result<(), ServiceError> {
        let id = extract_archetype_id(adl).ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::InvalidArchetype,
                "ADL 1.4 source is not a valid archetype (missing `archetype` \
                 header or a well-formed ARCHETYPE_ID)",
            )
        })?;
        sqlx::query(
            "INSERT INTO archetype_store (archetype_id, adl) VALUES ($1, $2) \
             ON CONFLICT (archetype_id) DO UPDATE \
             SET adl = EXCLUDED.adl, created_at = now()",
        )
        .bind(id.value)
        .bind(adl)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The ADL 1.4 source of the archetype with id `an_id`
    /// (`I_DEFINITION_ADL14.get_archetype`); absent → `artefact_does_not_exist`
    /// (`404`).
    pub(super) async fn archetype_get(&self, an_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>("SELECT adl FROM archetype_store WHERE archetype_id = $1")
            .bind(an_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::ArtefactDoesNotExist,
                    format!("archetype {an_id}"),
                )
            })
    }

    /// The ids of all stored ADL 1.4 archetypes (`list_archetypes`).
    pub(super) async fn archetype_list(&self, page: Page) -> Result<Vec<String>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT archetype_id FROM archetype_store ORDER BY archetype_id OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Archetype ids matching `id_pattern` (a regex; `list_matching_archetypes`).
    /// An uncompilable pattern → `invalid_id_pattern` (`400`).
    pub(super) async fn archetype_list_matching(
        &self,
        id_pattern: &str,
        page: Page,
    ) -> Result<Vec<String>, ServiceError> {
        let re = compile_pattern(id_pattern)?;
        let all: Vec<String> =
            sqlx::query_scalar("SELECT archetype_id FROM archetype_store ORDER BY archetype_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(paginate(all.into_iter().filter(|id| re.is_match(id)), page))
    }

    /// Delete a previously uploaded archetype (`delete_archetype`,
    /// `Pre_artefact_exists`/`Post_archetype_removed`); absent → `404`.
    pub(super) async fn archetype_delete(&self, an_id: &str) -> Result<(), ServiceError> {
        let deleted = sqlx::query("DELETE FROM archetype_store WHERE archetype_id = $1")
            .bind(an_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::sm(
                CallStatusType::ArtefactDoesNotExist,
                format!("archetype {an_id}"),
            ));
        }
        Ok(())
    }

    /// Total archetypes count (`archetypes_count`).
    pub(super) async fn archetype_count(&self) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM archetype_store")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    // ── ADL 1.4 OPTs (keyed by UUID; on `template_store`) ────────────────────

    /// True if an OPT with `id` (a `UUID`) is stored (`has_opt`). An
    /// unparseable UUID is a `400`.
    pub(super) async fn opt_exists(&self, an_opt_id: &str) -> Result<bool, ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM template_store WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// The OPT 1.4 canonical XML of the OPT with `id` (a `UUID`; `get_opt`);
    /// absent → `artefact_does_not_exist` (`404`). Unparseable UUID → `400`.
    pub(super) async fn opt_get(&self, an_opt_id: &str) -> Result<String, ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        sqlx::query_scalar::<_, String>("SELECT content FROM template_store WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::ArtefactDoesNotExist,
                    format!("OPT {an_opt_id}"),
                )
            })
    }

    /// The ids (`UUID`s) of all stored OPTs, oldest first (`list_opts`).
    pub(super) async fn opt_list(&self, page: Page) -> Result<Vec<String>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM template_store ORDER BY created_at, id OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(ids.into_iter().map(|u| u.to_string()).collect())
    }

    /// OPTs whose `template_id` matches `id_pattern` (a regex;
    /// `list_matching_opts`). Uncompilable pattern → `invalid_id_pattern`
    /// (`400`).
    ///
    /// PORT NOTE (spec defect): the SM types this `List<ARCHETYPE_ID>` though
    /// OPTs are UUID-keyed; we return the OPTs' `template_id` strings (the
    /// meaningful identifier a pattern is useful against).
    pub(super) async fn opt_list_matching(
        &self,
        id_pattern: &str,
        page: Page,
    ) -> Result<Vec<String>, ServiceError> {
        let re = compile_pattern(id_pattern)?;
        let all: Vec<String> =
            sqlx::query_scalar("SELECT template_id FROM template_store ORDER BY template_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(paginate(
            all.into_iter().filter(|tid| re.is_match(tid)),
            page,
        ))
    }

    /// Delete an OPT by `id` (a `UUID`; `delete_opt`,
    /// `Pre_has_opt`/`Post_opt_removed`); absent → `404`. Unparseable UUID →
    /// `400`.
    pub(super) async fn opt_delete(&self, an_opt_id: &str) -> Result<(), ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        let deleted = sqlx::query("DELETE FROM template_store WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::sm(
                CallStatusType::ArtefactDoesNotExist,
                format!("OPT {an_opt_id}"),
            ));
        }
        Ok(())
    }

    /// Total OPTs count (`opts_count`).
    pub(super) async fn opt_count(&self) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM template_store")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    // ── ADL2 artefacts (keyed by ARCHETYPE_HRID; on `adl2_artefact`) ──────────

    /// True if an ADL2 artefact with `ARCHETYPE_HRID` `an_id` is stored
    /// (`I_DEFINITION_ADL2.has_artefact`).
    pub(super) async fn adl2_exists(&self, an_id: &str) -> Result<bool, ServiceError> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM adl2_artefact WHERE hrid = $1)",
        )
        .bind(an_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Upload a valid ADL2 artefact, replacing any existing one with the same
    /// `ARCHETYPE_HRID` (`I_DEFINITION_ADL2.upload_artefact`: "If an artefact
    /// with the same physical identifier and namespace exists, replace it").
    /// Invalid source → `invalid artefact` (`422`). Returns the stored HRID (the
    /// wire needs it for the `Location` header + identifier body).
    pub(super) async fn adl2_upload(&self, adl2: &str) -> Result<String, ServiceError> {
        let (kind, hrid) = extract_adl2_header(adl2).ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::InvalidArtefact,
                "ADL2 source is not a valid artefact (missing an \
                 archetype/template/operational_template header or a well-formed \
                 ARCHETYPE_HRID)",
            )
        })?;
        sqlx::query(
            "INSERT INTO adl2_artefact (hrid, kind, adl) VALUES ($1, $2, $3) \
             ON CONFLICT (hrid) DO UPDATE \
             SET kind = EXCLUDED.kind, adl = EXCLUDED.adl, created_at = now()",
        )
        .bind(&hrid)
        .bind(kind)
        .bind(adl2)
        .execute(&self.pool)
        .await?;
        Ok(hrid)
    }

    /// The ADL2 source of the artefact with `ARCHETYPE_HRID` `an_id`
    /// (`I_DEFINITION_ADL2.get_artefact`); absent → `artefact_does_not_exist`
    /// (`404`).
    pub(super) async fn adl2_get(&self, an_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>("SELECT adl FROM adl2_artefact WHERE hrid = $1")
            .bind(an_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::ArtefactDoesNotExist,
                    format!("ADL2 artefact {an_id}"),
                )
            })
    }

    /// The `ARCHETYPE_HRID`s of all stored ADL2 artefacts (`list_artefacts`).
    pub(super) async fn adl2_list(&self, page: Page) -> Result<Vec<String>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT hrid FROM adl2_artefact ORDER BY hrid OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The `ARCHETYPE_HRID`s of stored ADL2 artefacts of one concrete `kind`
    /// (`list_archetypes` / `list_templates` / `list_opts`).
    pub(super) async fn adl2_list_by_kind(
        &self,
        kind: &str,
        page: Page,
    ) -> Result<Vec<String>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT hrid FROM adl2_artefact WHERE kind = $1 ORDER BY hrid OFFSET $2 LIMIT $3",
        )
        .bind(kind)
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// ADL2 artefact HRIDs matching `id_pattern` (a regex;
    /// `list_matching_artefacts`). An uncompilable pattern →
    /// `invalid_id_pattern` (`400`).
    pub(super) async fn adl2_list_matching(
        &self,
        id_pattern: &str,
        page: Page,
    ) -> Result<Vec<String>, ServiceError> {
        let re = compile_pattern(id_pattern)?;
        let all: Vec<String> = sqlx::query_scalar("SELECT hrid FROM adl2_artefact ORDER BY hrid")
            .fetch_all(&self.pool)
            .await?;
        Ok(paginate(all.into_iter().filter(|id| re.is_match(id)), page))
    }

    /// Delete the ADL2 artefact with `ARCHETYPE_HRID` `an_id`
    /// (`delete_artefact`); absent → `artefact_does_not_exist` (`404`).
    pub(super) async fn adl2_delete(&self, an_id: &str) -> Result<(), ServiceError> {
        let deleted = sqlx::query("DELETE FROM adl2_artefact WHERE hrid = $1")
            .bind(an_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::sm(
                CallStatusType::ArtefactDoesNotExist,
                format!("ADL2 artefact {an_id}"),
            ));
        }
        Ok(())
    }

    /// Total ADL2 artefacts count (`artefacts_count`).
    pub(super) async fn adl2_count(&self) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM adl2_artefact")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Total ADL2 artefacts of one concrete `kind` (`archetypes_count` /
    /// `templates_count` / `opts_count`).
    pub(super) async fn adl2_count_by_kind(&self, kind: &str) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM adl2_artefact WHERE kind = $1")
                .bind(kind)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// The wire list for `GET /definition/template/adl2` (the OAS `TemplateList`,
    /// `definition-codegen.openapi.yaml` `200_TemplateList_adl2`): the ADL2
    /// templates and OPTs, as `{template_id, created_timestamp}` metadata objects.
    ///
    /// PORT NOTE: the OAS `TemplateMetadata` also carries `concept` and
    /// `archetype_id`, which are derived from the cADL body (the archetype's
    /// concept description + `specialize` target). With no ADL2/cADL source
    /// parser yet those fields are omitted; `template_id` is the `ARCHETYPE_HRID`.
    /// The endpoint lists both `template` and `operational_template` kinds (the
    /// "templates" under `/definition/template/adl2`), not source archetypes.
    pub(super) async fn adl2_template_list(
        &self,
        page: Page,
    ) -> Result<Vec<serde_json::Value>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        let rows = sqlx::query(
            "SELECT hrid, created_at FROM adl2_artefact \
             WHERE kind IN ('template', 'operational_template') \
             ORDER BY hrid OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let hrid: String = row.try_get("hrid").unwrap_or_default();
                let created = row
                    .try_get::<jiff_sqlx::Timestamp, _>("created_at")
                    .map(|t| t.to_jiff().to_string())
                    .unwrap_or_default();
                serde_json::json!({ "template_id": hrid, "created_timestamp": created })
            })
            .collect())
    }

    // ── registered queries (on `stored_query`) ───────────────────────────────

    /// True if a query with the qualified name `a_query_name` is registered
    /// (`has_query`); the `"misc"` namespace is assumed when none is supplied.
    pub(super) async fn query_exists(&self, a_query_name: &str) -> Result<bool, ServiceError> {
        let qualified = qualify(a_query_name);
        let (rdn, semantic) = split_qualified(&qualified);
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM stored_query \
             WHERE reverse_domain_name = $1 AND semantic_id = $2)",
        )
        .bind(rdn)
        .bind(semantic)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Register a query, returning its [`QueryDescriptor`] (`store_query`).
    ///
    /// The name defaults to `"misc"` when unqualified, or is generated
    /// (`misc::q_<uuid>`) when absent. Reuses the existing (wire-tested)
    /// `store_query` upsert machinery unchanged.
    ///
    /// PORT NOTE (spec naming): the SM precondition names `is_valid_query`
    /// though the function is `valid_query`; we enforce `valid_query` and reject
    /// an invalid query as `invalid_query` (`422`).
    pub(super) async fn query_store_sm(
        &self,
        text: String,
        a_type: &str,
        name: Option<String>,
    ) -> Result<QueryDescriptor, ServiceError> {
        if !valid_query_text(&text, a_type) {
            return Err(ServiceError::sm(
                CallStatusType::InvalidQuery,
                "query text is not a valid instance of its formalism",
            ));
        }
        let qualified = match name {
            Some(n) => qualify(&n),
            None => format!("misc::q_{}", Uuid::now_v7().simple()),
        };
        let version = self.store_query(&qualified, None, text).await?;
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
             WHERE reverse_domain_name = $1 AND semantic_id = $2 AND semver = $3",
        )
        .bind(rdn)
        .bind(semantic)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("stored query {qualified}")))?;
        Ok(descriptor_from_row(&row))
    }

    /// All registered queries, as descriptors (`list_queries`).
    pub(super) async fn query_list(
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
        Ok(rows.iter().map(descriptor_from_row).collect())
    }

    /// Registered queries whose qualified name matches `id_pattern` (regex) and
    /// whose source matches `artefact_id_pattern` (regex, `None` = match any;
    /// `list_matching_queries`). Uncompilable pattern → `invalid_id_pattern`
    /// (`400`).
    ///
    /// PORT NOTE: `artefact_id_pattern` is spec'd against "archetype / template
    /// identifiers referenced in the query"; with no AQL artefact extractor yet,
    /// we approximate by regex-scanning the stored source text — a query matches
    /// when its source contains a substring matching the artefact pattern.
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
        let matched = rows.iter().map(descriptor_from_row).filter(|d| {
            id_re.is_match(&d.qualified_query_name)
                && artefact_re
                    .as_ref()
                    .is_none_or(|re| d.source.as_deref().is_some_and(|src| re.is_match(src)))
        });
        Ok(paginate(matched, page))
    }

    /// Delete every version of the query with qualified name `a_query_name`
    /// (`delete_query`, `Pre_has_query`/`Post_query_deleted`); absent → `404`.
    ///
    /// PORT NOTE: the SM keys deletion by *name* (not name+version), so this
    /// removes all stored versions of the qualified name.
    pub(super) async fn query_delete(&self, a_query_name: &str) -> Result<(), ServiceError> {
        let qualified = qualify(a_query_name);
        let (rdn, semantic) = split_qualified(&qualified);
        let deleted = sqlx::query(
            "DELETE FROM stored_query WHERE reverse_domain_name = $1 AND semantic_id = $2",
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

    /// Total count of queries (`queries_count`).
    ///
    /// PORT NOTE: counts distinct *qualified names* (a query with N stored
    /// versions counts once), the natural reading of "total count of queries".
    pub(super) async fn query_count(&self) -> Result<i64, ServiceError> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM \
             (SELECT DISTINCT reverse_domain_name, semantic_id FROM stored_query) t",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    // ── validity checks (pure; no DB) ────────────────────────────────────────

    /// `valid_archetype` — structural validity of ADL 1.4 source (see the
    /// module PORT NOTE). Stateless (an associated fn); no DB is consulted.
    #[must_use]
    pub(super) fn valid_archetype_source(adl: &str) -> bool {
        extract_archetype_id(adl).is_some()
    }

    /// `valid_opt` — the OPT parses (`opt14::from_xml`) and passes the upload
    /// structural check (`validate_opt_structure`).
    #[must_use]
    pub(super) fn valid_opt_xml(opt_xml: &str) -> bool {
        openehr_its::opt14::from_xml(opt_xml).is_ok()
            && super::template::validate_opt_structure(opt_xml).is_ok()
    }

    /// `valid_query` — `a_query_text` is a valid instance of formalism `a_type`.
    #[must_use]
    pub(super) fn valid_query_source(a_query_text: &str, a_type: &str) -> bool {
        valid_query_text(a_query_text, a_type)
    }

    /// `valid_artefact` (ADL2) — structural validity of ADL2 source (see the
    /// [`DefinitionAdl2Service`](ehrbase_sm::DefinitionAdl2Service) trait PORT
    /// NOTE). Stateless; no DB is consulted.
    #[must_use]
    pub(super) fn valid_adl2_source(adl2: &str) -> bool {
        extract_adl2_header(adl2).is_some()
    }
}

// ─── free helpers ────────────────────────────────────────────────────────────

/// Build a [`QueryDescriptor`] from a `stored_query` row (per
/// `query_descriptor.adoc`). The qualified name is reconstructed as the store
/// records it (`rdn::semantic`, or the bare `semantic` when the domain is
/// empty); `formalism` is the `query_type` lowercased (`QUERY_DESCRIPTOR`
/// spells AQL `"aql"`).
fn descriptor_from_row(row: &PgRow) -> QueryDescriptor {
    let rdn: String = row.try_get("reverse_domain_name").unwrap_or_default();
    let semantic: String = row.try_get("semantic_id").unwrap_or_default();
    let name = if rdn.is_empty() {
        semantic
    } else {
        format!("{rdn}::{semantic}")
    };
    let version: String = row.try_get("semver").unwrap_or_default();
    let formalism: String = row
        .try_get::<String, _>("query_type")
        .unwrap_or_else(|_| "AQL".to_owned())
        .to_ascii_lowercase();
    let source: String = row.try_get("query_text").unwrap_or_default();
    let registration_time = row
        .try_get::<jiff_sqlx::Timestamp, _>("created_at")
        .map(|t| t.to_jiff().to_string())
        .unwrap_or_default();
    QueryDescriptor {
        qualified_query_name: name,
        version: Some(version),
        registration_time,
        formalism,
        source: Some(source),
    }
}

/// Apply an SM [`Page`] to an iterator: skip `item_offset`, take `items_to_fetch`
/// (`None`/0 ⇒ all — `master02-overview.adoc` §List Handling).
fn paginate<T>(items: impl Iterator<Item = T>, page: Page) -> Vec<T> {
    let offset = usize::try_from(page.offset()).unwrap_or(usize::MAX);
    let skipped = items.skip(offset);
    match page.limit() {
        Some(n) => skipped
            .take(usize::try_from(n).unwrap_or(usize::MAX))
            .collect(),
        None => skipped.collect(),
    }
}

/// A [`Page`] as `(offset, limit)` SQL bind values; a `None` limit binds SQL
/// `NULL` (`LIMIT NULL` = all rows in `PostgreSQL`).
fn page_bounds(page: Page) -> (i64, Option<i64>) {
    let offset = i64::try_from(page.offset()).unwrap_or(i64::MAX);
    let limit = page.limit().and_then(|l| i64::try_from(l).ok());
    (offset, limit)
}

/// Compile an id-pattern regex; an uncompilable pattern is `invalid_id_pattern`
/// (`400`).
///
/// PORT NOTE: the SM spells these "PERL regular expression"; `regex` is
/// RE2-class, so PERL backreferences / lookaround are unsupported — a pattern
/// using them fails to compile and surfaces as `invalid_id_pattern`, which is
/// the correct SM outcome for an unusable pattern.
fn compile_pattern(pattern: &str) -> Result<Regex, ServiceError> {
    Regex::new(pattern).map_err(|e| {
        ServiceError::sm(
            CallStatusType::InvalidIdPattern,
            format!("invalid id pattern: {e}"),
        )
    })
}

/// Apply the SM `"misc"` default namespace: a name with no `::` is qualified
/// as `misc::<name>` (`master04-definition_package.adoc`: "If no namespace is
/// supplied, the namespace `"misc"` is assumed").
fn qualify(name: &str) -> String {
    if name.contains("::") {
        name.to_owned()
    } else {
        format!("misc::{name}")
    }
}

/// Split a (already-qualified) query name into `(reverse_domain_name,
/// semantic_id)` on the first `::` — matching the `stored_query` store's own
/// split so the SM and wire paths key rows identically.
fn split_qualified(qualified_name: &str) -> (&str, &str) {
    qualified_name
        .split_once("::")
        .unwrap_or(("", qualified_name))
}

/// True if `a_query_text` is a valid instance of the formalism `a_type`.
///
/// Only AQL major-version 1 is a known formalism (`valid_query` /
/// `master04-definition_package.adoc`); any other formalism → `false` (the SM
/// sanctions "any other string value"). AQL validity is a successful
/// `openehr_query` parse.
fn valid_query_text(text: &str, a_type: &str) -> bool {
    is_aql_v1(a_type) && openehr_query::parser::parse_str(text).is_ok()
}

/// Parse the `a_type` formalism per `master04-definition_package.adoc`
/// (case-insensitive name, optional `::version`, major `"1"` when absent) and
/// report whether it names AQL major-version 1.
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

/// Parse an OPT id UUID string; an unparseable value is a `400`.
fn parse_opt_uuid(an_opt_id: &str) -> Result<Uuid, ServiceError> {
    Uuid::parse_str(an_opt_id)
        .map_err(|_| ServiceError::BadRequest(format!("OPT id is not a UUID: {an_opt_id}")))
}

/// Extract the `ARCHETYPE_ID` from ADL 1.4 source: the source must begin with
/// the `archetype` keyword line (optionally `archetype (adl_version=…)`) and
/// carry a well-formed `ARCHETYPE_ID` on the next non-blank line.
fn extract_archetype_id(adl: &str) -> Option<ArchetypeId> {
    // Tolerate a leading UTF-8 BOM (present in the vendored .adl fixtures).
    let adl = adl.trim_start_matches('\u{feff}');
    let mut lines = adl.lines().map(str::trim).filter(|l| !l.is_empty());
    let header = lines.next()?;
    let is_header = header == "archetype"
        || header.starts_with("archetype ")
        || header.starts_with("archetype(");
    if !is_header {
        return None;
    }
    let id_line = lines.next()?;
    ArchetypeId::from_str(id_line).ok()
}

/// Extract `(kind, ARCHETYPE_HRID)` from ADL2 source. The source must begin
/// (after any BOM, blank lines and `--` line comments) with an artefact-kind
/// keyword line — `archetype`, `template`, or `operational_template`,
/// optionally followed by `(adl_version=2…; …)` attributes — and carry a
/// well-formed `ARCHETYPE_HRID` on the next non-blank line
/// (`docs/specs/openehr/SM/docs/UML/classes/i_definition_adl2.adoc`;
/// `master04-definition_package.adoc`). The keyword becomes the stored `kind`.
///
/// PORT NOTE: this is a lightweight structural probe, not a cADL parse (no ADL2
/// source parser exists in the tree yet); it accepts the source shape the ADL2
/// spec's `operational_template (…)` / `archetype (…)` / `template (…)` headers
/// use.
fn extract_adl2_header(adl2: &str) -> Option<(&'static str, String)> {
    let text = adl2.trim_start_matches('\u{feff}');
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("--"));
    let header = lines.next()?;
    // The kind keyword is the first token, before any whitespace or `(` attrs.
    let keyword = header.split(['(', ' ', '\t']).next()?;
    let kind = match keyword {
        "archetype" => "archetype",
        "template" => "template",
        "operational_template" => "operational_template",
        _ => return None,
    };
    let hrid_line = lines.next()?;
    valid_adl2_hrid(hrid_line).then(|| (kind, hrid_line.to_owned()))
}

/// Structural check for an `ARCHETYPE_HRID`: an optional `namespace::` prefix
/// followed by an openEHR HRID
/// (`rm_publisher '-' rm_package '-' rm_class '.' concept_id {'-' spec}* '.v' version`,
/// e.g. `openEHR-EHR-OBSERVATION.bp.v1.0.0`).
///
/// PORT NOTE: reuses [`ArchetypeId::from_str`], whose lexical form accepts the
/// HRID shape (it is a superset of the ADL 1.4 `ARCHETYPE_ID`, tolerating the
/// full multi-part `.vN.N.N` version). A stricter AOM2 `ARCHETYPE_HRID` grammar
/// (`version_status` / `build_count` suffixes) awaits the ADL2 parser.
fn valid_adl2_hrid(hrid: &str) -> bool {
    let core = hrid.rsplit_once("::").map_or(hrid, |(_, rest)| rest);
    ArchetypeId::from_str(core).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_applies_misc_default() {
        // master04: no namespace ⇒ "misc".
        assert_eq!(qualify("all_over_50"), "misc::all_over_50");
        assert_eq!(qualify("ehr::x"), "ehr::x");
        assert_eq!(qualify("ns::aql::x"), "ns::aql::x");
    }

    #[test]
    fn formalism_case_and_version_equivalence() {
        // "AQL" ≡ "aql" ≡ "AQL::1" (master04 §Query Formalism).
        assert!(is_aql_v1("AQL"));
        assert!(is_aql_v1("aql"));
        assert!(is_aql_v1("AQL::1"));
        assert!(is_aql_v1("aql::1.0.3"));
        // Other formalisms / other major versions are not known here.
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
    fn archetype_id_extraction() {
        let adl = "\u{feff}archetype (adl_version=1.4)\n\
                   \topenEHR-EHR-COMPOSITION.prescription.v1\n\n\
                   concept\n\t[at0000]";
        assert_eq!(
            extract_archetype_id(adl).map(|a| a.value),
            Some("openEHR-EHR-COMPOSITION.prescription.v1".to_owned())
        );
        // No `archetype` header → not valid.
        assert!(extract_archetype_id("concept\n[at0000]").is_none());
        // Header but the id line is not a well-formed ARCHETYPE_ID.
        assert!(extract_archetype_id("archetype\n  not-an-archetype-id").is_none());
    }

    #[test]
    fn adl2_header_extraction() {
        // operational_template header with attrs → kind + HRID (master04 example).
        let opt = "operational_template (adl_version=2.0.6; rm_release=1.0.2; generated)\n\
                   \topenEHR-EHR-COMPOSITION.t_clinical_info_ds_sf.v1.0.0\n\n\
                   specialize\n\topenEHR-EHR-COMPOSITION.discharge.v1";
        assert_eq!(
            extract_adl2_header(opt),
            Some((
                "operational_template",
                "openEHR-EHR-COMPOSITION.t_clinical_info_ds_sf.v1.0.0".to_owned()
            ))
        );
        // Bare `archetype` header + a namespace-qualified HRID.
        let arch = "-- a comment\narchetype\ncom.example::openEHR-EHR-OBSERVATION.bp.v1.0.0\n";
        assert_eq!(
            extract_adl2_header(arch),
            Some((
                "archetype",
                "com.example::openEHR-EHR-OBSERVATION.bp.v1.0.0".to_owned()
            ))
        );
        // `template` kind.
        assert_eq!(
            extract_adl2_header("template\nopenEHR-EHR-COMPOSITION.t_x.v2.0.0").map(|(k, _)| k),
            Some("template")
        );
        // Unrecognised header keyword → not an artefact.
        assert!(extract_adl2_header("concept\nopenEHR-EHR-OBSERVATION.bp.v1.0.0").is_none());
        // Recognised header but a malformed HRID → invalid.
        assert!(extract_adl2_header("archetype\nnot-an-hrid").is_none());
    }
}
