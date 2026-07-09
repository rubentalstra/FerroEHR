//! ITS-REST **adapter-support** extension traits — calls the openEHR SM does
//! **not** define, segregated here so the SM catalog traits stay pure.
//!
//! PORT NOTE: none of these are SM interface calls. They exist because the
//! ITS-REST 1.0.3 wire needs them: the `*_latest_meta` seams decorate a
//! `409`/`412` response with the current `version_uid` in `ETag`/`Location`
//! (`409_COMPOSITION_with_uid_based_id.yaml` / `412_*.yaml`), and the item-tag
//! CRUD is EHRbase's experimental tag extension — neither has an SM call. The
//! platform component implements them beside the SM catalog; the adapter
//! dispatches to them for the wire routes that need them.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SmError;
use crate::types::ResourceMeta;

/// Current-version metadata for the `409`/`412` `ETag`/`Location` decoration.
#[async_trait]
pub trait VersionMetaAdapter: Send + Sync {
    /// The current COMPOSITION version metadata (latest `version_uid`), for a
    /// `409`/`412` on `update`/`delete`. `None` if unknown.
    async fn composition_latest_meta(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError>;

    /// The current `EHR_STATUS` version metadata, for a `412` on
    /// `PUT /ehr_status`.
    async fn ehr_status_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError>;

    /// The current directory FOLDER version metadata, for a `412` on
    /// `PUT`/`DELETE /directory`.
    async fn directory_latest_meta(&self, an_ehr_id: Uuid)
    -> Result<Option<ResourceMeta>, SmError>;
}

/// The experimental item-tag CRUD extension (EHRbase; no SM call).
#[async_trait]
pub trait ItemTagAdapter: Send + Sync {
    /// `GET /ehr/{ehr_id}/tags` — all item tags in the EHR, filtered by the
    /// optional `key`/`value`/`target_path`.
    async fn ehr_tags_get(
        &self,
        an_ehr_id: Uuid,
        key: Option<String>,
        value: Option<String>,
        target_path: Option<String>,
    ) -> Result<Vec<Value>, SmError>;

    /// `GET …/{target}/{uid_based_id}/tags` — the tags on a versioned target
    /// (COMPOSITION or `EHR_STATUS`).
    async fn target_tags_get(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
    ) -> Result<Vec<Value>, SmError>;

    /// `PUT …/{target}/{uid_based_id}/tags` — replace the tags on a versioned
    /// target; `target_type` is the RM type name (`COMPOSITION`/`EHR_STATUS`).
    async fn target_tags_replace(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError>;

    /// `DELETE …/{target}/{uid_based_id}/tags/{key}` — delete one tag by key.
    async fn target_tag_delete(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        key: String,
    ) -> Result<(), SmError>;
}
