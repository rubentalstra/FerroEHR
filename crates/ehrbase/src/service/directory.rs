//! DIRECTORY (FOLDER) domain logic — one FOLDER tree per EHR, on the shared
//! [`vobject`](super::vobject) versioned-object machinery.

use serde_json::Value;
use uuid::Uuid;

use super::vobject::{self, Kind, change_type};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create the EHR's directory (its root FOLDER). Conflicts if one exists.
    pub(super) async fn create_directory(
        &self,
        ehr_id: Uuid,
        folder: Value,
    ) -> Result<Value, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        if self.current_vo(ehr_id, Kind::Folder).await?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already has a directory"
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "DIRECTORY creation");
        let committed =
            vobject::create(&mut tx, ehr_id, Kind::Folder, folder, None, &audit).await?;
        tx.commit().await?;

        self.directory_at(ehr_id, committed.vo_id).await
    }

    /// The EHR's directory FOLDER (current, or at an instant when `at` is given),
    /// optionally navigated to a sub-folder `path` (`/a/b`).
    pub(super) async fn directory_at_time(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
        path: Option<&str>,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self.directory_vo(ehr_id).await?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == ehr_id)
        .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))?;
        if read.deleted {
            return Err(ServiceError::NotFound(format!(
                "directory for EHR {ehr_id} is deleted"
            )));
        }
        let folder = self.with_uid(read.canonical, vo_id, read.sys_version);
        match path.map(str::trim).filter(|p| !p.is_empty() && *p != "/") {
            None => Ok(folder),
            Some(path) => select_subfolder(&folder, path)
                .ok_or_else(|| ServiceError::NotFound(format!("folder path {path:?}"))),
        }
    }

    /// A specific version of the directory (from a `version_uid`).
    pub(super) async fn directory_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: i32,
    ) -> Result<Value, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("directory {vo_id} v{version}")))?;
        Ok(self.with_uid(read.canonical, vo_id, read.sys_version))
    }

    /// Update the EHR's directory. `expected` (from `If-Match`) enforces
    /// optimistic concurrency.
    pub(super) async fn update_directory(
        &self,
        ehr_id: Uuid,
        folder: Value,
        expected: Option<i32>,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self.directory_vo(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "DIRECTORY update");
        vobject::update(
            &mut tx,
            ehr_id,
            vo_id,
            Kind::Folder,
            folder,
            expected,
            None,
            &audit,
        )
        .await?;
        tx.commit().await?;

        self.directory_at(ehr_id, vo_id).await
    }

    /// Logically delete the EHR's directory.
    pub(super) async fn delete_directory(
        &self,
        ehr_id: Uuid,
        expected: Option<i32>,
    ) -> Result<(), ServiceError> {
        let (vo_id, _) = self.directory_vo(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "DIRECTORY delete");
        vobject::delete(&mut tx, ehr_id, vo_id, Kind::Folder, expected, &audit).await?;
        tx.commit().await?;
        Ok(())
    }

    /// The EHR's directory versioned-object id, or `NotFound`.
    async fn directory_vo(&self, ehr_id: Uuid) -> Result<(Uuid, i32), ServiceError> {
        self.current_vo(ehr_id, Kind::Folder)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))
    }

    /// Load the current directory FOLDER (by vo id) with its `uid` set —
    /// the create/update response.
    async fn directory_at(&self, ehr_id: Uuid, vo_id: Uuid) -> Result<Value, ServiceError> {
        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))?;
        if read.deleted {
            return Err(ServiceError::NotFound(format!(
                "directory for EHR {ehr_id} is deleted"
            )));
        }
        Ok(self.with_uid(read.canonical, vo_id, read.sys_version))
    }
}

/// Navigate a FOLDER tree to a sub-folder by name path (`a/b/c`), matching each
/// segment against child `folders[].name.value`. Returns the sub-folder JSON.
fn select_subfolder(folder: &Value, path: &str) -> Option<Value> {
    let mut current = folder;
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        let children = current.get("folders")?.as_array()?;
        current = children.iter().find(|f| {
            f.get("name")
                .and_then(|n| n.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(segment)
        })?;
    }
    Some(current.clone())
}
