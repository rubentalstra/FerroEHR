//! `ITEM_TAG` CRUD — an ITS-REST **experimental** extension (the tags API is
//! development-branch only), on the `item_tag` table.
//!
//! Spec: RM `ITEM_TAG`
//! (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`
//! — the invariants `Inv_key_valid` / `Inv_value_valid`; RM ehr `ehr.adoc`
//! `EHR.tags`: "Tag target values can only be within the same EHR") and the
//! development-branch OAS `ItemTag` schema (`key`/`value`/`target_path`/
//! `target`/`owner_id`, `additionalProperties: false`). `PUT …/tags` "updates
//! the list of **all** `ITEM_TAG` resources associated with a given target …
//! an empty list will effectively remove all" — a full-collection replace.
//! Not an SM-EHR interface. The `item_tag` table SQL is spec-silent (G-10
//! storage seam — our own design).

use crate::ids::{EhrId, VoId};
use crate::service::status::SmError;
use serde_json::{Value, json};
use uuid::Uuid;

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
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = crate::storage::tag_repo::list_tags(
            &self.pool,
            Some(ehr_id),
            Some(target_vo_id),
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
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        // "Tag target values can only be within the same EHR" (RM ehr
        // `ehr.adoc` EHR.tags): the target versioned object must exist AND
        // belong to this EHR — the item_tag table is deliberately FK-less (a
        // tag may address a specific VERSION), so the ownership check lives
        // here.
        let owner = crate::storage::version_repo::meta::vo_owner(&self.pool, target_vo_id).await?;
        if owner != Some(Some(ehr_id)) {
            return Err(ServiceError::NotFound(format!(
                "tag target {target_vo_id} does not exist in EHR {ehr_id} \
                 (tag targets can only be within the same EHR)"
            )));
        }
        // Validate every tag before writing; the `replace_tags` upsert arm
        // covers same-key repetition (last-wins) in the EHR scope.
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
        crate::storage::tag_repo::replace_tags(&mut tx, Some(ehr_id), target_vo_id, &new_tags)
            .await?;
        tx.commit().await?;
        self.target_tags(ehr_id, target_vo_id).await
    }

    /// Delete a tag by key from a target object.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when no such tag exists on the target;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn delete_tag(
        &self,
        ehr_id: EhrId,
        target_vo_id: VoId,
        key: &str,
    ) -> Result<(), ServiceError> {
        if !crate::storage::tag_repo::delete_tag(&self.pool, Some(ehr_id), target_vo_id, key)
            .await?
        {
            return Err(ServiceError::NotFound(format!("item tag {key:?}")));
        }
        Ok(())
    }

    /// One `ITEM_TAG` in its wire shape: exactly the OAS `ItemTag`
    /// properties — `key`, optional `value`/`target_path`, OBJECT_REF-shaped
    /// `target` (the tagged versioned object, its RM type in `type`) and
    /// `owner_id` (the owning EHR). No extra fields — the schema is
    /// `additionalProperties: false` (`_type` is its discriminator).
    fn tag_json(ehr_id: EhrId, row: &crate::storage::tag_repo::TagRow) -> Value {
        let target_vo_id = row.target_vo_id;
        let mut tag = json!({
            "_type": "ITEM_TAG",
            "key": row.key.as_str(),
            "target": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": row.target_type.as_str(),
                "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
            },
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
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self.target_tags(an_ehr_id, vo_id).await?)
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
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self
            .replace_tags(an_ehr_id, vo_id, target_type, tags)
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
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        self.delete_tag(an_ehr_id, vo_id, &key).await?;
        Ok(())
    }
}
