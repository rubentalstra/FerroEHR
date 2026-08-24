// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `node`-table row I/O: bulk-write a decomposed version, and read one version
//! back as canonical JSON.
//!
//! No openEHR spec governs the `node` table — it is our own decomposed store
//!. This module is the single home for the
//! `node` write and the `node`→canonical reload: the version read path, the
//! dump/load export, and the AQL result assembly all funnel here.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 1): stored canonical fragments — a typed \
              round-trip drops forward-compatible keys (the openEHR release strategy: minors are compatible supersets)"
)]

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;
use sqlx::{PgConnection, PgPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::storage::codec::reassemble;
use crate::storage::error::StorageError;
use crate::storage::row::{NodeRow, ReadRow};
use crate::storage::version_repo::tier::{Tier, on_cold};

/// Bulk-insert the decomposed node rows of one stored version.
///
/// `rows` is the output of [`crate::storage::codec::decompose`]; the storage
/// context (`vo_id`/`sys_version`/`ehr_id`) is supplied here and written onto
/// every row.
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
    vo_id: VoId,
    sys_version: i32,
    ehr_id: Option<EhrId>,
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
    for leaf in crate::storage::promoted::PROMOTED_LEAVES {
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
        for (i, leaf) in crate::storage::promoted::PROMOTED_LEAVES.iter().enumerate() {
            let raw = row.promoted.get(i).and_then(Clone::clone);
            match leaf.kind {
                crate::storage::promoted::PromotedKind::Timestamp => {
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

/// The lean read-row statement: **only** the five columns
/// [`crate::storage::codec::reassemble`] and the nested-set contract need — the
/// promoted query columns are not read back.
const READ_ROWS_SQL: &str = "SELECT num, num_cap, parent_num, path, data \
                             FROM node WHERE vo_id = $1 AND sys_version = $2 ORDER BY num";

/// The same statement over the both-tier union view.
const READ_ROWS_ALL_SQL: &str = "SELECT num, num_cap, parent_num, path, data \
                                 FROM node_all WHERE vo_id = $1 AND sys_version = $2 ORDER BY num";

/// Fetch the lean read rows of one stored version, ordered by `num`, from the
/// requested storage tier (no openEHR spec governs storage tiering — our own
/// design; see [`crate::storage::version_repo::tier`]).
async fn read_rows(
    pool: &PgPool,
    vo_id: VoId,
    sys_version: i32,
    tier: Tier,
) -> Result<Vec<ReadRow>, StorageError> {
    let rows = match tier {
        Tier::Primary => {
            sqlx::query(READ_ROWS_SQL)
                .bind(vo_id)
                .bind(sys_version)
                .fetch_all(pool)
                .await?
        }
        Tier::Cold => on_cold!(pool, |conn| sqlx::query(READ_ROWS_SQL)
            .bind(vo_id)
            .bind(sys_version)
            .fetch_all(&mut *conn)
            .await),
        Tier::Both => {
            sqlx::query(READ_ROWS_ALL_SQL)
                .bind(vo_id)
                .bind(sys_version)
                .fetch_all(pool)
                .await?
        }
    };

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
/// single consolidated node→canonical reload (the version read path and the
/// message/admin exports all call this by name).
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
    vo_id: VoId,
    sys_version: i32,
) -> Result<Value, StorageError> {
    read_version_canonical_in(pool, vo_id, sys_version, Tier::Primary).await
}

/// Reassemble one stored version's canonical JSON from the requested storage
/// tier ([`crate::storage::version_repo::tier`]).
///
/// [`read_version_canonical`] is the primary-tier form every ordinary caller
/// uses.
///
/// # Errors
///
/// Returns [`StorageError`] on a DB error or if a non-empty row set does not
/// form one tree rooted at `num = 0`.
pub async fn read_version_canonical_in(
    pool: &PgPool,
    vo_id: VoId,
    sys_version: i32,
    tier: Tier,
) -> Result<Value, StorageError> {
    let rows = read_rows(pool, vo_id, sys_version, tier).await?;
    if rows.is_empty() {
        return Ok(Value::Null);
    }
    reassemble(&rows)
}

/// One whole-object cell's subtree locator: the `[num, num_cap]` interval of
/// a stored version.
///
/// The AQL executor collects one per whole-object cell across a whole
/// `RESULT_SET` page so [`read_subtrees_canonical`] can load them all in a
/// single round trip. No openEHR spec governs the `node` store — our own
/// decomposed design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubtreeAnchor {
    /// The versioned object id.
    pub vo_id: VoId,
    /// The stored version number.
    pub sys_version: i32,
    /// The anchor node's pre-order number (subtree lower bound).
    pub num: i32,
    /// The anchor node's `num_cap` (subtree upper bound).
    pub num_cap: i32,
}

/// Reassemble every distinct subtree in `anchors` in **one** statement,
/// returning a map from anchor to its canonical JSON.
///
/// Each anchor's subtree is re-based so the anchor node becomes the fragment
/// root (the codec requires `num == 0` and an empty path at the root).
///
/// This closes the AQL result-assembly N+1 — a P-row whole-object projection
/// page (e.g. `SELECT c FROM EHR e CONTAINS COMPOSITION c` on a dashboard)
/// would otherwise issue P separate subtree SELECTs, one per candidate row.
/// The rows of every anchor's subtree are instead fetched by a single
/// `unnest`-array join over the anchors, tagged by anchor index, then
/// reassembled per anchor in memory.
///
/// Anchors are de-duplicated: a page may project the same version more than once
/// (repeated rows, or two whole-object columns), and each distinct subtree is
/// reassembled exactly once. An anchor with no stored nodes (a logical delete —
/// data Void, RM common master06 §Logical Deletion) is **absent** from the map;
/// the caller treats a miss as [`Value::Null`], its empty-subtree result.
///
/// A ROOT anchor (`num == 0` — the whole version, the dominant projection
/// shape `SELECT c FROM … CONTAINS COMPOSITION c`) is served straight from the
/// materialized `vo_version.body` with zero node rows and zero reassembly;
/// only genuine sub-tree anchors take the interval join. The two forms are
/// byte-identical by the commit-time parity invariant (`body` is written from
/// the same rows the node store holds).
///
/// # Errors
/// Returns [`StorageError`] on a driver failure, or if any anchor's fetched rows
/// do not reassemble into one tree.
pub async fn read_subtrees_canonical(
    pool: &PgPool,
    anchors: &[SubtreeAnchor],
) -> Result<HashMap<SubtreeAnchor, Value>, StorageError> {
    if anchors.is_empty() {
        return Ok(HashMap::new());
    }

    // De-duplicate, preserving one entry per distinct anchor; the position in
    // `distinct` is the `idx` tag bound into the query and joined back below.
    let mut distinct: Vec<SubtreeAnchor> = Vec::new();
    let mut seen: HashMap<SubtreeAnchor, usize> = HashMap::new();
    for anchor in anchors {
        seen.entry(*anchor).or_insert_with(|| {
            distinct.push(*anchor);
            distinct.len() - 1
        });
    }

    // Root anchors (`num == 0`) are whole versions — served from the
    // materialized body with no node rows; only genuine sub-tree anchors
    // proceed to the interval join.
    let roots: Vec<SubtreeAnchor> = distinct.iter().filter(|a| a.num == 0).copied().collect();
    let mut out = read_root_bodies(pool, &roots).await?;
    let distinct: Vec<SubtreeAnchor> = distinct.into_iter().filter(|a| a.num != 0).collect();
    if distinct.is_empty() {
        return Ok(out);
    }

    let idx: Vec<i32> = (0..distinct.len())
        .map(|i| i32::try_from(i).unwrap_or(i32::MAX))
        .collect();
    let vo_ids: Vec<Uuid> = distinct.iter().map(|a| a.vo_id.0).collect();
    let sys_versions: Vec<i32> = distinct.iter().map(|a| a.sys_version).collect();
    let anums: Vec<i32> = distinct.iter().map(|a| a.num).collect();
    let acaps: Vec<i32> = distinct.iter().map(|a| a.num_cap).collect();

    // One interval join: each anchor row contributes its subtree's node rows,
    // tagged by `idx`. `num BETWEEN anum AND acap` is the nested-set containment
    // predicate the single-row read also uses, now driven by the anchor set.
    let db_rows = sqlx::query(
        "SELECT a.idx, n.num, n.path, n.data \
         FROM unnest($1::int[], $2::uuid[], $3::int[], $4::int[], $5::int[]) \
              AS a(idx, vo_id, sys_version, anum, acap) \
         JOIN node n \
           ON n.vo_id = a.vo_id AND n.sys_version = a.sys_version \
              AND n.num BETWEEN a.anum AND a.acap",
    )
    .bind(&idx)
    .bind(&vo_ids)
    .bind(&sys_versions)
    .bind(&anums)
    .bind(&acaps)
    .fetch_all(pool)
    .await?;

    // Bucket the returned node rows by anchor index.
    let mut grouped: BTreeMap<i32, Vec<(i32, String, Value)>> = BTreeMap::new();
    for row in db_rows {
        let group_idx: i32 = row.try_get("idx")?;
        grouped.entry(group_idx).or_default().push((
            row.try_get("num")?,
            row.try_get("path")?,
            row.try_get("data")?,
        ));
    }

    for (group_idx, mut nodes) in grouped {
        // The anchor is the lowest-`num` row (the queried `num`); its path is the
        // prefix to strip so descendants re-root at it (the anchor-relative
        // `num`/`path` rebasing the reassembly codec requires).
        nodes.sort_by_key(|(num, _, _)| *num);
        // `idx` was generated by the query from the same `distinct` list, so it
        // addresses one of its entries; fetched rather than indexed so a
        // mismatch skips the group instead of panicking on a read path.
        let Some(anchor) = usize::try_from(group_idx)
            .ok()
            .and_then(|i| distinct.get(i))
            .copied()
        else {
            continue;
        };
        let base_path = nodes
            .first()
            .map(|(_, path, _)| path.clone())
            .unwrap_or_default();
        let read_rows: Vec<ReadRow> = nodes
            .into_iter()
            .map(|(num, path, data)| {
                let rebased = path.strip_prefix(&base_path).unwrap_or(&path).to_owned();
                ReadRow {
                    num: num - anchor.num,
                    // num_cap/parent_num are unused by `reassemble`; re-root them.
                    num_cap: 0,
                    parent_num: 0,
                    path: rebased,
                    data,
                }
            })
            .collect();
        out.insert(anchor, reassemble(&read_rows)?);
    }
    Ok(out)
}

/// The materialized bodies of whole-version (root) anchors, keyed by anchor —
/// one `unnest` join against `vo_version.body`, no node rows, no reassembly.
///
/// An anchor whose version has a `NULL` body (a logical delete) is absent
/// from the map — the same miss contract the interval join produces.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
async fn read_root_bodies(
    pool: &PgPool,
    roots: &[SubtreeAnchor],
) -> Result<HashMap<SubtreeAnchor, Value>, StorageError> {
    let mut out: HashMap<SubtreeAnchor, Value> = HashMap::with_capacity(roots.len());
    if roots.is_empty() {
        return Ok(out);
    }
    let vo_ids: Vec<Uuid> = roots.iter().map(|a| a.vo_id.0).collect();
    let sys_versions: Vec<i32> = roots.iter().map(|a| a.sys_version).collect();
    let rows = sqlx::query(
        "SELECT v.vo_id, v.sys_version, v.body \
         FROM unnest($1::uuid[], $2::int[]) AS a(vo_id, sys_version) \
         JOIN vo_version v \
           ON v.vo_id = a.vo_id AND v.sys_version = a.sys_version \
         WHERE v.body IS NOT NULL",
    )
    .bind(&vo_ids)
    .bind(&sys_versions)
    .fetch_all(pool)
    .await?;
    let mut by_key: HashMap<(Uuid, i32), Value> = rows
        .into_iter()
        .map(|row| {
            Ok((
                (row.try_get("vo_id")?, row.try_get("sys_version")?),
                row.try_get("body")?,
            ))
        })
        .collect::<Result<_, StorageError>>()?;
    for anchor in roots {
        if let Some(body) = by_key.remove(&(anchor.vo_id.0, anchor.sys_version)) {
            out.insert(*anchor, body);
        }
    }
    Ok(out)
}

/// The `(archetype_node_id, category code)` of the FIRST stored content
/// version of an object — the two scalars the cross-version invariants
/// compare, read as text off the materialized `vo_version.body`.
///
/// `None` when no content version exists (e.g. every prior version deleted);
/// either scalar is `None` when the stored body lacks that field.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn first_version_root(
    tx: &mut PgConnection,
    vo_id: VoId,
) -> Result<Option<(Option<String>, Option<String>)>, StorageError> {
    // Two text scalars off the materialized body — never a fragment fetch and
    // a Value parse for a two-field comparison.
    Ok(sqlx::query_as(
        "SELECT body ->> 'archetype_node_id', \
         body #>> '{category,defining_code,code_string}' \
         FROM vo_version WHERE vo_id = $1 AND body IS NOT NULL \
         ORDER BY sys_version LIMIT 1",
    )
    .bind(vo_id)
    .fetch_optional(&mut *tx)
    .await?)
}
