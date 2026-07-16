//! `VERSIONED_PARTY` read surface — the demographic analogue of the EHR
//! `versioned_composition` reads (`VERSIONED_PARTY`, its `REVISION_HISTORY`, and
//! its `ORIGINAL_VERSION`s). ITS-REST 1.0.3 defines no demographic wire
//! contract, so this whole surface is our own extension by analogy with the EHR
//! group (register `docs/design/platform/04-service-demographic-ehr-index.md`).
//!
//! The version-spine reads go through `crate::storage::version_repo` (storage
//! owns the SQL — no openEHR spec governs the storage read, our own design; RM
//! common master04 §Revision History / master06 §Versioned Objects govern the
//! assembled wire shape). Versioning's `revision_history`/`versioned_object`
//! builders are EHR-scoped, so the demographic chapter maps the ehr-less
//! `VersionMeta` rows into the wire shape itself.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    CommitEnv, TreeId, audit_details, object_version_id, original_version, read_current,
    read_version, version_at,
};

impl EhrbaseService {
    /// The `VERSIONED_PARTY` for a party (any of the five kinds). A non-party id
    /// is `404`.
    pub(crate) async fn versioned_party(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        // `VERSIONED_OBJECT.time_created` is the commit time of the earliest
        // held version; for a locally-created party that earliest-held version
        // is v1.
        let time_created = crate::storage::version_repo::time_created(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("versioned party {vo_id}")))?;
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
        let metas = crate::storage::version_repo::all_version_meta(&self.pool, vo_id).await?;

        let mut items = Vec::with_capacity(metas.len());
        for meta in &metas {
            let tree =
                TreeId::from_columns(meta.trunk_version, meta.branch_number, meta.branch_version);
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": object_version_id(vo_id, &meta.creating_system_id, tree)
                },
                "audits": [audit_details(
                    &meta.audit_system_id,
                    &meta.audit_change_type,
                    meta.audit_description.as_deref(),
                    &meta.audit_committer,
                    &meta.time_committed,
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
