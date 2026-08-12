// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The cold archival storage tier: the physical move, its reverse, and the
//! marker-gated read fallback.
//!
//! No openEHR spec governs storage tiering — our own design/extension. The SM
//! operation it realizes is `I_ADMIN_ARCHIVE`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc`: "Move
//! selected EHRs to archival storage"); the `cold` schema mirrors of
//! `vo_version` / `node` / `vo_attestation` are where "archival storage"
//! physically is.
//!
//! Two rules keep the tier invisible on the wire:
//!
//! - a **write** always thaws first ([`thaw`]), so a versioned object is never
//!   split across tiers;
//! - a **read** consults the cold tier only after the primary tier misses
//!   (`on_cold`), so an unarchived read pays nothing at all.
//!
//! `on_cold` runs the caller's own statement — verbatim, no cold twin of the
//! SQL — inside a transaction whose `SET LOCAL search_path` resolves the
//! versioned-object spine to `cold`
//! (<https://www.postgresql.org/docs/18/sql-set.html>: `SET LOCAL` reverts at
//! transaction end, so the pooled connection is never left altered).

use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;

/// Search path resolving the versioned-object spine to the cold archival tier
/// while `audit` / `ehr` / the `ext` helpers keep resolving normally.
const COLD_SEARCH_PATH_SQL: &str = "SET LOCAL search_path TO cold, ehr, ext, public";

/// Which storage tier a read addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// The primary tier — every unarchived object, and the only tier a hot read
    /// touches.
    Primary,
    /// The cold archival tier alone — used on a primary-tier miss, once the
    /// version row itself has been found there.
    Cold,
    /// Both tiers at once, through the `*_all` union views — for the
    /// whole-repository readers (admin export, physical delete) that must see
    /// archived content by definition.
    Both,
}

/// Begins a transaction whose statements read the cold archival tier.
///
/// The caller runs its ordinary primary-tier SQL on the returned transaction
/// and commits it; every unqualified `vo_version` / `node` / `vo_attestation`
/// reference resolves to the `cold` mirror instead.
///
/// # Errors
/// Returns [`StorageError::Database`] when the transaction or the `SET LOCAL`
/// fails.
pub async fn begin_cold(pool: &PgPool) -> Result<Transaction<'_, Postgres>, StorageError> {
    let mut tx = pool.begin().await?;
    sqlx::query(COLD_SEARCH_PATH_SQL).execute(&mut *tx).await?;
    Ok(tx)
}

/// Runs one read statement against the cold archival tier.
///
/// The body is the SAME statement the primary-tier read uses — there is no
/// second copy of the SQL to drift — evaluated on a [`begin_cold`] transaction
/// that is committed before the value is returned.
///
/// # Examples
///
/// ```text
/// let kind: Option<String> = on_cold!(pool, |c| sqlx::query_scalar(SQL)
///     .bind(vo_id)
///     .fetch_optional(&mut *c)
///     .await);
/// ```
macro_rules! on_cold {
    ($pool:expr, |$conn:ident| $body:expr) => {{
        // Boxed so the archival retry's transaction state lives on the heap
        // instead of widening every calling future — the read seams sit deep
        // inside the request futures `clippy::large_futures` measures.
        let cold = ::std::boxed::Box::pin(async {
            let mut tx = $crate::storage::version_repo::tier::begin_cold($pool).await?;
            let $conn = &mut *tx;
            let outcome = $body;
            tx.commit().await?;
            ::std::result::Result::<_, $crate::storage::error::StorageError>::Ok(outcome?)
        });
        cold.await?
    }};
}

pub(crate) use on_cold;

/// Moves every row of `vo_ids` from the primary tier to the cold archival tier.
///
/// Runs inside the caller's transaction so the move is atomic with the
/// `vo_archive` markers that record it. Content and attestations are copied
/// before the version rows are deleted, because the primary `node` /
/// `vo_attestation` foreign keys cascade off `vo_version`.
///
/// Objects already in the cold tier select nothing and are silently skipped, so
/// re-archiving is idempotent.
///
/// # Errors
/// Returns [`StorageError::Database`] on any driver/statement failure.
pub async fn freeze(tx: &mut PgConnection, vo_ids: &[VoId]) -> Result<(), StorageError> {
    if vo_ids.is_empty() {
        return Ok(());
    }
    for sql in [
        "INSERT INTO cold.vo_version SELECT * FROM vo_version WHERE vo_id = ANY($1)",
        "INSERT INTO cold.node SELECT * FROM node WHERE vo_id = ANY($1)",
        "INSERT INTO cold.vo_attestation SELECT * FROM vo_attestation WHERE vo_id = ANY($1)",
        // Cascades the primary `node` + `vo_attestation` rows away.
        "DELETE FROM vo_version WHERE vo_id = ANY($1)",
    ] {
        sqlx::query(sql).bind(vo_ids).execute(&mut *tx).await?;
    }
    Ok(())
}

