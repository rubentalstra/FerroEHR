//! Admin dump/load (SM `I_ADMIN_DUMP_LOAD.export_ehrs` / `load_ehrs`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc`
//! (`export_ehrs`, `load_ehrs`, error `file_not_writable`), `export_spec.adoc`
//! (`EXPORT_SPEC`, incl. `segment_split_size` kb), `dump_load_fail_report.adoc`
//! (`DUMP_LOAD_FAIL_REPORT`), `export_format.adoc` / `compression_format.adoc`
//! (the format/compression enumerations). `master02-overview.adoc` frames Admin
//! as "administrative facilities … such as back-up". No openEHR spec defines an
//! on-disk archive format — the archive layout below is our own design
//! (`0001_baseline.sql` is the source schema).
//!
//! NOTE (the archive CONTAINER — SM `compression_format.adoc` member `zip`):
//! the entry set (`manifest.json`, `segment-NNNN.json`, `blobs/<hex>`) is
//! identical whichever container carries it. `compression_format` absent ⇒ the
//! entries are loose files under `file_sys_loc`; `zip` ⇒ the identical entries
//! are DEFLATE members of a single `archive.zip` there. `load_ehrs` takes only
//! `file_sys_loc` (`i_admin_dump_load.adoc`), so it never receives the
//! container choice and DETECTS it instead: a `manifest.json` in the directory
//! is the loose form, else an `archive.zip` is the packed one. No openEHR spec
//! defines the archive layout or the detection rule — our own design/extension.
//!
//! Export walks the greenfield storage and writes a **canonical-JSON archive**
//! to a file-system directory, split into segment entries no larger than
//! `segment_split_size` kb. Each EHR is one record carrying its `ehr` row, its
//! audit/contribution provenance, and one entry per stored version whose `body`
//! is the *reassembled canonical openEHR JSON* (the storage codec's lossless
//! inverse). Load reads the archive back and re-persists each EHR verbatim
//! (preserved ids/times), re-decomposing each `body` through the same codec —
//! so a read after load reassembles byte-equal canonical JSON. An EHR whose id
//! already exists is reported in a `DUMP_LOAD_FAIL_REPORT` and skipped ("import
//! EHRs with duplicate EHR ids will fail"), never a crash.
//!
//! NOTE (re-verify — `export_ehrs(an_ehr_id)` is EHR-scoped; the archive
//! carries EHR-owned content only): `ehr`, `audit`, `contribution`,
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
//! ([`crate::storage::version_repo::import::insert_version_verbatim`]); this module
//! keeps the archive format and orchestration.
//!
//! NOTE (load is a RESTORE, not an EHR creation): the load re-persists exactly
//! what the archive holds and never synthesizes content. In particular it does
//! NOT mint a missing `EHR_ACCESS` the way the EHR-Extract clone does (RM ehr
//! master04 §EHR Creation governs *creating* an EHR — an archive record is a
//! previously-created one), because inventing a versioned object with a fresh
//! uid and an extra CONTRIBUTION would break the archive⇒repository identity
//! this format guarantees. The one thing the load re-derives rather than
//! copies is the promoted `ehr` column projection (the subject + status-flag
//! cache of the loaded `EHR_STATUS`), which is not content.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::EhrbaseService;
use crate::service::admin::types::{
    CompressionFormat, DumpLoadFailReport, ExportFormat, ExportSpec,
};
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};
use crate::storage::codec::decompose;
use crate::storage::{node_repo, version_repo};

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
    vo_id: VoId,
}

