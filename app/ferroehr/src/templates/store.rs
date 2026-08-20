// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `template_store` access: the operational-template repository keyed by
//! its `template_id` on the ITS-REST `adl1.4` surface.
//!
//! # Spec basis
//!
//! - (`AM/docs/OPT2/master02-overview.adoc` §Purpose of the OPT,
//!   §Types of OPT): the stored artefact is the compiled OPT; "a production
//!   EHR … can safely run only using guaranteed *validated* templates" — hence
//!   the parse + structural + artefact-validity gates before an insert.
//! - (`BASE/docs/architecture_overview/master10-archetypes.adoc`
//!   §Overview): the template/archetype repository is separate from the EHR
//!   data.
//! - (`AM/docs/AOM2/master10-templates.adoc` §Template Identifiers): a
//!   `TEMPLATE_ID` identifies a template; equality follows the §Composite
//!   Identifiers and Case rule (see [`crate::templates::identity`]).
//!
//! NOTE (dual identity): the `template_store` row carries **both**
//! a surrogate `UUID` handle (the SM `I_DEFINITION_ADL14` OPT key,
//! `service/definition/adl14.rs`) and the wire `template_id` string (the
//! ITS-REST `adl1.4/{template_id}` address). Both are load-bearing; the DB
//! schema is spec-silent by construction ("no openEHR spec governs SQL").
//!
//! NOTE: the `template_store` DDL is owned by the storage layer
//! (`crate::storage`). This module only reads/writes rows.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): stored OPT/WebTemplate artefacts served verbatim \
              (families 1/8)"
)]

use serde_json::{Value, json};
use sqlx::Row;

use super::ingest;
use crate::service::FerroEhrService;
use crate::service::error::{ServiceError, Violation};

