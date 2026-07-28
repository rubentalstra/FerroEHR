//! The demographic `ITEM_TAG` surface — the RM `common.item_tag` extension
//! applied to parties (ehr-less: `ehr_id IS NULL`). Our own extension: ITS-REST
//! 1.0.3 defines no demographic wire contract. The tag store is
//! backed by the `item_tag` table via `crate::storage::tag_repo` (storage owns
//! the SQL — no openEHR spec governs the storage, our own design); the RM
//! `ITEM_TAG` invariant checks (`Inv_key_valid`/`Inv_value_valid`) and the wire
//! shape stay in the domain here (RM `common.item_tag` governs both).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::ids::VoId;
use crate::service::EhrbaseService;
use crate::service::demographic::types::PartyKind;
use crate::service::error::ServiceError;
use crate::service::status::CallStatusType;
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
        let sid = self.effective_system_id();
        Ok(rows.iter().map(|r| party_tag_json(&sid, r)).collect())
    }

    /// The tags on one party target — the `VERSIONED_PARTY` container
    /// (`target_version: None`) or ONE of its VERSIONs (`Some(tail)`). The two
    /// are DISJOINT collections of the same `target_vo_id` (RM
    /// `common.item_tag` `ITEM_TAG.target`: "may be a `VERSIONED_OBJECT<T>` or
    /// a `VERSION<T>`"; the released dual-form `uid_based_id` on every
    /// demographic tag operation).
    pub(super) async fn party_tags(
        &self,
        vo_id: VoId,
        target_version: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = tag_repo::list_tags(
            &self.pool,
            None,
            Some((vo_id, target_version)),
            None,
            None,
            None,
        )
        .await?;
        let sid = self.effective_system_id();
        Ok(rows.iter().map(|r| party_tag_json(&sid, r)).collect())
    }

    /// The demographic tag-target guard, mirroring the EHR side
    /// (`service::ehr::tags::ensure_tag_target`): the addressed versioned
    /// object must exist AND be a party of the routed kind (a person route
    /// must not reach an agent's tags — the kind-checked-routes law), and a
    /// VERSION-addressed target must name an existing version. The released
    /// 404 trigger: `404_unknown_uid_based_id.yaml` — "returned when the
    /// `uid_based_id` does not exist".
    pub(super) async fn ensure_party_tag_target(
        &self,
        kind: PartyKind,
        vo_id: VoId,
        target_version: Option<&str>,
    ) -> Result<(), ServiceError> {
        let stored = crate::versioning::read::object_kind(&self.pool, vo_id).await?;
        if stored != Some(crate::service::demographic::support::kind_of(kind)) {
            return Err(ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("tag target {} {vo_id}", kind.rm_type()),
            ));
        }
        if let Some(tail) = target_version {
            let tree = crate::versioning::object_version_id::parse_version_tail(tail)?;
            let (branch_number, branch_version) = tree.branch.unwrap_or((0, 0));
            if !crate::storage::version_repo::meta::version_exists(
                &self.pool,
                vo_id,
                tree.trunk,
                branch_number,
                branch_version,
            )
            .await?
            {
                return Err(ServiceError::sm(
                    CallStatusType::ObjectVersionDoesNotExist,
                    format!("tag target version {vo_id}::{tail}"),
                ));
            }
        }
        Ok(())
    }

    /// Replace the whole tag collection of a party with the posted set (PUT
    /// full-collection semantics; an empty list clears all). Duplicates in the
    /// body are last-wins on the ITEM_TAG identity — the (key, target_path)
    /// PAIR (ITS-REST overview Requests_and_responses.md §openehr-item-tag and
    /// openehr-version-item-tag: "uniquely identified by their `key` and
    /// `target_path` pair attributes"; RM common master07-tags). The RM
    /// `ITEM_TAG` invariants are enforced before any write: `Inv_key_valid`
    /// (non-empty, no surrounding whitespace) and `Inv_value_valid`
    /// (`value /= Void implies not value.is_empty`).
    pub(super) async fn replace_party_tags(
        &self,
        kind: PartyKind,
        vo_id: VoId,
        target_version: Option<&str>,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_party_tag_target(kind, vo_id, target_version)
            .await?;
        // Validate + dedup (last wins) before touching the DB, keyed on the
        // ITEM_TAG identity — the (key, target_path) PAIR, never the key alone
        // (two same-key tags on different target_paths coexist; keying by tag
        // key collapsed them — the run-2 triage defect, 2026-07-28, mirroring
        // the EHR-side #369 identity fix). BTreeMap ordering matches the
        // `ORDER BY key` read-back order on the leading component.
        let mut deduped: BTreeMap<(String, Option<String>), Option<String>> = BTreeMap::new();
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
                (key.to_owned(), target_path.map(str::to_owned)),
                value.map(str::to_owned),
            );
        }

        // A NULL (demographic) scope never collides on the unique index (NULLs
        // are distinct), so the pre-dedup above is what enforces last-wins.
        let new_tags: Vec<tag_repo::NewTag<'_>> = deduped
            .iter()
            .map(|((key, target_path), value)| tag_repo::NewTag {
                target_type: kind.rm_type(),
                key: key.as_str(),
                value: value.as_deref(),
                target_path: target_path.as_deref(),
            })
            .collect();
        let mut tx = self.pool.begin().await?;
        tag_repo::replace_tags(&mut tx, None, vo_id, target_version, &new_tags).await?;
        tx.commit().await?;
        self.party_tags(vo_id, target_version).await
    }

    /// Delete a target's tags by key (a SET delete over the `(key,
    /// target_path)` identities sharing the key, addressed to the container or
    /// ONE VERSION per the parsed `uid_based_id`). The target itself is gated
    /// first (`404_unknown_uid_based_id_or_key.yaml`: "the `uid_based_id` does
    /// not exist, or … the `ITEM_TAG` identified by the `key` does not exist");
    /// an unknown key on an existing target is the same `404`.
    pub(super) async fn delete_party_tag(
        &self,
        kind: PartyKind,
        vo_id: VoId,
        target_version: Option<&str>,
        key: &str,
    ) -> Result<(), ServiceError> {
        self.ensure_party_tag_target(kind, vo_id, target_version)
            .await?;
        if !tag_repo::delete_tag(&self.pool, None, vo_id, target_version, key).await? {
            return Err(ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("item tag {key:?}"),
            ));
        }
        Ok(())
    }
}

/// One demographic `ITEM_TAG` in its RM wire shape (`item_tag.adoc`): `key`,
/// optional `value`/`target_path`, `target` as a bare `UID_BASED_ID` — a
/// `HIER_OBJECT_ID` for a container target, an `OBJECT_VERSION_ID` for a
/// VERSION target ("may be a `VERSIONED_OBJECT<T>` or a `VERSION<T>`") —
/// exactly the EHR sibling's shape (the settled RM-target law; the stalled OAS
/// `ItemTag` schema's `OBJECT_REF` wrapper loses to the released RM).
///
/// NOTE: `owner_id` follows the released examples' shape — an `OBJECT_REF`
/// `{namespace: local, type: SYSTEM}` whose `id` carries the server's
/// configured system identifier (every
/// `schemas/demographic/ItemTagOf<T>.yaml` example; no demographic class
/// declares a `tags` containment, so the EHR side's `EHR.tags` anchor has no
/// analogue here — the register carries the fixed handling).
fn party_tag_json(system_id: &str, row: &tag_repo::TagRow) -> Value {
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
            "type": "SYSTEM",
            "id": { "_type": "HIER_OBJECT_ID", "value": system_id }
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
