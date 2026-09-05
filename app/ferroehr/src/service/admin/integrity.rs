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

use std::collections::HashMap;
use std::time::Instant;

use serde_json::Value;
use uuid::Uuid;

use crate::ids::VoId;
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::SmError;
use crate::storage::codec::reassemble;
use crate::storage::node_repo::read_version_rows_all;

/// How many version rows one page of the sweep's cursor query returns.
///
/// The page carries identifiers only (no content), so it stays small whatever
/// the documents weigh; content is then read a [`CONTENT_CHUNK`] at a time.
const PAGE_SIZE: i64 = 500;

/// How many versions' contents are read per round trip.
///
/// The sweep needs both copies of every version, so its cost is two statements
/// per chunk rather than two per version: a 60 000-version store goes from
/// about 120 000 round trips to under 4 000. The chunk is what bounds memory —
/// the reassembled tree and the stored body of 32 documents at once, not of a
/// whole 500-row page, and not one version at a time either.
const CONTENT_CHUNK: usize = 32;

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

/// What a sweep has read so far, or in total.
///
/// NOTE: no openEHR spec governs storage mechanics — our own design/extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageParityCounts {
    /// How many stored versions were read, across both storage tiers.
    pub versions_checked: u64,
    /// How many of them carry a materialized body (a content version).
    pub versions_with_body: u64,
    /// How many of them carry no body (a logical delete).
    pub versions_without_body: u64,
    /// How many mismatches were found.
    pub mismatch_count: u64,
}

/// One event of a running storage-parity sweep.
///
/// A sweep reads every byte of what it covers, so a whole-repository pass can
/// run far longer than any one request may take to answer. Reporting it as a
/// sequence of events is what lets a caller consume the findings while the
/// sweep is still running.
///
/// NOTE: no openEHR spec governs storage mechanics — our own design/extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageParityEvent {
    /// One stored version whose two content copies disagree.
    Mismatch(StorageParityMismatch),
    /// The counts so far, emitted once per enumerated page.
    Progress {
        /// What has been read up to this point.
        counts: StorageParityCounts,
    },
    /// The final counts, emitted once, after the last page.
    Summary {
        /// What the whole sweep read.
        counts: StorageParityCounts,
        /// Wall-clock duration of the sweep, in milliseconds.
        elapsed_ms: u64,
    },
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

