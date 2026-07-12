//! The demographic ITEM_TAG surface — the RM `common.item_tag` extension
//! applied to parties (ehr-less: `ehr_id IS NULL`). Our own extension: ITS-REST
//! 1.0.3 defines no demographic wire contract (register
//! `docs/design/platform/04-service-demographic-ehr-index.md`). The tag store is
//! direct SQL over the `item_tag` table (no openEHR spec governs the storage —
//! our own design; RM `common.item_tag` governs the wire shape + invariants).
//!
//! TODO(w3f-integrate): the `item_tag` reads/writes should move behind a
//! storage-owned repository (README cross-register ruling — storage owns the
//! SQL); the domain here would keep the RM `ITEM_TAG` invariant checks.

use ehrbase_sm::PartyKind;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// All demographic tags (ehr-less), optionally filtered by key/value/path.
    pub(crate) async fn demographic_tags(
        &self,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id IS NULL \
             AND ($1::text IS NULL OR key = $1) \
             AND ($2::text IS NULL OR value = $2) \
             AND ($3::text IS NULL OR target_path = $3) \
             ORDER BY key",
        )
        .bind(key)
        .bind(value)
        .bind(target_path)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(party_tag_json).collect())
    }

    /// The tags on one party.
    pub(crate) async fn party_tags(&self, vo_id: Uuid) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id IS NULL AND target_vo_id = $1 ORDER BY key",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(party_tag_json).collect())
    }

    /// Replace the whole tag collection of a party with the posted set (PUT
    /// full-collection semantics; an empty list clears all). Duplicate keys in
    /// the body are last-wins.
    pub(crate) async fn replace_party_tags(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_party(kind, vo_id).await?;
        // Validate + dedup (last wins) before touching the DB. A BTreeMap keys by
        // tag key, matching the `ORDER BY key` read-back order.
        let mut deduped: std::collections::BTreeMap<String, (Option<String>, Option<String>)> =
            std::collections::BTreeMap::new();
        for tag in &tags {
            let key = tag
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| ServiceError::Unprocessable("item tag requires a key".to_owned()))?;
            // RM ITEM_TAG Inv_key_valid: non-empty, no leading/trailing whitespace.
            if key.is_empty() || key.trim() != key {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag key {key:?} must be non-empty without leading/trailing whitespace"
                )));
            }
            let value = tag.get("value").and_then(Value::as_str);
            // RM ITEM_TAG Inv_value_valid: `value /= Void implies not value.is_empty`.
            if value == Some("") {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag {key:?}: a value, if set, may not be empty"
                )));
            }
            let target_path = tag.get("target_path").and_then(Value::as_str);
            deduped.insert(
                key.to_owned(),
                (value.map(str::to_owned), target_path.map(str::to_owned)),
            );
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM item_tag WHERE ehr_id IS NULL AND target_vo_id = $1")
            .bind(vo_id)
            .execute(&mut *tx)
            .await?;
        for (key, (value, target_path)) in &deduped {
            sqlx::query(
                "INSERT INTO item_tag (ehr_id, target_vo_id, target_type, key, value, target_path) \
                 VALUES (NULL, $1, $2, $3, $4, $5)",
            )
            .bind(vo_id)
            .bind(kind.rm_type())
            .bind(key)
            .bind(value)
            .bind(target_path)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.party_tags(vo_id).await
    }

    /// Delete a tag by key from a party.
    pub(crate) async fn delete_party_tag(
        &self,
        vo_id: Uuid,
        key: &str,
    ) -> Result<(), ServiceError> {
        let deleted = sqlx::query(
            "DELETE FROM item_tag WHERE ehr_id IS NULL AND target_vo_id = $1 AND key = $2",
        )
        .bind(vo_id)
        .bind(key)
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("item tag {key:?}")));
        }
        Ok(())
    }
}

/// One demographic `ITEM_TAG` in its wire shape (RM `common.item_tag`).
///
/// PORT NOTE (G-6): `owner_id` references the tagged party itself — there is no
/// owning EHR for a demographic tag (no openEHR spec governs the owner of an
/// ehr-less demographic tag — our own design).
fn party_tag_json(row: &sqlx::postgres::PgRow) -> Value {
    let target_vo_id: Uuid = row.try_get("target_vo_id").unwrap_or_default();
    let target_type: String = row.try_get("target_type").unwrap_or_default();
    let mut tag = json!({
        "_type": "ITEM_TAG",
        "key": row.try_get::<String, _>("key").unwrap_or_default(),
        "target": {
            "_type": "OBJECT_REF",
            "namespace": "demographic",
            "type": target_type,
            "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
        },
        "owner_id": {
            "_type": "OBJECT_REF",
            "namespace": "demographic",
            "type": target_type,
            "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
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
