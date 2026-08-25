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
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::storage::codec::reassemble;
use crate::storage::error::StorageError;
use crate::storage::row::{NodeRow, ReadRow};

/// Build the FIXED-text node insert: one array bind per column over `unnest`,
/// so the statement carries the same parameter count at ANY row count — no
/// per-row placeholders (PostgreSQL caps a statement at 65,535 parameters,
/// which a per-row shape hits at ~4,095 nodes), one prepared statement
/// forever. Built from the shared promoted-leaf registry
/// (`crate::storage::promoted`), each leaf bound through its kind's
/// conversion (non-castable text becomes NULL, the ext baseline). With
/// `per_row_context` the storage context (`vo_id`/`sys_version`/`ehr_id`)
/// unnests per row (the archive-load batch across versions); without, it
/// binds as three constant scalars (the single-version commit write).
fn write_nodes_sql(per_row_context: bool) -> String {
    use std::fmt::Write;
    let mut columns = String::new();
    let mut selects = String::new();
    let mut arrays = String::new();
    let mut names = String::new();
    for (i, leaf) in crate::storage::promoted::PROMOTED_LEAVES.iter().enumerate() {
        columns.push_str(", ");
        columns.push_str(leaf.column);
        match leaf.kind {
            crate::storage::promoted::PromotedKind::Timestamp => {
                let _ = write!(selects, ", ext.openehr_timestamp(t.p{i})");
            }
        }
        let _ = write!(arrays, ", ${}::text[]", 16 + i);
        let _ = write!(names, ", p{i}");
    }
    let (context_select, context_arrays, context_names) = if per_row_context {
        (
            "t.vo_id, t.sys_version, t.ehr_id",
            "$1::uuid[], $2::int[], $3::uuid[]",
            "vo_id, sys_version, ehr_id, ",
        )
    } else {
        ("$1, $2, $3", "", "")
    };
    let separator = if per_row_context { ", " } else { "" };
    format!(
        "INSERT INTO node (vo_id, sys_version, ehr_id, num, num_cap, parent_num, citem_num, \
         rm_type, archetype, arch_entity, arch_concept, arch_major, name, path, data{columns}) \
         SELECT {context_select}, t.num, t.num_cap, t.parent_num, t.citem_num, t.rm_type, \
         t.archetype, t.arch_entity, t.arch_concept, t.arch_major, t.name, t.path, t.data{selects} \
         FROM unnest({context_arrays}{separator}$4::int[], $5::int[], $6::int[], $7::int[], $8::text[], $9::text[], \
         $10::text[], $11::text[], $12::int[], $13::text[], $14::text[], $15::jsonb[]{arrays}) \
         AS t({context_names}num, num_cap, parent_num, citem_num, rm_type, archetype, arch_entity, \
         arch_concept, arch_major, name, path, data{names})"
    )
}

/// The single-version node insert (constant-scalar storage context).
static WRITE_NODES_SQL: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| write_nodes_sql(false));

