//! `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`) — the DIRECTORY (FOLDER) surface,
//! one hierarchy per EHR through the ITS-REST/SM `/directory` binding.
//!
//! Spec: RM ehr `master04-ehr_package.adoc` §Folders + the EHR class invariants
//! `Directory_in_folders` (`EHR.directory = folders.item(1)`) / `Folders_valid`;
//! RM common `org.openehr.rm.common.folder.adoc`. Versioned-object mechanics are
//! RM common master06, delegated to [`crate::versioning`].
//!
//! PORT NOTE (G-3): the read side is multi-hierarchy
//! ([`Self::live_folder_hierarchies`]); the write side manages the single
//! directory slot (= `folders[1]`) only. Additional hierarchies are committed
//! via CONTRIBUTION — ITS-REST/SM bind only the directory (RM ehr master04
//! §Folders). Multi-hierarchy write management is owned by WORKLIST W-6.

use ehrbase_sm::{EhrDirectoryService, ServiceResponse, SmError, UpdateVersion};
use openehr_base::prelude::ObjectVersionId;
use serde_json::Value;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    Kind, TreeId, change_type, components, create, delete, read_current, read_version, update,
    version_at, versioned_object,
};

use super::{ensure_if_match, parse_at_time};

impl EhrbaseService {
    /// Create the EHR's directory (its root FOLDER). Conflicts if one exists.
    pub(in crate::service) async fn create_directory(
        &self,
        ehr_id: Uuid,
        folder: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        validate_folder(&folder)?;
        // is_modifiable = False forbids content writes; the directory is EHR
        // content (RM ehr master04 §EHR Active Status).
        self.ensure_content_writable(ehr_id).await?;
        // `POST /directory` manages the single directory slot = EHR.directory (=
        // folders[1], RM ehr §EHR Class Directory_in_folders); it conflicts when a
        // hierarchy already occupies that slot.
        if self.directory_vo_opt(ehr_id).await?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already has a directory"
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "DIRECTORY creation");
        let committed = create(
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

        // The write response is metadata-only: `Committed` already carries the
        // written version identity + the commit instant (RM common master06
        // §Committal), so the create path never re-reads the row it just wrote —
        // a representation response re-reads at the protocol layer. This mirrors
        // the COMPOSITION create path.
        Ok(self.committed_response(ehr_id, &committed))
    }

