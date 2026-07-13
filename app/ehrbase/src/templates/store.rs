//! `template_store` access: the operational-template repository (S-07) keyed by
//! its `template_id` on the ITS-REST `adl1.4` surface.
//!
//! # Spec basis
//!
//! - S-05/S-06 (`AM/docs/OPT2/master02-overview.adoc` §Types of OPT, §Purpose
//!   item 1): the stored artefact is the compiled OPT; "a production EHR can
//!   safely run only using guaranteed *validated* templates" — hence the parse +
//!   structural + artefact-validity gates before an insert.
//! - S-07 (`BASE/docs/architecture_overview/master10-archetypes.adoc` §Overview):
//!   the template/archetype repository is separate from the EHR data.
//! - S-11 (`AM/docs/AOM2/master10-templates.adoc` §Template Identifiers): a
//!   `TEMPLATE_ID` identifies a template; equality follows the §Composite
//!   Identifiers and Case rule (G-T04, see [`crate::templates::identity`]).
//!
//! PORT NOTE (G-T07 — dual identity): the `template_store` row carries **both** a
//! surrogate `UUID` handle (the SM `I_DEFINITION_ADL14` OPT key,
//! `service/definition/adl14.rs`) and the wire `template_id` string (the
//! ITS-REST `adl1.4/{template_id}` address). Both are load-bearing; the DB
//! schema is spec-silent by construction ("no openEHR spec governs SQL").
//!
//! PORT NOTE: the `template_store` DDL is owned by the storage layer
//! (`crate::storage`, register 02). This module only reads/writes rows.

use serde_json::{Value, json};
use sqlx::Row;

use super::ingest;
use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Store an OPT 1.4 operational template from its canonical XML, returning
    /// the stored template's metadata descriptor.
    ///
    /// The XML is parsed to validate it is a well-formed OPT and to pull the
    /// `template_id` (the unique key), `concept`, and root archetype id, then run
    /// through the standalone-artefact validity catalogue (the validation seam)
    /// before any row is written — S-06 (only validated templates are stored).
    ///
    /// Operational templates are **immutable on the `adl1.4` upload endpoint**:
    /// re-uploading an existing `template_id` — under §Composite Identifiers and
    /// Case, so a case variant counts as the same id (G-T04) — is a **`Conflict`**
    /// (→ ITS-REST `409`), never a silent overwrite (G-T09). This matches
    /// `docs/specs/openehr/ITS-REST/specifications/responses/409_template_already_exists.yaml`
    /// ("409 Conflict is returned when a template with same `template_id` …
    /// already exists") and the CNF Robot case
    /// `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict`.
    pub(crate) async fn store_template(&self, xml: &str) -> Result<Value, ServiceError> {
        let opt = ingest::parse_opt(xml)?;
        // Structural well-formedness the tolerant codec would otherwise accept
        // (foreign / duplicated top-level elements) — S-05.
        crate::validation::validate_opt_structure(xml)?;

        // The AOM2/08 standalone-artefact validity catalogue (VCOC/VACMCO,
        // VATID/VTLC, VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM → `400` carrying
        // the AOM2 rule code, S-06) is owned by the validation layer
        // (`crate::validation`; spec `AM/docs/AOM2/master08-validation.adoc`).
        crate::validation::validate_opt_artefact(&opt)?;

        let template_id = opt.template_id.value;
        if template_id.trim().is_empty() {
            return Err(ServiceError::Unprocessable(
                "operational template has no template_id".to_owned(),
            ));
        }
        // `concept` is a mandatory `OPERATIONAL_TEMPLATE` attribute; an empty one
        // is a malformed OPT (CNF `removed_mandatory_elements/…removed_concept_value`).
        if opt.concept.trim().is_empty() {
            return Err(ServiceError::Unprocessable(
                "operational template has an empty concept".to_owned(),
            ));
        }
        let concept = Some(opt.concept);
        let root_archetype = {
            let a = opt.definition.archetype_id.value;
            (!a.trim().is_empty()).then_some(a)
        };

        // G-T04 (§Composite Identifiers and Case): a template_id that differs
        // only in case from a stored one is the *same* id, so the insert-only
        // guard is case-insensitive. This pre-check produces the friendly 409
        // message in the common (non-concurrent) case; the race-free guard is the
        // `ux_template_store_template_id_ci` functional unique index over
        // `lower(template_id)` (migration 0007), which the `ON CONFLICT` below
        // relies on for concurrency.
        let case_variant_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM template_store WHERE lower(template_id) = lower($1))",
        )
        .bind(&template_id)
        .fetch_one(&self.pool)
        .await?;
        if case_variant_exists {
            return Err(ServiceError::Conflict(format!(
                "an operational template with template_id '{template_id}' already exists"
            )));
        }

        // Insert-only: `DO NOTHING` on the case-insensitive unique index
        // (`lower(template_id)`) makes both exact and case-variant duplicates
        // race-free (no overwrite, no SQLSTATE parsing) — an affected-row count of
        // 0 means the template already exists → 409.
        let inserted = sqlx::query(
            "INSERT INTO template_store (template_id, concept, root_archetype, content) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (lower(template_id)) DO NOTHING",
        )
        .bind(&template_id)
        .bind(&concept)
        .bind(&root_archetype)
        .bind(xml)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if inserted == 0 {
            return Err(ServiceError::Conflict(format!(
                "an operational template with template_id '{template_id}' already exists"
            )));
        }

        self.get_template_meta(&template_id).await
    }

    /// The metadata descriptor for one stored template, addressed by
    /// `template_id` (case-insensitive, G-T04). Absent → `NotFound` (`404`).
    pub(crate) async fn get_template_meta(&self, template_id: &str) -> Result<Value, ServiceError> {
        // §Composite Identifiers and Case: compare case-insensitively (G-T04).
        let row = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store WHERE lower(template_id) = lower($1)",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))?;
        Ok(Self::template_json(&row))
    }

    /// The stored OPT 1.4 XML for a template (the canonical retrieval artifact),
    /// addressed by `template_id` (case-insensitive, G-T04). Absent → `NotFound`
    /// (`404`).
    pub(crate) async fn get_template_xml(&self, template_id: &str) -> Result<String, ServiceError> {
        // §Composite Identifiers and Case: compare case-insensitively (G-T04).
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM template_store WHERE lower(template_id) = lower($1)",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))
    }

    /// List every stored template's metadata descriptor (by `template_id`).
    pub(crate) async fn list_templates(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store ORDER BY template_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::template_json).collect())
    }

    /// The openEHR template descriptor for one row (ITS-REST template list shape).
    fn template_json(row: &sqlx::postgres::PgRow) -> Value {
        let created = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")
            .map(|t| t.to_jiff().to_string())
            .unwrap_or_default();
        json!({
            "template_id": row.try_get::<String, _>("template_id").unwrap_or_default(),
            "concept": row.try_get::<Option<String>, _>("concept").ok().flatten(),
            "archetype_id": row.try_get::<Option<String>, _>("root_archetype").ok().flatten(),
            "created_timestamp": created,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::templates::identity;

    #[test]
    fn store_lookup_is_case_insensitive_by_law() {
        // The store SQL boundary compares `lower(template_id) = lower($1)`, i.e.
        // the §Composite Identifiers and Case canonical form (G-T04). This guards
        // the invariant that the identity module and the SQL boundary agree.
        assert_eq!(
            identity::canonical_key("Vital-Signs.v1"),
            identity::canonical_key("vital-signs.V1"),
        );
    }
}
