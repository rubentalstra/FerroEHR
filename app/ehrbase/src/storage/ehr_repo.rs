//! Row I/O for the `ehr` table and the `ehr_folder` membership index — the
//! per-EHR reads/writes the EHR service chapter needs that are not part of the
//! versioned-object spine ([`crate::storage::version_repo`]).
//!
//! No openEHR spec governs the `ehr` / `ehr_folder` schema — it is our own
//! PG18-native design (`docs/architecture.md` §Storage). The EHR concepts these
//! rows realize are arch-overview `master06-design_of_the_ehr.adoc` §The EHR
//! (root, `system_id`, `time_created`) and RM ehr `master04-ehr_package.adoc`
//! §EHR Creation / §Folders / §EHR Status. All *semantics* (subject sync
//! policy, directory-slot resolution, the `is_modifiable` write guard) stay in
//! the service layer, which calls these functions with plain inputs.

use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::storage::StorageError;

/// Insert the `ehr` root row (id + immutable `system_id`), no-op on a duplicate
/// id. Returns `Some(time_created)` — the server-assigned `EHR.time_created`
/// (arch-overview master06 §The EHR), captured via `RETURNING` so the create
/// path can build the `EHR` wire body without a follow-up `ehr` header read —
/// or `None` when the id already existed (the caller maps that to a 409).
/// `EHR.system_id` is recorded at creation and immutable thereafter
/// (arch-overview master06 §System Identity).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver/insert failure.
pub async fn insert_ehr(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    system_id: &str,
) -> Result<Option<jiff::Timestamp>, StorageError> {
    let row = sqlx::query(
        "INSERT INTO ehr (id, system_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING RETURNING time_created",
    )
    .bind(ehr_id)
    .bind(system_id)
    .fetch_optional(&mut *tx)
    .await?;
    match row {
        Some(row) => Ok(Some(
            row.try_get::<jiff_sqlx::Timestamp, _>("time_created")?
                .to_jiff(),
        )),
        None => Ok(None),
    }
}

/// The id of the EHR whose promoted subject columns match `(subject_id,
/// namespace)`, or `None`. Served from the unique `ehr.subject_*` columns (one
/// EHR per subject — RM ehr master04 §EHR Status).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn ehr_id_by_subject(
    pool: &PgPool,
    subject_id: &str,
    namespace: &str,
) -> Result<Option<Uuid>, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT id FROM ehr WHERE subject_id = $1 AND subject_namespace = $2")
            .bind(subject_id)
            .bind(namespace)
            .fetch_optional(pool)
            .await?,
    )
}

/// The `ehr` row header `(system_id, time_created)`, or `None` when the EHR does
/// not exist. `system_id` is the stored per-EHR value (immutable, arch-overview
/// master06 §System Identity), never the live config.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn ehr_header(
    pool: &PgPool,
    ehr_id: Uuid,
) -> Result<Option<(String, jiff::Timestamp)>, StorageError> {
    let Some(row) = sqlx::query("SELECT system_id, time_created FROM ehr WHERE id = $1")
        .bind(ehr_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let system_id: String = row.try_get("system_id")?;
    let time_created = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
        .to_jiff();
    Ok(Some((system_id, time_created)))
}

/// The LIVE folder-hierarchy ids of an EHR in `rank` order — the members of
/// `EHR.folders` (RM ehr, EHR class `Folders_valid`; RM ehr master04 §Folders).
/// "Live" = the current trunk version exists and is not logically deleted
/// (lifecycle `523`). Empty when the EHR indexes no live hierarchy.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn live_folder_hierarchies(
    pool: &PgPool,
    ehr_id: Uuid,
) -> Result<Vec<Uuid>, StorageError> {
    let rows = sqlx::query(
        "SELECT f.vo_id FROM ehr_folder f \
         JOIN vo_version v ON v.vo_id = f.vo_id \
         AND upper_inf(v.sys_period) AND v.branch_number = 0 \
         WHERE f.ehr_id = $1 AND v.lifecycle_state <> '523' \
         ORDER BY f.rank",
    )
    .bind(ehr_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(|r| Ok(r.try_get("vo_id")?)).collect()
}

/// The versioned-object id of the EHR's directory — `EHR.directory`
/// (`folders.item(1)`, RM ehr, EHR class `Directory_in_folders`). Resolved as
/// the lowest-`rank` LIVE hierarchy, falling back to the lowest-`rank`
/// still-existing one so a read after a logical delete resolves to the deleted
/// version (→ 204) rather than 404. `None` when the EHR indexes no hierarchy.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn directory_vo(pool: &PgPool, ehr_id: Uuid) -> Result<Option<Uuid>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT f.vo_id FROM ehr_folder f \
         JOIN vo_version v ON v.vo_id = f.vo_id \
         AND upper_inf(v.sys_period) AND v.branch_number = 0 \
         WHERE f.ehr_id = $1 \
         ORDER BY (v.lifecycle_state = '523'), f.rank \
         LIMIT 1",
    )
    .bind(ehr_id)
    .fetch_optional(pool)
    .await?)
}

