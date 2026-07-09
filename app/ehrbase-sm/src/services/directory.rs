//! The SM `I_EHR_DIRECTORY` interface — the literal openEHR Platform Service
//! Model call set
//! (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_directory.adoc`; digest
//! `docs/design/sm-platform/02-ehr-service.md` §4). "Operations on EHR
//! directory, with implicit Contribution creation."

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use openehr_base::prelude::ObjectVersionId;

use crate::error::SmError;
use crate::types::UpdateVersion;

/// `I_EHR_DIRECTORY` — DIRECTORY (FOLDER) operations, one Rust method per SM
/// call. Reads return the canonical `FOLDER` as [`Value`] (`Value::Null` for a
/// logically deleted directory → wire `204`); the implicit-Contribution writes
/// return the new `version_uid`.
#[async_trait]
pub trait EhrDirectoryService: Send + Sync {
    /// `has_directory (ehr_id: UUID): Boolean` — pre `has_ehr`.
    async fn has_directory(&self, an_ehr_id: Uuid) -> Result<bool, SmError>;

    /// `has_path (ehr_id: UUID, a_path: String): Boolean` — `a_path` is a
    /// slash-separated list of Folder `name`s. Pre `has_ehr`. Error
    /// `ehr_id_does_not_exist`.
    async fn has_path(&self, an_ehr_id: Uuid, a_path: String) -> Result<bool, SmError>;

    /// `create_directory (ehr_id: UUID, a_dir_struct: UV_FOLDER)` — pre
    /// `has_ehr` + `definitions_valid` + `not has_directory` + `valid_content`.
    /// Creates `VERSIONED_OBJECT` + `ORIGINAL_VERSION` + CONTRIBUTION. Returns the
    /// new `version_uid` (wire `201_directory` `ETag`/`Location`).
    async fn create_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `get_directory_at_time (an_ehr_id: UUID, a_time: Iso8601_date_time
    /// [0..1]): FOLDER` — no time ⇒ latest; `Value::Null` if deleted/absent.
    /// `a_path` (ITS-REST `path` query) sub-selects a Folder by name path.
    async fn get_directory_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
        a_path: Option<String>,
    ) -> Result<Value, SmError>;

    /// `update_directory (ehr_id: UUID, a_dir_struct: UV_FOLDER)` — pre
    /// `has_ehr` + `definitions_valid` + `valid_content` + `has_directory`; the
    /// preceding version (in [`UpdateVersion::preceding_version_uid`]) must be
    /// supplied and correct (optimistic lock → `version_mismatch`). New
    /// `ORIGINAL_VERSION` + CONTRIBUTION. Returns the new `version_uid`.
    async fn update_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `delete_directory (ehr_id: UUID)` — logical delete (a new version with
    /// contents removed). Pre `has_ehr` + `has_directory`. The
    /// `preceding_version_uid` (from the wire `If-Match`) guards the delete.
    async fn delete_directory(
        &self,
        an_ehr_id: Uuid,
        preceding_version_uid: Option<ObjectVersionId>,
    ) -> Result<(), SmError>;

    /// `get_directory_at_version (an_ehr_id: UUID, a_version_uid: UUID): FOLDER`
    /// — `Value::Null` if that version is a logical delete. Error
    /// `version_does_not_exist`.
    async fn get_directory_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError>;
}
