//! Demographic (ehr-less) CONTRIBUTION — commit + retrieval. Our own extension:
//! ITS-REST 1.0.3 defines no demographic wire contract, and the SM demographic
//! service is abstract (register
//! `docs/design/platform/04-service-demographic-ehr-index.md`). A demographic
//! CONTRIBUTION wraps a change-set of party / relationship versions with
//! `ehr_id = None`, committed through the shared CONTRIBUTION engine
//! ([`crate::versioning::commit_version_set`]) with `party_only = true`
//! (RM common master06 §Contributions).
//!
//! TODO(w3f-integrate): the retrieval read is interim direct SQL over the
//! storage tables (no openEHR spec governs the storage read — our own design);
//! `versioning::get_contribution` is EHR-scoped and cannot serve an ehr-less
//! contribution, so this should move behind a `crate::storage::version_repo`
//! demographic helper (storage owns the SQL — README cross-register ruling).

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use ehrbase_rest::{ResourceMeta, ServiceResponse};

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{TreeId, audit_details, commit_version_set, object_version_id};

impl EhrbaseService {
    /// Commit a demographic CONTRIBUTION (ehr-less): its versions must reference
    /// party / relationship objects (an EHR-kind type inside is rejected `422`
    /// by the engine's scope check). Returns the assembled CONTRIBUTION with its
    /// `ETag`/`Location` (the contribution uid).
    pub(crate) async fn create_demographic_contribution(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        let contribution_id = commit_version_set(self, None, &body, true).await?;
        let body = self.demographic_contribution(contribution_id).await?;
        let meta = ResourceMeta::new(String::new(), contribution_id.to_string());
        Ok(ServiceResponse::new(body, meta))
    }

    /// Retrieve a demographic (ehr-less) CONTRIBUTION by id. An EHR-scoped
    /// contribution uid here is `404` (the demographic surface only sees
    /// `ehr_id IS NULL` contributions).
    pub(crate) async fn demographic_contribution(
        &self,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let meta = sqlx::query(
            "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
             FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.id = $1 AND c.ehr_id IS NULL",
        )
        .bind(contribution_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::NotFound(format!("demographic CONTRIBUTION {contribution_id}"))
        })?;

        let system_id: String = meta.try_get("system_id")?;
        let change_type: String = meta.try_get("change_type")?;
        let description: Option<String> = meta.try_get("description")?;
        let committer: Value = meta.try_get("committer")?;
        let time_committed: jiff::Timestamp = meta
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff();

        let version_rows = sqlx::query(
            "SELECT vo_id, trunk_version, branch_number, branch_version, creating_system_id, kind \
             FROM vo_version WHERE contribution_id = $1 ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;
        let versions: Vec<Value> = version_rows
            .iter()
            .map(|row| -> Result<Value, ServiceError> {
                let vo_id: Uuid = row.try_get("vo_id")?;
                let tree = TreeId::from_columns(
                    row.try_get("trunk_version")?,
                    row.try_get("branch_number")?,
                    row.try_get("branch_version")?,
                );
                let creating_system_id: String = row.try_get("creating_system_id")?;
                let kind: String = row.try_get("kind")?;
                Ok(json!({
                    "_type": "OBJECT_REF",
                    "namespace": "demographic",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": object_version_id(vo_id, &creating_system_id, tree)
                    }
                }))
            })
            .collect::<Result<_, _>>()?;

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": audit_details(
                &system_id, &change_type, description.as_deref(), &committer, &time_committed,
            ),
            "versions": versions
        }))
    }
}
