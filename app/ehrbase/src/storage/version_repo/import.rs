//! The import write path: `vo_version` rows with an **explicit** `sys_period`
//! — the EHR Extract import (master06 §Copying) and the admin archive load —
//! plus the lineage close and container-state read the import policy needs.
//!
//! No openEHR spec governs the SQL — our own design (`docs/architecture.md`
//! §Storage). The import *policy* (period-chain synthesis, Case 2/3
//! classification) lives in the versioning layer.

use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::storage::error::StorageError;
use crate::storage::version_repo::optional_json_array;

/// One `vo_version` row to insert with an **explicit** `sys_period`
/// (`[lower, upper)`, `upper = None` ⇒ still-open) — the import analogue of the
/// local commit row, carrying no `template_id`. The import path builds a
/// synthetic strictly-increasing local period chain per lineage (master06
/// §Copying).
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

/// Insert one imported `vo_version` row with an explicit `sys_period`
/// (master06 §Copying). Stores `template_id = NULL`.
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

/// One `vo_version` row to re-persist **verbatim** during an archive load — the
/// stored columns (`sys_period` bounds as ISO strings, `template_id`,
/// `creating_system_id`) are preserved exactly as dumped (SM `I_ADMIN_DUMP_LOAD`
/// round-trip; no openEHR spec governs the archive). Unlike
/// [`ImportedVersionRow`] it keeps `template_id`, and unlike a local commit row
/// the `sys_period` is explicit.
#[derive(Debug)]
pub struct VerbatimVersionRow<'a> {
    pub vo_id: Uuid,
    pub kind: &'a str,
    pub ehr_id: Uuid,
    pub sys_version: i32,
    pub trunk_version: i32,
    pub branch_number: i32,
    pub branch_version: i32,
    pub preceding_version_uid: Option<&'a str>,
    pub other_input_version_uids: Option<&'a Value>,
    /// Lower/upper `sys_period` bounds as ISO-8601 strings (`upper = None` ⇒
    /// still-open); bound with a `::timestamptz` cast.
    pub sys_period_lower: Option<&'a str>,
    pub sys_period_upper: Option<&'a str>,
    pub lifecycle_state: &'a str,
    pub contribution_id: Uuid,
    pub audit_id: Uuid,
    pub template_id: Option<&'a str>,
    pub signature: Option<&'a str>,
    pub creating_system_id: &'a str,
}

/// Insert one `vo_version` row verbatim from an archive record (explicit
/// `sys_period` + preserved `template_id`) — the load side of the admin
/// dump/load round-trip. The node rows are re-decomposed and written by the
/// caller.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_version_verbatim(
    tx: &mut PgConnection,
    row: &VerbatimVersionRow<'_>,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, \
         branch_version, preceding_version_uid, other_input_version_uids, sys_period, \
         lifecycle_state, contribution_id, audit_id, template_id, signature, creating_system_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, \
         tstzrange($10::timestamptz, $11::timestamptz, '[)'), $12, $13, $14, $15, $16, $17)",
    )
    .bind(row.vo_id)
    .bind(row.kind)
    .bind(row.ehr_id)
    .bind(row.sys_version)
    .bind(row.trunk_version)
    .bind(row.branch_number)
    .bind(row.branch_version)
    .bind(row.preceding_version_uid)
    .bind(row.other_input_version_uids)
    .bind(row.sys_period_lower)
    .bind(row.sys_period_upper)
    .bind(row.lifecycle_state)
    .bind(row.contribution_id)
    .bind(row.audit_id)
    .bind(row.template_id)
    .bind(row.signature)
    .bind(row.creating_system_id)
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
