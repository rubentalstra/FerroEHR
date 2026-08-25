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
//! - a **write** always thaws first ([`thaw`] on the admin restore path; the
//!   commit path's thaw rides the merged placement read,
//!   `crate::storage::version_repo::placement::next_placement`), so a
//!   versioned object is never split across tiers;
//! - a **read** goes through the `*_all` union views (`vo_version_all` /
//!   `node_all` / `vo_attestation_all`), so ONE statement serves both tiers
//!   and a miss never pays a retry transaction.

use sqlx::PgConnection;

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;

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
