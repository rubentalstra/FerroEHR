//! `ITEM_TAG` CRUD — an ITS-REST **experimental** extension (the tags API is
//! development-branch only), on the `item_tag` table.
//!
//! Spec: RM `ITEM_TAG`
//! (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`
//! — `target: UID_BASED_ID` "may be a `VERSIONED_OBJECT<T>` or a `VERSION<T>`",
//! `owner_id: OBJECT_REF`, the invariants `Inv_key_valid` / `Inv_value_valid`;
//! RM ehr `ehr.adoc` `EHR.tags`: "Tag target values can only be within the
//! same EHR") and ITS-REST overview `Requests_and_responses.md` §item-tag
//! headers (identity: "uniquely identified by their `key` and `target_path`
//! pair"; container vs specific-VERSION targets are distinct). `PUT …/tags`
//! is a full-collection replace of the ADDRESSED collection — the container's
//! or one VERSION's, never both. Not an SM-EHR interface. The `item_tag`
//! table SQL is spec-silent (storage seam — our own design).

use crate::ids::{EhrId, VoId};
use crate::service::status::{CallStatusType, SmError};
use serde_json::{Value, json};

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::object_version_id::parse_uid_based_id;

impl EhrbaseService {
    /// All tags in an EHR, optionally filtered by key / value / target path.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the tag listing fails.
    pub(in crate::service) async fn ehr_tags(
        &self,
        ehr_id: EhrId,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = crate::storage::tag_repo::list_tags(
            &self.pool,
            Some(ehr_id),
            None,
            key,
            value,
            target_path,
        )
        .await?;
        Ok(rows.iter().map(|r| Self::tag_json(ehr_id, r)).collect())
    }

    /// Tags on one target object (a COMPOSITION or `EHR_STATUS`).
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the tag listing fails.
    pub(in crate::service) async fn target_tags(
        &self,
        ehr_id: EhrId,
        target_vo_id: VoId,
        target_version: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = crate::storage::tag_repo::list_tags(
            &self.pool,
            Some(ehr_id),
            Some((target_vo_id, target_version)),
            None,
            None,
            None,
        )
        .await?;
        Ok(rows.iter().map(|r| Self::tag_json(ehr_id, r)).collect())
    }