/// The node-insert body as a CTE fragment for the FOLDED commit statements:
/// the same fixed-text `unnest` shape as [`write_nodes_sql`], with the
/// storage context spelled as the CALLER's parameter placeholders
/// (`vo`/`sys`/`ehr`, e.g. `"$8"`) and the array binds starting at
/// `first_array_param`. `CROSS JOIN v` orders this CTE after the caller's
/// version-row CTE `v`, which is also what makes the node rows' FK to the
/// version row satisfiable inside the one statement. Empty arrays insert
/// nothing — a logical delete folds through unchanged.
pub(crate) fn node_insert_cte(vo: &str, sys: &str, ehr: &str, first_array_param: usize) -> String {
    use std::fmt::Write;
    let mut columns = String::new();
    let mut selects = String::new();
    let mut arrays = String::new();
    let mut names = String::new();
    for (i, leaf) in crate::storage::promoted::PROMOTED_LEAVES.iter().enumerate() {
        columns.push_str(", ");
        columns.push_str(leaf.column);
        match leaf.kind {
            crate::storage::promoted::PromotedKind::Timestamp => {
                let _ = write!(selects, ", ext.openehr_timestamp(t.p{i})");
            }
        }
        let _ = write!(arrays, ", ${}::text[]", first_array_param + 12 + i);
        let _ = write!(names, ", p{i}");
    }
    let p = |offset: usize| first_array_param + offset;
    format!(
        "INSERT INTO node (vo_id, sys_version, ehr_id, num, num_cap, parent_num, citem_num, \
         rm_type, archetype, arch_entity, arch_concept, arch_major, name, path, data{columns}) \
         SELECT {vo}, {sys}, {ehr}, t.num, t.num_cap, t.parent_num, t.citem_num, t.rm_type, \
         t.archetype, t.arch_entity, t.arch_concept, t.arch_major, t.name, t.path, t.data{selects} \
         FROM unnest(${}::int[], ${}::int[], ${}::int[], ${}::int[], ${}::text[], ${}::text[], \
         ${}::text[], ${}::text[], ${}::int[], ${}::text[], ${}::text[], ${}::jsonb[]{arrays}) \
         AS t(num, num_cap, parent_num, citem_num, rm_type, archetype, arch_entity, \
         arch_concept, arch_major, name, path, data{names}) CROSS JOIN v",
        p(0),
        p(1),
        p(2),
        p(3),
        p(4),
        p(5),
        p(6),
        p(7),
        p(8),
        p(9),
        p(10),
        p(11),
    )
}

/// The cross-version batch node insert (per-row storage context).
static WRITE_NODES_BATCH_SQL: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| write_nodes_sql(true));

/// Bulk-insert the decomposed node rows of one stored version — ONE
/// fixed-text `unnest` statement whatever the row count.
///
/// `rows` is the output of [`crate::storage::codec::decompose`]; the storage
/// context (`vo_id`/`sys_version`/`ehr_id`) is supplied here and written onto
/// every row as constant scalars.
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
    let query = sqlx::query(sqlx::AssertSqlSafe(WRITE_NODES_SQL.as_str()))
        .bind(vo_id)
        .bind(sys_version)
        .bind(ehr_id);
    let refs: Vec<&NodeRow> = rows.iter().collect();
    bind_node_arrays(query, &refs).execute(&mut *tx).await?;
    Ok(())
}

/// Bulk-insert the decomposed node rows of MANY stored versions in ONE
/// statement.
///
/// The same fixed-text `unnest` shape as [`write_nodes`], with the storage
/// context (`vo_id`/`sys_version`/`ehr_id`) unnesting per row instead of
/// binding as constants. The archive load writes a whole record's node rows
/// through this (never one statement per version).
///
/// # Errors
///
/// Returns [`StorageError::Database`] on any driver/insert failure.
pub async fn write_nodes_batch(
    tx: &mut PgConnection,
    versions: &[(VoId, i32, Option<EhrId>, Vec<NodeRow>)],
) -> Result<(), StorageError> {
    let n: usize = versions.iter().map(|(_, _, _, rows)| rows.len()).sum();
    if n == 0 {
        return Ok(());
    }
    let mut vo_ids: Vec<Uuid> = Vec::with_capacity(n);
    let mut sys_versions: Vec<i32> = Vec::with_capacity(n);
    let mut ehr_ids: Vec<Option<Uuid>> = Vec::with_capacity(n);
    let mut refs: Vec<&NodeRow> = Vec::with_capacity(n);
    for (vo_id, sys_version, ehr_id, rows) in versions {
        for row in rows {
            vo_ids.push(vo_id.0);
            sys_versions.push(*sys_version);
            ehr_ids.push(ehr_id.map(|e| e.0));
            refs.push(row);
        }
    }
    let query = sqlx::query(sqlx::AssertSqlSafe(WRITE_NODES_BATCH_SQL.as_str()))
        .bind(vo_ids)
        .bind(sys_versions)
        .bind(ehr_ids);
    bind_node_arrays(query, &refs).execute(&mut *tx).await?;
    Ok(())
}