#[derive(Debug, Serialize, Deserialize)]
struct EhrRow {
    id: EhrId,
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
    vo_id: VoId,
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
    /// Whether `signature` was client-supplied (foreign — never re-verified at
    /// read; master06 §Digital Signature). `#[serde(default)]` so archives dumped
    /// before this field existed load as server-generated.
    #[serde(default)]
    signature_client_supplied: bool,
    creating_system_id: String,
    /// The reassembled canonical openEHR JSON, or `Value::Null` for a deleted
    /// version (which stores no node rows).
    body: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ItemTagRow {
    id: Uuid,
    target_vo_id: VoId,
    target_type: String,
    key: String,
    value: Option<String>,
    target_path: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveRow {
    vo_id: VoId,
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

/// Build a `file_not_writable` [`SmError`] (the only error `I_ADMIN_DUMP_LOAD`
/// declares) for a failed filesystem access to `path`.
fn file_not_writable(path: &Path, err: &std::io::Error) -> SmError {
    SmError::new(
        CallStatusType::FileNotWritable,
        format!("{}: {err}", path.display()),
    )
}

/// The archive manifest entry name (both containers).
const MANIFEST_ENTRY: &str = "manifest.json";
/// The packed container's file name inside `file_sys_loc`.
const ZIP_ENTRY_FILE: &str = "archive.zip";
/// The packed 7z container entry (SM `COMPRESSION_FORMAT.7z`).
const SEVENZ_ENTRY_FILE: &str = "archive.7z";
/// The entry-name prefix of an externalized `DV_MULTIMEDIA` blob.
const BLOB_PREFIX: &str = "blobs/";

/// Where an export's entries land. The entry NAMES are identical in both
/// containers (see the module docs); only the packaging differs.
enum ArchiveWriter {
    /// Loose files under `file_sys_loc` (no `compression_format`).
    Directory { dir: PathBuf },
    /// One `archive.zip` under `file_sys_loc` (SM `COMPRESSION_FORMAT.zip`).
    Zip {
        path: PathBuf,
        zip: Box<zip::ZipWriter<std::fs::File>>,
    },
    /// One `archive.7z` under `file_sys_loc` (SM `COMPRESSION_FORMAT.7z`).
    SevenZip {
        path: PathBuf,
        writer: Box<sevenz_rust2::ArchiveWriter<std::fs::File>>,
    },
}

impl ArchiveWriter {
    /// Create the container for `compression`, making `dir` if needed.
    fn create(dir: &Path, compression: Option<CompressionFormat>) -> Result<Self, SmError> {
        std::fs::create_dir_all(dir).map_err(|e| file_not_writable(dir, &e))?;
        match compression {
            None => Ok(Self::Directory {
                dir: dir.to_path_buf(),
            }),
            Some(CompressionFormat::Zip) => {
                let path = dir.join(ZIP_ENTRY_FILE);
                let file =
                    std::fs::File::create(&path).map_err(|e| file_not_writable(&path, &e))?;
                Ok(Self::Zip {
                    path,
                    zip: Box::new(zip::ZipWriter::new(file)),
                })
            }
            Some(CompressionFormat::SevenZip) => {
                let path = dir.join(SEVENZ_ENTRY_FILE);
                let writer = sevenz_rust2::ArchiveWriter::create(&path)
                    .map_err(|e| sevenz_fault(&path, "create", &e))?;
                Ok(Self::SevenZip {
                    path,
                    writer: Box::new(writer),
                })
            }
        }
    }

    /// Write one archive entry.
    fn write(&mut self, name: &str, bytes: &[u8]) -> Result<(), SmError> {
        match self {
            Self::Directory { dir } => {
                let path = dir.join(name);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| file_not_writable(parent, &e))?;
                }
                std::fs::write(&path, bytes).map_err(|e| file_not_writable(&path, &e))
            }
            Self::Zip { path, zip } => {
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                zip.start_file(name, options)
                    .map_err(|e| zip_fault(path, "start entry", &e))?;
                zip.write_all(bytes)
                    .map_err(|e| file_not_writable(path, &e))
            }
            Self::SevenZip { path, writer } => {
                let entry = sevenz_rust2::ArchiveEntry::new_file(name);
                writer
                    .push_archive_entry(entry, Some(bytes))
                    .map(|_| ())
                    .map_err(|e| sevenz_fault(path, &format!("write entry {name}"), &e))
            }
        }
    }

    /// Close the container (a no-op for the loose form).
    fn finish(self) -> Result<(), SmError> {
        match self {
            Self::Directory { .. } => Ok(()),
            Self::Zip { path, zip } => zip
                .finish()
                .map(|_| ())
                .map_err(|e| zip_fault(&path, "finish", &e)),
            Self::SevenZip { path, writer } => writer
                .finish()
                .map(|_| ())
                .map_err(|e| file_not_writable(&path, &e)),
        }
    }
}

/// The read side of [`ArchiveWriter`], with the container DETECTED from what
/// `file_sys_loc` holds (`load_ehrs` is passed no format — see the module docs).
enum ArchiveReader {
    Directory {
        dir: PathBuf,
    },
    Zip {
        path: PathBuf,
        zip: Box<zip::ZipArchive<std::fs::File>>,
    },
    SevenZip {
        path: PathBuf,
        reader: Box<sevenz_rust2::ArchiveReader<std::fs::File>>,
    },
}

impl ArchiveReader {
    /// Open the archive under `dir`: the loose form when it holds a
    /// `manifest.json`, else the packed `archive.zip`. When neither exists the
    /// manifest read failure is reported against the loose path, so the error
    /// names the entry a caller expects.
    fn open(dir: &Path) -> Result<Self, SmError> {
        if dir.join(MANIFEST_ENTRY).is_file() {
            return Ok(Self::Directory {
                dir: dir.to_path_buf(),
            });
        }
        let path = dir.join(ZIP_ENTRY_FILE);
        if path.is_file() {
            let file = std::fs::File::open(&path).map_err(|e| file_not_writable(&path, &e))?;
            let zip = zip::ZipArchive::new(file).map_err(|e| zip_fault(&path, "open", &e))?;
            return Ok(Self::Zip {
                path,
                zip: Box::new(zip),
            });
        }
        let path = dir.join(SEVENZ_ENTRY_FILE);
        if path.is_file() {
            let reader = sevenz_rust2::ArchiveReader::open(&path, sevenz_rust2::Password::empty())
                .map_err(|e| sevenz_fault(&path, "open", &e))?;
            return Ok(Self::SevenZip {
                path,
                reader: Box::new(reader),
            });
        }
        let manifest_path = dir.join(MANIFEST_ENTRY);
        Err(file_not_writable(
            &manifest_path,
            &std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "no archive at {} (no {MANIFEST_ENTRY}, no {ZIP_ENTRY_FILE}, \
                     no {SEVENZ_ENTRY_FILE})",
                    dir.display()
                ),
            ),
        ))
    }

    /// Read one archive entry.
    fn read(&mut self, name: &str) -> Result<Vec<u8>, SmError> {
        match self {
            Self::Directory { dir } => {
                let path = dir.join(name);
                std::fs::read(&path).map_err(|e| file_not_writable(&path, &e))
            }
            Self::Zip { path, zip } => {
                let mut entry = zip
                    .by_name(name)
                    .map_err(|e| zip_fault(path, &format!("read entry {name}"), &e))?;
                let mut bytes = Vec::new();
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|e| file_not_writable(path, &e))?;
                Ok(bytes)
            }
            Self::SevenZip { path, reader } => reader
                .read_file(name)
                .map_err(|e| sevenz_fault(path, &format!("read entry {name}"), &e)),
        }
    }
}

