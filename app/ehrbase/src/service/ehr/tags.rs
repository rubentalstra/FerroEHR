//! ITEM_TAG CRUD — an ITS-REST **experimental** extension (the tags API is
//! development-branch only), on the `item_tag` table.
//!
//! Spec: RM `ITEM_TAG`
//! (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`
//! — the invariants `Inv_key_valid` / `Inv_value_valid`; RM ehr `ehr.adoc`
//! `EHR.tags`: "Tag target values can only be within the same EHR") and the
//! development-branch OAS `ItemTag` schema (`key`/`value`/`target_path`/
//! `target`/`owner_id`, `additionalProperties: false`). `PUT …/tags` "updates
//! the list of **all** ITEM_TAG resources associated with a given target … an
//! empty list will effectively remove all" — a full-collection replace. Not an
//! SM-EHR interface. The `item_tag` table SQL is spec-silent (G-10 storage
//! seam — our own design).

use ehrbase_sm::{ItemTagAdapter, SmError};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::parse_uid_based_id;

impl EhrbaseService {
    /// All tags in an EHR, optionally filtered by key / value / target path.
    ///
    /// TODO(w3f-integrate): storage seam (G-10) — the `item_tag` reads/writes in
    /// this file.
    pub(in crate::service) async fn ehr_tags(
        &self,
        ehr_id: Uuid,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id = $1 \
             AND ($2::text IS NULL OR key = $2) \
             AND ($3::text IS NULL OR value = $3) \
             AND ($4::text IS NULL OR target_path = $4) \
             ORDER BY key",
        )
        .bind(ehr_id)
        .bind(key)
        .bind(value)
        .bind(target_path)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|r| Self::tag_json(ehr_id, r)).collect())
    }

    /// Tags on one target object (a COMPOSITION or `EHR_STATUS`).
    pub(in crate::service) async fn target_tags(
        &self,
        ehr_id: Uuid,
        target_vo_id: Uuid,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id = $1 AND target_vo_id = $2 ORDER BY key",
        )
        .bind(ehr_id)
        .bind(target_vo_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|r| Self::tag_json(ehr_id, r)).collect())
    }

    /// Replace the **whole** tag collection of a target with the posted set,
    /// returning the target's tags after — `PUT` full-collection semantics
    /// (F-03-05): tags omitted from the body are removed, and an empty list
    /// clears all tags on the target.
    pub(in crate::service) async fn replace_tags(
        &self,
        ehr_id: Uuid,
        target_vo_id: Uuid,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        // "Tag target values can only be within the same EHR" (RM ehr `ehr.adoc`
        // EHR.tags): the target versioned object must exist AND belong to this
        // EHR — the item_tag table is deliberately FK-less (a tag may address a
        // specific VERSION), so the ownership check lives here.
        let owner: Option<Uuid> =
            sqlx::query_scalar("SELECT ehr_id FROM vo_version WHERE vo_id = $1 LIMIT 1")
                .bind(target_vo_id)
                .fetch_optional(&self.pool)
                .await?
                .flatten();
        if owner != Some(ehr_id) {
            return Err(ServiceError::NotFound(format!(
                "tag target {target_vo_id} does not exist in EHR {ehr_id} \
                 (tag targets can only be within the same EHR)"
            )));
        }
        let mut tx = self.pool.begin().await?;
        // Full replace: drop the existing collection, then insert the posted set.
        sqlx::query("DELETE FROM item_tag WHERE ehr_id = $1 AND target_vo_id = $2")
            .bind(ehr_id)
            .bind(target_vo_id)
            .execute(&mut *tx)
            .await?;
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
            sqlx::query(
                "INSERT INTO item_tag (ehr_id, target_vo_id, target_type, key, value, target_path) \
                 VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT (ehr_id, target_vo_id, key) \
                 DO UPDATE SET value = EXCLUDED.value, target_path = EXCLUDED.target_path",
            )
            .bind(ehr_id)
            .bind(target_vo_id)
            .bind(target_type)
            .bind(key)
            .bind(value)
            .bind(target_path)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.target_tags(ehr_id, target_vo_id).await
    }

    /// Delete a tag by key from a target object.
    pub(in crate::service) async fn delete_tag(
        &self,
        ehr_id: Uuid,
        target_vo_id: Uuid,
        key: &str,
    ) -> Result<(), ServiceError> {
        let deleted = sqlx::query(
            "DELETE FROM item_tag WHERE ehr_id = $1 AND target_vo_id = $2 AND key = $3",
        )
        .bind(ehr_id)
        .bind(target_vo_id)
        .bind(key)
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("item tag {key:?}")));
        }
        Ok(())
    }

    /// One `ITEM_TAG` in its wire shape (F-03-06): exactly the OAS `ItemTag`
    /// properties — `key`, optional `value`/`target_path`, OBJECT_REF-shaped
    /// `target` (the tagged versioned object, its RM type in `type`) and
    /// `owner_id` (the owning EHR). No extra fields — the schema is
    /// `additionalProperties: false` (`_type` is its discriminator).
    fn tag_json(ehr_id: Uuid, row: &sqlx::postgres::PgRow) -> Value {
        let target_vo_id: Uuid = row.try_get("target_vo_id").unwrap_or_default();
        let target_type: String = row.try_get("target_type").unwrap_or_default();
        let mut tag = json!({
            "_type": "ITEM_TAG",
            "key": row.try_get::<String, _>("key").unwrap_or_default(),
            "target": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": target_type,
                "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
            },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "EHR",
                "id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() }
            },
        });
        if let Ok(Some(value)) = row.try_get::<Option<String>, _>("value") {
            tag["value"] = json!(value);
        }
        if let Ok(Some(path)) = row.try_get::<Option<String>, _>("target_path") {
            tag["target_path"] = json!(path);
        }
        tag
    }
}

#[async_trait::async_trait]
impl ItemTagAdapter for EhrbaseService {
    async fn ehr_tags_get(
        &self,
        an_ehr_id: Uuid,
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

    async fn target_tags_get(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self.target_tags(an_ehr_id, vo_id).await?)
    }

    async fn target_tags_replace(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self
            .replace_tags(an_ehr_id, vo_id, target_type, tags)
            .await?)
    }

    async fn target_tag_delete(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        key: String,
    ) -> Result<(), SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        self.delete_tag(an_ehr_id, vo_id, &key).await?;
        Ok(())
    }
}
