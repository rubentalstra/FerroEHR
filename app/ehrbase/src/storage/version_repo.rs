//! Row I/O for the versioned-object spine: `vo_version`, `audit`,
//! `contribution`, `vo_attestation`, plus the folder-membership and event-outbox
//! writes that ride along inside the same commit transaction.
//!
//! No openEHR spec governs the SQL schema (`docs/architecture.md` §Storage) —
//! this module is pure plumbing. All **semantics** (change classification,
//! version-tree placement, lifecycle, signing, attestation policy, import
//! policy) stay in the versioning layer, which hands these functions plain
//! value inputs and consumes the [`StoredVersion`] read shape. The change
//! control law these rows realize is RM common master06 (§Contributions,
//! §Committal and Audits, §Version tree, §Copying); AUDIT_DETAILS/ATTESTATION
//! are master04. Every write runs inside a caller-owned `sqlx` transaction so a
//! version + nodes + contribution + audit (+ outbox) commit atomically
//! (master06 §Committal: "similar to nested transactions").
//
// TODO(w3f-integrate): the versioning layer owns the value types (`AuditInput`,
// `Kind`, `TreeId`, `VersionRead`, `Committed`) that map onto the plain inputs
// and the [`StoredVersion`] output below (e.g. `Kind::as_str` → `kind: &str`,
// `TreeId::columns()` → the three tree ints); reconciled at the fix pass. Kept
// decoupled so storage never depends upward on versioning.

use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::storage::StorageError;
use crate::storage::node_repo::read_version_canonical;

/// The `AUDIT_DETAILS` fields to persist (master04 §Audit Details). `committer`
/// is the canonical `PARTY_PROXY` JSON; `change_type` is the numeric
/// `audit_change_type` group code, never a rubric (`Change_type_valid`).
#[derive(Debug)]
pub struct AuditRow<'a> {
    pub system_id: &'a str,
    pub change_type: &'a str,
    pub description: Option<&'a str>,
    pub committer: &'a Value,
}

/// One `vo_version` row to insert with validity `[now, ∞)` — the local
/// (non-import) write. The version tree is passed as its three column ints
/// (`VERSION_TREE_ID` = `trunk[.branch_number.branch_version]`, master05
/// §Syntaxes); `kind` is the `vo_version.kind` discriminator text.
#[derive(Debug)]
pub struct VersionRow<'a> {
    pub vo_id: Uuid,
    pub kind: &'a str,
    pub ehr_id: Option<Uuid>,
    /// The per-vo storage commit ordinal — NOT the wire version number.
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    /// The `version_lifecycle_state` numeric code (master06 §Version Lifecycle).
    pub lifecycle_state: &'a str,
    /// Immutable `creating_system_id` — the `OBJECT_VERSION_ID` middle part
    /// (master06 §Distributed Versioning).
    pub creating_system_id: &'a str,
    /// `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first version).
    pub preceding_version_uid: Option<&'a str>,
    /// `other_input_version_uids` (merge provenance; empty → stored NULL).
    pub other_input_version_uids: &'a [String],
    pub contribution_id: Uuid,
    pub audit_id: Uuid,
    /// The OPT `template_id` a COMPOSITION was committed against (else `None`).
    pub template_id: Option<&'a str>,
    /// `VERSION.signature` (master06 §Digital Signature; 0..1).
    pub signature: Option<&'a str>,
}

/// One `vo_version` row to insert with an **explicit** `sys_period`
/// (`[lower, upper)`, `upper = None` ⇒ still-open) — the import analogue of
/// [`VersionRow`], carrying no `template_id`. The import path builds a synthetic
/// strictly-increasing local period chain per lineage (master06 §Copying).
#[derive(Debug)]
pub struct ImportedVersionRow<'a> {
    pub vo_id: Uuid,
    pub kind: &'a str,
    pub ehr_id: Option<Uuid>,
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    pub lifecycle_state: &'a str,
    pub creating_system_id: &'a str,
    pub preceding_version_uid: Option<&'a str>,
    pub other_input_version_uids: &'a [String],
    pub contribution_id: Uuid,
    pub audit_id: Uuid,
    pub signature: Option<&'a str>,
    /// Lower bound of the synthetic local `sys_period`.
    pub lower: jiff::Timestamp,
    /// Upper bound (`None` = the still-open tip of this lineage).
    pub upper: Option<jiff::Timestamp>,
}

