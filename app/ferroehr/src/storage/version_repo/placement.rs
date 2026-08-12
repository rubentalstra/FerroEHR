// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The version-tree placement reads: the next storage commit ordinal, the
//! transaction timestamp, and the next branch number at a fork point.
//!
//! The placement *decision* (classify, tree placement, lifecycle) stays in
//! the versioning layer (`versioning::change`), which takes the per-vo
//! advisory lock first and then calls these reads.
//!
//! No openEHR spec governs the SQL — our own design; the version tree
//! realized is RM common master06 §The 'Virtual Version Tree'.

use sqlx::{PgConnection, Row};

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;

/// The preceding lineage-tip row read for the version-tree placement
/// decision: the addressed version (`expected = Some((t, b, v))`) or the
/// current open TRUNK tip (`expected = None`).
///
/// Plain row values — the versioning layer maps them onto its `PrecedingTip`
/// (tree id + kind + lifecycle).
#[derive(Debug, Clone)]
pub struct TipRow {
    /// The owning EHR, or `None` for a demographic versioned object.
    pub ehr_id: Option<EhrId>,
    /// The `vo_version.kind` discriminator text.
    pub kind: String,
    /// The per-object storage commit ordinal — NOT the wire version number.
    pub sys_version: i32,
    /// `VERSION_TREE_ID` first part.
    pub trunk_version: i32,
    /// `VERSION_TREE_ID` second part; `0` on a trunk row.
    pub branch_number: i32,
    /// `VERSION_TREE_ID` third part; `0` on a trunk row.
    pub branch_version: i32,
    /// The version's immutable `creating_system_id`.
    pub creating_system_id: String,
    /// The `version_lifecycle_state` numeric code.
    pub lifecycle_state: String,
    /// Whether the tip is still open (`upper_inf(sys_period)`).
    pub open: bool,
}

/// The merged placement read ([`next_placement`]).
#[derive(Debug)]
pub struct Placement {
    /// The preceding lineage tip, when the object has one.
    pub tip: Option<TipRow>,
    /// The next storage commit ordinal (`MAX(sys_version) + 1`).
    pub next_ordinal: i32,
    /// The transaction timestamp — the commit instant every row of this
    /// transaction stamps.
    pub now: jiff::Timestamp,
}

/// The version-tree placement read, merged into ONE statement.
///
/// Returns the preceding lineage tip (the version `expected` names, or the open
/// TRUNK tip), the next storage commit ordinal, and the transaction timestamp.
/// The timestamp is the commit instant every row of this transaction stamps
/// (`now()` is stable for the whole transaction), so the caller can compute the
/// `VERSION.signature` over `time_committed` BEFORE any insert (RM common
/// master06 §Digital Signature) and commit through the folded CTE
/// unconditionally.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn next_placement(
    tx: &mut PgConnection,
    vo_id: VoId,
    expected: Option<(i32, i32, i32)>,
) -> Result<Placement, StorageError> {
    // A new version must never land in the primary tier while its predecessors
    // sit in the cold one, so an archived object is brought back first — one
    // statement, a no-op for every unarchived object
    // (`crate::storage::version_repo::tier`).
    crate::storage::version_repo::tier::thaw_one(&mut *tx, vo_id).await?;
    macro_rules! placement_select {
        ($tip_where:literal) => {
            concat!(
                "SELECT o.next_ordinal, now() AS ts, tip.ehr_id, tip.kind, tip.sys_version, ",
                "tip.trunk_version, tip.branch_number, tip.branch_version, ",
                "tip.creating_system_id, tip.lifecycle_state, tip.open ",
                "FROM (SELECT (COALESCE(MAX(sys_version), 0) + 1)::int AS next_ordinal ",
                "      FROM vo_version WHERE vo_id = $1) o ",
                "LEFT JOIN LATERAL ( ",
                "    SELECT t.ehr_id, t.kind, t.sys_version, t.trunk_version, ",
                "           t.branch_number, t.branch_version, t.creating_system_id, ",
                "           t.lifecycle_state, upper_inf(t.sys_period) AS open ",
                "    FROM vo_version t WHERE ",
                $tip_where,
                ") tip ON true"
            )
        };
    }
    let row = match expected {
        None => {
            sqlx::query(placement_select!(
                "t.vo_id = $1 AND upper_inf(t.sys_period) AND t.branch_number = 0 "
            ))
            .bind(vo_id)
            .fetch_one(&mut *tx)
            .await?
        }
        Some((t, b, v)) => {
            sqlx::query(placement_select!(
                "t.vo_id = $1 AND t.trunk_version = $2 AND t.branch_number = $3 \
                 AND t.branch_version = $4 "
            ))
            .bind(vo_id)
            .bind(t)
            .bind(b)
            .bind(v)
            .fetch_one(&mut *tx)
            .await?
        }
    };
    let kind: Option<String> = row.try_get("kind")?;
    let tip = match kind {
        None => None,
        Some(kind) => Some(TipRow {
            ehr_id: row.try_get("ehr_id")?,
            kind,
            sys_version: row.try_get("sys_version")?,
            trunk_version: row.try_get("trunk_version")?,
            branch_number: row.try_get("branch_number")?,
            branch_version: row.try_get("branch_version")?,
            creating_system_id: row.try_get("creating_system_id")?,
            lifecycle_state: row.try_get("lifecycle_state")?,
            open: row.try_get("open")?,
        }),
    };
    Ok(Placement {
        tip,
        next_ordinal: row.try_get("next_ordinal")?,
        now: row.try_get::<jiff_sqlx::Timestamp, _>("ts")?.to_jiff(),
    })
}

/// The transaction timestamp (`now()`), stable for the whole transaction —
/// the commit instant for a create (no placement read exists to carry it).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn tx_now(tx: &mut PgConnection) -> Result<jiff::Timestamp, StorageError> {
    Ok(
        sqlx::query_scalar::<_, jiff_sqlx::Timestamp>("SELECT now()")
            .fetch_one(&mut *tx)
            .await?
            .to_jiff(),
    )
}

/// The next branch number at a trunk fork point (`MAX(branch_number) + 1`
/// among the versions at `trunk_version`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn next_branch_number(
    tx: &mut PgConnection,
    vo_id: VoId,
    trunk_version: i32,
) -> Result<i32, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(MAX(branch_number), 0) + 1 FROM vo_version \
         WHERE vo_id = $1 AND trunk_version = $2",
    )
    .bind(vo_id)
    .bind(trunk_version)
    .fetch_one(&mut *tx)
    .await?)
}
