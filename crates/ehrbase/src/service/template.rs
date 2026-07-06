//! Operational-template CRUD (ITS-REST DEFINITION `adl1.4` group), on the
//! `template_store` table. Templates are uploaded as OPT 1.4 canonical XML; the
//! XML is stored verbatim (the authoritative artifact, and what GET returns) and
//! parsed into [`openehr_its::opt14::OperationalTemplate`] to extract the
//! `template_id` / `concept` / root-archetype metadata for indexing and listing.

use std::sync::Arc;

use openehr_flat::WebTemplate;
use serde_json::{Value, json};
use sqlx::Row;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Resolve the (cached) [`WebTemplate`] for a stored operational template,
    /// building it from the stored OPT 1.4 XML on first use.
    ///
    /// A template that is not in the store is reported as **`Unprocessable`**
    /// (→ ITS-REST `422`), not `NotFound`: on a composition commit an unknown
    /// referenced template is a *semantic* error, per
    /// `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`
    /// ("the underlying template is not known"), and the CNF Robot case
    /// `I_EHR_COMPOSITION.create_composition-event_bad_opt` asserts `422`.
    pub(super) async fn web_template_for(
        &self,
        template_id: &str,
    ) -> Result<Arc<WebTemplate>, ServiceError> {
        let xml = match self.get_template_xml(template_id).await {
            Ok(xml) => xml,
            Err(ServiceError::NotFound(_)) => {
                return Err(ServiceError::Unprocessable(format!(
                    "operational template not known: {template_id}"
                )));
            }
            Err(e) => return Err(e),
        };
        self.web_templates
            .get_or_build(template_id, || {
                let opt = openehr_its::opt14::from_xml(&xml)
                    .map_err(|e| openehr_flat::FlatError::OptParse(e.to_string()))?;
                openehr_flat::build_web_template(&opt)
            })
            .await
            .map_err(|e| {
                ServiceError::Unprocessable(format!(
                    "operational template {template_id} could not be built into a WebTemplate: {e}"
                ))
            })
    }

    /// Store an OPT 1.4 operational template from its canonical XML, returning
    /// the stored template's metadata descriptor.
    ///
    /// The XML is parsed to validate it is a well-formed OPT and to pull the
    /// `template_id` (the unique key), `concept`, and root archetype id.
    ///
    /// Operational templates are **immutable on the `adl1.4` upload endpoint**:
    /// re-uploading an existing `template_id` is a **`Conflict`** (→ ITS-REST
    /// `409`), never a silent overwrite. This matches
    /// `docs/specs/openehr/ITS-REST/specifications/responses/409_template_already_exists.yaml`
    /// ("409 Conflict is returned when a template with same `template_id` …
    /// already exists") and the CNF Robot case
    /// `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict` ("upload same OPT
    /// again" → status 409). A legitimate replacement path (admin) is a later,
    /// separate concern; this endpoint must not mutate an existing template.
    pub(super) async fn store_template(&self, xml: &str) -> Result<Value, ServiceError> {
        let opt = openehr_its::opt14::from_xml(xml)
            .map_err(|e| ServiceError::Unprocessable(format!("invalid OPT 1.4 XML: {e}")))?;

        let template_id = opt.template_id.value;
        if template_id.trim().is_empty() {
            return Err(ServiceError::Unprocessable(
                "operational template has no template_id".to_owned(),
            ));
        }
        let concept = (!opt.concept.trim().is_empty()).then_some(opt.concept);
        let root_archetype = {
            let a = opt.definition.archetype_id.value;
            (!a.trim().is_empty()).then_some(a)
        };

        // Insert-only: `DO NOTHING` on the `template_id` unique key makes the
        // duplicate case race-free (no overwrite, no SQLSTATE parsing) — an
        // affected-row count of 0 means the template already exists → 409.
        let inserted = sqlx::query(
            "INSERT INTO template_store (template_id, concept, root_archetype, content) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (template_id) DO NOTHING",
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

    /// The metadata descriptor for one stored template.
    pub(super) async fn get_template_meta(&self, template_id: &str) -> Result<Value, ServiceError> {
        let row = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store WHERE template_id = $1",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))?;
        Ok(Self::template_json(&row))
    }

    /// The stored OPT 1.4 XML for a template (the canonical retrieval artifact).
    pub(super) async fn get_template_xml(&self, template_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>("SELECT content FROM template_store WHERE template_id = $1")
            .bind(template_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))
    }

    /// List every stored template's metadata descriptor (by `template_id`).
    pub(super) async fn list_templates(&self) -> Result<Vec<Value>, ServiceError> {
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