impl FerroEhrService {
    /// Store an OPT 1.4 operational template from its canonical XML, returning
    /// the stored template's metadata descriptor (the ITS-REST template list
    /// shape — the `adl1.4` upload response body).
    ///
    /// The XML is parsed to validate it is a well-formed OPT and to pull the
    /// `template_id` (the unique key), `concept`, and root archetype id, then
    /// run through the standalone-artefact validity catalogue (the validation
    /// seam) before any row is written (only validated templates are
    /// stored).
    ///
    /// Operational templates are **immutable on the `adl1.4` upload endpoint**:
    /// re-uploading an existing `template_id` — under §Composite Identifiers
    /// and Case, so a case variant counts as the same id — is a
    /// **`Conflict`** (→ ITS-REST `409`), never a silent overwrite.
    /// This matches
    /// `docs/specs/openehr/ITS-REST/specifications/responses/409_template_already_exists.yaml`
    /// ("409 Conflict is returned when a template with same `template_id`
    /// already exists") and the CNF Robot case
    /// `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict`.
    ///
    /// The write is a single statement: `INSERT … ON CONFLICT
    /// (lower(template_id)) DO NOTHING RETURNING …` on the case-insensitive
    /// functional unique index (`ux_template_store_template_id_ci`, baseline
    /// §`template_store`) makes both exact and case-variant duplicates
    /// race-free (no overwrite, no SQLSTATE parsing) and hands back the stored
    /// row for the response in the same round-trip — no pre-check `SELECT`, no
    /// post-insert re-read.
    ///
    /// # Errors
    ///
    /// - [`ServiceError::BadRequest`] (→ `400`) — the payload is not
    ///   well-formed XML (ITS-REST `responses/400.yaml`: "syntactically
    ///   invalid … content").
    /// - [`ServiceError::Unprocessable`] (→ `422`) — well-formed XML that does
    ///   not decode as an OPT 1.4 document, or a decoded OPT with an empty
    ///   `template_id` or `concept`.
    /// - The structural gate
    ///   (`crate::validation::validate_opt_structure`) and the
    ///   AOM2/08 artefact-validity catalogue
    ///   (`crate::validation::validate_opt_artefact`,
    ///   `AM/docs/AOM2/master08-validation.adoc`) propagate their own
    ///   typed rejections.
    /// - [`ServiceError::Conflict`] (→ `409`) — a template with the same
    ///   `template_id` (case-insensitively) already exists.
    /// - [`ServiceError::Database`] — the insert itself failed.
    pub(crate) async fn store_template(&self, xml: &str) -> Result<Value, ServiceError> {
        let opt = ingest::parse_opt(xml)?;
        // Structural well-formedness the tolerant codec would otherwise accept
        // (foreign / duplicated top-level elements).
        crate::validation::validate_opt_structure(xml)?;

        // The AOM2/08 standalone-artefact validity catalogue (VCOC/VACMCO,
        // VATID/VTLC, VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM → `422` carrying
        // the AOM2 rule code in `validationErrors[]`) is owned by the validation layer
        // (`crate::validation::opt`; spec `AM/docs/AOM2/master08-validation.adoc`).
        crate::validation::validate_opt_artefact(&opt)?;

        let template_id = opt.template_id.value;
        // NOTE: i_definition_adl14.adoc §upload_opt .Errors declares
        // invalid_template for a semantically invalid operational template.
        if template_id.trim().is_empty() {
            return Err(ServiceError::Unprocessable {
                status: crate::service::status::CallStatusType::InvalidTemplate,
                violation: Violation::new("is missing from the operational template")
                    .with_path("template_id"),
            });
        }
        // `concept` is a mandatory `OPERATIONAL_TEMPLATE` attribute; an empty one
        // is a malformed OPT (CNF `removed_mandatory_elements/…removed_concept_value`).
        if opt.concept.trim().is_empty() {
            return Err(ServiceError::Unprocessable {
                status: crate::service::status::CallStatusType::InvalidTemplate,
                violation: Violation::new("of the operational template is empty")
                    .with_path("concept"),
            });
        }
        let concept = Some(opt.concept);
        let root_archetype = {
            let a = opt.definition.archetype_id.value;
            (!a.trim().is_empty()).then_some(a)
        };

        // Insert-only: `DO NOTHING` arbitrated on the case-insensitive unique
        // index (`lower(template_id)`) rejects exact and case-variant duplicates
        // alike (a case variant is the *same* id, §Composite Identifiers
        // and Case) — no row returned means the template already exists → 409.
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "INSERT INTO template_store (template_id, concept, root_archetype, content) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (lower(template_id)) DO NOTHING \
             RETURNING template_id, concept, root_archetype, created_at",
        )
        .bind(&template_id)
        .bind(&concept)
        .bind(&root_archetype)
        .bind(xml)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ServiceError::conflict(format!(
                "an operational template with template_id '{template_id}' already exists"
            ))
        })?;
        // Register the wire address in `template_ref` (the vo_version.template_id
        // FK target) in the same transaction — the registry is the union of both
        // template dialects' addresses, so `DO NOTHING` absorbs an ADL2 claim of
        // the same id (`0001_baseline.sql` §template_ref).
        sqlx::query("INSERT INTO template_ref (template_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(&template_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        // No `WebTemplate`-cache invalidation is needed on the create path: this
        // insert is create-only (`ON CONFLICT DO NOTHING` never overwrites), and
        // `web_template_for` only caches a *successful* build, so no stale or
        // negative entry can pre-exist for a freshly stored template_id. The
        // cache is invalidated only where a template's lifetime ends — the delete
        // path (`service/definition/adl14.rs::opt_delete`). No openEHR spec
        // governs the cache; this is our own design.
        Self::template_json(&row)
    }

    /// The stored OPT 1.4 XML for a template (the canonical retrieval artifact
    /// of `GET /definition/template/adl1.4/{template_id}`), addressed by
    /// `template_id` (case-insensitive).
    ///
    /// # Errors
    ///
    /// - [`ServiceError::NotFound`] (→ `404`,
    ///   `responses/404_unknown_template_id.yaml`) — no template with this id
    ///   is stored.
    /// - [`ServiceError::Database`] — the lookup failed.
    pub(crate) async fn get_template_xml(&self, template_id: &str) -> Result<String, ServiceError> {
        // §Composite Identifiers and Case: compare case-insensitively.
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM template_store WHERE lower(template_id) = lower($1)",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::sm(
                crate::service::status::CallStatusType::TemplateDoesNotExist,
                format!("template {template_id}"),
            )
        })
    }

    /// List every stored template's metadata descriptor (by `template_id`) —
    /// the `GET /definition/template/adl1.4` list surface.
    ///
    /// # Errors
    ///
    /// - [`ServiceError::Database`] — the listing query failed.
    /// - [`ServiceError::Internal`]-class decode faults surface through
    ///   [`template_json`](Self::template_json)'s row-decode discipline.
    pub(crate) async fn template_summaries(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store ORDER BY template_id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::template_json).collect()
    }

    /// The openEHR template descriptor for one row (ITS-REST template list shape).
    ///
    /// `template_id`/`created_at` are `NOT NULL` (`0001_baseline.sql`
    /// §`template_store`), so a decode failure there is a genuine server fault:
    /// surface it (`?` → `500`) rather than silently blanking the field.
    /// `concept`/`root_archetype` are genuinely nullable, so a SQL
    /// `NULL` stays `None` while a *decode* error still propagates.
    ///
    /// The `version` field is the ITS-REST `TemplateMetadata.version` (optional +
    /// `deprecated`; `definition-codegen.openapi.yaml`
    /// §components.schemas.TemplateMetadata). It is **not** stored — it is a pure
    /// function of `template_id` (its `.vN` axis; `filter_version`: "taken from
    /// `template_id`"), so it is derived here rather than denormalised into a
    /// column (see [`crate::templates::identity::template_version`]) and emitted
    /// only when the id carries a version, matching the schema's optional field.
    ///
    /// # Errors
    ///
    /// [`ServiceError::Database`] — a column failed to decode (a server fault,
    /// never silently blanked).
    fn template_json(row: &sqlx::postgres::PgRow) -> Result<Value, ServiceError> {
        let created = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
            .to_jiff()
            .to_string();
        let template_id = row.try_get::<String, _>("template_id")?;
        let version = crate::templates::identity::template_version(&template_id);
        let mut descriptor = json!({
            "template_id": template_id,
            "concept": row.try_get::<Option<String>, _>("concept")?,
            "archetype_id": row.try_get::<Option<String>, _>("root_archetype")?,
            "created_timestamp": created,
        });
        // `version` is optional (deprecated) — include it only when derivable
        // from the id, never as an explicit `null`.
        if let Some(version) = version
            && let Some(obj) = descriptor.as_object_mut()
        {
            obj.insert("version".to_owned(), Value::String(version));
        }
        Ok(descriptor)
    }
}

#[cfg(test)]
mod tests {
    use crate::templates::identity;

    #[test]
    fn store_lookup_is_case_insensitive_by_law() {
        // The store SQL boundary compares `lower(template_id) = lower($1)`, i.e.
        // the §Composite Identifiers and Case canonical form. This guards
        // the invariant that the identity module and the SQL boundary agree.
        assert_eq!(
            identity::canonical_key("Vital-Signs.v1"),
            identity::canonical_key("vital-signs.V1"),
        );
    }
}