    /// Replace the **whole** tag collection of a target with the posted set,
    /// returning the target's tags after — `PUT` full-collection semantics
    ///: tags omitted from the body are removed, and an empty list
    /// clears all tags on the target.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR does not exist or the target
    /// versioned object is not in this EHR; [`ServiceError::Unprocessable`]
    /// when a tag violates `Inv_key_valid` / `Inv_value_valid`;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn replace_tags(
        &self,
        ehr_id: EhrId,
        target_vo_id: VoId,
        target_version: Option<&str>,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        self.ensure_tag_target(ehr_id, target_vo_id, target_version, target_type)
            .await?;
        // Validate every tag before writing; the storage replace dedupes the
        // posted set on the ITEM_TAG identity — the (key, target_path) PAIR
        // (ITS-REST Requests_and_responses.md §item-tag headers), last-wins.
        let mut new_tags: Vec<crate::storage::tag_repo::NewTag<'_>> =
            Vec::with_capacity(tags.len());
        for tag in &tags {
            let key = tag
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| ServiceError::Unprocessable("item tag requires a key".to_owned()))?;
            // RM ITEM_TAG Inv_key_valid: "not key.is_empty and key.is_justified"
            // (no leading/trailing whitespace).
            if key.is_empty() || key.trim() != key {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag key {key:?} must be non-empty without leading/trailing whitespace"
                )));
            }
            let value = tag.get("value").and_then(Value::as_str);
            // RM ITEM_TAG Inv_value_valid: "value /= Void implies not value.is_empty".
            if value == Some("") {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag {key:?}: a value, if set, may not be empty"
                )));
            }
            let target_path = tag.get("target_path").and_then(Value::as_str);
            new_tags.push(crate::storage::tag_repo::NewTag {
                target_type,
                key,
                value,
                target_path,
            });
        }
        let mut tx = self.pool.begin().await?;
        crate::storage::tag_repo::replace_tags(
            &mut tx,
            Some(ehr_id),
            target_vo_id,
            target_version,
            &new_tags,
        )
        .await?;
        tx.commit().await?;
        self.target_tags(ehr_id, target_vo_id, target_version).await
    }

    /// The tag-target guard: the addressed versioned object must exist in
    /// THIS EHR ("Tag target values can only be within the same EHR" — RM ehr
    /// `ehr.adoc` `EHR.tags`), its stored kind must match the route family
    /// (a COMPOSITION route must not tag an `EHR_STATUS` container), and a
    /// VERSION-addressed target must name an existing version. The `item_tag`
    /// table is deliberately FK-less, so these checks live here.
    async fn ensure_tag_target(
        &self,
        ehr_id: EhrId,
        target_vo_id: VoId,
        target_version: Option<&str>,
        target_type: &str,
    ) -> Result<(), ServiceError> {
        let not_found = || {
            ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!(
                    "tag target {target_vo_id} does not exist in EHR {ehr_id} \
                     (tag targets can only be within the same EHR)"
                ),
            )
        };
        let Some((owner, kind)) =
            crate::storage::version_repo::meta::vo_owner_kind(&self.pool, target_vo_id).await?
        else {
            return Err(not_found());
        };
        if owner != Some(ehr_id) || kind != target_type {
            return Err(not_found());
        }
        if let Some(tail) = target_version {
            let tree = crate::versioning::object_version_id::parse_version_tail(tail)?;
            let (branch_number, branch_version) = tree.branch.unwrap_or((0, 0));
            if !crate::storage::version_repo::meta::version_exists(
                &self.pool,
                target_vo_id,
                tree.trunk,
                branch_number,
                branch_version,
            )
            .await?
            {
                return Err(ServiceError::sm(
                    CallStatusType::ObjectVersionDoesNotExist,
                    format!("tag target version {target_vo_id}::{tail}"),
                ));
            }
        }
        Ok(())
    }

    /// Delete a target collection's tags by key. The wire addresses tags by
    /// `key` alone while the `ITEM_TAG` identity is the (`key`, `target_path`)
    /// pair — so this is a SET delete: every tag under the key in the
    /// addressed collection goes (ITS-REST `Requests_and_responses.md`
    /// §item-tag headers; the Release-1.1.0 tag routes carry no path
    /// selector).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when no such tag exists on the target;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn delete_tag(
        &self,
        ehr_id: EhrId,
        target_vo_id: VoId,
        target_version: Option<&str>,
        key: &str,
    ) -> Result<(), ServiceError> {
        if !crate::storage::tag_repo::delete_tag(
            &self.pool,
            Some(ehr_id),
            target_vo_id,
            target_version,
            key,
        )
        .await?
        {
            return Err(ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("item tag {key:?}"),
            ));
        }
        Ok(())
    }

    /// One `ITEM_TAG` in its RM wire shape (`item_tag.adoc`): `key`, optional
    /// `value`/`target_path`, `target` as a bare `UID_BASED_ID` — a
    /// `HIER_OBJECT_ID` for a container target, an `OBJECT_VERSION_ID` for a
    /// VERSION target ("may be a `VERSIONED_OBJECT<T>` or a `VERSION<T>`") —
    /// and `owner_id` as the RM's `OBJECT_REF` to the owning EHR.
    ///
    /// NOTE: `_type` is emitted on the tag itself for canonical-JSON
    /// consistency (ITS-REST `Resources.md` §JSON Format neither requires nor
    /// forbids it on a concrete standalone type); the stalled OAS `ItemTag`
    /// schema disagrees with the RM on `target`'s shape — the RM is the
    /// RELEASED component and wins (owner ruling 2026-07-24).
    fn tag_json(ehr_id: EhrId, row: &crate::storage::tag_repo::TagRow) -> Value {
        let target_vo_id = row.target_vo_id;
        let target = match &row.target_version {
            Some(tail) => json!({
                "_type": "OBJECT_VERSION_ID",
                "value": format!("{target_vo_id}::{tail}")
            }),
            None => json!({
                "_type": "HIER_OBJECT_ID",
                "value": target_vo_id.to_string()
            }),
        };
        let mut tag = json!({
            "_type": "ITEM_TAG",
            "key": row.key.as_str(),
            "target": target,
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "EHR",
                "id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() }
            },
        });
        if let Some(value) = &row.value {
            tag["value"] = json!(value.as_str());
        }
        if let Some(path) = &row.target_path {
            tag["target_path"] = json!(path.as_str());
        }
        tag
    }
}

