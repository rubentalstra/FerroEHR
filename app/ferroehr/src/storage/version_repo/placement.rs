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

/// The version-tree placement read, merged into ONE statement — the thaw
/// included.
///
/// Returns the preceding lineage tip (the version `expected` names, or the open
/// TRUNK tip), the next storage commit ordinal, and the transaction timestamp.
/// The timestamp is the commit instant every row of this transaction stamps
/// (`now()` is stable for the whole transaction), so the caller can compute the
/// `VERSION.signature` over `time_committed` BEFORE any insert (RM common
/// master06 §Digital Signature) and commit through the folded CTE
/// unconditionally.
///
/// A new version must never land in the primary tier while its predecessors sit
/// in the cold one, so the statement's leading data-modifying CTEs move any
/// archived rows back first — primary-key probes finding nothing for the
/// overwhelmingly common unarchived case. A same-statement `INSERT` is
/// invisible to the sibling scans
/// (<https://www.postgresql.org/docs/18/queries-with.html>), so the placement
/// reads run over `vo_version` UNION ALL the thaw's own `RETURNING` rows.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn next_placement(
    tx: &mut PgConnection,
    vo_id: VoId,
    expected: Option<(i32, i32, i32)>,
) -> Result<Placement, StorageError> {
    macro_rules! placement_select {
        ($tip_where:literal) => {
            concat!(
                "WITH cv AS (DELETE FROM cold.vo_version WHERE vo_id = $1 RETURNING *), ",
                "cn AS (DELETE FROM cold.node WHERE vo_id = $1 RETURNING *), ",
                "ct AS (DELETE FROM cold.vo_attestation WHERE vo_id = $1 RETURNING *), ",
                "cm AS (DELETE FROM vo_archive WHERE vo_id = $1), ",
                "iv AS (INSERT INTO vo_version SELECT * FROM cv), ",
                "inn AS (INSERT INTO node SELECT * FROM cn), ",
                "it AS (INSERT INTO vo_attestation SELECT * FROM ct), ",
                "src AS (SELECT vo_id, ehr_id, kind, sys_version, trunk_version, branch_number, ",
                "               branch_version, creating_system_id, lifecycle_state, sys_period ",
                "        FROM vo_version WHERE vo_id = $1 ",
                "        UNION ALL ",
                "        SELECT vo_id, ehr_id, kind, sys_version, trunk_version, branch_number, ",
                "               branch_version, creating_system_id, lifecycle_state, sys_period ",
                "        FROM cv) ",
                "SELECT o.next_ordinal, now() AS ts, tip.ehr_id, tip.kind, tip.sys_version, ",
                "tip.trunk_version, tip.branch_number, tip.branch_version, ",
                "tip.creating_system_id, tip.lifecycle_state, tip.open ",
                "FROM (SELECT (COALESCE(MAX(sys_version), 0) + 1)::int AS next_ordinal ",
                "      FROM src) o ",
                "LEFT JOIN LATERAL ( ",
                "    SELECT t.ehr_id, t.kind, t.sys_version, t.trunk_version, ",
                "           t.branch_number, t.branch_version, t.creating_system_id, ",
                "           t.lifecycle_state, upper_inf(t.sys_period) AS open ",
                "    FROM src t WHERE ",
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

/// The composition-update mega-read: [`next_placement`]'s current-trunk-tip
/// form plus every column the update pre-checks need
/// ([`UpdatePlacement`]).
#[derive(Debug)]
pub struct UpdatePlacement {
    /// The version-tree placement (tip + next ordinal + the transaction
    /// timestamp). `tip = None` when the object has no current open trunk
    /// version.
    pub placement: Placement,
    /// The tip's commit instant (`audit.time_committed`) — the `ETag` /
    /// `If-Match` metadata instant. `None` iff there is no tip.
    pub tip_time_committed: Option<jiff::Timestamp>,
    /// The owning EHR's promoted `is_modifiable` flag. `None` iff there is no
    /// tip or the tip has no owning EHR.
    pub is_modifiable: Option<bool>,
    /// The tip body's `archetype_details.template_id.value`, or `None` for a
    /// deleted tip (NULL body) or an undeclared template.
    pub stored_template: Option<String>,
    /// The FIRST stored content version's root fields —
    /// `(archetype_node_id, category code)` — for the `VERSIONED_COMPOSITION`
    /// cross-version invariants; `None` when no content version exists.
    pub first_root: Option<(Option<String>, Option<String>)>,
}

/// The composition-update placement + pre-check read, merged into ONE
/// in-transaction statement — the thaw included.
///
/// [`next_placement`]'s current-trunk-tip form (the update route's `If-Match`
/// gate has already pinned the addressed version to the current trunk tip, so
/// no expectation-addressed variant exists here) extended with the columns the
/// former pool pre-read (`super::meta::current_composition_meta`) carried: the
/// tip audit's commit instant, the owning EHR's `is_modifiable`, the stored
/// template id, and the first content version's root fields. Run under the
/// per-vo advisory lock inside the write transaction, it replaces that pool
/// round trip entirely. No openEHR spec governs the SQL — our own design.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn update_placement(
    tx: &mut PgConnection,
    vo_id: VoId,
) -> Result<UpdatePlacement, StorageError> {
    // `src` stays NARROW (no body): it is referenced more than once, so the
    // planner materializes it, and a body column would copy every version's
    // document into the tuplestore. The two body-derived facts are read by
    // targeted laterals instead — each touches exactly one row's body (the
    // tip's, and the earliest content version's).
    const SQL: &str = concat!(
        "WITH cv AS (DELETE FROM cold.vo_version WHERE vo_id = $1 RETURNING *), ",
        "cn AS (DELETE FROM cold.node WHERE vo_id = $1 RETURNING *), ",
        "ct AS (DELETE FROM cold.vo_attestation WHERE vo_id = $1 RETURNING *), ",
        "cm AS (DELETE FROM vo_archive WHERE vo_id = $1), ",
        "iv AS (INSERT INTO vo_version SELECT * FROM cv), ",
        "inn AS (INSERT INTO node SELECT * FROM cn), ",
        "it AS (INSERT INTO vo_attestation SELECT * FROM ct), ",
        "src AS (SELECT vo_id, ehr_id, kind, sys_version, trunk_version, branch_number, ",
        "               branch_version, creating_system_id, lifecycle_state, sys_period, ",
        "               audit_id ",
        "        FROM vo_version WHERE vo_id = $1 ",
        "        UNION ALL ",
        "        SELECT vo_id, ehr_id, kind, sys_version, trunk_version, branch_number, ",
        "               branch_version, creating_system_id, lifecycle_state, sys_period, ",
        "               audit_id ",
        "        FROM cv) ",
        "SELECT o.next_ordinal, now() AS ts, tip.ehr_id, tip.kind, tip.sys_version, ",
        "tip.trunk_version, tip.branch_number, tip.branch_version, ",
        "tip.creating_system_id, tip.lifecycle_state, tip.open, ",
        "a.time_committed, e.is_modifiable, tb.stored_template, ",
        "fv.found AS first_found, fv.ani AS first_ani, fv.category AS first_category ",
        "FROM (SELECT (COALESCE(MAX(sys_version), 0) + 1)::int AS next_ordinal ",
        "      FROM src) o ",
        "LEFT JOIN LATERAL ( ",
        "    SELECT t.ehr_id, t.kind, t.sys_version, t.trunk_version, ",
        "           t.branch_number, t.branch_version, t.creating_system_id, ",
        "           t.lifecycle_state, upper_inf(t.sys_period) AS open, t.audit_id ",
        "    FROM src t WHERE upper_inf(t.sys_period) AND t.branch_number = 0 ",
        ") tip ON true ",
        "LEFT JOIN audit a ON a.id = tip.audit_id ",
        "LEFT JOIN ehr e ON e.id = tip.ehr_id ",
        "LEFT JOIN LATERAL ( ",
        "    SELECT b.body #>> '{archetype_details,template_id,value}' AS stored_template ",
        "    FROM (SELECT body FROM vo_version ",
        "          WHERE vo_id = $1 AND sys_version = tip.sys_version ",
        "          UNION ALL ",
        "          SELECT body FROM cv WHERE cv.sys_version = tip.sys_version) b ",
        "    LIMIT 1 ",
        ") tb ON true ",
        "LEFT JOIN LATERAL ( ",
        // The ORDER BY + LIMIT sit INSIDE the subquery and the jsonb
        // extractions OUTSIDE it: evaluated inline, the planner computes the
        // extractions for every version row below the sort.
        "    SELECT true AS found, ",
        "           b.body ->> 'archetype_node_id' AS ani, ",
        "           b.body #>> '{category,defining_code,code_string}' AS category ",
        "    FROM (SELECT f.sys_version, f.body ",
        "          FROM (SELECT sys_version, body FROM vo_version ",
        "                WHERE vo_id = $1 AND body IS NOT NULL ",
        "                UNION ALL ",
        "                SELECT sys_version, body FROM cv WHERE body IS NOT NULL) f ",
        "          ORDER BY f.sys_version LIMIT 1) b ",
        ") fv ON true"
    );
    let row = sqlx::query(SQL).bind(vo_id).fetch_one(&mut *tx).await?;
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
    let first_root = match row.try_get::<Option<bool>, _>("first_found")? {
        Some(true) => Some((row.try_get("first_ani")?, row.try_get("first_category")?)),
        _ => None,
    };
    Ok(UpdatePlacement {
        placement: Placement {
            tip,
            next_ordinal: row.try_get("next_ordinal")?,
            now: row.try_get::<jiff_sqlx::Timestamp, _>("ts")?.to_jiff(),
        },
        tip_time_committed: row
            .try_get::<Option<jiff_sqlx::Timestamp>, _>("time_committed")?
            .map(jiff_sqlx::Timestamp::to_jiff),
        is_modifiable: row.try_get("is_modifiable")?,
        stored_template: row.try_get("stored_template")?,
        first_root,
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