/// A loaded `vo_version`⋈`audit` row plus its reassembled content and
/// attestations — the storage read shape the versioning layer maps into a
/// `VERSION`/`ORIGINAL_VERSION`. The tree is returned as its three column ints;
/// the audit fields are flattened (versioning rebuilds the `AUDIT_DETAILS`).
#[derive(Debug, Clone)]
pub struct StoredVersion {
    pub vo_id: Uuid,
    /// The `vo_version.kind` discriminator text (`COMPOSITION` / `EHR_STATUS` /
    /// `FOLDER` / …).
    pub kind: String,
    /// The owning EHR, or `None` for a demographic party (no EHR scope).
    pub ehr_id: Option<Uuid>,
    /// The per-vo storage commit ordinal.
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    pub preceding_version_uid: Option<String>,
    pub other_input_version_uids: Vec<String>,
    pub lifecycle_state: String,
    pub creating_system_id: String,
    pub contribution_id: Uuid,
    /// The version's `commit_audit` fields (master04 §Audit Details), flattened.
    pub audit_system_id: String,
    pub audit_change_type: String,
    pub audit_description: Option<String>,
    /// Canonical `PARTY_PROXY` of the committer.
    pub audit_committer: Value,
    /// Server-computed commit time (master06 §Committal).
    pub time_committed: jiff::Timestamp,
    pub template_id: Option<String>,
    pub signature: Option<String>,
    /// Reassembled canonical JSON, or [`Value::Null`] for a logically deleted
    /// version (no node rows — master06 §Logical Deletion).
    pub canonical: Value,
    /// `ORIGINAL_VERSION.attestations` in commit order (master06 §Attestation).
    pub attestations: Vec<Value>,
}

/// The `vo_version`⋈`audit` column list every version read selects, as a
/// compile-time string concatenation so each query stays a static literal
/// (`sqlx` 0.9 `SqlSafeStr` — no runtime SQL assembly).
macro_rules! version_select {
    ($tail:literal) => {
        concat!(
            "SELECT v.kind, v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, ",
            "v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, ",
            "v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, ",
            "a.system_id, a.change_type, a.description, a.committer, a.time_committed ",
            "FROM vo_version v JOIN audit a ON a.id = v.audit_id ",
            $tail
        )
    };
}

// ── Advisory lock ───────────────────────────────────────────────────────────

/// Take the per-vo transaction advisory lock that serializes concurrent writers
/// of one versioned object (so branch writers no longer all contend on one
/// current row). The versioning tree-placement decision calls this before it
/// reads the preceding version.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
// TODO(w3f-integrate): the preceding-version reads and next-ordinal/next-branch
// computation that surround this lock are the version-tree placement DECISION —
// they stay in the versioning layer (register 01 change.rs), which calls this
// lock first.
pub async fn advisory_lock(tx: &mut PgConnection, vo_id: Uuid) -> Result<(), StorageError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(vo_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

// ── audit + contribution ─────────────────────────────────────────────────────

/// Insert an `audit` row, returning its id and the server-computed
/// `time_committed` (master06 §Committal m3), captured via `RETURNING` so the
/// commit path can build the exact `ORIGINAL_VERSION` it will later serve — the
/// signed bytes must match the read-time canonical form.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_audit(
    tx: &mut PgConnection,
    audit: &AuditRow<'_>,
) -> Result<(Uuid, jiff::Timestamp), StorageError> {
    let row = sqlx::query(
        "INSERT INTO audit (system_id, change_type, description, committer) \
         VALUES ($1, $2, $3, $4) RETURNING id, time_committed",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(audit.description)
    .bind(audit.committer)
    .fetch_one(&mut *tx)
    .await?;
    let id: Uuid = row.try_get("id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((id, time_committed))
}

/// Insert an `audit` row with an **explicit** `time_committed` (RETURNING id
/// only) — used to preserve an imported `ORIGINAL_VERSION`'s original committal
/// time verbatim (the wrapped original is never modified, master06 §Copying).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_audit_at(
    tx: &mut PgConnection,
    audit: &AuditRow<'_>,
    time_committed: jiff::Timestamp,
) -> Result<Uuid, StorageError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO audit (system_id, change_type, description, committer, time_committed) \
         VALUES ($1, $2, $3, $4, $5::timestamptz) RETURNING id",
    )
    .bind(audit.system_id)
    .bind(audit.change_type)
    .bind(audit.description)
    .bind(audit.committer)
    .bind(time_committed.to_string())
    .fetch_one(&mut *tx)
    .await?)
}

/// Insert a `contribution` row referencing its audit, returning its id.
/// `ehr_id` is `None` for a demographic CONTRIBUTION (no EHR scope).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit_id: Uuid,
) -> Result<Uuid, StorageError> {
    insert_contribution_with_id(tx, ehr_id, audit_id, None).await
}

