//! `node`-table row I/O: bulk-write a decomposed version, and read one version
//! back as canonical JSON.
//!
//! No openEHR spec governs the `node` table — it is our own decomposed store
//! (`docs/architecture.md` §Storage). This module is the single home for the
//! `node` write and the `node`→canonical reload: the former lived in the
//! service layer (G-S2) and the latter was duplicated between the version read
//! path and the dump/load export (G-S1) — both now funnel here.

use serde_json::Value;
use sqlx::{PgConnection, PgPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::storage::row::{NodeRow, ReadRow};
use crate::storage::{StorageError, reassemble};

/// Bulk-insert the decomposed node rows of one stored version. `rows` is the
/// output of [`crate::storage::decompose`]; the storage context
/// (`vo_id`/`sys_version`/`ehr_id`) is supplied here and written onto every row.
///
/// A logically-deleted version (data Void, RM common master06 §Logical
/// Deletion) has no node rows — the caller simply passes an empty slice and no
/// statement runs.
///
/// # Errors
///
/// Returns [`StorageError::Database`] on any driver/insert failure.
// The versioning commit path calls `decompose` + `reassemble` (for the signed
// served form) itself, then hands the rows here — deliberately, so the signed
// bytes and the stored rows come from the one transform. This function stays a
// pure row-writer over pre-decomposed rows.
pub async fn write_nodes(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    ehr_id: Option<Uuid>,
    rows: &[NodeRow],
) -> Result<(), StorageError> {
    if rows.is_empty() {
        return Ok(());
    }
    // Base content columns + the promoted-leaf columns, generated from the
    // shared registry so adding a promoted column is a migration + one registry
    // entry, never an edit here (our own storage design — no openEHR spec
    // governs promoted columns; see `crate::storage::promoted`).
    let mut header = String::from(
        "INSERT INTO node (vo_id, sys_version, num, num_cap, parent_num, citem_num, ehr_id, \
         rm_type, archetype, arch_entity, arch_concept, arch_major, name, path, data",
    );
    for leaf in crate::storage::PROMOTED_LEAVES {
        header.push_str(", ");
        header.push_str(leaf.column);
    }
    header.push_str(") ");
    let mut qb = QueryBuilder::new(header);
    qb.push_values(rows, |mut b, row| {
        b.push_bind(vo_id)
            .push_bind(sys_version)
            .push_bind(row.num)
            .push_bind(row.num_cap)
            .push_bind(row.parent_num)
            .push_bind(row.citem_num)
            .push_bind(ehr_id)
            .push_bind(&row.rm_type)
            .push_bind(&row.archetype)
            .push_bind(&row.arch_entity)
            .push_bind(&row.arch_concept)
            .push_bind(row.arch_major)
            .push_bind(&row.name)
            .push_bind(&row.path)
            .push_bind(&row.data);
        // Each promoted value is bound through its kind's conversion so a
        // value the AQL query-time cast accepted yields the same stored value,
        // and non-castable text becomes NULL rather than failing the write
        // (ext.openehr_timestamp, ext baseline).
        for (i, leaf) in crate::storage::PROMOTED_LEAVES.iter().enumerate() {
            let raw = row.promoted.get(i).and_then(Clone::clone);
            match leaf.kind {
                crate::storage::PromotedKind::Timestamp => {
                    b.push("ext.openehr_timestamp(")
                        .push_bind_unseparated(raw)
                        .push_unseparated(")");
                }
            }
        }
    });
    qb.build().execute(&mut *tx).await?;
    Ok(())
}

/// Fetch the lean read rows of one stored version, ordered by `num`. Selects
/// **only** the five columns [`crate::storage::reassemble`] and the nested-set
/// contract need (G-S6) — the promoted query columns are not read back.
async fn read_rows(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<Vec<ReadRow>, StorageError> {
    let rows = sqlx::query(
        "SELECT num, num_cap, parent_num, path, data \
         FROM node WHERE vo_id = $1 AND sys_version = $2 ORDER BY num",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_all(pool)
    .await?;

    let mut read = Vec::with_capacity(rows.len());
    for row in rows {
        read.push(ReadRow {
            num: row.try_get("num")?,
            num_cap: row.try_get("num_cap")?,
            parent_num: row.try_get("parent_num")?,
            path: row.try_get("path")?,
            data: row.try_get("data")?,
        });
    }
    Ok(read)
}

/// Reassemble one stored version's canonical JSON from its `node` rows — the
/// single consolidated node→canonical reload (G-S1: replaces the former
/// duplicate in the version read path and the dump/load export; the
/// message/admin export calls this by name).
///
/// A version with no stored nodes (a logical delete — data Void, RM common
/// master06 §Logical Deletion) reassembles to [`Value::Null`], so callers need
/// no separate deleted-version guard before calling.
///
/// # Errors
///
/// Returns [`StorageError`] on a DB error or if a non-empty row set does not
/// form one tree rooted at `num = 0`.
pub async fn read_version_canonical(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<Value, StorageError> {
    let rows = read_rows(pool, vo_id, sys_version).await?;
    if rows.is_empty() {
        return Ok(Value::Null);
    }
    reassemble(&rows)
}

/// Reassemble one structure object from a node **subtree** `[num, num_cap]` of a
/// stored version, re-based so the anchor node (`num`) becomes the fragment root
/// (the codec requires `num == 0` and an empty path at the root). Reads the lean
/// [`ReadRow`] shape and reassembles through the shared codec — the whole-object
/// cell reload the AQL engine needs for a CONTAINS-anchored node. An empty
/// subtree reassembles to [`Value::Null`].
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn read_subtree_canonical(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
    num: i32,
    num_cap: i32,
) -> Result<Value, StorageError> {
    let rows = sqlx::query(
        "SELECT num, num_cap, parent_num, path, data \
         FROM node WHERE vo_id = $1 AND sys_version = $2 AND num BETWEEN $3 AND $4 ORDER BY num",
    )
    .bind(vo_id)
    .bind(sys_version)
    .bind(num)
    .bind(num_cap)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(Value::Null);
    }

    // The anchor is the lowest-num row (the queried `num`); its path is the
    // prefix to strip so descendants re-root at it.
    let base_path: String = rows
        .first()
        .and_then(|r| r.try_get::<String, _>("path").ok())
        .unwrap_or_default();

    let mut read_rows = Vec::with_capacity(rows.len());
    for r in &rows {
        let path: String = r.try_get("path")?;
        let rebased = path.strip_prefix(&base_path).unwrap_or(&path).to_owned();
        read_rows.push(ReadRow {
            num: r.try_get::<i32, _>("num")? - num,
            num_cap: r.try_get::<i32, _>("num_cap")? - num,
            // parent_num is not used by `reassemble`; re-root it.
            parent_num: 0,
            path: rebased,
            data: r.try_get("data")?,
        });
    }
    reassemble(&read_rows)
}

/// The root node fragment (`num = 0`) of the FIRST stored content version of
/// an object — the anchor for cross-version invariants (a root fragment is
/// small: children are pruned by the decomposition). `None` when no content
/// version exists (e.g. every prior version deleted).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn first_version_root(
    tx: &mut sqlx::PgConnection,
    vo_id: uuid::Uuid,
) -> Result<Option<serde_json::Value>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT data FROM node WHERE vo_id = $1 AND num = 0 ORDER BY sys_version LIMIT 1",
    )
    .bind(vo_id)
    .fetch_optional(&mut *tx)
    .await?)
}
