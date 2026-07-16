//! Admin dump/load (SM `I_ADMIN_DUMP_LOAD.export_ehrs` / `load_ehrs`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc`
//! (`export_ehrs`, `load_ehrs`, error `file_not_writable`), `export_spec.adoc`
//! (`EXPORT_SPEC`, incl. `segment_split_size` kb), `dump_load_fail_report.adoc`
//! (`DUMP_LOAD_FAIL_REPORT`), `export_format.adoc` / `compression_format.adoc`
//! (the format/compression enumerations). `master02-overview.adoc` frames Admin
//! as "administrative facilities … such as back-up". Design register:
//! `docs/design/platform/06-service-message-admin.md` §5.2. No openEHR spec
//! defines an on-disk archive format — the archive layout below is our own
//! design (`0001_baseline.sql` is the source schema).
//!
//! Export walks the greenfield storage and writes a **canonical-JSON archive**
//! to a file-system directory, split into segment files no larger than
//! `segment_split_size` kb. Each EHR is one record carrying its `ehr` row, its
//! audit/contribution provenance, and one entry per stored version whose `body`
//! is the *reassembled canonical openEHR JSON* (the storage codec's lossless
//! inverse). Load reads the archive back and re-persists each EHR verbatim
//! (preserved ids/times), re-decomposing each `body` through the same codec —
//! so a read after load reassembles byte-equal canonical JSON. An EHR whose id
//! already exists is reported in a `DUMP_LOAD_FAIL_REPORT` and skipped ("import
//! EHRs with duplicate EHR ids will fail"), never a crash.
//!
//! PORT NOTE (re-verify — `export_ehrs(an_ehr_id)` is EHR-scoped; the archive
//! carries EHR-owned content only; G-A6): `ehr`, `audit`, `contribution`,
//! `vo_version`, `node`, `ehr_folder` (the `EHR.folders` membership rows — RM
//! ehr master04 §Folders), `item_tag`, and any `vo_archive` markers for the
//! EHR's versioned objects. Global DEFINITION artefacts a version references
//! (`template_store` OPTs via `vo_version.template_id`, `stored_query`) are NOT
//! carried — they are provisioned through the DEFINITION API and must pre-exist
//! (a COMPOSITION referencing an absent template fails its FK on load, reported
//! per EHR). Demographic parties (ehr-less versioned objects) and standalone
//! `vo_attestation` rows are out of this EHR-scoped dump; a whole-repository
//! back-up would need a demographic dump wave (deferred). The verbatim
//! version-row re-persist is a storage seam
//! ([`crate::storage::version_repo::insert_version_verbatim`]); this module
//! keeps the archive format and orchestration.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::service::{CallStatusType, DumpLoadFailReport, ExportFormat, ExportSpec, SmError};

use crate::service::{EhrbaseService, ServiceError};
use crate::storage::{decompose, node_repo, version_repo};

/// Lifecycle-state code of a logically-deleted version (RM common master06
/// §Logical Deletion) — such versions store no `node` rows, so their exported
/// `body` is `null`.
const DELETED_LIFECYCLE: &str = "523";

/// The archive manifest (`manifest.json`) — enough to read the segments back.
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    /// The logical export format (always `openehr_canonical_json` this wave).
    format: String,
    /// Archive schema version (this module's on-disk contract).
    archive_version: u32,
    /// The requested segment split size in kb.
    segment_split_size_kb: i32,
    /// Number of EHR records across all segments.
    ehr_count: usize,
    /// Segment file names, in order.
    segments: Vec<String>,
    /// Externalized `DV_MULTIMEDIA` blob keys carried in the `blobs/` subdir.
    /// Empty (and defaulted for pre-blob archives) when externalization is off
    /// or no version references external media.
    #[serde(default)]
    blobs: Vec<String>,
}