/// Insert the contribution row, honouring a client-supplied CONTRIBUTION uid
/// when given (ITS-REST `contribution_create`: "if the `uid` is supplied it must
/// not already be in use"); a duplicate id is a
/// [`StorageError::ContributionUidInUse`] conflict, never an overwrite.
///
/// # Errors
/// Returns [`StorageError::ContributionUidInUse`] on a duplicate uid, else
/// [`StorageError::Database`].
pub async fn insert_contribution_with_id(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit_id: Uuid,
    supplied: Option<Uuid>,
) -> Result<Uuid, StorageError> {
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO contribution (id, ehr_id, audit_id) \
         VALUES (COALESCE($1, uuidv7()), $2, $3) \
         ON CONFLICT (id) DO NOTHING RETURNING id",
    )
    .bind(supplied)
    .bind(ehr_id)
    .bind(audit_id)
    .fetch_optional(&mut *tx)
    .await?;
    inserted.ok_or(StorageError::ContributionUidInUse(supplied))
}

/// Insert an `audit` row and its enclosing `contribution`, returning the
/// contribution id, the audit id, and the audit's `time_committed` (for the
/// version's `commit_audit`, which is signed).
///
/// # Errors
/// Returns [`StorageError`] on a driver/insert failure.
pub async fn write_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit: &AuditRow<'_>,
) -> Result<(Uuid, Uuid, jiff::Timestamp), StorageError> {
    let (audit_id, time_committed) = insert_audit(tx, audit).await?;
    let contribution_id = insert_contribution(tx, ehr_id, audit_id).await?;
    Ok((contribution_id, audit_id, time_committed))
}

// ── vo_version writes ─────────────────────────────────────────────────────────

/// Insert one `vo_version` row opening at `now()` (validity `[now, ∞)`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_vo_version(
    tx: &mut PgConnection,
    row: &VersionRow<'_>,
) -> Result<(), StorageError> {
    let other_input = optional_json_array(row.other_input_version_uids);
    sqlx::query(
        "INSERT INTO vo_version \
         (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
          sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
          other_input_version_uids, contribution_id, audit_id, template_id, signature) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, tstzrange(now(), NULL, '[)'), \
                 $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(row.vo_id)
    .bind(row.kind)
    .bind(row.ehr_id)
    .bind(row.sys_version)
    .bind(row.trunk_version)
    .bind(row.branch_number)
    .bind(row.branch_version)
    .bind(row.lifecycle_state)
    .bind(row.creating_system_id)
    .bind(row.preceding_version_uid)
    .bind(other_input)
    .bind(row.contribution_id)
    .bind(row.audit_id)
    .bind(row.template_id)
    .bind(row.signature)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Insert one imported `vo_version` row with an explicit `sys_period`
/// (the import analogue of [`insert_vo_version`]; master06 §Copying). Stores
/// `template_id = NULL`.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_imported_vo_version(
    tx: &mut PgConnection,
    row: &ImportedVersionRow<'_>,
) -> Result<(), StorageError> {
    let other_input = optional_json_array(row.other_input_version_uids);
    sqlx::query(
        "INSERT INTO vo_version \
         (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
          sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
          other_input_version_uids, contribution_id, audit_id, template_id, signature) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, \
                 tstzrange($8::timestamptz, $9::timestamptz, '[)'), \
                 $10, $11, $12, $13, $14, $15, NULL, $16)",
    )
    .bind(row.vo_id)
    .bind(row.kind)
    .bind(row.ehr_id)
    .bind(row.sys_version)
    .bind(row.trunk_version)
    .bind(row.branch_number)
    .bind(row.branch_version)
    .bind(row.lower.to_string())
    .bind(row.upper.map(|t| t.to_string()))
    .bind(row.lifecycle_state)
    .bind(row.creating_system_id)
    .bind(row.preceding_version_uid)
    .bind(other_input)
    .bind(row.contribution_id)
    .bind(row.audit_id)
    .bind(row.signature)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// `other_input_version_uids` stores NULL when empty (`Is_merged_validity`),
/// else the JSON array.
fn optional_json_array(uids: &[String]) -> Option<Value> {
    if uids.is_empty() {
        None
    } else {
        Some(serde_json::json!(uids))
    }
}

