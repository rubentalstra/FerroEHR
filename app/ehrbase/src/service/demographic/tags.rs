//! The demographic `ITEM_TAG` surface — the RM `common.item_tag` extension
//! applied to parties (ehr-less: `ehr_id IS NULL`). Our own extension: ITS-REST
//! 1.0.3 defines no demographic wire contract (register
//! `docs/design/platform/04-service-demographic-ehr-index.md`). The tag store is
//! backed by the `item_tag` table via `crate::storage::tag_repo` (storage owns
//! the SQL — no openEHR spec governs the storage, our own design); the RM
//! `ITEM_TAG` invariant checks (`Inv_key_valid`/`Inv_value_valid`) and the wire
//! shape stay in the domain here (RM `common.item_tag` governs both).

use std::collections::BTreeMap;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::demographic::types::PartyKind;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::storage::tag_repo;

impl EhrbaseService {
    /// All demographic tags (ehr-less), optionally filtered by key/value/path.
    pub(super) async fn demographic_tags(
        &self,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = tag_repo::list_tags(&self.pool, None, None, key, value, target_path).await?;
        Ok(rows.iter().map(party_tag_json).collect())
    }

    /// The tags on one party.
    pub(super) async fn party_tags(&self, vo_id: Uuid) -> Result<Vec<Value>, ServiceError> {
        let rows = tag_repo::list_tags(&self.pool, None, Some(vo_id), None, None, None).await?;
        Ok(rows.iter().map(party_tag_json).collect())
    }

    /// Replace the whole tag collection of a party with the posted set (PUT
    /// full-collection semantics; an empty list clears all). Duplicate keys in
    /// the body are last-wins. The RM `ITEM_TAG` invariants are enforced before
    /// any write: `Inv_key_valid` (non-empty, no surrounding whitespace) and
    /// `Inv_value_valid` (`value /= Void implies not value.is_empty`).
    pub(super) async fn replace_party_tags(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_party(kind, vo_id).await?;
        // Validate + dedup (last wins) before touching the DB. A BTreeMap keys by
        // tag key, matching the `ORDER BY key` read-back order.
        let mut deduped: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
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

        // A NULL (demographic) scope never collides on the unique index (NULLs
        // are distinct), so the pre-dedup above is what enforces last-wins.
        let new_tags: Vec<tag_repo::NewTag<'_>> = deduped
            .iter()
            .map(|(key, (value, target_path))| tag_repo::NewTag {
                target_type: kind.rm_type(),
                key: key.as_str(),
                value: value.as_deref(),
                target_path: target_path.as_deref(),
            })
            .collect();
        let mut tx = self.pool.begin().await?;
        tag_repo::replace_tags(&mut tx, None, vo_id, &new_tags).await?;
        tx.commit().await?;
        self.party_tags(vo_id).await
    }

    /// Delete a tag by key from a party. An unknown key is `404`.
    pub(super) async fn delete_party_tag(
        &self,
        vo_id: Uuid,
        key: &str,
    ) -> Result<(), ServiceError> {
        if !tag_repo::delete_tag(&self.pool, None, vo_id, key).await? {
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
fn party_tag_json(row: &tag_repo::TagRow) -> Value {
    let target_vo_id = row.target_vo_id;
    let target_type = row.target_type.as_str();
    let mut tag = json!({
        "_type": "ITEM_TAG",
        "key": row.key.as_str(),
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
    if let Some(value) = &row.value {
        tag["value"] = json!(value.as_str());
    }
    if let Some(path) = &row.target_path {
        tag["target_path"] = json!(path.as_str());
    }
    tag
}
