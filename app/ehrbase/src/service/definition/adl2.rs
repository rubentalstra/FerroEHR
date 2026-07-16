//! `I_DEFINITION_ADL2` (`i_definition_adl2.adoc`): ADL2 artefacts (archetype /
//! template / `operational_template`) keyed by `ARCHETYPE_HRID`, on the
//! `adl2_artefact` store.
//!
//! PORT NOTE (G-05-02, registration-subset validity): `valid_artefact` /
//! `upload_artefact` run the *registration* subset of the AOM2 validation
//! catalogue that is decidable on an uploaded source; the full AOM2 catalogue
//! validation is a validation-seam concern (register 09/10). The service seam
//! records the fact only.

use std::str::FromStr;

use crate::service::list::Page;
use crate::service::status::{CallStatusType, SmError};
use openehr_base::prelude::ArchetypeId;
use sqlx::Row;

use super::{compile_pattern, page_bounds, paginate};
use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// `has_artefact` — true if an ADL2 artefact with `ARCHETYPE_HRID` `an_id` is
    /// stored. HRID identity is case-insensitive (BASE master05 §Composite
    /// Identifiers and Case).
    pub(super) async fn adl2_exists(&self, an_id: &str) -> Result<bool, ServiceError> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM adl2_artefact WHERE lower(hrid) = lower($1))",
        )
        .bind(an_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// `upload_artefact` (Pre `valid_artefact`, Post `has_artefact`) — store a
    /// valid ADL2 artefact, replacing any existing one with the same
    /// `ARCHETYPE_HRID` ("If an artefact with the same physical identifier and
    /// namespace exists, replace it"). Invalid source → `invalid_artefact`
    /// (`422`). Returns the stored HRID (the wire needs it for `Location` + the
    /// identifier body).
    pub(super) async fn adl2_upload(&self, adl2: &str) -> Result<String, ServiceError> {
        let meta = crate::validation::validate_adl2_source(adl2).map_err(|v| {
            ServiceError::sm(
                CallStatusType::InvalidArtefact,
                format!("{}: {}", v.code, v.detail),
            )
        })?;
        // Validate the header HRID lexically (VARID analogue).
        if !valid_adl2_hrid(&meta.hrid) {
            return Err(ServiceError::sm(
                CallStatusType::InvalidArtefact,
                format!("'{}' is not a well-formed ARCHETYPE_HRID", meta.hrid),
            ));
        }
        // VACSD (AOM2 master08 Phase 1): when the parent named by `specialize`
        // is present in the registry, the child's specialisation depth must be
        // exactly parent depth + 1. Registration order is unconstrained, so an
        // absent parent skips the check. Parent lookup is case-insensitive
        // (BASE master05 §Composite Identifiers and Case).
        if let Some(parent_hrid) = &meta.parent_hrid
            && let Some(parent_src) = sqlx::query_scalar::<_, String>(
                "SELECT adl FROM adl2_artefact WHERE lower(hrid) = lower($1)",
            )
            .bind(parent_hrid)
            .fetch_optional(&self.pool)
            .await?
            && let Ok(parent) = crate::validation::validate_adl2_source(&parent_src)
        {
            crate::validation::check_specialisation_depth(&meta, parent.depth).map_err(|v| {
                ServiceError::sm(
                    CallStatusType::InvalidArtefact,
                    format!("{}: {}", v.code, v.detail),
                )
            })?;
        }
        let hrid = meta.hrid;
        let kind = store_kind(meta.kind);
        // Case-insensitive replace (BASE master05 §Composite Identifiers and
        // Case): remove any case-variant of this HRID, then insert verbatim.
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM adl2_artefact WHERE lower(hrid) = lower($1)")
            .bind(&hrid)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO adl2_artefact (hrid, kind, adl) VALUES ($1, $2, $3)")
            .bind(&hrid)
            .bind(kind)
            .bind(adl2)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(hrid)
    }

    /// `get_artefact` — the ADL2 source of the artefact with `ARCHETYPE_HRID`
    /// `an_id`; absent → `artefact_does_not_exist` (`404`). Returns the ADL2
    /// source (interchange form, G-05-03).
    pub(super) async fn adl2_get(&self, an_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>(
            "SELECT adl FROM adl2_artefact WHERE lower(hrid) = lower($1)",
        )
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

    /// `list_artefacts` — the `ARCHETYPE_HRID`s of all stored ADL2 artefacts.
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

    /// `list_matching_artefacts` — HRIDs matching `id_pattern` (a regex). An
    /// uncompilable pattern → `invalid_id_pattern` (`400`).
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

    /// `delete_artefact` — delete the ADL2 artefact with `ARCHETYPE_HRID`
    /// `an_id` (case-insensitive); absent → `artefact_does_not_exist` (`404`).
    pub(super) async fn adl2_delete(&self, an_id: &str) -> Result<(), ServiceError> {
        let deleted = sqlx::query("DELETE FROM adl2_artefact WHERE lower(hrid) = lower($1)")
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

    /// `artefacts_count` — total ADL2 artefacts.
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

    /// The wire list for `GET /definition/template/adl2`: the ADL2 templates and
    /// OPTs as `{template_id, created_timestamp}` metadata objects. Spec-silent
    /// wire shape (ITS-REST `TemplateList`), not an SM op.
    ///
    /// PORT NOTE: the OAS `TemplateMetadata` also carries `concept`/`archetype_id`
    /// derived from the cADL body; with no ADL2/cADL source parser yet those are
    /// omitted and `template_id` is the `ARCHETYPE_HRID`. Lists the `template`
    /// and `operational_template` kinds (the "templates" under
    /// `/definition/template/adl2`), not source archetypes.
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

    /// `valid_artefact` — registration-subset structural validity of ADL2 source
    /// plus a well-formed HRID (module PORT NOTE G-05-02). Stateless.
    #[must_use]
    pub(super) fn valid_adl2_source(adl2: &str) -> bool {
        crate::validation::validate_adl2_source(adl2).is_ok_and(|meta| valid_adl2_hrid(&meta.hrid))
    }
}

/// Map an artefact `kind` to the value the storage `kind` column accepts
/// (G-05-07). The AOM2 keyword set includes `template_overlay`, but the
/// storage `kind` domain is `{archetype, template, operational_template}` (our
/// own design — no openEHR spec governs the schema); an overlay is a
/// specialising fragment of a template, so it is stored under `template`. This
/// keeps an ADL2 upload from ever reaching a DB constraint (a malformed upload
/// is a `422` at validation, never a `500`).
fn store_kind(kind: &str) -> &str {
    match kind {
        "template_overlay" => "template",
        other => other,
    }
}

/// Structural check for an `ARCHETYPE_HRID`: an optional `namespace::` prefix
/// followed by an openEHR HRID (BASE `master05` §Archetype Identifiers).
///
/// PORT NOTE: reuses [`ArchetypeId::from_str`], whose lexical form accepts the
/// HRID shape (a superset of the ADL 1.4 `ARCHETYPE_ID`, tolerating the full
/// multi-part `.vN.N.N` version). A stricter AOM2 `ARCHETYPE_HRID` grammar
/// (`version_status` / `build_count` suffixes) awaits the ADL2 parser
/// (register 10).
fn valid_adl2_hrid(hrid: &str) -> bool {
    let core = hrid.rsplit_once("::").map_or(hrid, |(_, rest)| rest);
    ArchetypeId::from_str(core).is_ok()
}

// ── SM Definitions native API (I_DEFINITION_ADL2) ────────────────────────────

impl EhrbaseService {
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn has_artefact(&self, an_id: String) -> Result<bool, SmError> {
        Ok(self.adl2_exists(&an_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub fn valid_artefact(&self, adl2: &str) -> Result<bool, SmError> {
        Ok(Self::valid_adl2_source(adl2))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn upload_artefact(&self, adl2: String) -> Result<(), SmError> {
        // Replace-if-exists (same HRID); invalid source → 422 invalid_artefact.
        self.adl2_upload(&adl2).await?;
        Ok(())
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_artefact(&self, an_id: String) -> Result<String, SmError> {
        Ok(self.adl2_get(&an_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_artefacts(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list(page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_archetypes_adl2(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("archetype", page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_templates_adl2(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("template", page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_opts_adl2(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("operational_template", page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn list_matching_artefacts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_matching(&id_pattern, page).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn delete_artefact(&self, an_id: String) -> Result<(), SmError> {
        Ok(self.adl2_delete(&an_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn artefacts_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count().await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn archetypes_count_adl2(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("archetype").await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn templates_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("template").await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn opts_count_adl2(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("operational_template").await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_overlay_folds_into_the_template_storage_kind() {
        // G-05-07: the storage `kind` domain excludes `template_overlay`; it is
        // stored as `template` so an upload never hits the DB CHECK.
        assert_eq!(store_kind("template_overlay"), "template");
        assert_eq!(store_kind("archetype"), "archetype");
        assert_eq!(store_kind("template"), "template");
        assert_eq!(store_kind("operational_template"), "operational_template");
    }

    #[test]
    fn adl2_source_validation_drives_upload_metadata() {
        // The header keyword + HRID are extracted by the registration validator;
        // malformed keyword/HRID sources are invalid (SM master04
        // I_DEFINITION_ADL2 upload_artefact).
        assert!(!EhrbaseService::valid_adl2_source(
            "concept\nopenEHR-EHR-OBSERVATION.bp.v1.0.0"
        ));
        assert!(!EhrbaseService::valid_adl2_source("archetype\nnot-an-hrid"));
    }
}
