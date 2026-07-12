//! DIRECTORY (FOLDER) domain logic — one FOLDER tree per EHR, on the shared
//! [`vobject`](super::vobject) versioned-object machinery.

use ehrbase_rest::ServiceResponse;
use serde_json::Value;
use uuid::Uuid;

use super::codes::change_type;
use super::version_id::TreeId;
use super::vobject::{self, Kind};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create the EHR's directory (its root FOLDER). Conflicts if one exists.
    pub(super) async fn create_directory(
        &self,
        ehr_id: Uuid,
        folder: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        validate_folder(&folder)?;
        // EHR_STATUS.is_modifiable = False forbids content writes; the directory
        // (hierarchical Folders) is EHR content (ehr/master04 §"EHR Active Status").
        self.ensure_content_writable(ehr_id).await?;
        // `POST /directory` manages the single directory slot = `EHR.directory`
        // (= `folders[1]`, RM ehr §EHR Class `Directory_in_folders`); it conflicts
        // when a hierarchy already occupies that slot (i.e. `directory_vo_opt`
        // resolves). Additional hierarchies are added via CONTRIBUTION only —
        // ITS-REST/SM bind only the directory (RM ehr master04 §Folders).
        if self.directory_vo_opt(ehr_id).await?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already has a directory"
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "DIRECTORY creation");
        let committed = vobject::create(
            &mut tx,
            Some(ehr_id),
            Kind::Folder,
            folder,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
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
    ) -> Result<ServiceResponse, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))?;
        if read.deleted() {
            // Deleted → 204 (directory_get_at_time.yaml 204_because_deleted_at_time).
            return Ok(ServiceResponse::plain(Value::Null));
        }
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        let folder = self.with_uid(read.canonical, vo_id, &read.creating_system_id, read.tree);
        match path.map(str::trim).filter(|p| !p.is_empty() && *p != "/") {
            None => Ok(ServiceResponse::new(folder, meta)),
            Some(path) => select_subfolder(&folder, path)
                .map(|sub| ServiceResponse::new(sub, meta))
                .ok_or_else(|| ServiceError::NotFound(format!("folder path {path:?}"))),
        }
    }

    /// A specific version of the directory (from a `version_uid`).
    pub(super) async fn directory_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("directory {vo_id} v{version}")))?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `VERSIONED_OBJECT` for an EHR's directory (`get_versioned_directory`,
    /// `i_ehr_directory.adoc`). Resolves `EHR.directory` (= `folders[1]`, RM ehr
    /// §EHR Class `Directory_in_folders`) and wraps it exactly as the EHR_STATUS
    /// / COMPOSITION versioned-object views do
    /// ([`versioned_object`](EhrbaseService::versioned_object): `_type`
    /// `VERSIONED_OBJECT`, `uid`, `owner_id` → the owning EHR, `time_created`).
    pub(super) async fn versioned_directory(&self, ehr_id: Uuid) -> Result<Value, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        self.versioned_object(vo_id, ehr_id).await
    }

    /// Whether `version` of the directory versioned object `vo_id` exists for
    /// this EHR (`has_directory_version`). A logically deleted version still
    /// counts as existing (`i_ehr_directory.adoc`: "True if the directory has a
    /// version with specified id").
    pub(super) async fn has_directory_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<bool, ServiceError> {
        // The id must name THIS EHR's directory versioned object.
        if self.directory_vo_opt(ehr_id).await? != Some(vo_id) {
            return Ok(false);
        }
        Ok(vobject::read_version(&self.pool, vo_id, version)
            .await?
            .is_some_and(|r| r.ehr_id == Some(ehr_id)))
    }

    /// Update the EHR's directory. `expected` (from `If-Match`) enforces
    /// optimistic concurrency.
    pub(super) async fn update_directory(
        &self,
        ehr_id: Uuid,
        folder: Value,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        validate_folder(&folder)?;
        // EHR_STATUS.is_modifiable = False forbids content writes (ehr/master04
        // §"EHR Active Status").
        self.ensure_content_writable(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "DIRECTORY update");
        vobject::update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Folder,
            folder,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;

        self.directory_at(ehr_id, vo_id).await
    }

    /// Logically delete the EHR's directory. `204_because_deleted` declares no
    /// `ETag`/`Location`, so the response carries no metadata.
    pub(super) async fn delete_directory(
        &self,
        ehr_id: Uuid,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        // EHR_STATUS.is_modifiable = False forbids content writes, incl. logical
        // delete (ehr/master04 §"EHR Active Status").
        self.ensure_content_writable(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "DIRECTORY delete");
        vobject::delete(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Folder,
            expected,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    /// The EHR's directory versioned-object id (`EHR.directory` =
    /// `folders.item(1)`, RM ehr §EHR Class `Directory_in_folders`), or
    /// `NotFound`. The lowest-`rank` live hierarchy — see
    /// [`directory_vo_opt`](EhrbaseService::directory_vo_opt).
    async fn directory_vo(&self, ehr_id: Uuid) -> Result<Uuid, ServiceError> {
        self.directory_vo_opt(ehr_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))
    }

    /// Load the current directory FOLDER (by vo id) with its `uid` set and the
    /// version metadata — the create/update response
    /// (`ETag`/`Location` for `201_directory` / `200_directory_updated`).
    async fn directory_at(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "directory for EHR {ehr_id} is deleted"
            )));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }
}

