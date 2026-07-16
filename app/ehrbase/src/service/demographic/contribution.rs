//! Demographic (ehr-less) CONTRIBUTION — commit + retrieval. Our own extension:
//! ITS-REST 1.0.3 defines no demographic wire contract, and the SM demographic
//! service is abstract (register
//! `docs/design/platform/04-service-demographic-ehr-index.md`). A demographic
//! CONTRIBUTION wraps a change-set of party / relationship versions with
//! `ehr_id = None`, committed through the shared CONTRIBUTION engine
//! ([`crate::versioning::contribution::commit_version_set`]) with `party_only = true`
//! (RM common master06 §Contributions).
//!
//! The retrieval reads go through `crate::storage::version_repo` (storage owns
//! the SQL — no openEHR spec governs the storage read, our own design):
//! `crate::versioning::contribution::get_contribution` is EHR-scoped and cannot serve an ehr-less
//! contribution, so the demographic chapter reads the audit + version refs here.
//! The version-refs helper unions the versions a `666|attestation|` item
//! attested (RM common master06 §Contributions — an attestation affects an
//! existing version), so the list is the full change-set, not the narrower
//! committed-only rows.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::storage::version_repo;
use crate::versioning::audit::audit_details;
use crate::versioning::contribution::commit_version_set;
use crate::versioning::object_version_id::{TreeId, object_version_id};

impl EhrbaseService {
    /// Commit a demographic CONTRIBUTION (ehr-less): its versions must reference
    /// party / relationship objects (an EHR-kind type inside is rejected `422`
    /// by the engine's scope check). Returns the assembled CONTRIBUTION with its
    /// `ETag`/`Location` (the contribution uid).
    pub(super) async fn create_demographic_contribution(
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
    pub(super) async fn demographic_contribution(
        &self,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let audit =
            version_repo::contribution::contribution_audit(&self.pool, contribution_id, None)
                .await?
                .ok_or_else(|| {
                    ServiceError::NotFound(format!("demographic CONTRIBUTION {contribution_id}"))
                })?;

        // The refs helper also unions the versions this contribution's
        // `666|attestation|` items attested (RM common master06 §Contributions —
        // an attestation affects an existing version), i.e. the full change-set
        // the CONTRIBUTION covers, not just the committed rows.
        let version_refs =
            version_repo::contribution::contribution_version_refs(&self.pool, contribution_id)
                .await?;
        let versions: Vec<Value> = version_refs
            .into_iter()
            .map(|(vo_id, columns, creating_system_id, kind)| {
                let tree = TreeId::from_columns(columns.0, columns.1, columns.2);
                json!({
                    "_type": "OBJECT_REF",
                    "namespace": "demographic",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": object_version_id(vo_id, &creating_system_id, tree)
                    }
                })
            })
            .collect();

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": audit_details(
                &audit.system_id,
                &audit.change_type,
                audit.description.as_deref(),
                &audit.committer,
                &audit.time_committed,
            ),
            "versions": versions
        }))
    }
}