/// Close (supersede) one specific version row — the lineage tip a new version
/// replaces — at `now()`. Lineage-precise: a branch commit closes its branch
/// tip, a trunk commit the trunk tip; a FORK closes nothing (master06 §Version
/// tree, realized by the temporal `sys_period`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/update failure.
pub async fn close_ordinal_at_now(
    tx: &mut PgConnection,
    vo_id: Uuid,
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

/// Close the open (`upper_inf`) version of one LINEAGE of `vo_id` at an explicit
/// instant (the import base time). The trunk lineage is `branch_number = 0`; a
/// branch lineage is one `(creating_system_id, trunk_version, branch_number)`.
/// Used when importing further versions into an existing container (master06
/// §Copying "previous copies have been made for the item").
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/update failure.
pub async fn close_lineage_at(
    tx: &mut PgConnection,
    vo_id: Uuid,
    lineage: &(String, i32, i32),
    at: jiff::Timestamp,
) -> Result<(), StorageError> {
    let (csid, trunk, branch) = lineage;
    if *branch == 0 {
        sqlx::query(
            "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), $2::timestamptz, '[)') \
             WHERE vo_id = $1 AND upper_inf(sys_period) AND branch_number = 0",
        )
        .bind(vo_id)
        .bind(at.to_string())
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), $2::timestamptz, '[)') \
             WHERE vo_id = $1 AND upper_inf(sys_period) \
             AND creating_system_id = $3 AND trunk_version = $4 AND branch_number = $5",
        )
        .bind(vo_id)
        .bind(at.to_string())
        .bind(csid)
        .bind(trunk)
        .bind(branch)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// The current state of a to-be-imported container in the target store —
/// `kind: None` when the container is not present (first receipt — master06
/// §Copying Case 2). Mapping the kind text to a versioned-object kind is the
/// caller's.
#[derive(Debug, Clone, Default)]
pub struct ContainerStateRow {
    /// The stored kind text, if the `vo_id` already exists.
    pub kind: Option<String>,
    /// The owning EHR of the existing container.
    pub owner: Option<Uuid>,
    /// The highest trunk version currently held.
    pub max_trunk: i32,
    /// The highest storage ordinal currently held.
    pub max_ordinal: i32,
    /// Whether a still-open current TRUNK version exists.
    pub trunk_open: bool,
}

