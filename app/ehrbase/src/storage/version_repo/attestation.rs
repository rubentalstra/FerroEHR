//! `vo_attestation` row I/O: attach an `ATTESTATION` to a stored version, look
//! up the version an attestation item addresses, and enumerate an object's
//! attestations for `REVISION_HISTORY` assembly.
//!
//! No openEHR spec governs the SQL — our own design (`docs/architecture.md`
//! §Storage). The attestation semantics realized are RM common master06
//! §Attestation; the completed canonical `ATTESTATION` is stored verbatim.

use serde_json::Value;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::storage::error::StorageError;

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
/// (for `REVISION_HISTORY` assembly). Ordered by `time_committed, id`:
/// attestations committed in the same transaction share `now()`, so the
/// `uuidv7()` `id` breaks ties in insertion order.
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
