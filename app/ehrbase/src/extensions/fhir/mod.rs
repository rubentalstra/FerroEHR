//! The FHIR connector: mapping store + inbound ingest + read façade + outbound
//! reverse-map (G-12-03, G-12-04).
//!
//! **No openEHR spec governs this — our own design/extension.** master14's
//! integration model is archetype-to-archetype data conversion (`GENERIC_ENTRY` +
//! `FEEDER_AUDIT`), not FHIR resources; this connector maps directly to
//! *designed* templates (mapping-as-data), a different, spec-silent mechanism.
//! Quarantined under `crate::extensions` (`docs/design/platform/12-extensions.md`).
//! Gate: the `/fhir/r4/*` + `/admin/fhir_mapping` routes are config-gated in
//! `ehrbase-rest`; the outbound emitter behind [`FhirOutboundConfig`].
//!
//! Concerns, all on [`EhrbaseService`]:
//! * the **mapping store** — CRUD over `fhir_mapping` (the deployable
//!   "mapping-as-data" artefacts), mirroring the event-subscription store;
//! * the **inbound ingest** — [`FhirConnectorAdapter::fhir_ingest`]: resolve a
//!   mapping by resource type + profile, build a COMPOSITION from it (the pure
//!   transform in [`mapping`]), stamp `FEEDER_AUDIT` provenance ([`feeder_audit`]),
//!   and commit it through the NORMAL validated create path;
//! * the **read façade** ([`FhirConnectorAdapter::fhir_search`]) and the
//!   **outbound reverse-map** ([`EhrbaseService::fhir_outbound_messages`]) — the
//!   inverse transform ([`reverse`]).
//
// Cross-area seams (all landed): the `pub(crate)` `EhrbaseService.pool` field,
// `service::query::execute_aql` and `service::ehr::create_composition`
// (`pub(crate)`), and the storage `version_repo` reads.

mod config;
mod feeder_audit;
mod mapping;
mod outbound;
mod reverse;

pub use config::{FhirConfig, FhirOutboundConfig};
pub use outbound::{FhirOutboundHandle, start, start_with_publisher};

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::status::CallStatusType;
use crate::service::query::request::AqlQueryRequest;
use crate::service::response::ServiceResponse;
use crate::service::ehr_index::types::SubjectRef;
use crate::service::status::SmError;

use crate::service::{EhrbaseService, ServiceError};
use crate::storage::version_repo;