/// Read the [`ContainerStateRow`] of a to-be-imported container.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn imported_container_state(
    tx: &mut PgConnection,
    vo_id: Uuid,
) -> Result<ContainerStateRow, StorageError> {
    let row = sqlx::query(
        "SELECT max(trunk_version) FILTER (WHERE branch_number = 0) AS max_trunk, \
                max(sys_version) AS max_ordinal, \
                bool_or(upper_inf(sys_period) AND branch_number = 0) AS trunk_open, \
                (array_agg(kind))[1] AS kind, \
                (array_agg(ehr_id))[1] AS owner \
         FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(&mut *tx)
    .await?;
    let max_ordinal: Option<i32> = row.try_get("max_ordinal")?;
    let Some(max_ordinal) = max_ordinal else {
        return Ok(ContainerStateRow::default());
    };
    Ok(ContainerStateRow {
        kind: row.try_get("kind")?,
        owner: row.try_get("owner")?,
        max_trunk: row.try_get::<Option<i32>, _>("max_trunk")?.unwrap_or(0),
        max_ordinal,
        trunk_open: row
            .try_get::<Option<bool>, _>("trunk_open")?
            .unwrap_or(false),
    })
}

// ── attestations ──────────────────────────────────────────────────────────────

/// Insert one `ATTESTATION` row for a version (master06 §Attestation). Stores
/// the completed canonical `ATTESTATION` verbatim in `data` (no synthetic
/// fields); `vo_attestation.time_committed` takes the transaction timestamp
/// (`now()`), equal to the `data.time_committed` stamped by the versioning
/// layer with the same commit-act time.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_attestation(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    contribution_id: Uuid,
    data: &Value,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO vo_attestation (vo_id, sys_version, contribution_id, data) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(vo_id)
    .bind(sys_version)
    .bind(contribution_id)
    .bind(data)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Load the `ATTESTATION`s attached to one version, in commit order (master06
/// §Attestation). Ordered by `time_committed, id`: attestations committed in the
/// same transaction share `now()`, so the `uuidv7()` `id` breaks ties in
/// insertion order.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn read_attestations(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<Vec<Value>, StorageError> {
    let rows = sqlx::query(
        "SELECT data FROM vo_attestation WHERE vo_id = $1 AND sys_version = $2 \
         ORDER BY time_committed, id",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok(row.try_get::<Value, _>("data")?))
        .collect()
}

// ── folder membership ─────────────────────────────────────────────────────────

/// Append a new folder-hierarchy membership row for an EHR (RM ehr master04
/// §Folders; RM ehr EHR class `Directory_in_folders`). `rank` is 1-based,
/// append-only and never reused: the next rank is `max(rank)+1` for this EHR.
/// Called once per FOLDER *creation*. No openEHR spec governs the `ehr_folder`
/// storage mechanism (our own design).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_ehr_folder_rank(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    vo_id: Uuid,
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

/// Write the contribution-outbox event row **inside the commit transaction** it
/// announces — no commit without its event, no event without its commit. No
/// openEHR spec governs eventing (our own extension). The PHI-free `envelope` is
/// built by the caller; storage only records the intent to publish.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
/// The PHI-free per-version entries are built by the versioning layer
/// (`Committed::envelope_entry`); this function wraps them in the fixed
/// envelope shape the events-extension drainer consumes
/// (`{contribution_id, ehr_id, committed_at, versions[]}`).
pub async fn write_outbox(
    tx: &mut PgConnection,
    contribution_id: Uuid,
    ehr_id: Option<Uuid>,
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

// ── version reads ─────────────────────────────────────────────────────────────

/// Build a [`StoredVersion`] from a `vo_version`⋈`audit` row, resolving the
/// canonical body through [`read_version_canonical`] (which yields
/// [`Value::Null`] for a logically deleted version — no node rows — so no
/// lifecycle branch is needed here).
async fn stored_version(
    pool: &PgPool,
    vo_id: Uuid,
    row: &PgRow,
) -> Result<StoredVersion, StorageError> {
    let sys_version: i32 = row.try_get("sys_version")?;
    let other_input_version_uids: Vec<String> = row
        .try_get::<Option<Value>, _>("other_input_version_uids")?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let canonical = read_version_canonical(pool, vo_id, sys_version).await?;
    let attestations = read_attestations(pool, vo_id, sys_version).await?;
    Ok(StoredVersion {
        vo_id,
        kind: row.try_get("kind")?,
        ehr_id: row.try_get("ehr_id")?,
        sys_version,
        trunk_version: row.try_get("trunk_version")?,
        branch_number: row.try_get("branch_number")?,
        branch_version: row.try_get("branch_version")?,
        preceding_version_uid: row.try_get("preceding_version_uid")?,
        other_input_version_uids,
        lifecycle_state: row.try_get("lifecycle_state")?,
        creating_system_id: row.try_get("creating_system_id")?,
        contribution_id: row.try_get("contribution_id")?,
        audit_system_id: row.try_get("system_id")?,
        audit_change_type: row.try_get("change_type")?,
        audit_description: row.try_get("description")?,
        audit_committer: row.try_get("committer")?,
        time_committed: row
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff(),
        template_id: row.try_get("template_id")?,
        signature: row.try_get("signature")?,
        canonical,
        attestations,
    })
}

/// Read the current TRUNK version of an object by id (any kind). `None` if it
/// never existed (`latest_trunk_version`, master06 §Versioned Objects). A
/// deleted current version returns with `canonical = Null` and its `523`
/// lifecycle so the caller can distinguish 404 from a deleted read.
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn read_current(
    pool: &PgPool,
    vo_id: Uuid,
) -> Result<Option<StoredVersion>, StorageError> {
    let Some(row) = sqlx::query(version_select!(
        "WHERE v.vo_id = $1 AND upper_inf(v.sys_period) AND v.branch_number = 0"
    ))
    .bind(vo_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row).await?))
}

/// Read a specific version by its STORAGE ORDINAL (`sys_version`) — for internal
/// callers that key rows by ordinal (the FHIR mapping table, extract export
/// iteration), never for wire version ids.
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn read_version_by_ordinal(
    pool: &PgPool,
    vo_id: Uuid,
    ordinal: i32,
) -> Result<Option<StoredVersion>, StorageError> {
    let Some(row) = sqlx::query(version_select!("WHERE v.vo_id = $1 AND v.sys_version = $2"))
    .bind(vo_id)
    .bind(ordinal)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row).await?))
}

/// Read a specific version by its `VERSION_TREE_ID` column ints (for
/// `.../version/{version_uid}` — trunk or branch; master05 §Syntaxes).
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn read_version(
    pool: &PgPool,
    vo_id: Uuid,
    trunk_version: i32,
    branch_number: i32,
    branch_version: i32,
) -> Result<Option<StoredVersion>, StorageError> {
    let Some(row) = sqlx::query(version_select!(
        "WHERE v.vo_id = $1 AND v.trunk_version = $2 \
         AND v.branch_number = $3 AND v.branch_version = $4"
    ))
    .bind(vo_id)
    .bind(trunk_version)
    .bind(branch_number)
    .bind(branch_version)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row).await?))
}

