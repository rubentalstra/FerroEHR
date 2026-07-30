//! Row I/O for `item_tag` — the `ITEM_TAG` store shared by the EHR-scoped tag
//! surface (`ehr_id = <uuid>`) and the demographic (ehr-less,
//! `ehr_id IS NULL`) one.
//!
//! No openEHR spec governs the SQL (our own design). The RM `ITEM_TAG`
//! invariants (`Inv_key_valid`, `Inv_value_valid`) are enforced by the service
//! chapters before rows reach this module; the table is deliberately FK-less
//! (a tag may address a specific VERSION), so target-ownership checks also
//! live with the callers.

use sqlx::{PgConnection, PgPool, Row};

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;

/// One `item_tag` row.
#[derive(Debug, Clone)]
pub struct TagRow {
    /// The tagged versioned object.
    pub target_vo_id: VoId,
    /// The `{creating_system_id}::{version_tree_id}` tail of a
    /// VERSION-addressed target; `None` = the container.
    pub target_version: Option<String>,
    /// The RM type of the tagged object.
    pub target_type: String,
    /// The tag key.
    pub key: String,
    /// The tag value, absent when the tag is a bare marker.
    pub value: Option<String>,
    /// The path within the target the tag applies to, if it is not whole-object.
    pub target_path: Option<String>,
}

/// One tag to write (`replace_tags`).
#[derive(Debug, Clone)]
pub struct NewTag<'a> {
    /// The RM type of the tagged object.
    pub target_type: &'a str,
    /// The tag key.
    pub key: &'a str,
    /// The tag value, or `None` for a bare marker.
    pub value: Option<&'a str>,
    /// The path within the target the tag applies to, if it is not whole-object.
    pub target_path: Option<&'a str>,
}

fn tag_row(row: &sqlx::postgres::PgRow) -> Result<TagRow, StorageError> {
    Ok(TagRow {
        target_vo_id: row.try_get("target_vo_id")?,
        target_version: row.try_get("target_version")?,
        target_type: row.try_get("target_type")?,
        key: row.try_get("key")?,
        value: row.try_get("value")?,
        target_path: row.try_get("target_path")?,
    })
}

/// List tags in one scope (`ehr_id = $1`, `NULL` = the demographic store),
/// optionally narrowed to one target object and/or filtered by key / value /
/// target path, ordered by key.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn list_tags(
    pool: &PgPool,
    ehr_scope: Option<EhrId>,
    target: Option<(VoId, Option<&str>)>,
    key: Option<&str>,
    value: Option<&str>,
    target_path: Option<&str>,
) -> Result<Vec<TagRow>, StorageError> {
    // A named target narrows to ONE collection: the container's (`NULL`
    // target_version) or one VERSION's — the two are disjoint sets of the
    // same target_vo_id (RM ITEM_TAG.target: container or VERSION).
    let (target_vo_id, target_version) = match target {
        Some((vo, version)) => (Some(vo), version),
        None => (None, None),
    };
    let rows = sqlx::query(
        "SELECT target_vo_id, target_version, target_type, key, value, target_path \
         FROM item_tag \
         WHERE ehr_id IS NOT DISTINCT FROM $1 \
         AND ($2::uuid IS NULL OR (target_vo_id = $2 \
              AND target_version IS NOT DISTINCT FROM $3)) \
         AND ($4::text IS NULL OR key = $4) \
         AND ($5::text IS NULL OR value = $5) \
         AND ($6::text IS NULL OR target_path = $6) \
         ORDER BY key, target_path NULLS FIRST",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(target_version)
    .bind(key)
    .bind(value)
    .bind(target_path)
    .fetch_all(pool)
    .await?;
    rows.iter().map(tag_row).collect()
}

/// Replace the whole tag collection of one target in one scope: drop the
/// existing collection, insert the given set (`PUT` full-collection
/// semantics; an empty set clears all). Runs on the caller's transaction.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn replace_tags(
    tx: &mut PgConnection,
    ehr_scope: Option<EhrId>,
    target_vo_id: VoId,
    target_version: Option<&str>,
    tags: &[NewTag<'_>],
) -> Result<(), StorageError> {
    // Replace exactly ONE collection: the container's or one VERSION's —
    // never both (RM ITEM_TAG.target: container or VERSION are distinct
    // targets).
    sqlx::query(
        "DELETE FROM item_tag WHERE ehr_id IS NOT DISTINCT FROM $1 \
         AND target_vo_id = $2 AND target_version IS NOT DISTINCT FROM $3",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(target_version)
    .execute(&mut *tx)
    .await?;
    if tags.is_empty() {
        return Ok(());
    }
    // One multi-row insert for the whole set (never a per-tag round trip).
    // The ITEM_TAG identity within one target is the (key, target_path) PAIR
    // (ITS-REST overview Requests_and_responses.md §item-tag headers), so the
    // in-set last-wins dedupe keys on the pair — same-key tags on different
    // paths coexist. A single INSERT cannot upsert over its own rows ("ON
    // CONFLICT DO UPDATE command cannot affect row a second time").
    let mut last_by_identity: indexmap::IndexMap<(&str, Option<&str>), &NewTag<'_>> =
        indexmap::IndexMap::new();
    for tag in tags {
        last_by_identity.insert((tag.key, tag.target_path), tag);
    }
    let (mut types, mut keys, mut values, mut paths) = (
        Vec::with_capacity(last_by_identity.len()),
        Vec::with_capacity(last_by_identity.len()),
        Vec::with_capacity(last_by_identity.len()),
        Vec::with_capacity(last_by_identity.len()),
    );
    for tag in last_by_identity.values() {
        types.push(tag.target_type);
        keys.push(tag.key);
        values.push(tag.value);
        paths.push(tag.target_path);
    }
    sqlx::query(
        "INSERT INTO item_tag \
         (ehr_id, target_vo_id, target_version, target_type, key, value, target_path) \
         SELECT $1, $2, $3, t.* FROM UNNEST($4::text[], $5::text[], $6::text[], $7::text[]) AS t",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(target_version)
    .bind(&types)
    .bind(&keys)
    .bind(&values)
    .bind(&paths)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Delete a target collection's tags by key, returning whether any row was
/// deleted. The wire addresses tags by `key` alone (the Release-1.1.0 tag
/// routes carry no path selector), while the `ITEM_TAG` identity is the
/// (`key`, `target_path`) pair — so a key delete removes EVERY tag under that key
/// in the addressed collection, a set delete.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn delete_tag(
    pool: &PgPool,
    ehr_scope: Option<EhrId>,
    target_vo_id: VoId,
    target_version: Option<&str>,
    key: &str,
) -> Result<bool, StorageError> {
    let deleted = sqlx::query(
        "DELETE FROM item_tag WHERE ehr_id IS NOT DISTINCT FROM $1 \
         AND target_vo_id = $2 AND target_version IS NOT DISTINCT FROM $3 AND key = $4",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(target_version)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(deleted.rows_affected() > 0)
}