/// Build a `file_not_writable` [`SmError`] for a ZIP container fault — the
/// same SM error the loose form raises, since from the operation's point of
/// view the archive file could not be written/read either way.
fn sevenz_fault(path: &Path, what: &str, err: &sevenz_rust2::Error) -> SmError {
    SmError::new(
        CallStatusType::FileNotWritable,
        format!("{} ({what}): {err}", path.display()),
    )
}

fn zip_fault(path: &Path, what: &str, err: &zip::result::ZipError) -> SmError {
    SmError::new(
        CallStatusType::FileNotWritable,
        format!("{} ({what}): {err}", path.display()),
    )
}

impl EhrbaseService {
    /// SM `export_ehrs`: export every EHR to a canonical-JSON archive under
    /// `file_sys_loc`. Returns a per-entity report; an empty list means every
    /// EHR was dumped successfully (the report carries only failures).
    ///
    /// NOTE (`export_format.adoc` / `compression_format.adoc` — which
    /// enumeration members this service realizes). `COMPRESSION_FORMAT` is
    /// realized in FULL: absent (loose files), `zip`, and `7z` (owner-approved
    /// 2026-07-29; `sevenz-rust2`). `EXPORT_FORMAT`: only
    /// `openehr_canonical_json`, because the storage IS verbatim canonical JSON
    /// so that export is translation-free — an `openehr_canonical_xml` ARCHIVE
    /// has no defined envelope shape (the manifest/segment/record structure is
    /// our own design with no XML story), so inventing one ad hoc is worse
    /// than the honest boundary. Neither the SM interface nor the
    /// `EXPORT_SPEC` class says a service must realize every member, and
    /// `i_admin_dump_load.adoc` declares no unsupported-format error — that
    /// silence is adjudicated in the CNF ambiguity register. A well-formed
    /// request for the unrealized member is answered `not_implemented`
    /// (RFC 9110 §15.6.2 `501`), never silently downgraded and never called
    /// malformed. TODO: design the canonical-XML archive envelope (tracker
    /// issue; needs its own register adjudication) and realize the member.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a non-positive
    ///   `segment_split_size`.
    /// - `not_implemented` (`501`) — the spec requests
    ///   `openehr_canonical_xml`.
    /// - `file_not_writable` — the directory, a segment entry, a blob entry, or
    ///   the manifest cannot be created/written.
    /// - `exception` — a database/codec fault while collecting records, or a
    ///   blob-store fault while exporting referenced multimedia.
    pub async fn export_ehrs(
        &self,
        file_sys_loc: String,
        spec: ExportSpec,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        let dir = Path::new(&file_sys_loc);
        match spec.logical_format {
            None | Some(ExportFormat::OpenehrCanonicalJson) => {}
            Some(ExportFormat::OpenehrCanonicalXml) => {
                return Err(SmError::new(
                    CallStatusType::NotImplemented,
                    "logical format openehr_canonical_xml is not implemented by this service \
                     (openehr_canonical_json is)",
                ));
            }
        }
        if spec.segment_split_size <= 0 {
            return Err(SmError::precondition(
                "segment_split_size must be a positive number of kb",
            ));
        }

        // Opening the container FIRST is what makes an unrealized compression
        // member cost nothing: the refusal happens before any storage read.
        let mut archive = ArchiveWriter::create(dir, spec.compression_format)?;

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

        let mut segment_names = Vec::with_capacity(ranges.len());
        for (seg_no, range) in ranges.iter().enumerate() {
            let name = format!("segment-{seg_no:04}.json");
            // Re-serialize the segment as one JSON array of records.
            let slice = &records[range.clone()];
            let bytes = serde_json::to_vec(slice).map_err(ServiceError::from)?;
            archive.write(&name, &bytes)?;
            segment_names.push(name);
        }

        // Our own extension (no openEHR spec governs multimedia offload): carry
        // every externalized DV_MULTIMEDIA blob the exported versions reference
        // as `blobs/<hex>` entries, so a load into an empty target re-populates
        // the object store.
        let blob_keys = self.export_referenced_blobs(&mut archive, &records).await?;
        let archive_version = if blob_keys.is_empty() { 1 } else { 2 };

        let manifest = Manifest {
            format: ExportFormat::OpenehrCanonicalJson.sm_name().to_owned(),
            archive_version,
            segment_split_size_kb: spec.segment_split_size,
            ehr_count: records.len(),
            segments: segment_names,
            blobs: blob_keys,
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(ServiceError::from)?;
        archive.write(MANIFEST_ENTRY, &manifest_bytes)?;
        archive.finish()?;

        // Every EHR dumped successfully → no failure entries.
        Ok(Vec::new())
    }

    /// SM `load_ehrs`: populate the repository from a canonical-JSON archive
    /// under `file_sys_loc`. Duplicate EHR ids — and a subject this repository
    /// already holds under another EHR — are reported (`dump_status = false`)
    /// and skipped; all other EHRs are re-persisted verbatim.
    ///
    /// # Errors
    /// - `file_not_writable` — `file_sys_loc` holds neither archive container,
    ///   or the manifest, a segment entry, or a blob entry cannot be read.
    /// - `precondition_violation` (`400`) — the archive carries externalized
    ///   multimedia blobs but this server has no multimedia store configured.
    /// - `unprocessable` — an archive record carries overlapping version
    ///   validity periods (a corrupted/hand-crafted archive; the record's
    ///   transaction is rolled back).
    /// - `exception` — a malformed manifest/segment JSON, a database/codec
    ///   fault while re-persisting, or a blob-store fault while importing.
    pub async fn load_ehrs(
        &self,
        file_sys_loc: String,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        let dir = Path::new(&file_sys_loc);
        // `load_ehrs` is passed no format, so the container is detected from
        // what the location holds (module docs).
        let mut archive = ArchiveReader::open(dir)?;
        let manifest_bytes = archive.read(MANIFEST_ENTRY)?;
        let manifest: Manifest =
            serde_json::from_slice(&manifest_bytes).map_err(ServiceError::from)?;

        // Our own extension: re-populate the object store from the archive's
        // `blobs/` entries before loading versions that reference them.
        self.import_blobs(&mut archive, &manifest.blobs).await?;

        let mut reports = Vec::new();
        for segment in &manifest.segments {
            let bytes = archive.read(segment)?;
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
                match self.load_one_ehr(&record).await {
                    Ok(()) => {}
                    // A per-EHR conflict — currently a subject this repository
                    // already holds under another EHR (one EHR per subject, RM
                    // ehr master04 §EHR Status), reachable only on a PARTIAL
                    // load into a non-empty repository. Reported and skipped
                    // (its transaction rolled back) exactly like a duplicate
                    // EHR id, so the rest of the archive still loads.
                    Err(ServiceError::Conflict(message)) => reports.push(DumpLoadFailReport {
                        entity_type: "EHR".to_owned(),
                        entity_id: ehr_id.to_string(),
                        dump_status: false,
                        error: Some(message),
                    }),
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(reports)
    }

    /// Fetch every externalized `DV_MULTIMEDIA` blob referenced by the exported
    /// records into `blobs/<hex>` archive entries, returning the blob keys
    /// written (empty when externalization is off). Our own extension — no
    /// openEHR spec governs multimedia offload.
    async fn export_referenced_blobs(
        &self,
        archive: &mut ArchiveWriter,
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
        for hex in &keys {
            let bytes = engine
                .store()
                .get(hex)
                .await
                .map_err(|e| SmError::exception(format!("exporting blob {hex}: {e}")))?;
            archive.write(&format!("{BLOB_PREFIX}{hex}"), &bytes)?;
        }
        Ok(keys)
    }

    /// Re-put each archived blob (`blobs/<hex>`) into the object store on load
    /// (idempotent, content-addressed). A no-op when the archive carries no
    /// blobs. Our own extension.
    async fn import_blobs(
        &self,
        archive: &mut ArchiveReader,
        blobs: &[String],
    ) -> Result<(), SmError> {
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
        for hex in blobs {
            let bytes = archive.read(&format!("{BLOB_PREFIX}{hex}"))?;
            engine
                .store()
                .put_if_absent(hex, bytes)
                .await
                .map_err(|e| SmError::exception(format!("importing blob {hex}: {e}")))?;
        }
        Ok(())
    }

    /// Whether an EHR with `ehr_id` already exists in the target repository.
    async fn ehr_row_exists(&self, ehr_id: EhrId) -> Result<bool, ServiceError> {
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
        let ehr_ids: Vec<EhrId> = sqlx::query_scalar("SELECT id FROM ehr ORDER BY id")
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
    async fn collect_one_ehr(&self, ehr_id: EhrId) -> Result<EhrRecord, ServiceError> {
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
             template_id, signature, signature_client_supplied, creating_system_id \
             FROM vo_version WHERE ehr_id = $1 ORDER BY vo_id, sys_version",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut versions = Vec::with_capacity(version_rows.len());
        for r in version_rows {
            let vo_id: VoId = r.try_get("vo_id")?;
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
                signature_client_supplied: r.try_get("signature_client_supplied")?,
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

        insert_ehr_row(&mut tx, &record.ehr).await?;

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

        // The load re-decomposed the EHR_STATUS versions directly, so the
        // promoted `ehr` columns are re-derived from the loaded current status
        // — the EHR_STATUS content is the truth, the exported columns only its
        // cached projection (an archive written before a promotion fix, or by a
        // path that never promoted, carries a stale/absent subject). This makes
        // a loaded EHR visible to the subject lookup (SM
        // `I_EHR_SERVICE.get_ehrs_for_subject`) and bound by the
        // one-EHR-per-subject rule (RM ehr master04 §EHR Status), and keeps
        // `is_queryable` / `is_modifiable` matching the loaded state for the AQL
        // full-population gate (SM I_QUERY_SERVICE — full population =
        // queryable EHRs) and the content-write guard (§EHR Active Status).
        self.resync_promoted_columns(&mut tx, ehr_id).await?;

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
        // baseline schema NOTE). A corrupted or hand-crafted archive with
        // overlapping validity fails the whole record before commit.
        let overlap: bool = sqlx::query_scalar(
            "SELECT EXISTS ( \
                 SELECT 1 FROM vo_version a \
                 JOIN vo_version b ON a.vo_id = b.vo_id \
                     AND a.branch_number = b.branch_number \
                     AND (a.branch_number = 0 \
                          OR (a.creating_system_id = b.creating_system_id \
                              AND a.trunk_version = b.trunk_version)) \
                     AND a.sys_version < b.sys_version \
                     AND a.sys_period && b.sys_period \
                 WHERE a.ehr_id = $1)",
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

/// Re-persist the archived `ehr` root row verbatim (preserved id, immutable
/// `system_id` and `time_created`, plus the archived promoted-column
/// projection, which [`EhrbaseService::resync_promoted_columns`] then re-derives
/// from the loaded `EHR_STATUS`).
///
/// # Errors
/// [`ServiceError::Conflict`] when the record's subject is already held by
/// another EHR in this repository — possible only on a PARTIAL load into a
/// non-empty repository, since a self-consistent dump cannot clash with itself
/// (one EHR per subject — RM ehr master04 §EHR Status). The caller reports that
/// per record instead of aborting the load with an opaque constraint fault.
/// [`ServiceError::Database`] on any other insert failure (a duplicate EHR id
/// is pre-checked by the caller).
async fn insert_ehr_row(tx: &mut PgConnection, ehr: &EhrRow) -> Result<(), ServiceError> {
    sqlx::query(
        "INSERT INTO ehr (id, system_id, time_created, subject_id, subject_namespace) \
         VALUES ($1, $2, $3::timestamptz, $4, $5)",
    )
    .bind(ehr.id)
    .bind(&ehr.system_id)
    .bind(&ehr.time_created)
    .bind(&ehr.subject_id)
    .bind(&ehr.subject_namespace)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e
            && db.constraint() == Some("uq_ehr_subject")
        {
            return ServiceError::Conflict(format!(
                "EHR {} names subject {}@{}, which another EHR in this repository already \
                 holds (one EHR per subject)",
                ehr.id,
                ehr.subject_id.as_deref().unwrap_or("?"),
                ehr.subject_namespace.as_deref().unwrap_or("?"),
            ));
        }
        ServiceError::Database(e)
    })?;
    Ok(())
}

/// Insert one version row and its re-decomposed node rows (through the storage
/// codec). The `vo_version` row I/O is delegated to
/// [`crate::storage::version_repo::import::insert_version_verbatim`] (our own design
/// over the greenfield schema — no openEHR spec governs it); the node rows are
/// re-decomposed here through the shared codec.
async fn insert_version(
    tx: &mut PgConnection,
    ehr_id: EhrId,
    v: &VersionRecord,
) -> Result<(), ServiceError> {
    version_repo::import::insert_version_verbatim(
        tx,
        &version_repo::import::VerbatimVersionRow {
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
            signature_client_supplied: v.signature_client_supplied,
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

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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
