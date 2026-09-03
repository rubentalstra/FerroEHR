// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The storage-parity sweep: the two stored copies of every version's content
//! are re-derived and compared.
//!
//! NOTE: no openEHR spec governs storage mechanics — our own design/extension;
//! the storage keeps a version's content twice, as `vo_version.body` (the
//! materialized projection point reads serve) and as the decomposed `node`
//! rows (the AQL index), and their equality is an invariant of the commit path.
//!
//! The read-time signature verification in `crate::versioning::integrity` covers
//! the `body` copy, recomputing at a point read the signature taken over the
//! version's canonical serialized form (RM common
//! `master06-change_control_package.adoc` §Digital Signature), and says nothing
//! about the `node` rows. This sweep reassembles each version from its node rows
//! and compares the result to the stored body, so a tampered or corrupt row in
//! either copy becomes visible.
//!
//! It runs off the request path on an administrator's call and reads every
//! stored version in both storage tiers. It never logs content; a mismatch is
//! reported by identifier alone.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 1): stored canonical fragments — a typed \
              round-trip drops forward-compatible keys (the openEHR release strategy: minors are compatible supersets)"
)]

use std::time::Instant;

use serde_json::Value;
use uuid::Uuid;

use crate::ids::VoId;
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::SmError;
use crate::storage::node_repo::read_version_canonical_all;

/// How many version rows one page of the sweep's cursor query returns.
///
/// The page carries identifiers only (no content), so it stays small whatever
/// the documents weigh; each version's body and node rows are then read one
/// version at a time, which bounds the sweep's memory to a single document.
const PAGE_SIZE: i64 = 500;

/// How many mismatches the returned report carries in full.
///
/// Every mismatch is also logged at `warn`, so nothing is lost when a sweep
/// over a badly damaged store passes this cap; the report still counts them
/// all and says it was truncated.
const MAX_REPORTED_MISMATCHES: usize = 1000;

/// The way one stored version's two content copies disagree.
///
/// NOTE: no openEHR spec governs storage mechanics — our own design/extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageParityDefect {
    /// The version has both copies and they are not the same value.
    ContentDiffers,
    /// The version has a stored body but no node rows at all.
    NodesMissing,
    /// The version has node rows that do not reassemble into one tree, so the
    /// node copy cannot be compared to anything.
    NodesUnreadable,
    /// The version is a logical delete (data Void, RM common
    /// `master06-change_control_package.adoc` §Logical Deletion) and therefore
    /// stores no body, yet node rows exist for it.
    UnexpectedNodes,
}

impl StorageParityDefect {
    /// Returns the stable wire token for this defect.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContentDiffers => "content_differs",
            Self::NodesMissing => "nodes_missing",
            Self::NodesUnreadable => "nodes_unreadable",
            Self::UnexpectedNodes => "unexpected_nodes",
        }
    }
}

/// One stored version whose two content copies disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageParityMismatch {
    /// The versioned object the version belongs to.
    pub vo_id: Uuid,
    /// The per-object storage commit ordinal of the version.
    pub sys_version: i32,
    /// The `vo_version.kind` discriminator (`COMPOSITION` / `EHR_STATUS` /
    /// `FOLDER` / a demographic PARTY class / …).
    pub kind: String,
    /// How the two copies disagree.
    pub defect: StorageParityDefect,
}

/// The outcome of one storage-parity sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageParityReport {
    /// How many stored versions were read, across both storage tiers.
    pub versions_checked: u64,
    /// How many of them carry a materialized body (a content version).
    pub versions_with_body: u64,
    /// How many of them carry no body (a logical delete).
    pub versions_without_body: u64,
    /// How many mismatches were found in total, whatever `mismatches` holds.
    pub mismatch_count: u64,
    /// The mismatches, up to the reporting cap.
    pub mismatches: Vec<StorageParityMismatch>,
    /// Whether `mismatches` was cut short by the cap.
    pub truncated: bool,
    /// Wall-clock duration of the sweep, in milliseconds.
    pub elapsed_ms: u64,
}

impl StorageParityReport {
    /// Returns `true` when every stored version's two content copies agree.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.mismatch_count == 0
    }
}

/// One page row of the sweep cursor: the identifiers of a stored version plus
/// whether it carries a body. No content is fetched here.
struct VersionKey {
    vo_id: Uuid,
    sys_version: i32,
    kind: String,
    has_body: bool,
}

impl FerroEhrService {
    /// Re-derives every stored version's content from its `node` rows and
    /// compares it to the materialized `vo_version.body`, reporting every
    /// disagreement.
    ///
    /// The sweep reads BOTH storage tiers (the `vo_version_all` / `node_all`
    /// union views), so archived content is checked like everything else. It
    /// takes no lock and holds no transaction: a version committed while the
    /// sweep runs is simply checked or not, and a version read mid-commit
    /// cannot be seen half-written because a commit writes both copies in one
    /// transaction.
    ///
    /// NOTE: no openEHR spec governs storage mechanics — our own
    /// design/extension (the module docs carry the full derivation).
    ///
    /// # Errors
    /// - `exception` — a database fault while enumerating or reading versions.
    ///   A version whose node rows are unreadable is a REPORTED mismatch, not
    ///   an error: one damaged record must not abort the sweep that found it.
    pub async fn verify_storage_parity(&self) -> Result<StorageParityReport, SmError> {
        Ok(self.sweep_storage_parity().await?)
    }

