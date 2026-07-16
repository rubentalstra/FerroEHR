//! `I_DEFINITION_ADL14` (`i_definition_adl14.adoc`; `master04` §Archetypes and
//! Templates): ADL 1.4 source archetypes keyed by `ARCHETYPE_ID` (on
//! `archetype_store`) and OPTs keyed by `UUID` (on `template_store`).
//!
//! PORT NOTE (G-05-01, no AOM 1.4 source parser): the tree has no ADL 1.4
//! *source* parser (OPT XML is ingested by the templates seam; ADL2 has its own
//! registration validator). ADL 1.4 archetype "validity" is therefore a
//! lightweight *structural* check — the source must open with the `archetype`
//! keyword line and carry a well-formed `ARCHETYPE_ID`
//! ([`openehr_base::prelude::ArchetypeId`]) on the next line. Full AOM 1.4
//! validation lands with the ADL 1.4 source parser (register 10 dependency).

use std::str::FromStr;

use openehr_base::prelude::ArchetypeId;
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::service::list::Page;
use crate::service::status::{CallStatusType, SmError};

use super::{compile_pattern, page_bounds, paginate};

// ── SM Definitions native API (I_DEFINITION_ADL14) — the catalog contract ────

impl EhrbaseService {
    /// `has_archetype` — true if an ADL 1.4 archetype with id `an_id` is
    /// stored. Identity is compared case-insensitively (BASE master05
    /// §Composite Identifiers and Case).
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn has_archetype(&self, an_id: String) -> Result<bool, SmError> {
        Ok(self.archetype_exists(&an_id).await?)
    }

    /// `valid_archetype` — structural validity of ADL 1.4 source (module PORT
    /// NOTE G-05-01): the source opens with the `archetype` keyword line and
    /// carries a well-formed `ARCHETYPE_ID` on the next non-blank line.
    /// Stateless.
    ///
    /// # Errors
    ///
    /// Never — the `Result` shape mirrors the SM catalog; validity is reported
    /// in the `Ok` boolean.
    pub fn valid_archetype(&self, adl: &str) -> Result<bool, SmError> {
        Ok(extract_archetype_id(adl).is_some())
    }

    /// `upload_archetype` (`Post_has_archetype`) — store a valid ADL 1.4
    /// archetype, replacing any existing one with the same id.
    ///
    /// Identity is case-insensitive but storage case-preserving (BASE master05
    /// §Composite Identifiers and Case): the write removes any case-variant of
    /// the id in the same transaction, then inserts the source verbatim — so a
    /// re-upload with different case *replaces* rather than duplicates.
    ///
    /// # Errors
    ///
    /// - Structurally invalid ADL 1.4 source (no `archetype` header or no
    /// well-formed `ARCHETYPE_ID`) → `invalid_archetype` (`422`).
    /// - A database failure (`exception` → `500`).
    pub async fn upload_archetype(&self, adl: String) -> Result<(), SmError> {
        Ok(self.archetype_upload(&adl).await?)
    }

    /// `get_archetype` — the ADL 1.4 source of the archetype with id `an_id`
    /// (interchange form, G-05-03). Identity is case-insensitive.
    ///
    /// # Errors
    ///
    /// - No archetype with that id → `artefact_does_not_exist` (`404`).
    /// - A database failure (`exception` → `500`).
    pub async fn get_archetype(&self, an_id: String) -> Result<String, SmError> {
        Ok(self.archetype_get(&an_id).await?)
    }

