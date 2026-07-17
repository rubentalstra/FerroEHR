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
    pub target_vo_id: VoId,
    pub target_type: String,
    pub key: String,
    pub value: Option<String>,
    pub target_path: Option<String>,
}

/// One tag to write (`replace_tags`).
#[derive(Debug, Clone)]
pub struct NewTag<'a> {
    pub target_type: &'a str,
    pub key: &'a str,
    pub value: Option<&'a str>,
    pub target_path: Option<&'a str>,
}

fn tag_row(row: &sqlx::postgres::PgRow) -> Result<TagRow, StorageError> {
    Ok(TagRow {
        target_vo_id: row.try_get("target_vo_id")?,
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
    target_vo_id: Option<VoId>,
    key: Option<&str>,
    value: Option<&str>,
    target_path: Option<&str>,
) -> Result<Vec<TagRow>, StorageError> {
    let rows = sqlx::query(
        "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
         WHERE ehr_id IS NOT DISTINCT FROM $1 \
         AND ($2::uuid IS NULL OR target_vo_id = $2) \
         AND ($3::text IS NULL OR key = $3) \
         AND ($4::text IS NULL OR value = $4) \
         AND ($5::text IS NULL OR target_path = $5) \
         ORDER BY key",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
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
    tags: &[NewTag<'_>],
) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM item_tag WHERE ehr_id IS NOT DISTINCT FROM $1 AND target_vo_id = $2")
        .bind(ehr_scope)
        .bind(target_vo_id)
        .execute(&mut *tx)
        .await?;
    if tags.is_empty() {
        return Ok(());
    }
    // One multi-row insert for the whole set (never a per-tag round trip).
    // Repeated keys keep the loop era's last-wins semantics via an in-memory
    // dedupe — a single INSERT cannot upsert over its own rows ("ON CONFLICT
    // DO UPDATE command cannot affect row a second time").
    let mut last_by_key: indexmap::IndexMap<&str, &NewTag<'_>> = indexmap::IndexMap::new();
    for tag in tags {
        last_by_key.insert(tag.key, tag);
    }
    let (mut types, mut keys, mut values, mut paths) = (
        Vec::with_capacity(last_by_key.len()),
        Vec::with_capacity(last_by_key.len()),
        Vec::with_capacity(last_by_key.len()),
        Vec::with_capacity(last_by_key.len()),
    );
    for tag in last_by_key.values() {
        types.push(tag.target_type);
        keys.push(tag.key);
        values.push(tag.value);
        paths.push(tag.target_path);
    }
    sqlx::query(
        "INSERT INTO item_tag (ehr_id, target_vo_id, target_type, key, value, target_path) \
         SELECT $1, $2, t.* FROM UNNEST($3::text[], $4::text[], $5::text[], $6::text[]) AS t",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(&types)
    .bind(&keys)
    .bind(&values)
    .bind(&paths)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Delete one tag by key from a target in one scope, returning whether a row
/// was deleted.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn delete_tag(
    pool: &PgPool,
    ehr_scope: Option<EhrId>,
    target_vo_id: VoId,
    key: &str,
) -> Result<bool, StorageError> {
    let deleted = sqlx::query(
        "DELETE FROM item_tag WHERE ehr_id IS NOT DISTINCT FROM $1 \
         AND target_vo_id = $2 AND key = $3",
    )
    .bind(ehr_scope)
    .bind(target_vo_id)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(deleted.rows_affected() > 0)
}
