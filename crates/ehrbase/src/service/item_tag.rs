//! Item Tag CRUD (ITS-REST experimental tags API), on the `item_tag` table.
//! Tags annotate a COMPOSITION or `EHR_STATUS` within an EHR with a `key`,
//! optional `value`, and optional `target_path`.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// All tags in an EHR, optionally filtered by key / value / target path.
    pub(super) async fn ehr_tags(
        &self,
        ehr_id: Uuid,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, target_vo_id, target_type, key, value, target_path FROM item_tag \
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
    pub(super) async fn target_tags(
        &self,
        ehr_id: Uuid,
        target_vo_id: Uuid,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id = $1 AND target_vo_id = $2 ORDER BY key",
        )
        .bind(ehr_id)
        .bind(target_vo_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|r| Self::tag_json(ehr_id, r)).collect())
    }

    /// Upsert a batch of tags on a target, returning the target's tags after.
    pub(super) async fn upsert_tags(
        &self,
        ehr_id: Uuid,
        target_vo_id: Uuid,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        let mut tx = self.pool.begin().await?;
        for tag in &tags {
            let key = tag
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| ServiceError::Unprocessable("item tag requires a key".to_owned()))?;
            let value = tag.get("value").and_then(Value::as_str);
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
    pub(super) async fn delete_tag(
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

    fn tag_json(ehr_id: Uuid, row: &sqlx::postgres::PgRow) -> Value {
        let id: Uuid = row.try_get("id").unwrap_or_default();
        let target_vo_id: Uuid = row.try_get("target_vo_id").unwrap_or_default();
        let target_type: String = row.try_get("target_type").unwrap_or_default();
        let mut tag = json!({
            "_type": "ITEM_TAG",
            "id": id.to_string(),
            "owner_id": ehr_id.to_string(),
            "target": target_vo_id.to_string(),
            "target_type": target_type,
            "key": row.try_get::<String, _>("key").unwrap_or_default(),
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
