// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Reading and writing `linkage.party_ehr`.
//!
//! **No openEHR spec governs this — our own design/extension.** Five
//! statements, all of them scoped to the mapping IN FORCE (`upper_inf`) except
//! the close, which is what turns an in-force mapping into a historical one.
//! Nothing here deletes: a merge or a split closes a period and opens the
//! next, so "which party was the subject of this EHR when that composition was
//! written" stays answerable (GDPR Art. 5(1)(d) accuracy is served by
//! correcting forward, not by erasing the record of what was believed).
//!
//! Every statement names its relations unqualified and resolves through the
//! linkage pool's `search_path`, which carries neither `ehr` nor
//! `demographic`. A statement here therefore cannot reach either domain even
//! if it tried, and the tenant predicate is the table's own row policy rather
//! than a `WHERE` clause a caller could forget.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};

/// `SQLSTATE` 23P01 `exclusion_violation` — what `PostgreSQL` reports when a
/// key carrying `WITHOUT OVERLAPS` is violated, that key being enforced by a
/// `GiST` exclusion index (`PostgreSQL` 18 docs § Appendix A "`PostgreSQL`
/// Error Codes", class 23,
/// <https://www.postgresql.org/docs/18/errcodes-appendix.html>).
const SQLSTATE_EXCLUSION_VIOLATION: &str = "23P01";

/// Whether `PostgreSQL` refused this statement because the party already holds
/// a mapping in force.
///
/// The temporal primary key is the enforcement, so the refusal arrives as a
/// driver error and is classified here once, rather than every caller reading
/// `SQLSTATE`s.
pub(super) fn is_overlap(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|code| code == SQLSTATE_EXCLUSION_VIOLATION)
}

/// The EHR `party` is the subject of right now, or `None`.
///
/// # Errors
/// The driver error, when the read fails.
pub(super) async fn open_mapping(pool: &PgPool, party: VoId) -> Result<Option<EhrId>, sqlx::Error> {
    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT ehr_id FROM party_ehr WHERE party_id = $1 AND upper_inf(sys_period)",
    )
    .bind(party.0)
    .fetch_optional(pool)
    .await?;
    Ok(found.map(EhrId))
}

/// Open a mapping from `party` to `ehr`, starting now.
///
/// # Errors
/// The driver error, including the `23P01` refusal when `party` already holds
/// a mapping in force ([`is_overlap`]).
pub(super) async fn open(
    conn: &mut PgConnection,
    party: VoId,
    ehr: EhrId,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO party_ehr (party_id, ehr_id) VALUES ($1, $2)")
        .bind(party.0)
        .bind(ehr.0)
        .execute(conn)
        .await?;
    Ok(())
}

/// Close `party`'s mapping in force and return the EHR it named, or `None`
/// when the party holds none.
///
/// The upper bound is `now()` — the TRANSACTION timestamp, so the closing row
/// and the successor the same transaction opens meet at one instant and the
/// half-open periods do not overlap.
///
/// # Errors
/// The driver error, when the update fails.
pub(super) async fn close(
    conn: &mut PgConnection,
    party: VoId,
) -> Result<Option<EhrId>, sqlx::Error> {
    let closed: Option<Uuid> = sqlx::query_scalar(
        "UPDATE party_ehr SET sys_period = tstzrange(lower(sys_period), now(), '[)') \
         WHERE party_id = $1 AND upper_inf(sys_period) RETURNING ehr_id",
    )
    .bind(party.0)
    .fetch_optional(conn)
    .await?;
    Ok(closed.map(EhrId))
}

/// Every EHR the `parties` are the subject of right now, distinct.
///
/// The batch form of [`open_mapping`], and the crossing step of a cohort query:
/// identifiers in, identifiers out, in one round trip. An empty input makes no
/// round trip at all — an `= ANY('{}')` would be a statement whose answer is
/// already known.
///
/// # Errors
/// The driver error, when the read fails.
pub(super) async fn open_mappings(
    pool: &PgPool,
    parties: &[VoId],
) -> Result<Vec<EhrId>, sqlx::Error> {
    if parties.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = parties.iter().map(|party| party.0).collect();
    let found: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT ehr_id FROM party_ehr WHERE party_id = ANY($1) AND upper_inf(sys_period)",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    Ok(found.into_iter().map(EhrId).collect())
}
