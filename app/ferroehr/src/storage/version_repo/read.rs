// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The full version reads.
//!
//! One `vo_version`⋈`audit` statement (attestations folded in as a LATERAL
//! aggregate) plus the node→canonical reassembly, yielding the
//! [`StoredVersion`] shape the versioning layer maps into a
//! `VERSION`/`ORIGINAL_VERSION`.
//!
//! No openEHR spec governs the SQL — our own design. The version-access semantics realized are RM common master06
//! (§Versioned Objects, §Logical Deletion) and master08 §Change Management
//! (time-travel).
//!
//! NOTE (no openEHR spec governs storage tiering — our own design): each read
//! retries against the cold archival tier
//! ([`crate::storage::version_repo::tier`]) ONLY when the primary tier has no
//! such version, so an archived object stays retrievable while an unarchived
//! read is untouched.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 1): stored canonical fragments — a typed \
              round-trip drops forward-compatible keys (the openEHR release strategy: minors are compatible supersets)"
)]

use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::storage::error::StorageError;
use crate::storage::node_repo::read_version_canonical_in;
use crate::storage::version_repo::tier::{Tier, on_cold};

/// A loaded `vo_version`⋈`audit` row plus its reassembled content and
/// attestations — the storage read shape the versioning layer maps into a
/// `VERSION`/`ORIGINAL_VERSION`.
///
/// The tree is returned as its three column ints; the audit fields are
/// flattened (versioning rebuilds the `AUDIT_DETAILS`).
#[derive(Debug, Clone)]
pub struct StoredVersion {
    /// The versioned object's id.
    pub vo_id: VoId,
    /// The `vo_version.kind` discriminator text (`COMPOSITION` / `EHR_STATUS` /
    /// `FOLDER` / …).
    pub kind: String,
    /// The owning EHR, or `None` for a demographic party (no EHR scope).
    pub ehr_id: Option<EhrId>,
    /// The per-vo storage commit ordinal.
    pub sys_version: i32,
    /// `VERSION_TREE_ID` first part.
    pub trunk_version: i32,
    /// `VERSION_TREE_ID` second part; `0` on a trunk row.
    pub branch_number: i32,
    /// `VERSION_TREE_ID` third part; `0` on a trunk row.
    pub branch_version: i32,
    /// `ORIGINAL_VERSION.preceding_version_uid`; `None` for a first version.
    pub preceding_version_uid: Option<String>,
    /// The merge provenance (`other_input_version_uids`); empty when not a merge.
    pub other_input_version_uids: Vec<String>,
    /// The `version_lifecycle_state` numeric code.
    pub lifecycle_state: String,
    /// The version's immutable `creating_system_id`.
    pub creating_system_id: String,
    /// The CONTRIBUTION this version was committed in — on an imported row the
    /// LOCAL import CONTRIBUTION (RM common master06 §Committal and Audits).
    pub contribution_id: Uuid,
    /// The version's `commit_audit` fields (master04 §Audit Details), flattened.
    pub audit_system_id: String,
    /// The version audit's numeric `audit_change_type` group code.
    pub audit_change_type: String,
    /// The canonical `DV_TEXT` fragment of the version audit's description,
    /// when the committer supplied one.
    pub audit_description: Option<Value>,
    /// Canonical `PARTY_PROXY` of the committer.
    pub audit_committer: Value,
    /// The canonical fragment of the `ATTESTATION`-declared attributes when the
    /// commit audit is an `ATTESTATION` (RM common master06 §Attestation).
    pub audit_attestation: Option<Value>,
    /// Server-computed commit time (master06 §Committal).
    pub time_committed: jiff::Timestamp,
    /// The OPT `template_id` a COMPOSITION was committed against (else `None`).
    pub template_id: Option<String>,
    /// `VERSION.signature` (0..1), opaque radix-64 — on an imported row the
    /// `IMPORTED_VERSION` wrapper's own.
    pub signature: Option<String>,
    /// Whether `signature` was supplied verbatim by the client (foreign — never
    /// re-verified at read; master06 §Digital Signature) rather than generated
    /// by this server.
    pub signature_client_supplied: bool,
    /// On an `IMPORTED_VERSION` row, the wrapped `ORIGINAL_VERSION`'s own
    /// `{contribution, commit_audit, signature?}` fragment; `None` on a locally
    /// created version (master06 §Committal and Audits).
    pub wrapped_original: Option<Value>,
    /// Reassembled canonical JSON, or [`Value::Null`] for a logically deleted
    /// version (no node rows — master06 §Logical Deletion).
    pub canonical: Value,
    /// The `ORIGINAL_VERSION.attestations` that were on the version AT the act
    /// of committal, in commit order (master06 §Attestation "Signing content at
    /// committal") — the ones inside the version's signed canonical form.
    pub attestations_at_committal: Vec<Value>,
    /// The `ORIGINAL_VERSION.attestations` added AFTER committal, in commit
    /// order (master06 §Attestation: "Attestations can be added at any time
    /// after committal of the content being attested") — outside the signed
    /// canonical form.
    pub attestations_after_committal: Vec<Value>,
}

