//! Operational-template CRUD (ITS-REST DEFINITION `adl1.4` group), on the
//! `template_store` table. Templates are uploaded as OPT 1.4 canonical XML; the
//! XML is stored verbatim (the authoritative artifact, and what GET returns) and
//! parsed into [`openehr_its::opt14::OperationalTemplate`] to extract the
//! `template_id` / `concept` / root-archetype metadata for indexing and listing.

use serde_json::{Value, json};
use sqlx::Row;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Store (or replace) an OPT 1.4 operational template from its canonical XML,
    /// returning the stored template's metadata descriptor.
    ///
    /// The XML is parsed to validate it is a well-formed OPT and to pull the
    /// `template_id` (the unique key), `concept`, and root archetype id.
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

        sqlx::query(
            "INSERT INTO template_store (template_id, concept, root_archetype, content) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (template_id) DO UPDATE SET \
               concept = EXCLUDED.concept, \
               root_archetype = EXCLUDED.root_archetype, \
               content = EXCLUDED.content, \
               created_at = now()",
        )
        .bind(&template_id)
        .bind(&concept)
        .bind(&root_archetype)
        .bind(xml)
        .execute(&self.pool)
        .await?;

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
