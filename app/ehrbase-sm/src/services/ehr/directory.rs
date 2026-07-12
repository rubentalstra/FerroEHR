//! `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`) — "Operations on EHR directory,
//! with implicit Contribution creation."

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use openehr_base::prelude::ObjectVersionId;

use crate::common::{SmError, UpdateVersion};

/// `I_EHR_DIRECTORY` — DIRECTORY (FOLDER) operations, one Rust method per SM
/// call. Reads return the canonical `FOLDER` as [`Value`] (`Value::Null` for
/// a logically deleted/absent directory — the SM's "if it exists, else Void"
/// → wire `204`); the implicit-Contribution writes return the new
/// `version_uid`. The write payload is the chapter's `UV_FOLDER`, i.e.
/// [`UpdateVersion`]`<FOLDER>`.
#[async_trait]
pub trait EhrDirectoryService: Send + Sync {
    /// `has_directory (ehr_id: UUID): Boolean` — "True if the EHR has a
    /// directory structure." Pre `has_ehr`.
    async fn has_directory(&self, an_ehr_id: Uuid) -> Result<bool, SmError>;

    /// `has_path (ehr_id: UUID, a_path: String): Boolean` — "True if path
    /// `a_path` exists in directory. The `a_path` argument consists of
    /// slash-separated values of the name attribute of Folders in the
    /// directory." Pre `has_ehr`. Error `ehr_id_does_not_exist`.
    async fn has_path(&self, an_ehr_id: Uuid, a_path: String) -> Result<bool, SmError>;

    /// `create_directory (ehr_id: UUID, a_dir_struct: UV_FOLDER)` — "Create a
    /// directory in the EHR … Causes server-side creation of a new
    /// `VERSIONED_OBJECT`, `ORIGINAL_VERSION` and new `CONTRIBUTION`."
    /// Pre `has_ehr` + `definitions_valid` + `not has_directory` +
    /// `valid_content`. Errors `ehr_id_does_not_exist`, `definition_unknown`,
    /// `content_invalid`. Returns the new `version_uid` (wire `201_directory`
    /// `ETag`/`Location`).
    async fn create_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `get_directory_at_time (an_ehr_id: UUID, a_time: Iso8601_date_time
    /// [0..1]): FOLDER` — "the version of the Directory extant at time
    /// `a_time`. If no time supplied, get the latest" (subsuming the SM's
    /// separate `get_directory`, whose semantics are `a_time = None`).
    /// `a_path` (ITS-REST `path` query) sub-selects a Folder by name path.
    async fn get_directory_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
        a_path: Option<String>,
    ) -> Result<Value, SmError>;

    /// `update_directory (ehr_id: UUID, a_dir_struct: UV_FOLDER)` — "Create
    /// or update a directory from a complete structure. Preceding version
    /// must be supplied and correct if EHR directory already exists." Pre
    /// `has_ehr` + `definitions_valid` + `valid_content` + `has_directory`;
    /// optimistic lock via [`UpdateVersion::preceding_version_uid`] →
    /// `version_mismatch`. New `ORIGINAL_VERSION` + `CONTRIBUTION`; returns
    /// the new `version_uid`.
    async fn update_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `delete_directory (ehr_id: UUID)` — "Logically delete the directory by
    /// creating a new version in which the contents are removed." Pre
    /// `has_ehr` + `has_directory`. The `preceding_version_uid` (from the
    /// wire `If-Match`) guards the delete.
    async fn delete_directory(
        &self,
        an_ehr_id: Uuid,
        preceding_version_uid: Option<ObjectVersionId>,
    ) -> Result<(), SmError>;

    /// `has_directory_version (an_ehr_id: UUID, a_version_uid: UUID):
    /// Boolean` — "True if the directory has a version with specified id."
    /// Error `ehr_id_does_not_exist`.
    async fn has_directory_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError>;

    /// `get_directory_at_version (an_ehr_id: UUID, a_version_uid: UUID):
    /// FOLDER` — "Get a particular version of the EHR Directory"
    /// (`Value::Null` if that version is a logical delete). Error
    /// `version_does_not_exist`.
    async fn get_directory_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError>;

    /// `get_versioned_directory (an_ehr_id: UUID): VERSIONED_FOLDER` — "Get
    /// the `VERSIONED_FOLDER` Directory object for the EHR with `ehr_id`."
    /// Pre `has_ehr`. Error `ehr_id_does_not_exist`.
    async fn get_versioned_directory(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;
}