/// Read the version of an object current at a given instant (time-travel): the
/// row whose `sys_period` contains `at` (master08 §Change Management — any
/// previous state reconstructable). `None` if the object did not exist then.
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn version_at(
    pool: &PgPool,
    vo_id: Uuid,
    at: jiff::Timestamp,
) -> Result<Option<StoredVersion>, StorageError> {
    let Some(row) = sqlx::query(version_select!(
        "WHERE v.vo_id = $1 AND v.sys_period @> $2::timestamptz \
         AND v.branch_number = 0"
    ))
    .bind(vo_id)
    .bind(at.to_string())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row).await?))
}

// ── version-tree placement reads (the DECISION stays in versioning) ──────────

/// The preceding lineage-tip row read for the version-tree placement decision:
/// the addressed version (`expected = Some((t, b, v))`) or the current open
/// TRUNK tip (`expected = None`). Plain row values — the versioning layer maps
/// them onto its `PrecedingTip` (tree id + kind + lifecycle).
#[derive(Debug, Clone)]
pub struct TipRow {
    pub ehr_id: Option<Uuid>,
    pub kind: String,
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    pub creating_system_id: String,
    pub lifecycle_state: String,
    /// Whether the tip is still open (`upper_inf(sys_period)`).
    pub open: bool,
}

fn tip_row(row: &PgRow) -> Result<TipRow, StorageError> {
    Ok(TipRow {
        ehr_id: row.try_get("ehr_id")?,
        kind: row.try_get("kind")?,
        sys_version: row.try_get("sys_version")?,
        trunk_version: row.try_get("trunk_version")?,
        branch_number: row.try_get("branch_number")?,
        branch_version: row.try_get("branch_version")?,
        creating_system_id: row.try_get("creating_system_id")?,
        lifecycle_state: row.try_get("lifecycle_state")?,
        open: row.try_get("open")?,
    })
}

macro_rules! tip_select {
    ($tail:literal) => {
        concat!(
            "SELECT ehr_id, kind, sys_version, trunk_version, branch_number, ",
            "branch_version, creating_system_id, lifecycle_state, ",
            "upper_inf(sys_period) AS open FROM vo_version ",
            $tail
        )
    };
}

/// Read the preceding lineage tip: the version `expected` names (trunk or
/// branch), or the current open TRUNK tip when `expected` is `None`.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn lineage_tip(
    tx: &mut PgConnection,
    vo_id: Uuid,
    expected: Option<(i32, i32, i32)>,
) -> Result<Option<TipRow>, StorageError> {
    let row = match expected {
        None => {
            sqlx::query(tip_select!(
                "WHERE vo_id = $1 AND upper_inf(sys_period) AND branch_number = 0"
            ))
            .bind(vo_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        Some((t, b, v)) => {
            sqlx::query(tip_select!(
                "WHERE vo_id = $1 AND trunk_version = $2 AND branch_number = $3 \
                 AND branch_version = $4"
            ))
            .bind(vo_id)
            .bind(t)
            .bind(b)
            .bind(v)
            .fetch_optional(&mut *tx)
            .await?
        }
    };
    row.as_ref().map(tip_row).transpose()
}

/// The next storage commit ordinal for an object (`MAX(sys_version) + 1`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn next_ordinal(tx: &mut PgConnection, vo_id: Uuid) -> Result<i32, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(MAX(sys_version), 0) + 1 FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(&mut *tx)
    .await?)
}

/// The next branch number at a trunk fork point (`MAX(branch_number) + 1`
/// among the versions at `trunk_version`).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn next_branch_number(
    tx: &mut PgConnection,
    vo_id: Uuid,
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

/// The kind text of the current version of an object, or `None` if it does not
/// exist. Mapping to a versioned-object kind is the caller's.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn object_kind(pool: &PgPool, vo_id: Uuid) -> Result<Option<String>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT kind FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period) \
         AND branch_number = 0",
    )
    .bind(vo_id)
    .fetch_optional(pool)
    .await?)
}

/// Whether the EHR row exists (the CONTRIBUTION-commit `Pre_has_ehr` precheck
/// and the contribution-listing 404 distinction).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn ehr_exists(pool: &PgPool, ehr_id: Uuid) -> Result<bool, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
            .bind(ehr_id)
            .fetch_one(pool)
            .await?,
    )
}

