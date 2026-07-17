//! The full version reads: one `vo_version`⋈`audit` statement (attestations
//! folded in as a LATERAL aggregate) plus the node→canonical reassembly,
//! yielding the [`StoredVersion`] shape the versioning layer maps into a
//! `VERSION`/`ORIGINAL_VERSION`.
//!
//! No openEHR spec governs the SQL — our own design (`docs/architecture.md`
//! §Storage). The version-access semantics realized are RM common master06
//! (§Versioned Objects, §Logical Deletion) and master08 §Change Management
//! (time-travel).

use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;
use crate::storage::node_repo::read_version_canonical;

/// A loaded `vo_version`⋈`audit` row plus its reassembled content and
/// attestations — the storage read shape the versioning layer maps into a
/// `VERSION`/`ORIGINAL_VERSION`. The tree is returned as its three column ints;
/// the audit fields are flattened (versioning rebuilds the `AUDIT_DETAILS`).
#[derive(Debug, Clone)]
pub struct StoredVersion {
    pub vo_id: VoId,
    /// The `vo_version.kind` discriminator text (`COMPOSITION` / `EHR_STATUS` /
    /// `FOLDER` / …).
    pub kind: String,
    /// The owning EHR, or `None` for a demographic party (no EHR scope).
    pub ehr_id: Option<EhrId>,
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
///
/// The version's `ATTESTATION`s (RM common master06 §Attestation) are folded in
/// as an aggregated `attestations` jsonb column via a `LEFT JOIN LATERAL`, so
/// one statement carries the whole version read instead of a second round trip
/// per versioned read (empty in the common case → `[]`). The aggregate's
/// `ORDER BY time_committed, id` is the same commit order the per-object
/// attestation read
/// ([`crate::storage::version_repo::attestation::read_attestations_all`]) applies.
macro_rules! version_select {
    ($tail:literal) => {
        concat!(
            "SELECT v.kind, v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, ",
            "v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, ",
            "v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, ",
            "a.system_id, a.change_type, a.description, a.committer, a.time_committed, ",
            "att.attestations ",
            "FROM vo_version v JOIN audit a ON a.id = v.audit_id ",
            "LEFT JOIN LATERAL (",
            "SELECT coalesce(jsonb_agg(x.data ORDER BY x.time_committed, x.id), '[]'::jsonb) ",
            "AS attestations FROM vo_attestation x ",
            "WHERE x.vo_id = v.vo_id AND x.sys_version = v.sys_version",
            ") att ON true ",
            $tail
        )
    };
}

/// Build a [`StoredVersion`] from a `vo_version`⋈`audit` row, resolving the
/// canonical body through [`read_version_canonical`] (which yields
/// [`Value::Null`] for a logically deleted version — no node rows — so no
/// lifecycle branch is needed here).
async fn stored_version(
    pool: &PgPool,
    vo_id: VoId,
    row: &PgRow,
) -> Result<StoredVersion, StorageError> {
    let sys_version: i32 = row.try_get("sys_version")?;
    let other_input_version_uids: Vec<String> = row
        .try_get::<Option<Value>, _>("other_input_version_uids")?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let canonical = read_version_canonical(pool, vo_id, sys_version).await?;
    // The attestations arrive folded into the version-select row (the LATERAL
    // aggregate in `version_select!`), in commit order — no separate round trip.
    let attestations = row
        .try_get::<Value, _>("attestations")?
        .as_array()
        .cloned()
        .unwrap_or_default();
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
    vo_id: VoId,
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
    vo_id: VoId,
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
    vo_id: VoId,
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
    vo_id: VoId,
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