    /// `list_archetypes` — the ids of all stored ADL 1.4 archetypes, cursored
    /// by `page` (`master02-overview.adoc` §List Handling).
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn list_archetypes_adl14(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.archetype_list(page).await?)
    }

    /// `list_matching_archetypes` — archetype ids matching `id_pattern` (a
    /// regex), cursored by `page`.
    ///
    /// # Errors
    ///
    /// - An uncompilable `id_pattern` → `invalid_id_pattern` (`400`).
    /// - A database failure (`exception` → `500`).
    pub async fn list_matching_archetypes(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.archetype_list_matching(&id_pattern, page).await?)
    }

    /// `delete_archetype` (`Pre_artefact_exists` / `Post_archetype_removed`) —
    /// delete an archetype by id (case-insensitive).
    ///
    /// # Errors
    ///
    /// - No archetype with that id → `artefact_does_not_exist` (`404`).
    /// - A database failure (`exception` → `500`).
    pub async fn delete_archetype(&self, an_id: String) -> Result<(), SmError> {
        Ok(self.archetype_delete(&an_id).await?)
    }

    /// `archetypes_count` — total stored ADL 1.4 archetypes.
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn archetypes_count_adl14(&self) -> Result<i64, SmError> {
        Ok(self.archetype_count().await?)
    }

    /// `has_opt` — true if an OPT with `an_opt_id` (a `UUID`) is stored.
    ///
    /// # Errors
    ///
    /// - `an_opt_id` is not a parseable `UUID` → `precondition_violation`
    /// (`400`).
    /// - A database failure (`exception` → `500`).
    pub async fn has_opt(&self, an_opt_id: String) -> Result<bool, SmError> {
        Ok(self.opt_exists(&an_opt_id).await?)
    }

    /// `valid_opt` — the OPT parses (`opt14::from_xml`) and passes the
    /// templates seam's structural check. Stateless.
    ///
    /// # Errors
    ///
    /// Never — the `Result` shape mirrors the SM catalog; validity is reported
    /// in the `Ok` boolean.
    pub fn valid_opt(&self, opt_xml: &str) -> Result<bool, SmError> {
        Ok(valid_opt_xml(opt_xml))
    }

    /// `upload_opt` — store an OPT 1.4 canonical-XML template. Ingestion runs
    /// in the templates layer: `store_template` parses + structurally
    /// validates the OPT and stores it create-only.
    ///
    /// # Errors
    ///
    /// - Unparseable / structurally invalid OPT XML → `invalid_template`
    /// (`422`).
    /// - A template with the same `template_id` already stored → conflict
    /// (`409`).
    /// - A database failure (`exception` → `500`).
    pub async fn upload_opt(&self, opt_xml: String) -> Result<(), SmError> {
        self.store_template(&opt_xml).await?;
        Ok(())
    }

    /// `get_opt` — the OPT 1.4 canonical XML of the OPT with `an_opt_id` (a
    /// `UUID`; interchange form, G-05-03).
    ///
    /// # Errors
    ///
    /// - `an_opt_id` is not a parseable `UUID` → `precondition_violation`
    /// (`400`).
    /// - No OPT with that id → `template_does_not_exist` (`404`, G-05-15).
    /// - A database failure (`exception` → `500`).
    pub async fn get_opt(&self, an_opt_id: String) -> Result<String, SmError> {
        Ok(self.opt_get(&an_opt_id).await?)
    }

    /// `list_opts` — the ids (`UUID`s) of all stored OPTs, oldest first,
    /// cursored by `page`.
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn list_opts_adl14(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.opt_list(page).await?)
    }

    /// `list_matching_opts` — OPTs whose `template_id` matches `id_pattern`
    /// (a regex), cursored by `page`.
    ///
    /// PORT NOTE (G-05-08, spec defect): the SM types this `List<ARCHETYPE_ID>`
    /// though OPTs are UUID-keyed; we return the OPTs' `template_id` strings
    /// (the meaningful identifier a pattern is useful against).
    ///
    /// # Errors
    ///
    /// - An uncompilable `id_pattern` → `invalid_id_pattern` (`400`).
    /// - A database failure (`exception` → `500`).
    pub async fn list_matching_opts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.opt_list_matching(&id_pattern, page).await?)
    }

    /// `delete_opt` (`Pre_has_opt` / `Post_opt_removed`) — delete an OPT by
    /// `an_opt_id` (a `UUID`), evicting its derived-runtime (`WebTemplate`)
    /// cache entry.
    ///
    /// # Errors
    ///
    /// - `an_opt_id` is not a parseable `UUID` → `precondition_violation`
    /// (`400`).
    /// - No OPT with that id → `template_does_not_exist` (`404`, G-05-15).
    /// - A database failure (`exception` → `500`).
    pub async fn delete_opt(&self, an_opt_id: String) -> Result<(), SmError> {
        Ok(self.opt_delete(&an_opt_id).await?)
    }

    /// `opts_count` — total stored OPTs.
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn opts_count_adl14(&self) -> Result<i64, SmError> {
        Ok(self.opt_count().await?)
    }
}