// ── attestation target ────────────────────────────────────────────────────────

/// The row an attestation attaches to: the addressed `(vo_id, tree, kind)`
/// version's owner, ordinal and `creating_system_id`.
#[derive(Debug, Clone)]
pub struct AttestTargetRow {
    pub ehr_id: Option<Uuid>,
    pub sys_version: i32,
    pub creating_system_id: String,
}

/// Look up the target version of a `666|attestation|` item by its tree id and
/// kind text. `None` when no such version exists.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn attestation_target(
    tx: &mut PgConnection,
    vo_id: Uuid,
    tree: (i32, i32, i32),
    kind: &str,
) -> Result<Option<AttestTargetRow>, StorageError> {
    let (t, b, v) = tree;
    let row = sqlx::query(
        "SELECT ehr_id, sys_version, creating_system_id FROM vo_version \
         WHERE vo_id = $1 AND trunk_version = $2 AND branch_number = $3 \
         AND branch_version = $4 AND kind = $5",
    )
    .bind(vo_id)
    .bind(t)
    .bind(b)
    .bind(v)
    .bind(kind)
    .fetch_optional(&mut *tx)
    .await?;
    row.map(|row| -> Result<AttestTargetRow, StorageError> {
        Ok(AttestTargetRow {
            ehr_id: row.try_get("ehr_id")?,
            sys_version: row.try_get("sys_version")?,
            creating_system_id: row.try_get("creating_system_id")?,
        })
    })
    .transpose()
}

/// All `ATTESTATION`s of an object keyed by version ordinal, in commit order
/// (for `REVISION_HISTORY` assembly).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn read_attestations_all(
    pool: &PgPool,
    vo_id: Uuid,
) -> Result<Vec<(i32, Value)>, StorageError> {
    let rows = sqlx::query(
        "SELECT sys_version, data FROM vo_attestation WHERE vo_id = $1 \
         ORDER BY time_committed, id",
    )
    .bind(vo_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("sys_version")?, row.try_get("data")?)))
        .collect()
}

// ── history / metadata reads ──────────────────────────────────────────────────

/// One version's metadata row (`vo_version` ⋈ `audit`, no canonical body, no
/// attestations) — the lean list shape for `REVISION_HISTORY` and version
/// enumeration, where reassembling every body would be wasted work.
#[derive(Debug, Clone)]
pub struct VersionMeta {
    pub ehr_id: Option<Uuid>,
    pub kind: String,
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    pub creating_system_id: String,
    pub lifecycle_state: String,
    pub audit_system_id: String,
    pub audit_change_type: String,
    pub audit_description: Option<String>,
    pub audit_committer: Value,
    pub time_committed: jiff::Timestamp,
}

/// All version metadata rows of an object, ordered by storage ordinal.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn all_version_meta(pool: &PgPool, vo_id: Uuid) -> Result<Vec<VersionMeta>, StorageError> {
    let rows = sqlx::query(
        "SELECT v.ehr_id, v.kind, v.sys_version, v.trunk_version, v.branch_number, \
         v.branch_version, v.creating_system_id, v.lifecycle_state, \
         a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 ORDER BY v.sys_version",
    )
    .bind(vo_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(VersionMeta {
                ehr_id: row.try_get("ehr_id")?,
                kind: row.try_get("kind")?,
                sys_version: row.try_get("sys_version")?,
                trunk_version: row.try_get("trunk_version")?,
                branch_number: row.try_get("branch_number")?,
                branch_version: row.try_get("branch_version")?,
                creating_system_id: row.try_get("creating_system_id")?,
                lifecycle_state: row.try_get("lifecycle_state")?,
                audit_system_id: row.try_get("system_id")?,
                audit_change_type: row.try_get("change_type")?,
                audit_description: row.try_get("description")?,
                audit_committer: row.try_get("committer")?,
                time_committed: row
                    .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                    .to_jiff(),
            })
        })
        .collect()
}

/// The commit time of the earliest **held** version of an object
/// (`VERSIONED_OBJECT.time_created`; a latest-only import clone legitimately
/// starts above version 1). `None` when the object does not exist.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn time_created(
    pool: &PgPool,
    vo_id: Uuid,
) -> Result<Option<jiff::Timestamp>, StorageError> {
    Ok(sqlx::query_scalar::<_, jiff_sqlx::Timestamp>(
        "SELECT a.time_committed FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 ORDER BY v.sys_version LIMIT 1",
    )
    .bind(vo_id)
    .fetch_optional(pool)
    .await?
    .map(jiff_sqlx::Timestamp::to_jiff))
}