/// Bind the twelve per-node column arrays plus the promoted-leaf arrays onto a
/// node-insert statement whose storage-context parameters (`$1..$3`) are
/// already bound.
pub(crate) fn bind_node_arrays<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    rows: &[&'q NodeRow],
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let n = rows.len();
    let mut nums = Vec::with_capacity(n);
    let mut num_caps = Vec::with_capacity(n);
    let mut parent_nums = Vec::with_capacity(n);
    let mut citem_nums = Vec::with_capacity(n);
    let mut rm_types: Vec<&str> = Vec::with_capacity(n);
    let mut archetypes: Vec<Option<&str>> = Vec::with_capacity(n);
    let mut arch_entities: Vec<Option<&str>> = Vec::with_capacity(n);
    let mut arch_concepts: Vec<Option<&str>> = Vec::with_capacity(n);
    let mut arch_majors: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut names: Vec<Option<&str>> = Vec::with_capacity(n);
    let mut paths: Vec<&str> = Vec::with_capacity(n);
    let mut datas: Vec<&Value> = Vec::with_capacity(n);
    for row in rows {
        nums.push(row.num);
        num_caps.push(row.num_cap);
        parent_nums.push(row.parent_num);
        citem_nums.push(row.citem_num);
        rm_types.push(&row.rm_type);
        archetypes.push(row.archetype.as_deref());
        arch_entities.push(row.arch_entity.as_deref());
        arch_concepts.push(row.arch_concept.as_deref());
        arch_majors.push(row.arch_major);
        names.push(row.name.as_deref());
        paths.push(&row.path);
        datas.push(&row.data);
    }
    query = query
        .bind(nums)
        .bind(num_caps)
        .bind(parent_nums)
        .bind(citem_nums)
        .bind(rm_types)
        .bind(archetypes)
        .bind(arch_entities)
        .bind(arch_concepts)
        .bind(arch_majors)
        .bind(names)
        .bind(paths)
        .bind(datas);
    for (i, _leaf) in crate::storage::promoted::PROMOTED_LEAVES.iter().enumerate() {
        let leaf_values: Vec<Option<&str>> = rows
            .iter()
            .map(|row| row.promoted.get(i).and_then(Option::as_deref))
            .collect();
        query = query.bind(leaf_values);
    }
    query
}

/// The lean read-row statement: **only** the five columns
/// [`crate::storage::codec::reassemble`] and the nested-set contract need — the
/// promoted query columns are not read back.
const READ_ROWS_SQL: &str = "SELECT num, num_cap, parent_num, path, data \
                             FROM node WHERE vo_id = $1 AND sys_version = $2 ORDER BY num";

/// The same statement over the both-tier union view.
const READ_ROWS_ALL_SQL: &str = "SELECT num, num_cap, parent_num, path, data \
                                 FROM node_all WHERE vo_id = $1 AND sys_version = $2 ORDER BY num";

/// Fetch the lean read rows of one stored version, ordered by `num` —
/// primary tier only, or both tiers through the `node_all` union view (no
/// openEHR spec governs storage tiering — our own design).
async fn read_rows(
    pool: &PgPool,
    vo_id: VoId,
    sys_version: i32,
    both_tiers: bool,
) -> Result<Vec<ReadRow>, StorageError> {
    let sql = if both_tiers {
        READ_ROWS_ALL_SQL
    } else {
        READ_ROWS_SQL
    };
    let rows = sqlx::query(sql)
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

/// Reassemble one stored version's canonical JSON from its primary-tier
/// `node` rows — the consolidated node→canonical reload.
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
    let rows = read_rows(pool, vo_id, sys_version, false).await?;
    if rows.is_empty() {
        return Ok(Value::Null);
    }
    reassemble(&rows)
}

/// [`read_version_canonical`] over BOTH storage tiers (the `node_all` union
/// view) — for the whole-repository readers (admin export, physical delete)
/// that must see archived content by definition.
///
/// # Errors
///
/// Returns [`StorageError`] on a DB error or if a non-empty row set does not
/// form one tree rooted at `num = 0`.
pub async fn read_version_canonical_all(
    pool: &PgPool,
    vo_id: VoId,
    sys_version: i32,
) -> Result<Value, StorageError> {
    let rows = read_rows(pool, vo_id, sys_version, true).await?;
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