    /// The EHR's directory FOLDER (current, or at an instant when `at` is given),
    /// optionally navigated to a sub-folder `path` (`/a/b`). A deleted directory
    /// resolves to `Value::Null` (→ 204).
    pub(in crate::service) async fn directory_at_time(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
        path: Option<&str>,
    ) -> Result<ServiceResponse, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))?;
        if read.deleted() {
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
    pub(in crate::service) async fn directory_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("directory {vo_id} v{version}")))?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `VERSIONED_OBJECT` for an EHR's directory (`get_versioned_directory`).
    pub(in crate::service) async fn versioned_directory(
        &self,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let vo_id = self.directory_vo(ehr_id).await?;
        versioned_object(&self.pool, vo_id, ehr_id, "VERSIONED_FOLDER").await
    }

    /// Whether `version` of the directory versioned object `vo_id` exists for
    /// this EHR (`has_directory_version`). A logically deleted version still
    /// counts as existing.
    pub(in crate::service) async fn has_directory_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<bool, ServiceError> {
        // The id must name THIS EHR's directory versioned object.
        if self.directory_vo_opt(ehr_id).await? != Some(vo_id) {
            return Ok(false);
        }
        Ok(read_version(&self.pool, vo_id, version)
            .await?
            .is_some_and(|r| r.ehr_id == Some(ehr_id)))
    }

    /// Update the EHR's directory. `vo_id` is the directory-slot versioned object
    /// (resolved once by the caller's `If-Match` meta pre-read, so the JOIN is not
    /// re-run here); `expected` (from `If-Match`) enforces optimistic concurrency.
    pub(in crate::service) async fn update_directory(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        folder: Value,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        validate_folder(&folder)?;
        self.ensure_content_writable(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "DIRECTORY update");
        let committed = update(
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

        // Metadata-only write response from `Committed` (see `create_directory`).
        Ok(self.committed_response(ehr_id, &committed))
    }

    /// Logically delete the EHR's directory. `204_because_deleted` declares no
    /// `ETag`/`Location`, so the response carries no metadata.
    pub(in crate::service) async fn delete_directory_at(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_content_writable(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "DIRECTORY delete");
        delete(
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

    /// The current directory FOLDER version metadata (for a `412`
    /// `ETag`/`Location`). Resolves `EHR.directory` (= `folders.item(1)`, RM ehr
    /// §EHR Class `Directory_in_folders`) rather than assuming a single FOLDER
    /// versioned object.
    pub(in crate::service) async fn directory_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ehrbase_sm::ResourceMeta>, ServiceError> {
        Ok(self.directory_meta_with_vo(ehr_id).await?.map(|(_, m)| m))
    }

    /// The directory-slot versioned-object id **and** its current version
    /// metadata, resolved and read in ONE `ehr_folder`⋈`vo_version`⋈`audit`
    /// statement ([`crate::storage::ehr_repo::directory_current_meta`] — no node
    /// reassembly, no attestation read). The slot JOIN and the metadata-only
    /// current-version read are folded into a single round trip; threading the
    /// `vo_id` back to the caller lets the inner write skip re-running the slot
    /// JOIN. `None` when the EHR indexes no directory hierarchy.
    pub(in crate::service) async fn directory_meta_with_vo(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<(Uuid, ehrbase_sm::ResourceMeta)>, ServiceError> {
        let Some(m) = crate::storage::ehr_repo::directory_current_meta(&self.pool, ehr_id).await?
        else {
            return Ok(None);
        };
        let tree = TreeId::from_columns(m.trunk_version, m.branch_number, m.branch_version);
        let meta = self.version_meta(
            ehr_id,
            m.vo_id,
            &m.creating_system_id,
            tree,
            m.time_committed,
        );
        Ok(Some((m.vo_id, meta)))
    }

    /// The versioned-object id of the EHR's directory — `EHR.directory` (=
    /// `folders.item(1)`, RM ehr §EHR Class `Directory_in_folders`). Resolved as
    /// the lowest-`rank` LIVE hierarchy; when none is live it falls back to the
    /// lowest-`rank` hierarchy that still exists, so a read after a logical delete
    /// resolves to the deleted version (→ 204) rather than 404. `None` when the
    /// EHR indexes no folder hierarchy.
    ///
    /// The `ehr_folder` ⋈ `vo_version` resolution is a storage seam
    /// ([`crate::storage::ehr_repo::directory_vo`]; no openEHR spec governs the
    /// SQL — our own design).
    pub(in crate::service) async fn directory_vo_opt(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<Uuid>, ServiceError> {
        Ok(crate::storage::ehr_repo::directory_vo(&self.pool, ehr_id).await?)
    }

    /// The EHR's directory versioned-object id, or `NotFound`.
    async fn directory_vo(&self, ehr_id: Uuid) -> Result<Uuid, ServiceError> {
        self.directory_vo_opt(ehr_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("directory for EHR {ehr_id}")))
    }
}

/// Validate a client-supplied FOLDER tree before it is committed (directory
/// create/update and the CONTRIBUTION FOLDER path). RM common `folder.adoc` + RM
/// ehr master04 §Folders:
///
/// - each node is a `FOLDER` (foreign `_type` rejected) with `name` (1..1) and a
///   non-empty `archetype_node_id` (`Archetype_node_id_valid`);
/// - `items` members are `OBJECT_REF`s — "Folder structures do not contain
///   Compositions, only references to them" (master04 §Folders): a member must
///   carry `id` + `namespace` + `type`, and a LOCATABLE-by-value payload is
///   rejected;
/// - `folders` members recurse.
pub(in crate::service) fn validate_folder(folder: &Value) -> Result<(), ServiceError> {
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
                "{path}: FOLDER.archetype_node_id is mandatory and non-empty \
                 (LOCATABLE.Archetype_node_id_valid)"
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
                        "{path}/items[{i}]: FOLDER.items members must be OBJECT_REFs \
                         (id + namespace + type) — Folder structures do not contain \
                         Compositions by value, only references to them \
                         (RM ehr master04 §Folders)"
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
/// segment against child `folders[].name.value` (SM `has_path`: slash-separated
/// Folder names). Returns the sub-folder JSON.
fn select_subfolder(folder: &Value, path: &str) -> Option<Value> {
    let mut current = folder;
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        let children = current.get("folders")?.as_array()?;
        current = children.iter().find(|f| {
            f.get("name")
                .and_then(|n| n.get("value"))
                .and_then(Value::as_str)
                == Some(segment)
        })?;
    }
    Some(current.clone())
}

#[async_trait::async_trait]
impl EhrDirectoryService for EhrbaseService {
    async fn has_directory(&self, an_ehr_id: Uuid) -> Result<bool, SmError> {
        // EHR.directory (= folders[1]) — resolve the directory slot rather than
        // assuming a single FOLDER (RM ehr master04 §Folders).
        Ok(self.directory_vo_opt(an_ehr_id).await?.is_some())
    }

    async fn has_path(&self, an_ehr_id: Uuid, a_path: String) -> Result<bool, SmError> {
        match self.directory_at_time(an_ehr_id, None, Some(&a_path)).await {
            Ok(resp) => Ok(!resp.body.is_null()),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError> {
        super::version_uid(self.create_directory(an_ehr_id, a_dir_struct.data).await?)
    }

    async fn get_directory_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
        a_path: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self
            .directory_at_time(an_ehr_id, at, a_path.as_deref())
            .await?
            .body)
    }

    async fn update_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError> {
        // Resolve the directory-slot vo_id + its current version metadata ONCE
        // (the `If-Match` pre-read); a missing directory is `NotFound` (the same
        // error the inner write's slot resolution produced). The vo_id is
        // threaded into the write so the slot JOIN is not re-run.
        let Some((vo_id, latest)) = self.directory_meta_with_vo(an_ehr_id).await? else {
            return Err(ServiceError::NotFound(format!("directory for EHR {an_ehr_id}")).into());
        };
        ensure_if_match(a_dir_struct.preceding_version_uid.as_ref(), Some(&latest))?;
        let expected = a_dir_struct
            .preceding_version_uid
            .as_ref()
            .map(|o| components(o).map(|(_, v)| v))
            .transpose()?;
        super::version_uid(
            self.update_directory(an_ehr_id, vo_id, a_dir_struct.data, expected)
                .await?,
        )
    }

    async fn delete_directory(
        &self,
        an_ehr_id: Uuid,
        preceding_version_uid: Option<ObjectVersionId>,
    ) -> Result<(), SmError> {
        let Some((vo_id, latest)) = self.directory_meta_with_vo(an_ehr_id).await? else {
            return Err(ServiceError::NotFound(format!("directory for EHR {an_ehr_id}")).into());
        };
        ensure_if_match(preceding_version_uid.as_ref(), Some(&latest))?;
        let expected = preceding_version_uid
            .as_ref()
            .map(|o| components(o).map(|(_, v)| v))
            .transpose()?;
        self.delete_directory_at(an_ehr_id, vo_id, expected).await?;
        Ok(())
    }

    async fn get_directory_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        Ok(self
            .directory_version(an_ehr_id, vo_id, version)
            .await?
            .body)
    }

    async fn has_directory_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        Ok(self
            .has_directory_version(an_ehr_id, vo_id, version)
            .await?)
    }

    async fn get_versioned_directory(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.versioned_directory(an_ehr_id).await?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::validate_folder;

    /// FOLDER trees hold `OBJECT_REF` items only — never content by value
    /// (RM ehr master04 §Folders; RM common `folder.adoc`).
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
