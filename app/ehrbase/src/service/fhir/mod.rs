//! The FHIR-connector mapping store + inbound ingest (ADR-016 / E3).
//!
//! Two concerns, both on [`EhrbaseService`]:
//!
//! * the **mapping store** — CRUD over `fhir_mapping` (the deployable
//!   "mapping-as-data" artefacts, ADR-016 §Decision 2), mirroring the
//!   event-subscription store;
//! * the **inbound ingest** — [`FhirConnectorAdapter::fhir_ingest`]: resolve a
//!   mapping by resource type + profile, build a COMPOSITION from it (the pure
//!   transform in [`mapping`]), stamp `FEEDER_AUDIT` provenance, and commit it
//!   through the NORMAL validated create path (ADR-016 §Decision 3).
//!
//! PORT NOTE (crate layout): this module lives inside `service` (not a
//! top-level `ehrbase::fhir`) because the ingest reuses the `pub(super)`
//! validated-commit seam ([`EhrbaseService::create_composition`]) and the
//! moka-cached [`EhrbaseService::web_template_for`] — both service-internal.
//! The protocol adapter (the `/fhir/r4/*` + `/admin/fhir_mapping` routes,
//! config-gated) lives in `ehrbase-rest` and dispatches to
//! [`FhirConnectorAdapter`]; FHIR↔openEHR mapping is spec-silent (ADR-016).

mod mapping;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use ehrbase_sm::error::CallStatusType;
use ehrbase_sm::types::{ServiceResponse, SubjectRef};
use ehrbase_sm::{EhrService, FhirConnectorAdapter, SmError};

