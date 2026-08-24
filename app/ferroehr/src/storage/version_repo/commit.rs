// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The local commit write path.
//!
//! Covers the `audit` + `contribution` inserts, the folded one-statement
//! version commit, the lineage-tip close, and the folder-membership +
//! event-outbox writes that ride along inside the same commit transaction.
//!
//! No openEHR spec governs the SQL — our own design. The change-control law realized here is RM common master06
//! (§Committal and Audits, §The 'Virtual Version Tree'); `AUDIT_DETAILS` is
//! master04.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 1): stored canonical fragments — a typed \
              round-trip drops forward-compatible keys (the openEHR release strategy: minors are compatible supersets)"
)]

use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;

/// The `AUDIT_DETAILS` fields to persist, as the `audit` row's own columns.
///
/// master04 §Audit Details: `change_type` is the numeric `audit_change_type`
/// group code, never a rubric (`Change_type_valid`), and the three jsonb
/// columns arrive as the canonical RM fragments they store
/// (`DV_TEXT` / `PARTY_PROXY` / the `ATTESTATION`-declared attributes).
///
/// The fragments are OWNED because the versioning layer holds these attributes
/// as their RM values and encodes them once, here at its boundary
/// (`crate::versioning::audit::AuditInput::row`): storage takes plain value
/// inputs and never decodes RM types (see the module docs), and the
/// `ATTESTATION`-declared subset is not an RM class it could name.
#[derive(Debug)]
pub struct AuditRow<'a> {
    /// `AUDIT_DETAILS.system_id`.
    pub system_id: &'a str,
    /// The numeric `audit_change_type` group code.
    pub change_type: &'a str,
    /// The canonical `DV_TEXT` fragment of `AUDIT_DETAILS.description`, when
    /// the committer supplied one.
    pub description: Option<Value>,
    /// The canonical `PARTY_PROXY` JSON of the committer.
    pub committer: Value,
    /// The canonical fragment of the `ATTESTATION`-declared attributes when
    /// this commit audit is an `ATTESTATION` (master06 §Attestation), else
    /// `None`.
    pub attestation: Option<Value>,
}

/// Take the per-vo transaction advisory lock that serializes concurrent
/// writers of one versioned object (so branch writers no longer all contend
/// on one current row).
///
/// The versioning tree-placement decision calls this before it reads the
/// preceding version.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
// The preceding-version reads and the next-ordinal/next-branch computation that
// surround this lock are the version-tree placement DECISION — they live in the
// versioning layer (`versioning::change`), which calls this lock first.
pub async fn advisory_lock(tx: &mut PgConnection, vo_id: VoId) -> Result<(), StorageError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(vo_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

/// Inserts an `audit` row, returning its id and the server-computed timestamp.
///
/// The `time_committed` (master06 §Committal m3) is captured via `RETURNING` so
/// the commit path can build the exact `ORIGINAL_VERSION` it will later serve —
/// the signed bytes must match the read-time canonical form.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_audit(
    tx: &mut PgConnection,
    audit: &AuditRow<'_>,
) -> Result<(Uuid, jiff::Timestamp), StorageError> {
    let row = sqlx::query(
        "INSERT INTO audit (system_id, change_type, description, committer, attestation) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id, time_committed",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .bind(&audit.attestation)
    .fetch_one(&mut *tx)
    .await?;
    let id: Uuid = row.try_get("id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((id, time_committed))
}

/// Insert a `contribution` row referencing its audit, returning its
/// server-generated (`uuidv7()`) id. `ehr_id` is `None` for a demographic
/// CONTRIBUTION (no EHR scope).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
/// ([`StorageError::ContributionUidInUse`] is structurally unreachable here —
/// the id is freshly generated — but kept as the absent-`RETURNING` mapping so
/// the statement stays identical to the supplied-uid path in
/// [`write_contribution`].)
pub async fn insert_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<EhrId>,
    audit_id: Uuid,
) -> Result<Uuid, StorageError> {
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO contribution (id, ehr_id, audit_id) \
         VALUES (COALESCE($1, uuidv7()), $2, $3) \
         ON CONFLICT (id) DO NOTHING RETURNING id",
    )
    .bind(None::<Uuid>)
    .bind(ehr_id)
    .bind(audit_id)
    .fetch_optional(&mut *tx)
    .await?;
    inserted.ok_or(StorageError::ContributionUidInUse(None))
}