use self::mapping::FhirMappingDefinition;

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
        let existing = self.get_ehrs_for_subject(subject.clone()).await?;
        if let Some(summary) = existing.first() {
            return Uuid::parse_str(&summary.ehr_id).map_err(|e| {
                SmError::new(
                    CallStatusType::Exception,
                    format!("stored ehr id invalid: {e}"),
                )
            });
        }
        self.create_ehr_for_subject(subject, None).await
    }

    /// Every enabled mapping definition for a resource type, across all
    /// profiles (the read façade queries each). Ordered default-last so a
    /// profiled mapping's rows precede the type default's.
    async fn enabled_definitions_for_type(
        &self,
        resource_type: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT definition FROM fhir_mapping WHERE resource_type = $1 AND enabled \
             ORDER BY (profile_url IS NULL), profile_url, id",
        )
        .bind(resource_type)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| r.try_get::<Value, _>("definition").map_err(Into::into))
            .collect()
    }

    /// Resolve the façade `patient` parameter to a query scope + the subject id
    /// used to reconstruct each resource's `subject.reference`. A UUID is an EHR
    /// id (its stored subject is looked up); anything else is a subject external
    /// id (matched by the engine's subject scope).
    async fn resolve_patient_scope(&self, patient: &str) -> Result<PatientScope, ServiceError> {
        if let Ok(ehr_id) = Uuid::parse_str(patient) {
            let subject_id =
                sqlx::query_scalar::<_, Option<String>>("SELECT subject_id FROM ehr WHERE id = $1")
                    .bind(ehr_id)
                    .fetch_optional(&self.pool)
                    .await?
                    .flatten();
            Ok(PatientScope {
                ehr_id: Some(ehr_id),
                subject_scope: None,
                subject_id,
            })
        } else {
            Ok(PatientScope {
                ehr_id: None,
                subject_scope: Some(patient.to_owned()),
                subject_id: Some(patient.to_owned()),
            })
        }
    }

    /// Assemble the FHIR `searchset` Bundle for the read façade: for each enabled
    /// mapping of `resource_type`, run its template-bound COMPOSITION query
    /// scoped to `patient`, reverse-map each hit, and collect the entries. A type
    /// with no enabled mapping yields an empty Bundle (not an error). `count`
    /// caps rows per mapping (`_count`).
    async fn fhir_search_bundle(
        &self,
        resource_type: &str,
        patient: &str,
        count: Option<i64>,
    ) -> Result<Value, SmError> {
        let scope = self.resolve_patient_scope(patient).await?;
        let mut entries: Vec<Value> = Vec::new();
        for raw in self.enabled_definitions_for_type(resource_type).await? {
            let def: FhirMappingDefinition = serde_json::from_value(raw).map_err(|e| {
                SmError::new(
                    CallStatusType::Exception,
                    format!("stored FHIR mapping definition is invalid: {e}"),
                )
            })?;
            let wt = self.web_template_for(&def.template_id).await?;
            let request = AqlQueryRequest {
                ehr_ids: scope.ehr_id.map(|u| u.to_string()).into_iter().collect(),
                subject_scope: scope.subject_scope.clone(),
                fetch: count,
                parameters: std::iter::once((
                    "templateId".to_owned(),
                    Value::String(def.template_id.clone()),
                ))
                .collect(),
                ..AqlQueryRequest::default()
            };
            let outcome = self.execute_aql(FHIR_SEARCH_AQL, None, &request).await?;
            let rows = outcome
                .result_set
                .get("rows")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for row in &rows {
                // Row shape: [ <uid string> ] (SELECT c/uid/value). The synthesized
                // uid is `<vo_id>::<system>::<ver>`; the COMPOSITION body carries no
                // uid on the AQL read path, so the body is loaded through the
                // versioned read seam by uid (see the FHIR_SEARCH_AQL PORT NOTE).
                let Some(uid) = row.get(0).and_then(Value::as_str) else {
                    continue;
                };
                let mut parts = uid.split("::");
                let (Some(vo_id), _system, Some(sys_version)) = (
                    parts.next().and_then(|s| Uuid::parse_str(s).ok()),
                    parts.next(),
                    parts.next().and_then(|v| v.parse::<i32>().ok()),
                ) else {
                    continue;
                };
                let Some(read) =
                    version_repo::read_version_by_ordinal(&self.pool, vo_id, sys_version).await?
                else {
                    continue;
                };
                // A logically deleted version reassembles to Value::Null (no node
                // rows) — nothing to map.
                if read.canonical.is_null() || !read.canonical.is_object() {
                    continue;
                }
                let mut fhir = reverse::to_fhir(
                    resource_type,
                    &read.canonical,
                    &wt,
                    &def,
                    scope.subject_id.as_deref(),
                )
                .map_err(|e| SmError::new(CallStatusType::Exception, e.to_string()))?;
                // The versioned-object id (a UUID) is the FHIR logical id + the
                // `urn:uuid:` fullUrl.
                if let Value::Object(m) = &mut fhir {
                    m.insert("id".to_owned(), Value::String(vo_id.to_string()));
                }
                entries.push(json!({
                    "fullUrl": format!("urn:uuid:{vo_id}"),
                    "resource": fhir,
                }));
            }
        }
        // PORT NOTE: `total` is the number of entries in
        // this Bundle, not a separate full-match count — the façade is a
        // stateless connector, not a FHIR Search engine, so with
        // `_count` it reports the returned page size. No `Bundle.link`
        // paging is emitted (explicit params only, by design).
        Ok(json!({
            "resourceType": "Bundle",
            "type": "searchset",
            "total": entries.len(),
            "entry": entries,
        }))
    }
}