// ── The ITS-REST tags call surface ────────────────────────────────────────────

impl EhrbaseService {
    /// `GET /ehr/{ehr_id}/tags` — all tags in an EHR, optionally filtered.
    ///
    /// # Errors
    /// [`SmError`] if the tag listing fails.
    pub async fn ehr_tags_get(
        &self,
        an_ehr_id: EhrId,
        key: Option<String>,
        value: Option<String>,
        target_path: Option<String>,
    ) -> Result<Vec<Value>, SmError> {
        Ok(self
            .ehr_tags(
                an_ehr_id,
                key.as_deref(),
                value.as_deref(),
                target_path.as_deref(),
            )
            .await?)
    }

    /// `GET …/{uid_based_id}/tags` — the tags on one target object.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `uid_based_id` (400-equivalent) or a
    /// failing tag listing.
    pub async fn target_tags_get(
        &self,
        an_ehr_id: EhrId,
        uid_based_id: String,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, tail) = parse_tag_target(&uid_based_id)?;
        Ok(self.target_tags(an_ehr_id, vo_id, tail).await?)
    }

    /// `PUT …/{uid_based_id}/tags` — full-collection replace of a target's
    /// tags, returning the collection after the write.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `uid_based_id`, a missing EHR or target
    /// (404-equivalent), an invalid tag (422-equivalent), or a storage
    /// failure.
    pub async fn target_tags_replace(
        &self,
        an_ehr_id: EhrId,
        uid_based_id: String,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, tail) = parse_tag_target(&uid_based_id)?;
        Ok(self
            .replace_tags(an_ehr_id, vo_id, tail, target_type, tags)
            .await?)
    }

    /// `DELETE …/{uid_based_id}/tags/{key}` — delete one tag by key.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `uid_based_id`, a missing tag
    /// (404-equivalent), or a storage failure.
    pub async fn target_tag_delete(
        &self,
        an_ehr_id: EhrId,
        uid_based_id: String,
        key: String,
    ) -> Result<(), SmError> {
        let (vo_id, tail) = parse_tag_target(&uid_based_id)?;
        self.delete_tag(an_ehr_id, vo_id, tail, &key).await?;
        Ok(())
    }
}

/// Split a tag route's `uid_based_id` into the versioned-object id and — for a
/// VERSION-addressed target — the verbatim `creating_system_id::version_tree_id`
/// tail (RM `item_tag.adoc`: `target` "may be a `VERSIONED_OBJECT<T>` or a
/// `VERSION<T>`"). The tail is validated as a well-formed `OBJECT_VERSION_ID`
/// before it is kept verbatim.
fn parse_tag_target(uid_based_id: &str) -> Result<(VoId, Option<&str>), SmError> {
    let (vo_id, tree) = parse_uid_based_id(uid_based_id)?;
    if tree.is_none() {
        return Ok((vo_id, None));
    }
    let tail = uid_based_id
        .split_once("::")
        .map(|(_, tail)| tail)
        .filter(|tail| !tail.is_empty())
        .ok_or_else(|| {
            SmError::new(
                CallStatusType::PreconditionViolation,
                format!("malformed version-addressed tag target {uid_based_id:?}"),
            )
        })?;
    Ok((vo_id, Some(tail)))
}
