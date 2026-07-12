//! `VERSIONED_PARTY` read surface — the demographic analogue of the EHR
//! `versioned_composition` reads (`VERSIONED_PARTY`, its `REVISION_HISTORY`, and
//! its `ORIGINAL_VERSION`s). ITS-REST 1.0.3 defines no demographic wire
//! contract, so this whole surface is our own extension by analogy with the EHR
//! group (register `docs/design/platform/04-service-demographic-ehr-index.md`).
//!
//! The version-spine reads are interim direct SQL over the storage tables (no
//! openEHR spec governs the storage read — our own design; RM common master04
//! §Revision History / master06 §Versioned Objects govern the assembled wire
//! shape).
//!
//! TODO(w3f-integrate): move the ehr-less `vo_version`⋈`audit` reads behind a
//! `crate::storage::version_repo` demographic helper — storage owns the SQL
//! (cross-register ruling `docs/design/platform/README.md`); versioning's
//! `revision_history`/`versioned_object` builders are EHR-scoped and cannot be
//! reused for an ehr-less party.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    CommitEnv, TreeId, audit_details, object_version_id, original_version, read_current,
    read_version, version_at,
};
use ehrbase_rest::{ResourceMeta, ServiceResponse};

impl EhrbaseService {
    /// The `VERSIONED_PARTY` for a party (any of the five kinds). A non-party id
    /// is `404`.
    pub(crate) async fn versioned_party(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let time_created: jiff_sqlx::Timestamp = sqlx::query_scalar(
            "SELECT a.time_committed FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 AND v.sys_version = 1",
        )
        .bind(vo_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned party {vo_id}")))?;
        let time_created = time_created.to_jiff();
        // PORT NOTE (G-6): `VERSIONED_OBJECT.owner_id` (1..1) has no EHR owner for
        // a demographic party (no openEHR spec governs the owner of an ehr-less
        // demographic versioned object — our own design); we reference the
        // party's own versioned-object id as its owner (the demographics
        // repository owns it).
        Ok(json!({
            "_type": "VERSIONED_PARTY",
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": "PARTY",
                "id": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        }))
    }

    /// The `REVISION_HISTORY` of a party: one item per version with its
    /// `OBJECT_VERSION_ID` and the change's `AUDIT_DETAILS` (RM common master04
    /// §Revision History). A non-party id is `404`.
    pub(crate) async fn party_revision_history(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let rows = sqlx::query(
            "SELECT v.trunk_version, v.branch_number, v.branch_version, \
             v.creating_system_id, a.system_id, a.change_type, \
             a.description, a.committer, a.time_committed \
             FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 ORDER BY v.sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let tree = TreeId::from_columns(
                row.try_get("trunk_version")?,
                row.try_get("branch_number")?,
                row.try_get("branch_version")?,
            );
            let creating_system_id: String = row.try_get("creating_system_id")?;
            let system_id: String = row.try_get("system_id")?;
            let change_type: String = row.try_get("change_type")?;
            let description: Option<String> = row.try_get("description")?;
            let committer: Value = row.try_get("committer")?;
            let time_committed: jiff::Timestamp = row
                .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                .to_jiff();
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": object_version_id(vo_id, &creating_system_id, tree)
                },
                "audits": [audit_details(
                    &system_id, &change_type, description.as_deref(), &committer, &time_committed,
                )]
            }));
        }
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// An `ORIGINAL_VERSION` of a party at a specific version. A non-party id is
    /// `404`.
    pub(crate) async fn party_version(
        &self,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id.is_none())
            .ok_or_else(|| ServiceError::NotFound(format!("party {vo_id} v{version}")))?;
        let signer = CommitEnv::signing_ctx(self).signer;
        original_version(&read, signer)
    }

    /// The `ORIGINAL_VERSION` of a party extant at `at`, or the latest when `at`
    /// is `None`, with `ETag`/`Location` metadata for the VERSION resource.
    pub(crate) async fn party_version_at_time(
        &self,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("party {vo_id} version at time")))?;
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        let signer = CommitEnv::signing_ctx(self).signer;
        let ov = original_version(&read, signer)?;
        Ok(ServiceResponse::new(ov, meta))
    }
}