/// Whether the EHR's current `EHR_STATUS` has `is_modifiable = true`, read from
/// the `EHR_STATUS` root node's canonical `data` fragment (`num = 0`, RM ehr
/// `EHR_STATUS.is_modifiable`). `None` when the EHR has no current `EHR_STATUS`
/// (the caller treats that as modifiable so the guard never spuriously blocks).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn ehr_is_modifiable(pool: &PgPool, ehr_id: Uuid) -> Result<Option<bool>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT (n.data->>'is_modifiable') = 'true' \
         FROM vo_version v \
         JOIN node n ON n.vo_id = v.vo_id AND n.sys_version = v.sys_version AND n.num = 0 \
         WHERE v.ehr_id = $1 AND v.kind = 'EHR_STATUS' AND upper_inf(v.sys_period) \
         AND v.branch_number = 0",
    )
    .bind(ehr_id)
    .fetch_optional(pool)
    .await?)
}

/// The two content-write pre-checks in ONE round trip: whether the EHR exists,
/// and whether its current `EHR_STATUS` has `is_modifiable = true`. Returns
/// `(exists, is_modifiable)` where `is_modifiable` is `None` when the EHR has no
/// current `EHR_STATUS` (the caller treats that as modifiable so the guard never
/// spuriously blocks) — identical to reading [`ehr_is_modifiable`] on its own.
/// The concepts guarded are RM ehr master04 §EHR Creation (existence) and §EHR
/// Active Status (`EHR_STATUS.is_modifiable`); collapsing the existence EXISTS
/// and the `is_modifiable` root-node read into one statement is our own design —
/// no openEHR spec governs the query shape.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn ehr_writability(
    pool: &PgPool,
    ehr_id: Uuid,
) -> Result<(bool, Option<bool>), StorageError> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1) AS ehr_exists, \
         (SELECT (n.data->>'is_modifiable') = 'true' \
            FROM vo_version v \
            JOIN node n ON n.vo_id = v.vo_id AND n.sys_version = v.sys_version AND n.num = 0 \
            WHERE v.ehr_id = $1 AND v.kind = 'EHR_STATUS' AND upper_inf(v.sys_period) \
            AND v.branch_number = 0) AS is_modifiable",
    )
    .bind(ehr_id)
    .fetch_one(pool)
    .await?;
    Ok((row.try_get("ehr_exists")?, row.try_get("is_modifiable")?))
}

/// Whether the EHR already has a LIVE folder hierarchy whose root carries the
/// given `archetype_node_id` AND name — the LOCATABLE identity pair that
/// distinguishes same-archetype siblings (RM common, paths: the name predicate
/// disambiguates same-archetype nodes). Backs the CONTRIBUTION-route
/// duplicate-directory rejection.
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn live_folder_root_exists(
    pool: &PgPool,
    ehr_id: Uuid,
    root_archetype_node_id: &str,
    root_name: &str,
) -> Result<bool, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM ehr_folder f \
           JOIN vo_version v ON v.vo_id = f.vo_id \
             AND upper_inf(v.sys_period) AND v.branch_number = 0 \
           JOIN node n ON n.vo_id = v.vo_id AND n.sys_version = v.sys_version \
             AND n.num = 0 \
           WHERE f.ehr_id = $1 AND v.lifecycle_state <> '523' \
             AND n.data->>'archetype_node_id' = $2 \
             AND n.data#>>'{name,value}' = $3)",
    )
    .bind(ehr_id)
    .bind(root_archetype_node_id)
    .bind(root_name)
    .fetch_one(pool)
    .await?)
}