/// Insert an `audit` row and its enclosing `contribution` in ONE round trip
/// via a data-modifying CTE, returning `(contribution_id, audit_id,
/// time_committed)`.
///
/// The `contribution` references the just-inserted `audit`; `time_committed`
/// is the server-computed commit instant (master06 §Committal m3) the
/// version's `commit_audit` is signed against. A client-supplied CONTRIBUTION
/// uid is honoured (`supplied`); a duplicate id is a
/// [`StorageError::ContributionUidInUse`] conflict, never an overwrite
/// (ITS-REST `contribution_create`).
///
/// The audit → contribution insert is a dependent chain (the CONTRIBUTION and
/// its `AUDIT_DETAILS` commit together — master06 §Committal and Audits); merging
/// the two statements into one CTE is a round-trip optimisation only — the rows
/// written and the values returned are byte-identical to two separate inserts,
/// and both still run inside the caller's transaction so a conflict (or any
/// later failure) rolls back the orphan audit row. No openEHR spec governs
/// statement batching — our own design.
///
/// On a `supplied`-uid conflict the `contribution` CTE inserts nothing (`ON
/// CONFLICT DO NOTHING`), so the outer `LEFT JOIN` yields a NULL
/// `contribution_id` → [`StorageError::ContributionUidInUse`]; the audit CTE has
/// already run but is discarded when the transaction unwinds.
///
/// # Errors
/// Returns [`StorageError::ContributionUidInUse`] on a duplicate supplied uid,
/// else [`StorageError::Database`] on a driver/insert failure.
pub async fn write_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<EhrId>,
    audit: &AuditRow<'_>,
    supplied: Option<Uuid>,
) -> Result<(Uuid, Uuid, jiff::Timestamp), StorageError> {
    let row = sqlx::query(
        "WITH a AS ( \
             INSERT INTO audit (system_id, change_type, description, committer, attestation) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id, time_committed \
         ), c AS ( \
             INSERT INTO contribution (id, ehr_id, audit_id) \
             SELECT COALESCE($6, uuidv7()), $7, a.id FROM a \
             ON CONFLICT (id) DO NOTHING \
             RETURNING id \
         ) \
         SELECT a.id AS audit_id, a.time_committed, c.id AS contribution_id \
         FROM a LEFT JOIN c ON true",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .bind(&audit.attestation)
    .bind(supplied)
    .bind(ehr_id)
    .fetch_one(&mut *tx)
    .await?;
    let contribution_id: Option<Uuid> = row.try_get("contribution_id")?;
    let contribution_id = contribution_id.ok_or(StorageError::ContributionUidInUse(supplied))?;
    let audit_id: Uuid = row.try_get("audit_id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((contribution_id, audit_id, time_committed))
}

/// Close (supersede) one specific version row — the lineage tip a new version
/// replaces — at `now()`.
///
/// Lineage-precise: a branch commit closes its branch tip, a trunk commit the
/// trunk tip; a FORK closes nothing (master06 §Version tree, realized by the
/// temporal `sys_period`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/update failure.
pub async fn close_ordinal_at_now(
    tx: &mut PgConnection,
    vo_id: VoId,
    ordinal: i32,
) -> Result<(), StorageError> {
    sqlx::query(
        "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), now(), '[)') \
         WHERE vo_id = $1 AND sys_version = $2 AND upper_inf(sys_period)",
    )
    .bind(vo_id)
    .bind(ordinal)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// The `vo_version` columns for a **folded** commit — every content column of
/// a stored version EXCEPT `contribution_id`/`audit_id`, which come from the
/// same statement's `contribution`/`audit` CTEs.
///
/// `time_committed` is the caller's pre-read commit instant (master06
/// §Committal m3 — still server-assigned: it is a database `now()` the caller
/// fetched earlier on this request). Binding it makes the stored audit time,
/// the `sys_period` open bound, and the instant the `VERSION.signature` was
/// computed over one value BY CONSTRUCTION.
///
/// A superseded lineage tip is closed by the caller in a **separate, prior**
/// statement ([`close_ordinal_at_now`]) — never folded into this insert: the
/// one-open-row-per-lineage partial unique indexes (`uq_vo_version_current` /
/// `uq_vo_version_branch_current`) require the old open row to be gone before
/// the new one is inserted, and data-modifying CTEs in one statement share a
/// snapshot with undefined ordering, so a fold could momentarily hold two open
/// rows for the lineage. Close-then-insert stays ordered (master06 §The 'Virtual Version Tree').
#[derive(Debug)]
pub struct FoldedVersion<'a> {
    /// The versioned object's id.
    pub vo_id: VoId,
    /// The `vo_version.kind` discriminator text.
    pub kind: &'a str,
    /// The owning EHR, or `None` for a demographic versioned object.
    pub ehr_id: Option<EhrId>,
    /// The per-object storage commit ordinal — NOT the wire version number.
    pub sys_version: i32,
    /// `VERSION_TREE_ID` first part.
    pub trunk_version: i32,
    /// `VERSION_TREE_ID` second part; `0` on a trunk row.
    pub branch_number: i32,
    /// `VERSION_TREE_ID` third part; `0` on a trunk row.
    pub branch_version: i32,
    /// The `version_lifecycle_state` numeric code.
    pub lifecycle_state: &'a str,
    /// The version's immutable `creating_system_id`.
    pub creating_system_id: &'a str,
    /// `ORIGINAL_VERSION.preceding_version_uid`; `None` for a first version.
    pub preceding_version_uid: Option<&'a str>,
    /// The OPT `template_id` a COMPOSITION was committed against (else `None`).
    pub template_id: Option<&'a str>,
    /// `VERSION.signature` (0..1), opaque radix-64.
    pub signature: Option<&'a str>,
    /// Whether `signature` was supplied verbatim by the client (foreign — never
    /// re-verified at read; master06 §Digital Signature) vs generated by this
    /// server. `false` for a server signature or an unsigned version.
    pub signature_client_supplied: bool,
    /// Whether the RELEASED openEHR generation set can express this version's
    /// body — the `vo_version.stable_compatible` stamp the read-time
    /// `spec_profile` gate consults. No openEHR spec governs runtime
    /// generation selection — our own design/extension.
    pub stable_compatible: bool,
    /// The assembled canonical body (`vo_version.body`) — the SAME in-memory
    /// value the node rows are decomposed from; `None` on a logical delete
    /// (no content — RM common master06 §Logical Deletion).
    pub body: Option<&'a Value>,
    /// The commit instant: the database `now()` the caller read on this
    /// request (the placement read, the writability gate, or the owning
    /// CONTRIBUTION's committal), stored as the audit `time_committed` and
    /// the `sys_period` open bound.
    pub time_committed: jiff::Timestamp,
}

/// A **standalone** folded commit: `audit` + `contribution` + `vo_version` in
/// ONE data-modifying CTE, returning `(contribution_id, audit_id,
/// time_committed)`.
///
/// The single audit row serves both the CONTRIBUTION and the version's
/// `commit_audit` (a direct write is one CONTRIBUTION of one change —
/// master06 §Committal and Audits). `time_committed` is the server-computed
/// commit instant (master06 §Committal m3).
///
/// This is the round-trip-collapsed equivalent of [`write_contribution`]
/// followed by a plain `vo_version` insert: the rows written and the values
/// returned are byte-identical (the version's `sys_period` and the audit both
/// open at the caller's bound [`FoldedVersion::time_committed`] — the instant
/// the `VERSION.signature` was computed over), and everything still runs
/// inside the caller's transaction so any failure rolls the whole set back.
/// Any lineage-tip close is a separate prior statement (see
/// [`FoldedVersion`]). No openEHR spec governs statement batching — our own
/// design.
///
/// # Errors
/// Returns [`StorageError::ContributionUidInUse`] on a duplicate supplied uid
/// (the `contribution` CTE inserts nothing → NULL `contribution_id`), else
/// [`StorageError::Database`] on a driver/insert failure.
pub async fn commit_new_version(
    tx: &mut PgConnection,
    audit: &AuditRow<'_>,
    supplied: Option<Uuid>,
    v: &FoldedVersion<'_>,
) -> Result<(Uuid, Uuid, jiff::Timestamp), StorageError> {
    let row = sqlx::query(
        "WITH a AS ( \
             INSERT INTO audit (system_id, change_type, description, committer, attestation, \
                                time_committed) \
             VALUES ($1, $2, $3, $4, $5, $22::timestamptz) RETURNING id, time_committed \
         ), c AS ( \
             INSERT INTO contribution (id, ehr_id, audit_id) \
             SELECT COALESCE($6, uuidv7()), $7, a.id FROM a \
             ON CONFLICT (id) DO NOTHING \
             RETURNING id \
         ), v AS ( \
             INSERT INTO vo_version \
               (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
                sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
                contribution_id, audit_id, template_id, signature, \
                signature_client_supplied, stable_compatible, body) \
             SELECT $8, $9, $7, $10, $11, $12, $13, tstzrange($22::timestamptz, NULL, '[)'), \
                    $14, $15, $16, c.id, a.id, $17, $18, $19, $20, $21 \
             FROM a, c \
             RETURNING 1 \
         ) \
         SELECT a.id AS audit_id, a.time_committed, c.id AS contribution_id \
         FROM a LEFT JOIN c ON true",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .bind(&audit.attestation)
    .bind(supplied)
    .bind(v.ehr_id)
    .bind(v.vo_id)
    .bind(v.kind)
    .bind(v.sys_version)
    .bind(v.trunk_version)
    .bind(v.branch_number)
    .bind(v.branch_version)
    .bind(v.lifecycle_state)
    .bind(v.creating_system_id)
    .bind(v.preceding_version_uid)
    .bind(v.template_id)
    .bind(v.signature)
    .bind(v.signature_client_supplied)
    .bind(v.stable_compatible)
    .bind(v.body)
    .bind(v.time_committed.to_string())
    .fetch_one(&mut *tx)
    .await?;
    let contribution_id: Option<Uuid> = row.try_get("contribution_id")?;
    let contribution_id = contribution_id.ok_or(StorageError::ContributionUidInUse(supplied))?;
    let audit_id: Uuid = row.try_get("audit_id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((contribution_id, audit_id, time_committed))
}

/// A folded commit WITHIN an already-opened CONTRIBUTION: the version's own
/// `commit_audit` + `vo_version` in ONE data-modifying CTE, referencing the
/// pre-existing `contribution_id`.
///
/// Returns `(audit_id, time_committed)`. The CONTRIBUTION and its own audit
/// were written earlier in the same transaction ([`write_contribution`]);
/// each change carries its own `commit_audit` (master06 §Committal and
/// Audits), opened at the caller's bound [`FoldedVersion::time_committed`]
/// exactly as [`commit_new_version`] does. Any lineage-tip close is a
/// separate prior statement. No openEHR spec governs statement batching —
/// our own design.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn commit_version_into(
    tx: &mut PgConnection,
    audit: &AuditRow<'_>,
    contribution_id: Uuid,
    v: &FoldedVersion<'_>,
) -> Result<(Uuid, jiff::Timestamp), StorageError> {
    let row = sqlx::query(
        "WITH a AS ( \
             INSERT INTO audit (system_id, change_type, description, committer, attestation, \
                                time_committed) \
             VALUES ($1, $2, $3, $4, $5, $22::timestamptz) RETURNING id, time_committed \
         ), v AS ( \
             INSERT INTO vo_version \
               (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
                sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
                contribution_id, audit_id, template_id, signature, \
                signature_client_supplied, stable_compatible, body) \
             SELECT $6, $7, $8, $9, $10, $11, $12, tstzrange($22::timestamptz, NULL, '[)'), \
                    $13, $14, $15, $16, a.id, $17, $18, $19, $20, $21 \
             FROM a \
             RETURNING 1 \
         ) \
         SELECT a.id AS audit_id, a.time_committed FROM a",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .bind(&audit.attestation)
    .bind(v.vo_id)
    .bind(v.kind)
    .bind(v.ehr_id)
    .bind(v.sys_version)
    .bind(v.trunk_version)
    .bind(v.branch_number)
    .bind(v.branch_version)
    .bind(v.lifecycle_state)
    .bind(v.creating_system_id)
    .bind(v.preceding_version_uid)
    .bind(contribution_id)
    .bind(v.template_id)
    .bind(v.signature)
    .bind(v.signature_client_supplied)
    .bind(v.stable_compatible)
    .bind(v.body)
    .bind(v.time_committed.to_string())
    .fetch_one(&mut *tx)
    .await?;
    let audit_id: Uuid = row.try_get("audit_id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((audit_id, time_committed))
}

// ── folder membership ─────────────────────────────────────────────────────────

/// Append a new folder-hierarchy membership row for an EHR (RM ehr master04
/// §Folders; RM ehr EHR class `Directory_in_folders`).
///
/// `rank` is 1-based, append-only and never reused: the next rank is
/// `max(rank)+1` for this EHR. Called once per FOLDER *creation*. No openEHR
/// spec governs the `ehr_folder` storage mechanism (our own design).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_ehr_folder_rank(
    tx: &mut PgConnection,
    ehr_id: EhrId,
    vo_id: VoId,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO ehr_folder (ehr_id, rank, vo_id) VALUES \
         ($1, (SELECT COALESCE(MAX(rank), 0) + 1 FROM ehr_folder WHERE ehr_id = $1), $2)",
    )
    .bind(ehr_id)
    .bind(vo_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

// ── event outbox ──────────────────────────────────────────────────────────────

/// Write the contribution-outbox event row **inside the commit transaction**
/// it announces — no commit without its event, no event without its commit.
///
/// No openEHR spec governs eventing (our own extension). The PHI-free
/// per-version entries are built by the versioning layer
/// (`Committed::envelope_entry`); this function wraps them in the fixed
/// envelope shape the events-extension drainer consumes (`{contribution_id,
/// ehr_id, committed_at, versions[]}`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn write_outbox(
    tx: &mut PgConnection,
    contribution_id: Uuid,
    ehr_id: Option<EhrId>,
    committed_at: jiff::Timestamp,
    versions: Vec<Value>,
) -> Result<(), StorageError> {
    let envelope = serde_json::json!({
        "contribution_id": contribution_id,
        "ehr_id": ehr_id,
        "committed_at": committed_at.to_string(),
        "versions": versions,
    });
    sqlx::query(
        "INSERT INTO event_outbox (contribution_id, ehr_id, envelope, committed_at) \
         VALUES ($1, $2, $3, $4::timestamptz)",
    )
    .bind(contribution_id)
    .bind(ehr_id)
    .bind(&envelope)
    .bind(committed_at.to_string())
    .execute(&mut *tx)
    .await?;
    Ok(())
}