/// The `vo_version`⋈`audit` column list every version read selects, as a
/// compile-time string concatenation so each query stays a static literal
/// (`sqlx` 0.9 `SqlSafeStr` — no runtime SQL assembly).
///
/// The version's `ATTESTATION`s (RM common master06 §Attestation) are folded in
/// as aggregated jsonb columns via a `LEFT JOIN LATERAL`, so one statement
/// carries the whole version read instead of a second round trip per versioned
/// read (empty in the common case → `[]`). The aggregates'
/// `ORDER BY time_committed, id` is the same commit order the per-object
/// attestation read
/// ([`crate::storage::version_repo::attestation::read_attestations_all`]) applies.
///
/// They arrive SPLIT on `at_committal`, because the two halves stand in
/// different relations to `VERSION.signature`: an attestation present at the act
/// of committal is inside the version's signed canonical form ("serialising the
/// entire Version object", master06 §Digital Signature), while one added later
/// ("Attestations can be added at any time after committal", §Attestation) is
/// not. The split uses the standard aggregate `FILTER` clause
/// (<https://www.postgresql.org/docs/18/sql-expressions.html#SYNTAX-AGGREGATES>).
macro_rules! version_select {
    ($tail:literal) => {
        concat!(
            "SELECT v.kind, v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, ",
            "v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, ",
            "v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, ",
            "v.signature_client_supplied, v.wrapped_original, ",
            "a.system_id, a.change_type, a.description, a.committer, a.attestation, ",
            "a.time_committed, ",
            "att.attestations_at_committal, att.attestations_after_committal ",
            "FROM vo_version v JOIN audit a ON a.id = v.audit_id ",
            "LEFT JOIN LATERAL (",
            "SELECT coalesce(jsonb_agg(x.data ORDER BY x.time_committed, x.id) ",
            "FILTER (WHERE x.at_committal), '[]'::jsonb) AS attestations_at_committal, ",
            "coalesce(jsonb_agg(x.data ORDER BY x.time_committed, x.id) ",
            "FILTER (WHERE NOT x.at_committal), '[]'::jsonb) AS attestations_after_committal ",
            "FROM vo_attestation x ",
            "WHERE x.vo_id = v.vo_id AND x.sys_version = v.sys_version",
            ") att ON true ",
            $tail
        )
    };
}

/// Build a [`StoredVersion`] from a `vo_version`⋈`audit` row, resolving the
/// canonical body through [`read_version_canonical_in`] (which yields
/// [`Value::Null`] for a logically deleted version — no node rows — so no
/// lifecycle branch is needed here).
///
/// `tier` says which tier the row came from, so the body is reassembled from
/// the same one ([`crate::storage::version_repo::tier`]).
async fn stored_version(
    pool: &PgPool,
    vo_id: VoId,
    row: &PgRow,
    tier: Tier,
) -> Result<StoredVersion, StorageError> {
    let sys_version: i32 = row.try_get("sys_version")?;
    // A stored `other_input_version_uids` that does not decode is OUR data
    // gone wrong (the merge inputs of an IMPORTED_VERSION, RM common master06
    // §Distributed Versioning) — serving the version with an empty merge list
    // would answer wrongly rather than loudly, so the decode failure is a codec
    // fault, not a default.
    let other_input_version_uids: Vec<String> = row
        .try_get::<Option<Value>, _>("other_input_version_uids")?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| {
            StorageError::InvalidRows(format!(
                "vo_version.other_input_version_uids of {vo_id} is not a list of \
                 version uids: {e}"
            ))
        })?
        .unwrap_or_default();
    let canonical = read_version_canonical_in(pool, vo_id, sys_version, tier).await?;
    // The attestations arrive folded into the version-select row (the LATERAL
    // aggregates in `version_select!`), in commit order and already split on
    // `at_committal` — no separate round trip.
    let attestation_list = |column: &str| -> Result<Vec<Value>, StorageError> {
        Ok(row
            .try_get::<Value, _>(column)?
            .as_array()
            .cloned()
            .unwrap_or_default())
    };
    let attestations_at_committal = attestation_list("attestations_at_committal")?;
    let attestations_after_committal = attestation_list("attestations_after_committal")?;
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
        audit_attestation: row.try_get("attestation")?,
        time_committed: row
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff(),
        template_id: row.try_get("template_id")?,
        signature: row.try_get("signature")?,
        signature_client_supplied: row.try_get("signature_client_supplied")?,
        wrapped_original: row.try_get("wrapped_original")?,
        canonical,
        attestations_at_committal,
        attestations_after_committal,
    })
}