use self::mapping::FhirMappingDefinition;
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Map a `fhir_mapping` row to its JSON record.
    fn mapping_row(row: &sqlx::postgres::PgRow) -> Result<Value, ServiceError> {
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let resource_type: String = row.try_get("resource_type")?;
        let profile_url: Option<String> = row.try_get("profile_url")?;
        let template_id: String = row.try_get("template_id")?;
        let definition: Value = row.try_get("definition")?;
        let enabled: bool = row.try_get("enabled")?;
        let created_at = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
            .to_jiff();
        Ok(json!({
            "id": id.to_string(),
            "name": name,
            "resource_type": resource_type,
            "profile_url": profile_url,
            "template_id": template_id,
            "definition": definition,
            "enabled": enabled,
            "created_at": created_at.to_string(),
        }))
    }

    /// List every stored mapping (newest first).
    async fn list_mappings(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, name, resource_type, profile_url, template_id, definition, enabled, \
             created_at FROM fhir_mapping ORDER BY created_at DESC, id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::mapping_row).collect()
    }

    /// Fetch one mapping by id, or `NotFound`.
    async fn get_mapping(&self, id: Uuid) -> Result<Value, ServiceError> {
        let row = sqlx::query(
            "SELECT id, name, resource_type, profile_url, template_id, definition, enabled, \
             created_at FROM fhir_mapping WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("FHIR mapping {id}")))?;
        Self::mapping_row(&row)
    }

    /// Create a mapping from a JSON body: `{name, enabled?, definition}` where
    /// `definition` is the [`FhirMappingDefinition`] (validated on upload, its
    /// `resource_type`/`profile_url`/`template_id` projected into columns). A
    /// duplicate name is a `Conflict`; an unknown `template_id` (FK) is a
    /// `BadRequest`.
    async fn create_mapping(&self, body: &Value) -> Result<Value, ServiceError> {
        let name = validated_name(body)?;
        let (definition, def) = validated_definition(body)?;
        let enabled = body.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let row = sqlx::query(
            "INSERT INTO fhir_mapping \
             (name, resource_type, profile_url, template_id, definition, enabled) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, name, resource_type, profile_url, template_id, definition, enabled, \
             created_at",
        )
        .bind(&name)
        .bind(&def.resource_type)
        .bind(def.profile_url.as_deref())
        .bind(&def.template_id)
        .bind(&definition)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(map_insert_error)?;
        Self::mapping_row(&row)
    }

    /// Replace a mapping's fields (name is immutable — it is the stable
    /// deployable identity). `NotFound` if the id is unknown.
    async fn update_mapping(&self, id: Uuid, body: &Value) -> Result<Value, ServiceError> {
        let (definition, def) = validated_definition(body)?;
        let enabled = body.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let row = sqlx::query(
            "UPDATE fhir_mapping \
             SET resource_type = $2, profile_url = $3, template_id = $4, definition = $5, \
             enabled = $6 WHERE id = $1 \
             RETURNING id, name, resource_type, profile_url, template_id, definition, enabled, \
             created_at",
        )
        .bind(id)
        .bind(&def.resource_type)
        .bind(def.profile_url.as_deref())
        .bind(&def.template_id)
        .bind(&definition)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_insert_error)?
        .ok_or_else(|| ServiceError::NotFound(format!("FHIR mapping {id}")))?;
        Self::mapping_row(&row)
    }

    /// Delete a mapping by id. `NotFound` if the id is unknown.
    async fn delete_mapping(&self, id: Uuid) -> Result<(), ServiceError> {
        let deleted = sqlx::query("DELETE FROM fhir_mapping WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::NotFound(format!("FHIR mapping {id}")));
        }
        Ok(())
    }

    /// Resolve the enabled mapping definition for a resource type + optional
    /// profile: prefer an exact `profile_url` match, else the NULL-profile
    /// default. `None` when no enabled mapping matches.
    async fn resolve_mapping(
        &self,
        resource_type: &str,
        profile: Option<&str>,
    ) -> Result<Option<Value>, ServiceError> {
        // `profile_url = $2` is never true when $2 is NULL (SQL NULL semantics),
        // so a resource with no profile matches only the NULL-profile default;
        // ORDER BY (profile_url IS NULL) ASC puts an exact match ahead of it.
        let row = sqlx::query(
            "SELECT definition FROM fhir_mapping \
             WHERE resource_type = $1 AND enabled AND (profile_url = $2 OR profile_url IS NULL) \
             ORDER BY (profile_url IS NULL) LIMIT 1",
        )
        .bind(resource_type)
        .bind(profile)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.try_get::<Value, _>("definition").map_err(Into::into))
            .transpose()
    }

    /// Resolve-or-create the EHR whose `EHR_STATUS.subject` matches `subject`.
    async fn ensure_ehr_for_subject(&self, subject: SubjectRef) -> Result<Uuid, SmError> {
        let existing = EhrService::get_ehrs_for_subject(self, subject.clone()).await?;
        if let Some(summary) = existing.first() {
            return Uuid::parse_str(&summary.ehr_id).map_err(|e| {
                SmError::new(
                    CallStatusType::Exception,
                    format!("stored ehr id invalid: {e}"),
                )
            });
        }
        EhrService::create_ehr_for_subject(self, subject, None).await
    }
}

/// Read + validate `name`: non-empty, `[A-Za-z0-9_.-]` (a clean, addressable
/// deployable identity).
fn validated_name(body: &Value) -> Result<String, ServiceError> {
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ServiceError::BadRequest("FHIR mapping requires a non-empty 'name'".into())
        })?;
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(ServiceError::BadRequest(
            "FHIR mapping 'name' must match [A-Za-z0-9_.-]".into(),
        ));
    }
    Ok(name.to_owned())
}

/// Validate the `definition` field: it must be present and deserialise into a
/// [`FhirMappingDefinition`] (ADR-016 §Decision 2 — validated on upload).
/// Returns the raw JSON (stored verbatim) + the parsed form (for the column
/// projection).
fn validated_definition(body: &Value) -> Result<(Value, FhirMappingDefinition), ServiceError> {
    let raw = body
        .get("definition")
        .cloned()
        .ok_or_else(|| ServiceError::BadRequest("FHIR mapping requires a 'definition'".into()))?;
    let def: FhirMappingDefinition = serde_json::from_value(raw.clone())
        .map_err(|e| ServiceError::BadRequest(format!("invalid FHIR mapping definition: {e}")))?;
    Ok((raw, def))
}