/// Validate a client-supplied FOLDER tree before it is committed (directory
/// create/update and the CONTRIBUTION FOLDER path). RM common
/// `org.openehr.rm.common.folder.adoc` + RM ehr master04 §Folders:
///
/// - each node is a `FOLDER` (foreign `_type` rejected) with `name` (1..1,
///   LOCATABLE) and a non-empty `archetype_node_id`
///   (`Archetype_node_id_valid`);
/// - `items` members are `OBJECT_REF`s — "Folder structures do not contain
///   Compositions, only references to them" (master04 §Folders): a member
///   must carry `id` + `namespace` + `type`, and a LOCATABLE-by-value payload
///   (e.g. an inline COMPOSITION) is rejected;
/// - `folders` members recurse.
pub(super) fn validate_folder(folder: &Value) -> Result<(), ServiceError> {
    fn walk(node: &Value, path: &str) -> Result<(), ServiceError> {
        let unproc = |m: String| ServiceError::Unprocessable(m);
        let obj = node
            .as_object()
            .ok_or_else(|| unproc(format!("{path}: FOLDER must be a JSON object")))?;
        match obj.get("_type").and_then(Value::as_str) {
            None | Some("FOLDER") => {}
            Some(other) => {
                return Err(unproc(format!(
                    "{path}: expected a FOLDER, got _type {other:?}"
                )));
            }
        }
        if obj.get("name").is_none_or(Value::is_null) {
            return Err(unproc(format!(
                "{path}: FOLDER.name is mandatory (LOCATABLE.name 1..1)"
            )));
        }
        if obj
            .get("archetype_node_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(unproc(format!(
                "{path}: FOLDER.archetype_node_id is mandatory and non-empty                  (LOCATABLE.Archetype_node_id_valid)"
            )));
        }
        if let Some(items) = obj.get("items").and_then(Value::as_array) {
            for (i, item) in items.iter().enumerate() {
                let ok = item.get("id").is_some_and(Value::is_object)
                    && item
                        .get("namespace")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                    && item
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                    // A LOCATABLE by value carries archetype_node_id — an
                    // OBJECT_REF never does.
                    && item.get("archetype_node_id").is_none();
                if !ok {
                    return Err(unproc(format!(
                        "{path}/items[{i}]: FOLDER.items members must be OBJECT_REFs                          (id + namespace + type) — Folder structures do not contain                          Compositions by value, only references to them                          (RM ehr master04 §Folders)"
                    )));
                }
            }
        }
        if let Some(folders) = obj.get("folders").and_then(Value::as_array) {
            for (i, sub) in folders.iter().enumerate() {
                walk(sub, &format!("{path}/folders[{i}]"))?;
            }
        }
        Ok(())
    }
    walk(folder, "")
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::validate_folder;

    /// FOLDER trees hold `OBJECT_REF` items only — never content by value
    /// (RM ehr master04 §Folders; RM common `folder.adoc`; A1 rm-ehr-R30).
    #[test]
    fn folder_items_must_be_object_refs() {
        let good = json!({
            "_type": "FOLDER",
            "name": { "_type": "DV_TEXT", "value": "root" },
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "items": [{
                "_type": "OBJECT_REF", "namespace": "local", "type": "VERSIONED_COMPOSITION",
                "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
            }],
            "folders": [{
                "_type": "FOLDER",
                "name": { "_type": "DV_TEXT", "value": "sub" },
                "archetype_node_id": "at0001"
            }]
        });
        validate_folder(&good).expect("a ref-holding folder tree is valid");

        // A COMPOSITION by value inside items is rejected.
        let mut bad = good.clone();
        bad["items"][0] = json!({
            "_type": "COMPOSITION",
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "name": { "_type": "DV_TEXT", "value": "inline!" }
        });
        let err = validate_folder(&bad).expect_err("content by value must be rejected");
        assert!(err.to_string().contains("OBJECT_REF"), "got {err}");

        // A sub-folder without a name violates LOCATABLE.name 1..1.
        let mut bad = good;
        bad["folders"][0].as_object_mut().unwrap().remove("name");
        let err = validate_folder(&bad).expect_err("nameless sub-folder rejected");
        assert!(err.to_string().contains("name"), "got {err}");
    }
}