// ── domain logic (the ServiceError layer under the catalog) ──────────────────

impl EhrbaseService {
    /// True if an ADL 1.4 archetype with id `an_id` is stored
    /// (case-insensitive identity).
    async fn archetype_exists(&self, an_id: &str) -> Result<bool, ServiceError> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM archetype_store WHERE lower(archetype_id) = lower($1))",
        )
        .bind(an_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Store a valid ADL 1.4 archetype, replacing any case-variant of the same
    /// id in the same transaction; invalid source → `invalid_archetype` (`422`).
    async fn archetype_upload(&self, adl: &str) -> Result<(), ServiceError> {
        let id = extract_archetype_id(adl).ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::InvalidArchetype,
                "ADL 1.4 source is not a valid archetype (missing `archetype` \
                 header or a well-formed ARCHETYPE_ID)",
            )
        })?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM archetype_store WHERE lower(archetype_id) = lower($1)")
            .bind(&id.value)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO archetype_store (archetype_id, adl) VALUES ($1, $2)")
            .bind(&id.value)
            .bind(adl)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The ADL 1.4 source of the archetype with id `an_id`; absent →
    /// `artefact_does_not_exist` (`404`).
    async fn archetype_get(&self, an_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>(
            "SELECT adl FROM archetype_store WHERE lower(archetype_id) = lower($1)",
        )
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

    /// The ids of all stored ADL 1.4 archetypes, paged in SQL.
    async fn archetype_list(&self, page: Page) -> Result<Vec<String>, ServiceError> {
        let (offset, limit) = page_bounds(page);
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT archetype_id FROM archetype_store ORDER BY archetype_id OFFSET $1 LIMIT $2",
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Archetype ids matching `id_pattern` (regex; uncompilable →
    /// `invalid_id_pattern`, `400`), then paged.
    async fn archetype_list_matching(
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

    /// Delete an archetype by id (case-insensitive); absent →
    /// `artefact_does_not_exist` (`404`).
    async fn archetype_delete(&self, an_id: &str) -> Result<(), ServiceError> {
        let deleted =
            sqlx::query("DELETE FROM archetype_store WHERE lower(archetype_id) = lower($1)")
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

    /// Total stored ADL 1.4 archetypes.
    async fn archetype_count(&self) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM archetype_store")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// True if an OPT with `an_opt_id` (a `UUID`) is stored; unparseable UUID
    /// → `400`.
    async fn opt_exists(&self, an_opt_id: &str) -> Result<bool, ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM template_store WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// The OPT 1.4 canonical XML of the OPT with `an_opt_id` (a `UUID`);
    /// absent → `template_does_not_exist` (`404`, G-05-15); unparseable UUID
    /// → `400`.
    async fn opt_get(&self, an_opt_id: &str) -> Result<String, ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        sqlx::query_scalar::<_, String>("SELECT content FROM template_store WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::TemplateDoesNotExist,
                    format!("OPT {an_opt_id}"),
                )
            })
    }

    /// The OPT 1.4 canonical XML addressed by its `template_id` string (the
    /// ITS-REST wire address, unlike the SM's UUID-keyed [`opt_get`](Self::opt_get)).
    /// Spec-silent: `template_id` addressing is our own wire helper, the SM
    /// keys OPTs by `UUID`. Absent → `template_does_not_exist` (`404`).
    pub(super) async fn opt_get_by_template_id(
        &self,
        template_id: &str,
    ) -> Result<String, ServiceError> {
        // Identity of the TEMPLATE_ID is case-insensitive (BASE master05
        // §Composite Identifiers and Case).
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM template_store WHERE lower(template_id) = lower($1)",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::TemplateDoesNotExist,
                format!("OPT template_id {template_id}"),
            )
        })
    }

    /// The ids (`UUID`s) of all stored OPTs, oldest first, paged in SQL.
    async fn opt_list(&self, page: Page) -> Result<Vec<String>, ServiceError> {
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

    /// OPTs whose `template_id` matches `id_pattern` (regex; uncompilable →
    /// `invalid_id_pattern`, `400`), then paged. Returns `template_id`
    /// strings.
    async fn opt_list_matching(
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

    /// Delete an OPT by `an_opt_id` (a `UUID`), evicting the deleted
    /// template's `WebTemplate` cache entry; absent →
    /// `template_does_not_exist` (`404`); unparseable UUID → `400`.
    async fn opt_delete(&self, an_opt_id: &str) -> Result<(), ServiceError> {
        let id = parse_opt_uuid(an_opt_id)?;
        // `RETURNING template_id` gives us the wire id of the deleted row so we
        // can drop its derived-runtime (`WebTemplate`) cache entry in the same
        // operation. Absent row → `None` → 404.
        let deleted_template_id: Option<String> =
            sqlx::query_scalar("DELETE FROM template_store WHERE id = $1 RETURNING template_id")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(template_id) = deleted_template_id else {
            return Err(ServiceError::sm(
                CallStatusType::TemplateDoesNotExist,
                format!("OPT {an_opt_id}"),
            ));
        };
        // Delete is the only mutation that ends a stored template's lifetime
        // (uploads are create-only — `store_template`'s `ON CONFLICT DO NOTHING`
        // never overwrites, and `web_template_for` never caches a negative
        // result), so this is the single cache-invalidation point. Key on the
        // identity-canonical form so a case variant of the id is evicted too
        // (BASE master05 §Composite Identifiers and Case). No openEHR spec
        // governs the cache; cross-instance eviction (a delete on node A does
        // not evict node B's in-memory cache) is out of scope for this
        // single-node optimisation — our own design.
        self.web_templates
            .invalidate(&crate::templates::identity::canonical_key(&template_id))
            .await;
        Ok(())
    }

    /// Total stored OPTs.
    async fn opt_count(&self) -> Result<i64, ServiceError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM template_store")
                .fetch_one(&self.pool)
                .await?,
        )
    }
}

// ── stateless validity helpers ────────────────────────────────────────────────

/// `valid_opt` core — the OPT parses (`opt14::from_xml`) and passes the
/// templates seam's structural check.
fn valid_opt_xml(opt_xml: &str) -> bool {
    openehr_its::opt14::from_xml(opt_xml).is_ok()
        && crate::validation::validate_opt_structure(opt_xml).is_ok()
}

/// Parse an OPT id UUID string; an unparseable value is a `400`.
fn parse_opt_uuid(an_opt_id: &str) -> Result<Uuid, ServiceError> {
    Uuid::parse_str(an_opt_id)
        .map_err(|_| ServiceError::BadRequest(format!("OPT id is not a UUID: {an_opt_id}")))
}

/// Extract the `ARCHETYPE_ID` from ADL 1.4 source: the source must begin with
/// the `archetype` keyword line (optionally `archetype (adl_version=…)`) and
/// carry a well-formed `ARCHETYPE_ID` (BASE `master05` §Archetype Identifiers)
/// on the next non-blank line.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