impl EhrbaseService {
    /// Reverse-map a committed COMPOSITION version for the **outbound emitter**:
    /// load the version at `(vo_id, sys_version)`, read its bound template from
    /// the canonical `archetype_details/template_id`, and for every enabled
    /// `fhir_mapping` on that template reverse-map it, returning
    /// `(resource_type, template_id, resource)` per mapping.
    ///
    /// Returns an empty vec (nothing to emit) when the version is absent, a
    /// logical delete (a deleted COMPOSITION has no content to map — a FHIR
    /// delete notification is out of the starter scope), carries no template, or
    /// its template has no enabled mapping. Reuses the versioned read seam
    /// ([`version_repo::read_version_by_ordinal`]) and the reverse transform.
    ///
    /// PORT NOTE: the template is read from the COMPOSITION itself (as
    /// the read façade's AQL also does), NOT from `vo_version.template_id` — that
    /// column is currently left NULL on the commit path, so relying on it would
    /// emit nothing. Deriving it from the canonical body avoids touching the
    /// versioning path.
    pub(crate) async fn fhir_outbound_messages(
        &self,
        ehr_id: Option<Uuid>,
        vo_id: Uuid,
        sys_version: i32,
    ) -> Result<Vec<(String, String, Value)>, ServiceError> {
        // Load the exact committed version; skip absent / logically-deleted ones.
        let Some(read) =
            version_repo::read_version_by_ordinal(&self.pool, vo_id, sys_version).await?
        else {
            return Ok(Vec::new());
        };
        // A logically deleted version reassembles to Value::Null (no node rows).
        if read.canonical.is_null() || !read.canonical.is_object() {
            return Ok(Vec::new());
        }
        // The template the COMPOSITION was built against (its own self-description).
        let Some(template_id) = read
            .canonical
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
        else {
            return Ok(Vec::new());
        };
        let rows = sqlx::query(
            "SELECT resource_type, definition FROM fhir_mapping \
             WHERE template_id = $1 AND enabled ORDER BY id",
        )
        .bind(&template_id)
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        // The owning EHR's subject id reconstructs each resource's reference.
        let subject_id = match ehr_id {
            Some(id) => {
                sqlx::query_scalar::<_, Option<String>>("SELECT subject_id FROM ehr WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?
                    .flatten()
            }
            None => None,
        };
        let wt = self.web_template_for(&template_id).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let resource_type: String = row.try_get("resource_type")?;
            let definition: Value = row.try_get("definition")?;
            let def: FhirMappingDefinition = serde_json::from_value(definition).map_err(|e| {
                ServiceError::Unprocessable(format!(
                    "stored FHIR mapping definition is invalid: {e}"
                ))
            })?;
            let resource = reverse::to_fhir(
                &resource_type,
                &read.canonical,
                &wt,
                &def,
                subject_id.as_deref(),
            )
            .map_err(|e| ServiceError::Unprocessable(e.to_string()))?;
            out.push((resource_type, template_id.clone(), resource));
        }
        Ok(out)
    }
}

/// The read-façade query scope resolved from the `patient` parameter.
struct PatientScope {
    /// The EHR id to scope to when `patient` is a UUID.
    ehr_id: Option<Uuid>,
    /// The subject external id to scope to when `patient` is not a UUID.
    subject_scope: Option<String>,
    /// The subject external id to reconstruct each resource's
    /// `subject.reference` from (the looked-up EHR subject, or the param).
    subject_id: Option<String>,
}

/// The read-façade AQL: the version uid of every COMPOSITION of the mapped
/// template in scope. The template id binds as a parameter (no string
/// interpolation → no AQL injection).
///
/// PORT NOTE: the query selects the synthesized VERSION
/// uid `v/uid/value` (`<vo_id>::<system>::<ver>`) via a `CONTAINS VERSION v
/// CONTAINS COMPOSITION c` chain — a COMPOSITION variable's own `c/uid/value` is
/// a (null) RM leaf on the AQL read path (the reassembled body carries no uid),
/// whereas the VERSION variable's uid is the engine-synthesized object-version
/// id. The COMPOSITION body is then loaded through the versioned read seam
/// ([`version_repo::read_version_by_ordinal`]) by that uid, keeping the façade on
/// the query seam and reusing the same read seam the outbound emitter uses.
const FHIR_SEARCH_AQL: &str = "SELECT v/uid/value FROM EHR e \
     CONTAINS VERSION v CONTAINS COMPOSITION c \
     WHERE c/archetype_details/template_id/value = $templateId";

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
/// [`FhirMappingDefinition`].
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

impl EhrbaseService {
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_mapping_list(&self) -> Result<Vec<Value>, SmError> {
        Ok(self.list_mappings().await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_mapping_create(&self, a_mapping: Value) -> Result<Value, SmError> {
        Ok(self.create_mapping(&a_mapping).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_mapping_get(&self, a_mapping_id: Uuid) -> Result<Value, SmError> {
        Ok(self.get_mapping(a_mapping_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_mapping_update(
        &self,
        a_mapping_id: Uuid,
        a_mapping: Value,
    ) -> Result<Value, SmError> {
        Ok(self.update_mapping(a_mapping_id, &a_mapping).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_mapping_delete(&self, a_mapping_id: Uuid) -> Result<(), SmError> {
        Ok(self.delete_mapping(a_mapping_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_ingest(
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

        // 4. Stamp FEEDER_AUDIT provenance.
        let feeder = feeder_audit::feeder_audit(
            &resource_type,
            &feeder_audit::resource_id(&a_resource, &resource_type),
            feeder_audit::resource_version(&a_resource).as_deref(),
            &feeder_audit::now_iso(),
        );
        feeder_audit::inject_feeder_audit(&mut composition, feeder);

        // 5. Commit through the NORMAL validated path — a resource that maps to
        //    an invalid COMPOSITION is rejected here (content_invalid → 422),
        //    never partially stored.
        Ok(self.create_composition_response(ehr_id, composition).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn fhir_search(
        &self,
        resource_type: String,
        patient: String,
        count: Option<i64>,
    ) -> Result<Value, SmError> {
        self.fhir_search_bundle(&resource_type, &patient, count)
            .await
    }
}