/// Moves every row of `vo_ids` back from the cold archival tier to the primary
/// tier and drops their archive markers — the exact reverse of [`freeze`].
///
/// Version rows are restored first: the primary `node` / `vo_attestation`
/// foreign keys reference them.
///
/// # Errors
/// Returns [`StorageError::Database`] on any driver/statement failure.
pub async fn thaw(tx: &mut PgConnection, vo_ids: &[VoId]) -> Result<(), StorageError> {
    if vo_ids.is_empty() {
        return Ok(());
    }
    for sql in [
        "INSERT INTO vo_version SELECT * FROM cold.vo_version WHERE vo_id = ANY($1)",
        "INSERT INTO node SELECT * FROM cold.node WHERE vo_id = ANY($1)",
        "INSERT INTO vo_attestation SELECT * FROM cold.vo_attestation WHERE vo_id = ANY($1)",
        "DELETE FROM cold.node WHERE vo_id = ANY($1)",
        "DELETE FROM cold.vo_attestation WHERE vo_id = ANY($1)",
        "DELETE FROM cold.vo_version WHERE vo_id = ANY($1)",
        "DELETE FROM vo_archive WHERE vo_id = ANY($1)",
    ] {
        sqlx::query(sql).bind(vo_ids).execute(&mut *tx).await?;
    }
    Ok(())
}

/// Thaws one versioned object before it is written to, so a new version never
/// lands in the primary tier while its predecessors sit in the cold one.
///
/// A no-op (three primary-key probes finding nothing) for the overwhelmingly
/// common unarchived case; the whole thaw is folded into ONE statement so the
/// write path pays a single round trip for the guarantee. The data-modifying
/// `WITH` clauses all run in the same statement, so the deferred `node` →
/// `vo_version` foreign key is satisfied at statement end
/// (<https://www.postgresql.org/docs/18/queries-with.html>).
///
/// # Errors
/// Returns [`StorageError::Database`] on a driver failure.
pub async fn thaw_one(tx: &mut PgConnection, vo_id: VoId) -> Result<(), StorageError> {
    sqlx::query(
        "WITH v AS (DELETE FROM cold.vo_version WHERE vo_id = $1 RETURNING *), \
              n AS (DELETE FROM cold.node WHERE vo_id = $1 RETURNING *), \
              t AS (DELETE FROM cold.vo_attestation WHERE vo_id = $1 RETURNING *), \
              m AS (DELETE FROM vo_archive WHERE vo_id = $1), \
              iv AS (INSERT INTO vo_version SELECT * FROM v), \
              inn AS (INSERT INTO node SELECT * FROM n) \
         INSERT INTO vo_attestation SELECT * FROM t",
    )
    .bind(vo_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Deletes every cold-tier row of one EHR, plus the archive markers of the
/// objects removed.
///
/// The tier's half of a physical EHR delete, which cannot reach it by cascade
/// (the mirrors are foreign-key-free by design).
///
/// # Errors
/// Returns [`StorageError::Database`] on any driver/statement failure.
pub async fn purge_ehrs(tx: &mut PgConnection, ehr_ids: &[EhrId]) -> Result<(), StorageError> {
    if ehr_ids.is_empty() {
        return Ok(());
    }
    for sql in [
        "DELETE FROM vo_archive WHERE vo_id IN \
         (SELECT vo_id FROM cold.vo_version WHERE ehr_id = ANY($1))",
        "DELETE FROM cold.vo_attestation WHERE vo_id IN \
         (SELECT vo_id FROM cold.vo_version WHERE ehr_id = ANY($1))",
        "DELETE FROM cold.node WHERE ehr_id = ANY($1)",
        "DELETE FROM cold.vo_version WHERE ehr_id = ANY($1)",
    ] {
        sqlx::query(sql).bind(ehr_ids).execute(&mut *tx).await?;
    }
    Ok(())
}

/// Deletes every cold-tier row of the named versioned objects, plus their
/// archive markers — the tier's half of a physical PARTY delete.
///
/// # Errors
/// Returns [`StorageError::Database`] on any driver/statement failure.
pub async fn purge_vos(tx: &mut PgConnection, vo_ids: &[VoId]) -> Result<(), StorageError> {
    if vo_ids.is_empty() {
        return Ok(());
    }
    for sql in [
        "DELETE FROM cold.vo_attestation WHERE vo_id = ANY($1)",
        "DELETE FROM cold.node WHERE vo_id = ANY($1)",
        "DELETE FROM cold.vo_version WHERE vo_id = ANY($1)",
        "DELETE FROM vo_archive WHERE vo_id = ANY($1)",
    ] {
        sqlx::query(sql).bind(vo_ids).execute(&mut *tx).await?;
    }
    Ok(())
}