/// Which stored versions a sweep covers.
///
/// A sweep reads every byte of the versions it covers, so an operator
/// verifying one record, or everything committed since an incident, should not
/// have to read the whole repository to do it. Both bounds are optional and
/// compose; the default covers everything.
///
/// No openEHR spec governs storage mechanics — our own design/extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageParityScope {
    /// Cover only versions belonging to this EHR.
    pub ehr_id: Option<Uuid>,
    /// Cover only versions whose validity begins at or after this instant.
    pub committed_since: Option<jiff::Timestamp>,
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
    pub async fn verify_storage_parity(
        &self,
        scope: StorageParityScope,
    ) -> Result<StorageParityReport, SmError> {
        Ok(self.collect_storage_parity(scope).await?)
    }

    /// Starts a sweep the caller advances itself, one batch of events at a
    /// time.
    ///
    /// This is the sweep; [`Self::verify_storage_parity`] is one way of
    /// consuming it. A caller that must report findings while the sweep is
    /// still running — a response written as a stream of lines — drives this
    /// instead, and gets exactly the same reads, counts and log lines.
    ///
    /// NOTE: no openEHR spec governs storage mechanics — our own
    /// design/extension.
    #[must_use]
    pub fn storage_parity_sweep(&self, scope: StorageParityScope) -> StorageParitySweep<'_> {
        StorageParitySweep {
            service: self,
            scope,
            cursor: None,
            page: Vec::new().into_iter(),
            counts: StorageParityCounts::default(),
            started: Instant::now(),
            page_finished: false,
            done: false,
        }
    }

    /// Drain a sweep into the aggregated report.
    async fn collect_storage_parity(
        &self,
        scope: StorageParityScope,
    ) -> Result<StorageParityReport, ServiceError> {
        let mut sweep = self.storage_parity_sweep(scope);
        let mut report = StorageParityReport {
            versions_checked: 0,
            versions_with_body: 0,
            versions_without_body: 0,
            mismatch_count: 0,
            mismatches: Vec::new(),
            truncated: false,
            elapsed_ms: 0,
        };
        while let Some(events) = sweep.next_batch().await? {
            for event in events {
                match event {
                    StorageParityEvent::Mismatch(mismatch) => {
                        if report.mismatches.len() < MAX_REPORTED_MISMATCHES {
                            report.mismatches.push(mismatch);
                        } else {
                            report.truncated = true;
                        }
                    }
                    // The aggregated form answers once, at the end, so a
                    // running count has nobody to tell.
                    StorageParityEvent::Progress { .. } => {}
                    StorageParityEvent::Summary { counts, elapsed_ms } => {
                        report.versions_checked = counts.versions_checked;
                        report.versions_with_body = counts.versions_with_body;
                        report.versions_without_body = counts.versions_without_body;
                        report.mismatch_count = counts.mismatch_count;
                        report.elapsed_ms = elapsed_ms;
                    }
                }
            }
        }
        Ok(report)
    }

    /// Compare one chunk of versions in two round trips: the reassembled node
    /// content of all of them, then their stored bodies.
    ///
    /// Returns one verdict per input key, in input order, so the caller's
    /// counters follow the enumeration rather than the map's iteration.
    async fn chunk_parity_defects<'k>(
        &self,
        chunk: &'k [VersionKey],
    ) -> Result<Vec<(&'k VersionKey, Option<StorageParityDefect>)>, ServiceError> {
        let keys: Vec<(VoId, i32)> = chunk
            .iter()
            .map(|key| (VoId(key.vo_id), key.sys_version))
            .collect();
        let rows = read_version_rows_all(&self.pool, &keys).await?;
        let bodies = self.chunk_bodies(&keys).await?;

        Ok(chunk
            .iter()
            .map(|key| {
                let id = (VoId(key.vo_id), key.sys_version);
                // A set of rows that does not form one tree is a verdict, never
                // a propagated error: the sweep exists to find damaged records,
                // so it reports them and keeps going.
                let nodes = rows.get(&id).map(|rows| reassemble(rows));
                let defect = match (key.has_body, bodies.get(&id), nodes) {
                    // A logical delete (RM common master06 §Logical Deletion)
                    // stores no body, so any node row for it is unexpected.
                    (false, _, Some(_)) => Some(StorageParityDefect::UnexpectedNodes),
                    // Nothing to compare: a logical delete with no node rows is
                    // correct, and a version with no body left the primary tier
                    // or the repository between the page read and this one.
                    (false, _, None) | (true, None, _) => None,
                    (true, Some(_), None) => Some(StorageParityDefect::NodesMissing),
                    (true, Some(_), Some(Err(_))) => Some(StorageParityDefect::NodesUnreadable),
                    (true, Some(body), Some(Ok(value))) => {
                        (&value != body).then_some(StorageParityDefect::ContentDiffers)
                    }
                };
                (key, defect)
            })
            .collect())
    }

    /// The materialized bodies of one chunk of versions, in one statement.
    ///
    /// A version with a `NULL` body, or one that left the repository between
    /// the page read and this one, is absent from the map.
    async fn chunk_bodies(
        &self,
        keys: &[(VoId, i32)],
    ) -> Result<HashMap<(VoId, i32), Value>, ServiceError> {
        let vo_ids: Vec<Uuid> = keys.iter().map(|(vo_id, _)| vo_id.0).collect();
        let sys_versions: Vec<i32> = keys.iter().map(|(_, version)| *version).collect();
        let rows: Vec<(Uuid, i32, String)> = sqlx::query_as(
            "SELECT v.vo_id, v.sys_version, v.body \
             FROM unnest($1::uuid[], $2::int[]) AS k(vo_id, sys_version) \
             JOIN vo_version_all v \
               ON v.vo_id = k.vo_id AND v.sys_version = k.sys_version \
             WHERE v.body IS NOT NULL",
        )
        .bind(&vo_ids)
        .bind(&sys_versions)
        .fetch_all(&self.pool)
        .await?;
        let mut out = HashMap::with_capacity(rows.len());
        for (vo_id, sys_version, text) in rows {
            let body = serde_json::from_str(&text)
                .map_err(|e| ServiceError::internal("parse the stored canonical body", e))?;
            out.insert((VoId(vo_id), sys_version), body);
        }
        Ok(out)
    }

    /// One page of stored-version identifiers after `cursor`, in
    /// `(vo_id, sys_version)` order — a keyset walk, so a long sweep never
    /// pays a growing `OFFSET`.
    async fn parity_page(
        &self,
        cursor: Option<(Uuid, i32)>,
        scope: StorageParityScope,
    ) -> Result<Vec<VersionKey>, ServiceError> {
        let (after_vo, after_version) = match cursor {
            Some((vo_id, sys_version)) => (Some(vo_id), Some(sys_version)),
            None => (None, None),
        };
        let rows: Vec<(Uuid, i32, String, bool)> = sqlx::query_as(
            "SELECT vo_id, sys_version, kind, body IS NOT NULL \
             FROM vo_version_all \
             WHERE ($1::uuid IS NULL OR (vo_id, sys_version) > ($1::uuid, $2::int)) \
               AND ($4::uuid IS NULL OR ehr_id = $4::uuid) \
               AND ($5::timestamptz IS NULL OR lower(sys_period) >= $5::timestamptz) \
             ORDER BY vo_id, sys_version \
             LIMIT $3",
        )
        .bind(after_vo)
        .bind(after_version)
        .bind(PAGE_SIZE)
        .bind(scope.ehr_id)
        .bind(scope.committed_since.map(jiff_sqlx::Timestamp::from))
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
}