    /// The sweep itself, over the storage error domain.
    async fn sweep_storage_parity(&self) -> Result<StorageParityReport, ServiceError> {
        let started = Instant::now();
        let mut report = StorageParityReport {
            versions_checked: 0,
            versions_with_body: 0,
            versions_without_body: 0,
            mismatch_count: 0,
            mismatches: Vec::new(),
            truncated: false,
            elapsed_ms: 0,
        };
        let mut cursor: Option<(Uuid, i32)> = None;

        loop {
            let page = self.parity_page(cursor).await?;
            let Some(last) = page.last() else { break };
            cursor = Some((last.vo_id, last.sys_version));

            for key in &page {
                report.versions_checked += 1;
                if key.has_body {
                    report.versions_with_body += 1;
                } else {
                    report.versions_without_body += 1;
                }
                if let Some(defect) = self.version_parity_defect(key).await? {
                    record(&mut report, key, defect);
                }
            }
        }

        report.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(report)
    }

    /// One page of stored-version identifiers after `cursor`, in
    /// `(vo_id, sys_version)` order — a keyset walk, so a long sweep never
    /// pays a growing `OFFSET`.
    async fn parity_page(
        &self,
        cursor: Option<(Uuid, i32)>,
    ) -> Result<Vec<VersionKey>, ServiceError> {
        let (after_vo, after_version) = match cursor {
            Some((vo_id, sys_version)) => (Some(vo_id), Some(sys_version)),
            None => (None, None),
        };
        let rows: Vec<(Uuid, i32, String, bool)> = sqlx::query_as(
            "SELECT vo_id, sys_version, kind, body IS NOT NULL \
             FROM vo_version_all \
             WHERE ($1::uuid IS NULL OR (vo_id, sys_version) > ($1::uuid, $2::int)) \
             ORDER BY vo_id, sys_version \
             LIMIT $3",
        )
        .bind(after_vo)
        .bind(after_version)
        .bind(PAGE_SIZE)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(vo_id, sys_version, kind, has_body)| VersionKey {
                vo_id,
                sys_version,
                kind,
                has_body,
            })
            .collect())
    }

    /// How one stored version's two copies disagree, or `None` when they
    /// agree.
    ///
    /// A reassembly failure is a verdict, never a propagated error: the sweep
    /// exists to find damaged records, so it reports them and keeps going.
    async fn version_parity_defect(
        &self,
        key: &VersionKey,
    ) -> Result<Option<StorageParityDefect>, ServiceError> {
        let reassembled =
            read_version_canonical_all(&self.pool, VoId(key.vo_id), key.sys_version).await;
        if !key.has_body {
            return Ok(match reassembled {
                Ok(Value::Null) => None,
                Ok(_) | Err(_) => Some(StorageParityDefect::UnexpectedNodes),
            });
        }
        let Some(body) = self.stored_body(key).await? else {
            // The version left the primary tier (an archive move) or the
            // repository (a physical delete) between the page read and this
            // one; there is nothing left to compare, and no defect to claim.
            return Ok(None);
        };
        Ok(match reassembled {
            Ok(Value::Null) => Some(StorageParityDefect::NodesMissing),
            Ok(value) if value == body => None,
            Ok(_) => Some(StorageParityDefect::ContentDiffers),
            Err(_) => Some(StorageParityDefect::NodesUnreadable),
        })
    }

    /// The materialized body of one stored version, read one version at a time
    /// so the sweep holds a single document in memory.
    async fn stored_body(&self, key: &VersionKey) -> Result<Option<Value>, ServiceError> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT body FROM vo_version_all WHERE vo_id = $1 AND sys_version = $2")
                .bind(key.vo_id)
                .bind(key.sys_version)
                .fetch_optional(&self.pool)
                .await?;
        row.and_then(|(body,)| body)
            .map(|text| {
                serde_json::from_str(&text)
                    .map_err(|e| ServiceError::internal("parse the stored canonical body", e))
            })
            .transpose()
    }
}

/// Count a mismatch, log it by identifier, and record it while the report has
/// room.
///
/// The log line carries identifiers and the defect token only: a body fragment
/// would put clinical content into an operational log.
fn record(report: &mut StorageParityReport, key: &VersionKey, defect: StorageParityDefect) {
    report.mismatch_count += 1;
    tracing::warn!(
        vo_id = %key.vo_id,
        sys_version = key.sys_version,
        kind = %key.kind,
        defect = defect.as_str(),
        "storage parity sweep: the node rows and the materialized body of a stored version disagree"
    );
    if report.mismatches.len() < MAX_REPORTED_MISMATCHES {
        report.mismatches.push(StorageParityMismatch {
            vo_id: key.vo_id,
            sys_version: key.sys_version,
            kind: key.kind.clone(),
            defect,
        });
    } else {
        report.truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defect_tokens_are_the_documented_wire_values() {
        assert_eq!(
            StorageParityDefect::ContentDiffers.as_str(),
            "content_differs"
        );
        assert_eq!(StorageParityDefect::NodesMissing.as_str(), "nodes_missing");
        assert_eq!(
            StorageParityDefect::NodesUnreadable.as_str(),
            "nodes_unreadable"
        );
        assert_eq!(
            StorageParityDefect::UnexpectedNodes.as_str(),
            "unexpected_nodes"
        );
    }

    #[test]
    fn a_report_with_no_mismatch_count_is_clean() {
        let mut report = StorageParityReport {
            versions_checked: 3,
            versions_with_body: 2,
            versions_without_body: 1,
            mismatch_count: 0,
            mismatches: Vec::new(),
            truncated: false,
            elapsed_ms: 1,
        };
        assert!(report.is_clean());
        report.mismatch_count = 1;
        assert!(!report.is_clean());
    }
}