/// Read the current TRUNK version of an object by id (any kind).
///
/// `None` if it never existed (`latest_trunk_version`, master06 §Versioned
/// Objects). A deleted current version returns with `canonical = Null` and
/// its `523` lifecycle so the caller can distinguish 404 from a deleted read.
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn read_current(
    pool: &PgPool,
    vo_id: VoId,
) -> Result<Option<StoredVersion>, StorageError> {
    const SQL: &str =
        version_select!("WHERE v.vo_id = $1 AND upper_inf(v.sys_period) AND v.branch_number = 0");
    let primary = sqlx::query(SQL).bind(vo_id).fetch_optional(pool).await?;
    if let Some(row) = primary {
        return Ok(Some(
            stored_version(pool, vo_id, &row, Tier::Primary).await?,
        ));
    }
    let Some(row) = on_cold!(pool, |conn| sqlx::query(SQL)
        .bind(vo_id)
        .fetch_optional(&mut *conn)
        .await)
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row, Tier::Cold).await?))
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
    const SQL: &str = version_select!("WHERE v.vo_id = $1 AND v.sys_version = $2");
    let primary = sqlx::query(SQL)
        .bind(vo_id)
        .bind(ordinal)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = primary {
        return Ok(Some(
            stored_version(pool, vo_id, &row, Tier::Primary).await?,
        ));
    }
    let Some(row) = on_cold!(pool, |conn| sqlx::query(SQL)
        .bind(vo_id)
        .bind(ordinal)
        .fetch_optional(&mut *conn)
        .await)
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row, Tier::Cold).await?))
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
    const SQL: &str = version_select!(
        "WHERE v.vo_id = $1 AND v.trunk_version = $2 \
         AND v.branch_number = $3 AND v.branch_version = $4"
    );
    let primary = sqlx::query(SQL)
        .bind(vo_id)
        .bind(trunk_version)
        .bind(branch_number)
        .bind(branch_version)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = primary {
        return Ok(Some(
            stored_version(pool, vo_id, &row, Tier::Primary).await?,
        ));
    }
    let Some(row) = on_cold!(pool, |conn| sqlx::query(SQL)
        .bind(vo_id)
        .bind(trunk_version)
        .bind(branch_number)
        .bind(branch_version)
        .fetch_optional(&mut *conn)
        .await)
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row, Tier::Cold).await?))
}

/// Read the version of an object current at a given instant (time-travel):
/// the TRUNK row whose `sys_period` contains `at` (master08 §Change
/// Management — any previous state reconstructable).
///
/// `None` if the object had no trunk version then.
///
/// NOTE: the trunk restriction is deliberate. `VERSIONED_OBJECT.version_at_time
/// (a_time): VERSION[1]` returns exactly ONE version
/// (`UML/classes/org.openehr.rm.common.versioned_object.adoc` §Functions), yet
/// at any instant a container may have several valid tips — the trunk tip plus
/// one per open branch — so an unrestricted as-of read has no single answer to
/// give. The trunk is the lineage that makes it unique, and it is the one the
/// class treats as the container's own line elsewhere: `latest_version` is
/// documented as "the most recently added version (i.e. on trunk or any
/// branch)" while `latest_trunk_version` and `trunk_lifecycle_state` read the
/// trunk alone, the latter being how the spec says to determine "if the version
/// container is logically deleted".
///
/// A container holding branches but NO trunk version — the one shape this
/// returns `None` for that a caller might not expect — is a state RM common
/// `master06-change_control_package.adoc` §Copying §Subsequent Local
/// Modifications rules out: its second systematic rule is that "branch versions
/// from the original systems that are copied to another system cannot be copied
/// without their corresponding preceding versions on the same branch (if any)
/// and trunk versions also being copied", and a locally authored branch forks
/// from a version already held. So every branch in a well-formed container has
/// its trunk ancestry beside it.
///
/// # Errors
/// Returns [`StorageError`] on a driver/reassembly failure.
pub async fn version_at(
    pool: &PgPool,
    vo_id: VoId,
    at: jiff::Timestamp,
) -> Result<Option<StoredVersion>, StorageError> {
    const SQL: &str = version_select!(
        "WHERE v.vo_id = $1 AND v.sys_period @> $2::timestamptz \
         AND v.branch_number = 0"
    );
    let at = at.to_string();
    let primary = sqlx::query(SQL)
        .bind(vo_id)
        .bind(&at)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = primary {
        return Ok(Some(
            stored_version(pool, vo_id, &row, Tier::Primary).await?,
        ));
    }
    let Some(row) = on_cold!(pool, |conn| sqlx::query(SQL)
        .bind(vo_id)
        .bind(&at)
        .fetch_optional(&mut *conn)
        .await)
    else {
        return Ok(None);
    };
    Ok(Some(stored_version(pool, vo_id, &row, Tier::Cold).await?))
}