/// Map an INSERT/UPDATE failure: a unique-name violation is a `Conflict` (409);
/// a foreign-key violation (unknown `template_id`) is a `BadRequest` (400);
/// anything else is the underlying DB error.
fn map_insert_error(e: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return ServiceError::Conflict("a FHIR mapping with that name exists".into());
        }
        if db.is_foreign_key_violation() {
            return ServiceError::BadRequest(
                "FHIR mapping references an unknown template_id (ingest the OPT first)".into(),
            );
        }
    }
    ServiceError::Database(e)
}

/// The FHIR import timestamp for `FEEDER_AUDIT` (ISO 8601, UTC).
fn now_iso() -> String {
    jiff::Timestamp::now().to_string()
}

#[async_trait]
impl FhirConnectorAdapter for EhrbaseService {
    async fn fhir_mapping_list(&self) -> Result<Vec<Value>, SmError> {
        Ok(self.list_mappings().await?)
    }

    async fn fhir_mapping_create(&self, a_mapping: Value) -> Result<Value, SmError> {
        Ok(self.create_mapping(&a_mapping).await?)
    }

    async fn fhir_mapping_get(&self, a_mapping_id: Uuid) -> Result<Value, SmError> {
        Ok(self.get_mapping(a_mapping_id).await?)
    }

    async fn fhir_mapping_update(
        &self,
        a_mapping_id: Uuid,
        a_mapping: Value,
    ) -> Result<Value, SmError> {
        Ok(self.update_mapping(a_mapping_id, &a_mapping).await?)
    }

    async fn fhir_mapping_delete(&self, a_mapping_id: Uuid) -> Result<(), SmError> {
        Ok(self.delete_mapping(a_mapping_id).await?)
    }

    async fn fhir_ingest(
        &self,
        resource_type: String,
        profile: Option<String>,
        a_resource: Value,
    ) -> Result<ServiceResponse, SmError> {
        // 1. Resolve the mapping (no enabled match → 404).
        let def_value = self
            .resolve_mapping(&resource_type, profile.as_deref())
            .await?
            .ok_or_else(|| {
                SmError::new(
                    CallStatusType::VersionedObjectDoesNotExist,
                    format!("no enabled FHIR mapping for resource type '{resource_type}'"),
                )
            })?;
        let def: FhirMappingDefinition = serde_json::from_value(def_value).map_err(|e| {
            SmError::new(
                CallStatusType::Exception,
                format!("stored FHIR mapping definition is invalid: {e}"),
            )
        })?;

        // 2. Resolve-or-create the target EHR from the resource's subject.
        let subject = mapping::extract_subject(&a_resource, &def)
            .map_err(|e| SmError::precondition(e.to_string()))?;
        let ehr_id = self.ensure_ehr_for_subject(subject).await?;

        // 3. Build the FLAT map + the canonical COMPOSITION.
        let wt = self.web_template_for(&def.template_id).await?;
        let flat = mapping::build_flat(&a_resource, &def)
            .map_err(|e| SmError::precondition(e.to_string()))?;
        let mut composition = openehr_flat::from_flat(&flat, &wt).map_err(|e| {
            SmError::new(
                CallStatusType::ContentInvalid,
                format!("FHIR resource did not map to a valid COMPOSITION: {e}"),
            )
        })?;

        // 4. Stamp FEEDER_AUDIT provenance (ADR-016 §Decision 3).
        let feeder = mapping::feeder_audit(
            &resource_type,
            &mapping::resource_id(&a_resource, &resource_type),
            mapping::resource_version(&a_resource).as_deref(),
            &now_iso(),
        );
        mapping::inject_feeder_audit(&mut composition, feeder);

        // 5. Commit through the NORMAL validated path — a resource that maps to
        //    an invalid COMPOSITION is rejected here (content_invalid → 422),
        //    never partially stored (ADR-016 §Decision 6).
        Ok(self.create_composition(ehr_id, composition).await?)
    }
}