/// A storage-parity sweep in progress, advanced one batch of events at a time.
///
/// The sweep is a cursor rather than one call so both response shapes share ONE
/// implementation: the aggregated report drains it, and a streamed response
/// writes each event as a line as it arrives. Advancing only when the caller
/// asks is also what gives a stream its backpressure and its cancellation —
/// dropping the sweep stops the reads at the next batch boundary.
///
/// NOTE: no openEHR spec governs storage mechanics — our own design/extension.
pub struct StorageParitySweep<'a> {
    service: &'a FerroEhrService,
    scope: StorageParityScope,
    cursor: Option<(Uuid, i32)>,
    page: std::vec::IntoIter<VersionKey>,
    counts: StorageParityCounts,
    started: Instant,
    page_finished: bool,
    done: bool,
}

impl std::fmt::Debug for StorageParitySweep<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageParitySweep")
            .field("scope", &self.scope)
            .field("cursor", &self.cursor)
            .field("counts", &self.counts)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

impl StorageParitySweep<'_> {
    /// Advances the sweep, returning the next batch of events, or `None` once
    /// the summary has been handed over.
    ///
    /// A batch is what one chunk of versions produced, so it is empty whenever
    /// those versions agreed — which is the common case. The last batch before
    /// `None` carries the [`StorageParityEvent::Summary`].
    ///
    /// # Errors
    /// - `exception` — a database fault while enumerating or reading versions.
    ///   A version whose node rows are unreadable is a REPORTED mismatch, not
    ///   an error: one damaged record must not abort the sweep that found it.
    pub async fn next_batch(&mut self) -> Result<Option<Vec<StorageParityEvent>>, ServiceError> {
        loop {
            let chunk: Vec<VersionKey> = self.page.by_ref().take(CONTENT_CHUNK).collect();
            if !chunk.is_empty() {
                return Ok(Some(self.check_chunk(&chunk).await?));
            }
            if self.done {
                return Ok(None);
            }
            // A whole page has been read. The tick both tells a consumer the
            // sweep is advancing and keeps a streamed response writing bytes,
            // which is what stops an intermediary closing a long clean sweep.
            if self.page_finished {
                self.page_finished = false;
                return Ok(Some(vec![StorageParityEvent::Progress {
                    counts: self.counts,
                }]));
            }
            let page = self.service.parity_page(self.cursor, self.scope).await?;
            let Some(last) = page.last() else {
                self.done = true;
                let elapsed_ms =
                    u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
                return Ok(Some(vec![StorageParityEvent::Summary {
                    counts: self.counts,
                    elapsed_ms,
                }]));
            };
            self.cursor = Some((last.vo_id, last.sys_version));
            self.page = page.into_iter();
            self.page_finished = true;
        }
    }

    /// Compare one chunk, count what it read, and turn its defects into events.
    async fn check_chunk(
        &mut self,
        chunk: &[VersionKey],
    ) -> Result<Vec<StorageParityEvent>, ServiceError> {
        let mut events = Vec::new();
        for (key, defect) in self.service.chunk_parity_defects(chunk).await? {
            self.counts.versions_checked += 1;
            if key.has_body {
                self.counts.versions_with_body += 1;
            } else {
                self.counts.versions_without_body += 1;
            }
            if let Some(defect) = defect {
                self.counts.mismatch_count += 1;
                // The log line carries identifiers and the defect token only: a
                // body fragment would put clinical content into an operational
                // log. It is emitted here, in the one sweep, so a finding is
                // logged whichever response shape asked for it.
                tracing::warn!(
                    vo_id = %key.vo_id,
                    sys_version = key.sys_version,
                    kind = %key.kind,
                    defect = defect.as_str(),
                    "storage parity sweep: the node rows and the materialized body of a stored version disagree"
                );
                events.push(StorageParityEvent::Mismatch(StorageParityMismatch {
                    vo_id: key.vo_id,
                    sys_version: key.sys_version,
                    kind: key.kind.clone(),
                    defect,
                }));
            }
        }
        Ok(events)
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