// ── contribution reads ────────────────────────────────────────────────────────

/// A CONTRIBUTION's own audit row (`contribution` ⋈ `audit`), flattened.
#[derive(Debug, Clone)]
pub struct ContributionAudit {
    pub system_id: String,
    pub change_type: String,
    pub description: Option<String>,
    pub committer: Value,
    pub time_committed: jiff::Timestamp,
}

/// Read a CONTRIBUTION's audit, scoped to its owning EHR. `None` when the
/// contribution does not exist in that EHR.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn contribution_audit(
    pool: &PgPool,
    contribution_id: Uuid,
    ehr_id: Uuid,
) -> Result<Option<ContributionAudit>, StorageError> {
    let Some(row) = sqlx::query(
        "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM contribution c JOIN audit a ON a.id = c.audit_id \
         WHERE c.id = $1 AND c.ehr_id = $2",
    )
    .bind(contribution_id)
    .bind(ehr_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(ContributionAudit {
        system_id: row.try_get("system_id")?,
        change_type: row.try_get("change_type")?,
        description: row.try_get("description")?,
        committer: row.try_get("committer")?,
        time_committed: row
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff(),
    }))
}

/// The versions a CONTRIBUTION affected: the rows it committed, unioned with
/// the rows its `666|attestation|` items attested (which add no new version) —
/// deduplicated. Returned as `(vo_id, (trunk, branch_number, branch_version),
/// creating_system_id, kind_text)`.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn contribution_version_refs(
    pool: &PgPool,
    contribution_id: Uuid,
) -> Result<Vec<(Uuid, (i32, i32, i32), String, String)>, StorageError> {
    let rows = sqlx::query(
        "SELECT vo_id, trunk_version, branch_number, branch_version, creating_system_id, \
         kind FROM vo_version \
         WHERE contribution_id = $1 \
         UNION \
         SELECT v.vo_id, v.trunk_version, v.branch_number, v.branch_version, \
         v.creating_system_id, v.kind FROM vo_version v \
         JOIN vo_attestation att ON att.vo_id = v.vo_id AND att.sys_version = v.sys_version \
         WHERE att.contribution_id = $1 \
         ORDER BY vo_id",
    )
    .bind(contribution_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get("vo_id")?,
                (
                    row.try_get("trunk_version")?,
                    row.try_get("branch_number")?,
                    row.try_get("branch_version")?,
                ),
                row.try_get("creating_system_id")?,
                row.try_get("kind")?,
            ))
        })
        .collect()
}

/// The ids of an EHR's CONTRIBUTIONs, oldest-first (audit `time_committed`,
/// then id), within the optional inclusive commit-time window, paged. A NULL
/// bound disables that side; a NULL LIMIT returns all rows.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn list_contributions(
    pool: &PgPool,
    ehr_id: Uuid,
    lower: Option<jiff::Timestamp>,
    upper: Option<jiff::Timestamp>,
    offset: i64,
    limit: Option<i64>,
) -> Result<Vec<Uuid>, StorageError> {
    let rows = sqlx::query(
        "SELECT c.id FROM contribution c JOIN audit a ON a.id = c.audit_id \
         WHERE c.ehr_id = $1 \
           AND ($2::timestamptz IS NULL OR a.time_committed >= $2::timestamptz) \
           AND ($3::timestamptz IS NULL OR a.time_committed <= $3::timestamptz) \
         ORDER BY a.time_committed, c.id \
         OFFSET $4 LIMIT $5",
    )
    .bind(ehr_id)
    .bind(lower.map(|t| t.to_string()))
    .bind(upper.map(|t| t.to_string()))
    .bind(offset)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter().map(|row| Ok(row.try_get("id")?)).collect()
}

/// The number of an EHR's CONTRIBUTIONs within the optional inclusive
/// commit-time window.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn count_contributions(
    pool: &PgPool,
    ehr_id: Uuid,
    lower: Option<jiff::Timestamp>,
    upper: Option<jiff::Timestamp>,
) -> Result<i64, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM contribution c JOIN audit a ON a.id = c.audit_id \
         WHERE c.ehr_id = $1 \
           AND ($2::timestamptz IS NULL OR a.time_committed >= $2::timestamptz) \
           AND ($3::timestamptz IS NULL OR a.time_committed <= $3::timestamptz)",
    )
    .bind(ehr_id)
    .bind(lower.map(|t| t.to_string()))
    .bind(upper.map(|t| t.to_string()))
    .fetch_one(pool)
    .await?)
}