/// One EHR's full exported content bundle.
#[derive(Debug, Serialize, Deserialize)]
struct EhrRecord {
    ehr: EhrRow,
    audits: Vec<AuditRow>,
    contributions: Vec<ContributionRow>,
    versions: Vec<VersionRecord>,
    /// `EHR.folders` membership: (`rank`, `vo_id`) per folder hierarchy (RM ehr
    /// §EHR Class `Directory_in_folders`; ranks are append-only). `default`
    /// tolerates pre-folders dumps (no hierarchies → empty).
    #[serde(default)]
    folder_ranks: Vec<FolderRankRow>,
    item_tags: Vec<ItemTagRow>,
    archives: Vec<ArchiveRow>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FolderRankRow {
    rank: i32,
    vo_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct EhrRow {
    id: Uuid,
    system_id: String,
    time_created: String,
    subject_id: Option<String>,
    subject_namespace: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuditRow {
    id: Uuid,
    time_committed: String,
    system_id: String,
    change_type: String,
    description: Option<String>,
    committer: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContributionRow {
    id: Uuid,
    audit_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionRecord {
    vo_id: Uuid,
    kind: String,
    sys_version: i32,
    /// `VERSION_TREE_ID` columns (0/0 = trunk row).
    trunk_version: i32,
    branch_number: i32,
    branch_version: i32,
    /// Stored `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first
    /// version) and merge provenance (`None` when not a merge).
    preceding_version_uid: Option<String>,
    other_input_version_uids: Option<Value>,
    /// Lower/upper bounds of the temporal `sys_period` (`upper = None` ⇒ the
    /// current, still-open version).
    sys_period_lower: Option<String>,
    sys_period_upper: Option<String>,
    lifecycle_state: String,
    contribution_id: Uuid,
    audit_id: Uuid,
    template_id: Option<String>,
    signature: Option<String>,
    creating_system_id: String,
    /// The reassembled canonical openEHR JSON, or `Value::Null` for a deleted
    /// version (which stores no node rows).
    body: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ItemTagRow {
    id: Uuid,
    target_vo_id: Uuid,
    target_type: String,
    key: String,
    value: Option<String>,
    target_path: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveRow {
    vo_id: Uuid,
    archived_at: String,
    reason: Option<String>,
}

/// Plan the segmenting of records with the given serialized byte `sizes` into
/// contiguous index ranges, each staying at or under `limit_bytes` unless a
/// single record already exceeds it (then it is its own segment). Records keep
/// their order; every record lands in exactly one segment.
///
/// A pure function so the segmenting policy is unit-testable without a database.
fn plan_segments(sizes: &[usize], limit_bytes: usize) -> Vec<std::ops::Range<usize>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut acc = 0usize;
    for (i, &size) in sizes.iter().enumerate() {
        // Close the current (non-empty) segment before adding a record that
        // would push it past the limit.
        if i > start && acc + size > limit_bytes {
            segments.push(start..i);
            start = i;
            acc = 0;
        }
        acc += size;
    }
    if start < sizes.len() {
        segments.push(start..sizes.len());
    }
    segments
}

impl EhrbaseService {
    /// Export every EHR to a canonical-JSON archive under `dir` (SM
    /// `export_ehrs`). Returns a per-entity report; an empty list means every
    /// EHR was dumped successfully (the report carries only failures).
    ///
    /// PORT NOTE (re-verify — `export_format.adoc` /
    /// `compression_format.adoc`; G-A4): only `openehr_canonical_json` and no
    /// compression are supported this wave. The storage IS verbatim canonical
    /// JSON, so JSON export is translation-free, whereas
    /// `openehr_canonical_xml` would re-serialize via `openehr-its` and
    /// `zip`/`7z` would add a dependency for an ops-only nicety; both are cited
    /// as deliberately-unbuilt spec enum members. A requested XML or non-`None`
    /// compression format is a `precondition_violation` (400), never a silent
    /// downgrade.
    pub(super) async fn export_ehrs_to(
        &self,
        dir: &Path,
        spec: &ExportSpec,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        match spec.logical_format {
            None | Some(ExportFormat::OpenehrCanonicalJson) => {}
            Some(ExportFormat::OpenehrCanonicalXml) => {
                return Err(SmError::precondition(
                    "openehr_canonical_xml export is not supported (canonical JSON only)",
                ));
            }
        }
        if let Some(fmt) = spec.compression_format {
            return Err(SmError::precondition(format!(
                "compression format {} is not supported (uncompressed only)",
                fmt.sm_name()
            )));
        }
        if spec.segment_split_size <= 0 {
            return Err(SmError::precondition(
                "segment_split_size must be a positive number of kb",
            ));
        }

        let records = self.collect_ehr_records().await?;

        // Serialize each record once; the byte length drives segmenting.
        let mut blobs = Vec::with_capacity(records.len());
        for record in &records {
            blobs.push(serde_json::to_vec(record).map_err(ServiceError::from)?);
        }
        let sizes: Vec<usize> = blobs.iter().map(Vec::len).collect();
        let limit = usize::try_from(spec.segment_split_size.max(0))
            .unwrap_or(0)
            .saturating_mul(1024);
        let ranges = plan_segments(&sizes, limit);

        std::fs::create_dir_all(dir).map_err(|e| file_not_writable(dir, &e))?;

        let mut segment_names = Vec::with_capacity(ranges.len());
        for (seg_no, range) in ranges.iter().enumerate() {
            let name = format!("segment-{seg_no:04}.json");
            let path = dir.join(&name);
            // Re-serialize the segment as one JSON array of records.
            let slice = &records[range.clone()];
            let bytes = serde_json::to_vec(slice).map_err(ServiceError::from)?;
            std::fs::write(&path, bytes).map_err(|e| file_not_writable(&path, &e))?;
            segment_names.push(name);
        }

        // Our own extension (no openEHR spec governs multimedia offload): carry
        // every externalized DV_MULTIMEDIA blob the exported versions reference
        // into a `blobs/<hex>` subdir, so a load into an empty target
        // re-populates the object store.
        let blob_keys = self.export_referenced_blobs(dir, &records).await?;
        let archive_version = if blob_keys.is_empty() { 1 } else { 2 };

        let manifest = Manifest {
            format: ExportFormat::OpenehrCanonicalJson.sm_name().to_owned(),
            archive_version,
            segment_split_size_kb: spec.segment_split_size,
            ehr_count: records.len(),
            segments: segment_names,
            blobs: blob_keys,
        };
        let manifest_path = dir.join("manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(ServiceError::from)?;
        std::fs::write(&manifest_path, manifest_bytes)
            .map_err(|e| file_not_writable(&manifest_path, &e))?;

        // Every EHR dumped successfully → no failure entries.
        Ok(Vec::new())
    }

    /// Populate the repository from a canonical-JSON archive under `dir` (SM
    /// `load_ehrs`). Duplicate EHR ids are reported (`dump_status = false`) and
    /// skipped; all other EHRs are re-persisted verbatim.
    pub(super) async fn load_ehrs_from(
        &self,
        dir: &Path,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        let manifest_path = dir.join("manifest.json");
        let manifest_bytes =
            std::fs::read(&manifest_path).map_err(|e| file_not_writable(&manifest_path, &e))?;
        let manifest: Manifest =
            serde_json::from_slice(&manifest_bytes).map_err(ServiceError::from)?;

        // Our own extension: re-populate the object store from the archive's
        // `blobs/` subdir before loading versions that reference them.
        self.import_blobs(dir, &manifest.blobs).await?;

        let mut reports = Vec::new();
        for segment in &manifest.segments {
            let path = dir.join(segment);
            let bytes = std::fs::read(&path).map_err(|e| file_not_writable(&path, &e))?;
            let records: Vec<EhrRecord> =
                serde_json::from_slice(&bytes).map_err(ServiceError::from)?;
            for record in records {
                let ehr_id = record.ehr.id;
                if self.ehr_row_exists(ehr_id).await? {
                    // "import EHRs with duplicate EHR ids will fail" — reported,
                    // not fatal; the rest of the archive still loads.
                    reports.push(DumpLoadFailReport {
                        entity_type: "EHR".to_owned(),
                        entity_id: ehr_id.to_string(),
                        dump_status: false,
                        error: Some("an EHR with this id already exists".to_owned()),
                    });
                    continue;
                }
                self.load_one_ehr(&record).await?;
            }
        }
        Ok(reports)
    }

    /// Fetch every externalized `DV_MULTIMEDIA` blob referenced by the exported
    /// records into a `blobs/<hex>` subdir, returning the blob keys written
    /// (empty when externalization is off). Our own extension — no openEHR spec
    /// governs multimedia offload.
    async fn export_referenced_blobs(
        &self,
        dir: &Path,
        records: &[EhrRecord],
    ) -> Result<Vec<String>, SmError> {
        let Some(engine) = &self.multimedia else {
            return Ok(Vec::new());
        };
        let mut keys: Vec<String> = records
            .iter()
            .flat_map(|r| r.versions.iter())
            .flat_map(|v| engine.referenced_keys(&v.body))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        if keys.is_empty() {
            return Ok(keys);
        }
        let blob_dir = dir.join("blobs");
        std::fs::create_dir_all(&blob_dir).map_err(|e| file_not_writable(&blob_dir, &e))?;
        for hex in &keys {
            let bytes = engine
                .store()
                .get(hex)
                .await
                .map_err(|e| SmError::exception(format!("exporting blob {hex}: {e}")))?;
            let path = blob_dir.join(hex);
            std::fs::write(&path, &bytes).map_err(|e| file_not_writable(&path, &e))?;
        }
        Ok(keys)
    }

    /// Re-put each archived blob (`blobs/<hex>`) into the object store on load
    /// (idempotent, content-addressed). A no-op when externalization is off or
    /// the archive carries no blobs. Our own extension.
    async fn import_blobs(&self, dir: &Path, blobs: &[String]) -> Result<(), SmError> {
        if blobs.is_empty() {
            return Ok(());
        }
        let Some(engine) = &self.multimedia else {
            // The archive carries blobs but this target has no store configured.
            return Err(SmError::precondition(
                "archive carries externalized multimedia blobs but multimedia \
                 externalization is not enabled on this server",
            ));
        };
        let blob_dir = dir.join("blobs");
        for hex in blobs {
            let path = blob_dir.join(hex);
            let bytes = std::fs::read(&path).map_err(|e| file_not_writable(&path, &e))?;
            engine
                .store()
                .put_if_absent(hex, bytes)
                .await
                .map_err(|e| SmError::exception(format!("importing blob {hex}: {e}")))?;
        }
        Ok(())
    }

    /// Whether an EHR with `ehr_id` already exists in the target repository.
    async fn ehr_row_exists(&self, ehr_id: Uuid) -> Result<bool, ServiceError> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Read every EHR's content into export records (ordered by EHR id for a
    /// deterministic archive).
    async fn collect_ehr_records(&self) -> Result<Vec<EhrRecord>, ServiceError> {
        let ehr_ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ehr ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        let mut records = Vec::with_capacity(ehr_ids.len());
        for ehr_id in ehr_ids {
            records.push(self.collect_one_ehr(ehr_id).await?);
        }
        Ok(records)
    }

    /// Read one EHR's `ehr`/audit/contribution/version/tag/archive content. The
    /// per-version canonical body is reassembled through the storage codec
    /// ([`node_repo::read_version_canonical`] — the codec's lossless inverse).
    #[allow(clippy::too_many_lines)] // one linear per-EHR collection pass
    async fn collect_one_ehr(&self, ehr_id: Uuid) -> Result<EhrRecord, ServiceError> {
        let row = sqlx::query(
            "SELECT system_id, time_created::text, subject_id, subject_namespace \
             FROM ehr WHERE id = $1",
        )
        .bind(ehr_id)
        .fetch_one(&self.pool)
        .await?;
        let ehr = EhrRow {
            id: ehr_id,
            system_id: row.try_get("system_id")?,
            time_created: row.try_get("time_created")?,
            subject_id: row.try_get("subject_id")?,
            subject_namespace: row.try_get("subject_namespace")?,
        };

        // Every audit referenced by this EHR's contributions or versions.
        let audit_rows = sqlx::query(
            "SELECT id, time_committed::text AS time_committed, system_id, change_type, \
             description, committer FROM audit \
             WHERE id IN (SELECT audit_id FROM contribution WHERE ehr_id = $1 \
                          UNION SELECT audit_id FROM vo_version WHERE ehr_id = $1) \
             ORDER BY id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut audits = Vec::with_capacity(audit_rows.len());
        for r in audit_rows {
            audits.push(AuditRow {
                id: r.try_get("id")?,
                time_committed: r.try_get("time_committed")?,
                system_id: r.try_get("system_id")?,
                change_type: r.try_get("change_type")?,
                description: r.try_get("description")?,
                committer: r.try_get("committer")?,
            });
        }

        let contribution_rows =
            sqlx::query("SELECT id, audit_id FROM contribution WHERE ehr_id = $1 ORDER BY id")
                .bind(ehr_id)
                .fetch_all(&self.pool)
                .await?;
        let mut contributions = Vec::with_capacity(contribution_rows.len());
        for r in contribution_rows {
            contributions.push(ContributionRow {
                id: r.try_get("id")?,
                audit_id: r.try_get("audit_id")?,
            });
        }

        let version_rows = sqlx::query(
            "SELECT vo_id, kind, sys_version, trunk_version, branch_number, branch_version, \
             preceding_version_uid, other_input_version_uids, lower(sys_period)::text AS lo, \
             upper(sys_period)::text AS hi, lifecycle_state, contribution_id, audit_id, \
             template_id, signature, creating_system_id \
             FROM vo_version WHERE ehr_id = $1 ORDER BY vo_id, sys_version",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut versions = Vec::with_capacity(version_rows.len());
        for r in version_rows {
            let vo_id: Uuid = r.try_get("vo_id")?;
            let sys_version: i32 = r.try_get("sys_version")?;
            let lifecycle_state: String = r.try_get("lifecycle_state")?;
            // A deleted version has no node rows; its body stays `null`.
            let body = if lifecycle_state == DELETED_LIFECYCLE {
                Value::Null
            } else {
                node_repo::read_version_canonical(&self.pool, vo_id, sys_version).await?
            };
            versions.push(VersionRecord {
                vo_id,
                kind: r.try_get("kind")?,
                sys_version,
                trunk_version: r.try_get("trunk_version")?,
                branch_number: r.try_get("branch_number")?,
                branch_version: r.try_get("branch_version")?,
                preceding_version_uid: r.try_get("preceding_version_uid")?,
                other_input_version_uids: r.try_get("other_input_version_uids")?,
                sys_period_lower: r.try_get("lo")?,
                sys_period_upper: r.try_get("hi")?,
                lifecycle_state,
                contribution_id: r.try_get("contribution_id")?,
                audit_id: r.try_get("audit_id")?,
                template_id: r.try_get("template_id")?,
                signature: r.try_get("signature")?,
                creating_system_id: r.try_get("creating_system_id")?,
                body,
            });
        }

        let folder_rank_rows =
            sqlx::query("SELECT rank, vo_id FROM ehr_folder WHERE ehr_id = $1 ORDER BY rank")
                .bind(ehr_id)
                .fetch_all(&self.pool)
                .await?;
        let mut folder_ranks = Vec::with_capacity(folder_rank_rows.len());
        for r in folder_rank_rows {
            folder_ranks.push(FolderRankRow {
                rank: r.try_get("rank")?,
                vo_id: r.try_get("vo_id")?,
            });
        }

        let tag_rows = sqlx::query(
            "SELECT id, target_vo_id, target_type, key, value, target_path, \
             created_at::text AS created_at FROM item_tag WHERE ehr_id = $1 ORDER BY id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut item_tags = Vec::with_capacity(tag_rows.len());
        for r in tag_rows {
            item_tags.push(ItemTagRow {
                id: r.try_get("id")?,
                target_vo_id: r.try_get("target_vo_id")?,
                target_type: r.try_get("target_type")?,
                key: r.try_get("key")?,
                value: r.try_get("value")?,
                target_path: r.try_get("target_path")?,
                created_at: r.try_get("created_at")?,
            });
        }

        let archive_rows = sqlx::query(
            "SELECT vo_id, archived_at::text AS archived_at, reason FROM vo_archive \
             WHERE vo_id IN (SELECT DISTINCT vo_id FROM vo_version WHERE ehr_id = $1) \
             ORDER BY vo_id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut archives = Vec::with_capacity(archive_rows.len());
        for r in archive_rows {
            archives.push(ArchiveRow {
                vo_id: r.try_get("vo_id")?,
                archived_at: r.try_get("archived_at")?,
                reason: r.try_get("reason")?,
            });
        }

        Ok(EhrRecord {
            ehr,
            audits,
            contributions,
            versions,
            folder_ranks,
            item_tags,
            archives,
        })
    }

    /// Re-persist one EHR record verbatim in a single transaction: `ehr`, its
    /// audits/contributions, each version (`vo_version` + re-decomposed `node`
    /// rows through the storage codec), its item tags, and any archive markers —
    /// preserved ids, provenance and commit times (a lossless migration; RM
    /// common master06 §Copying "the `ORIGINAL_VERSION` is never modified").
    async fn load_one_ehr(&self, record: &EhrRecord) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        let ehr_id = record.ehr.id;

        sqlx::query(
            "INSERT INTO ehr (id, system_id, time_created, subject_id, subject_namespace) \
             VALUES ($1, $2, $3::timestamptz, $4, $5)",
        )
        .bind(ehr_id)
        .bind(&record.ehr.system_id)
        .bind(&record.ehr.time_created)
        .bind(&record.ehr.subject_id)
        .bind(&record.ehr.subject_namespace)
        .execute(&mut *tx)
        .await?;

        for a in &record.audits {
            sqlx::query(
                "INSERT INTO audit (id, time_committed, system_id, change_type, description, \
                 committer) VALUES ($1, $2::timestamptz, $3, $4, $5, $6)",
            )
            .bind(a.id)
            .bind(&a.time_committed)
            .bind(&a.system_id)
            .bind(&a.change_type)
            .bind(&a.description)
            .bind(&a.committer)
            .execute(&mut *tx)
            .await?;
        }

        for c in &record.contributions {
            sqlx::query("INSERT INTO contribution (id, ehr_id, audit_id) VALUES ($1, $2, $3)")
                .bind(c.id)
                .bind(ehr_id)
                .bind(c.audit_id)
                .execute(&mut *tx)
                .await?;
        }

        for v in &record.versions {
            insert_version(&mut tx, ehr_id, v).await?;
        }

        // The load re-decomposed the EHR_STATUS versions directly (the export
        // record carries subject columns but not the status flags). Backfill the
        // promoted `ehr.is_queryable` / `is_modifiable` from the stored current
        // status so the AQL full-population gate and the content-write guard
        // match the loaded state (RM ehr master04 §EHR Status / §EHR Active
        // Status; SM I_QUERY_SERVICE — full population = queryable EHRs).
        crate::storage::ehr_repo::sync_status_flags(&mut tx, ehr_id).await?;

        // EHR.folders membership rows, verbatim (rank fidelity — RM ehr §EHR
        // Class Directory_in_folders: folders.item(1) = directory).
        for f in &record.folder_ranks {
            sqlx::query("INSERT INTO ehr_folder (ehr_id, rank, vo_id) VALUES ($1, $2, $3)")
                .bind(ehr_id)
                .bind(f.rank)
                .bind(f.vo_id)
                .execute(&mut *tx)
                .await?;
        }

        for t in &record.item_tags {
            sqlx::query(
                "INSERT INTO item_tag (id, ehr_id, target_vo_id, target_type, key, value, \
                 target_path, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz)",
            )
            .bind(t.id)
            .bind(ehr_id)
            .bind(t.target_vo_id)
            .bind(&t.target_type)
            .bind(&t.key)
            .bind(&t.value)
            .bind(&t.target_path)
            .bind(&t.created_at)
            .execute(&mut *tx)
            .await?;
        }

        for ar in &record.archives {
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, archived_at, reason) \
                 VALUES ($1, $2::timestamptz, $3) ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(ar.vo_id)
            .bind(&ar.archived_at)
            .bind(&ar.reason)
            .execute(&mut *tx)
            .await?;
        }

        // Lineage keys mirror the removed EXCLUDE constraints exactly: trunk
        // rows are one lineage per vo_id; branch rows are per
        // {vo, creating system, fork point, branch number}.
        // The archive is the ONLY path writing explicit historical
        // `sys_period` bounds, so it carries the per-lineage temporal
        // non-overlap invariant check the regular write path holds by
        // construction (RM common master06: one valid version per lineage at
        // any instant; the enforcement mechanism is our own design — the
        // baseline's P20 NOTE). A corrupted or hand-crafted archive with
        // overlapping validity fails the whole record before commit.
        let overlap: bool = sqlx::query_scalar(
            "SELECT EXISTS (                  SELECT 1 FROM vo_version a                  JOIN vo_version b ON a.vo_id = b.vo_id                      AND a.branch_number = b.branch_number                      AND (a.branch_number = 0                           OR (a.creating_system_id = b.creating_system_id                               AND a.trunk_version = b.trunk_version))                      AND a.sys_version < b.sys_version                      AND a.sys_period && b.sys_period                  WHERE a.ehr_id = $1)",
        )
        .bind(ehr_id)
        .fetch_one(&mut *tx)
        .await?;
        if overlap {
            return Err(ServiceError::Unprocessable(format!(
                "archive for EHR {ehr_id} carries overlapping version validity periods"
            )));
        }

        tx.commit().await?;
        Ok(())
    }
}

/// Insert one version row and its re-decomposed node rows (through the storage
/// codec). The `vo_version` row I/O is delegated to
/// [`crate::storage::version_repo::insert_version_verbatim`] (our own design
/// over the greenfield schema — no openEHR spec governs it); the node rows are
/// re-decomposed here through the shared codec.
async fn insert_version(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    v: &VersionRecord,
) -> Result<(), ServiceError> {
    version_repo::insert_version_verbatim(
        tx,
        &version_repo::VerbatimVersionRow {
            vo_id: v.vo_id,
            kind: &v.kind,
            ehr_id,
            sys_version: v.sys_version,
            trunk_version: v.trunk_version,
            branch_number: v.branch_number,
            branch_version: v.branch_version,
            preceding_version_uid: v.preceding_version_uid.as_deref(),
            other_input_version_uids: v.other_input_version_uids.as_ref(),
            sys_period_lower: v.sys_period_lower.as_deref(),
            sys_period_upper: v.sys_period_upper.as_deref(),
            lifecycle_state: &v.lifecycle_state,
            contribution_id: v.contribution_id,
            audit_id: v.audit_id,
            template_id: v.template_id.as_deref(),
            signature: v.signature.as_deref(),
            creating_system_id: &v.creating_system_id,
        },
    )
    .await?;

    // A deleted version (null body) stores no node rows.
    if v.body.is_null() {
        return Ok(());
    }
    let rows = decompose(v.body.clone())?;
    node_repo::write_nodes(tx, v.vo_id, v.sys_version, Some(ehr_id), &rows).await?;
    Ok(())
}

/// Build a `file_not_writable` [`SmError`] (the only error `I_ADMIN_DUMP_LOAD`
/// declares) for a failed filesystem access to `path`.
fn file_not_writable(path: &Path, err: &std::io::Error) -> SmError {
    SmError::new(
        CallStatusType::FileNotWritable,
        format!("{}: {err}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::plan_segments;

    #[test]
    fn segments_group_greedily_under_the_limit() {
        // Three 40-byte records, limit 100 → [0,1] then [2].
        assert_eq!(plan_segments(&[40, 40, 40], 100), vec![0..2, 2..3]);
    }

    #[test]
    fn one_record_per_segment_when_each_exceeds_the_limit() {
        assert_eq!(plan_segments(&[200, 200], 100), vec![0..1, 1..2]);
    }

    #[test]
    fn a_single_oversized_record_is_its_own_segment() {
        // The 250-byte record cannot fit but still gets a segment of its own;
        // the two small ones share the next.
        assert_eq!(plan_segments(&[250, 10, 10], 100), vec![0..1, 1..3]);
    }

    #[test]
    fn everything_fits_in_one_segment() {
        assert_eq!(plan_segments(&[10, 10, 10], 1000), vec![0..3]);
    }

    #[test]
    fn no_records_no_segments() {
        assert_eq!(
            plan_segments(&[], 100),
            Vec::<std::ops::Range<usize>>::new()
        );
    }
}
