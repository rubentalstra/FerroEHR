// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
//! The archive CONTAINER (SM `compression_format.adoc`): the entry set
//! (`manifest.json`, `segment-NNNN.json`, `blobs/<hex>`,
//! `versions/<version_uid>.xml`) is identical whichever container carries it —
//! `compression_format` absent ⇒ loose files under `file_sys_loc`, `zip` ⇒ the
//! same entries as DEFLATE members of one `archive.zip` there. `load_ehrs` takes
//! only `file_sys_loc` (`i_admin_dump_load.adoc`), so it never receives the
//! container choice and DETECTS it: a `manifest.json` in the directory is the
//! loose form, else an `archive.zip`, else an `archive.7z`.
//!
//! NOTE: no openEHR spec defines the container set or the detection rule — our
//! own design/extension.
//!
//! Export walks the greenfield storage and writes an archive to a file-system
//! directory, split into segment entries no larger than `segment_split_size`
//! kb. Each EHR is one record carrying its `ehr` row, its audit/contribution
//! provenance, and one entry per stored version. Load reads the archive back
//! and re-persists each EHR verbatim (preserved ids/times), re-decomposing each
//! version payload through the same codec — so a read after load reassembles
//! byte-equal canonical JSON. An EHR whose id already exists is reported in a
//! `DUMP_LOAD_FAIL_REPORT` and skipped ("import EHRs with duplicate EHR ids
//! will fail"), never a crash.
//!
//! NOTE (`export_format.adoc` — what the two `EXPORT_FORMAT` members mean
//! here, and why only the PAYLOAD changes between them). `EXPORT_SPEC`
//! (`export_spec.adoc`) calls `logical_format` the "logical format to use, i.e.
//! flavour of XML, JSON etc.", so it governs the serialization of the exported
//! CONTENT, not the archive's envelope:
//!
//! * The **envelope** — `manifest.json`, the `segment-NNNN.json` skeleton of
//!   storage rows, `blobs/` — stays JSON in BOTH members, because no XML form
//!   of it exists to target: the published ITS-XML bundles declare NO global
//!   element for `EHR`, `CONTRIBUTION` or a standalone `AUDIT_DETAILS` (both
//!   lineages checked; `crates/openehr-its/schemas/xml/`), so an XML envelope
//!   could only be invented. No openEHR spec defines an on-disk archive format
//!   at all — our own design/extension.
//! * The **payloads** DO have a published XML form, so they follow the member.
//!   `openehr_canonical_json` (the default) keeps each version's reassembled
//!   canonical openEHR JSON inline in the skeleton's `body`.
//!   `openehr_canonical_xml` externalizes it exactly the way `blobs/` already
//!   externalizes multimedia: the record carries `body_entry:
//!   "versions/<version_uid>.xml"` and the entry is a complete
//!   `ORIGINAL_VERSION` document under `ALL/Version.xsd`'s published
//!   `<version>` root (RM common master06 §Version and its Subtypes), in the
//!   nsv1 lineage the server also serves by default. A logically-deleted
//!   version stores no content, so it gets no document and no `body_entry` —
//!   symmetric with its `null` inline body.
//!
//! `segment_split_size` keeps its one meaning in both members (the skeleton
//! segments split by cumulative byte size); an externalized payload is one
//! document per version and is never split, like a blob. `load_ehrs` is passed
//! no format (`i_admin_dump_load.adoc`: `load_ehrs(file_sys_loc)`), so the
//! manifest's own `format` member tells it which payload form the archive
//! holds.
//!
//! `export_ehrs(an_ehr_id)` is EHR-scoped, and the archive carries EHR-owned
//! content only: `ehr`, `audit`, `contribution`, `vo_version`, `node`,
//! `ehr_folder` (the `EHR.folders` membership rows — RM ehr master04 §Folders),
//! `item_tag`, and any `vo_archive` markers for the EHR's versioned objects.
//! Global DEFINITION artefacts a version references (OPT 1.4 / ADL2 templates
//! via `vo_version.template_id` → `template_ref`, `stored_query`) are NOT
//! carried — they are provisioned through the DEFINITION API and must pre-exist
//! (a COMPOSITION referencing an absent template fails its FK on load, reported
//! per EHR). Demographic parties (ehr-less versioned objects) are outside this
//! EHR-scoped dump. The verbatim version-row re-persist is a storage seam
//! ([`crate::storage::version_repo::import::insert_version_verbatim`]); this module
//! keeps the archive format and orchestration.
//!
//! NOTE: `vo_attestation` rows are carried with their `at_committal` flag,
//! because an at-committal attestation is inside the version's signed canonical
//! form (RM common master06 §Digital Signature: "serialising the entire Version
//! object") — a restored version's stored `signature` verifies only if its
//! attestations restore with it.
//!
//! NOTE (load is a RESTORE, not an EHR creation): the load re-persists exactly
//! what the archive holds and never synthesizes content — in particular it does
//! NOT mint a missing `EHR_ACCESS` the way the EHR-Extract clone does, because
//! RM ehr master04 §EHR Creation governs *creating* an EHR and an archive record
//! is a previously-created one.
//!
//! The one thing the load re-derives rather than copies is the promoted `ehr`
//! column projection (the subject + status-flag cache of the loaded
//! `EHR_STATUS`), which is not content.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 3): EHR-Extract/TDD/dump-load compose over \
              verbatim stored content (RM common master06 §Copying)"
)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use openehr_its::xml::runtime::{FromXml, Namespace, ToXml};
use openehr_rm::prelude::{
    Agent, Composition, EhrAccess, EhrStatus, Folder, Group, Organisation, OriginalVersion, Person,
    Role,
};
use openehr_rm::v1_2::demographic::party_relationship::PartyRelationship;
use serde::de::DeserializeOwned;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::admin::types::{
    CompressionFormat, DumpLoadFailReport, ExportFormat, ExportSpec,
};
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};
use crate::storage::codec::decompose;
use crate::storage::{node_repo, version_repo};
use crate::versioning::audit::AuditInput;
use crate::versioning::object_version_id::{TreeId, object_version_id};
use crate::versioning::wire::{OriginalVersionParts, build_original_version, contribution_ref};

/// Lifecycle-state code of a logically-deleted version (RM common master06
/// §Logical Deletion) — such versions store no `node` rows, so their exported
/// `body` is `null`.
const DELETED_LIFECYCLE: &str = crate::versioning::lifecycle::state::DELETED;

/// The archive manifest (`manifest.json`) — enough to read the segments back.
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    /// The `EXPORT_FORMAT` member the export was written in
    /// (`export_format.adoc`, the vendored literal verbatim). `load_ehrs` is
    /// passed no format, so this is what tells it whether the segments carry
    /// inline JSON payloads or `versions/*.xml` entry references.
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
    /// The demographic wave's shared entry (`demographic-commons.json`) —
    /// contribution rows + their audits for the ehr-less scope. Absent
    /// (defaulted) for pre-wave archives and for repositories with no
    /// standalone demographic content.
    #[serde(default)]
    demographic_commons: Option<String>,
    /// The demographic wave's per-container segment files, in order. Empty
    /// (and defaulted for pre-wave archives) when the repository holds no
    /// ehr-less containers.
    #[serde(default)]
    demographic_segments: Vec<String>,
}

/// The demographic wave's shared rows: the ehr-less contributions and their
/// commit audits (a demographic contribution may commit several containers,
/// so these cannot live in any one container's record).
#[derive(Debug, Serialize, Deserialize)]
struct DemographicCommons {
    audits: Vec<AuditRow>,
    contributions: Vec<ContributionRow>,
}

/// One ehr-less demographic container's exported bundle (RM demographic
/// master02 §Versioning Semantics: every Party is stored in its own Version
/// container). Mirrors [`EhrRecord`] minus the EHR-scoped parts; the
/// per-version commit audits ride the record so the XML externalization can
/// build each `ORIGINAL_VERSION` envelope without the commons.
#[derive(Debug, Serialize, Deserialize)]
struct DemographicRecord {
    vo_id: VoId,
    kind: String,
    audits: Vec<AuditRow>,
    versions: Vec<VersionRecord>,
    #[serde(default)]
    attestations: Vec<AttestationRow>,
    #[serde(default)]
    item_tags: Vec<ItemTagRow>,
    #[serde(default)]
    archives: Vec<ArchiveRow>,
}

/// Insert one commit-audit row, identity-preserving on an existing id (the
/// demographic wave's audits may already exist on a partial load).
async fn insert_audit_row(tx: &mut PgConnection, a: &AuditRow) -> Result<(), ServiceError> {
    sqlx::query(
        "INSERT INTO audit (id, time_committed, system_id, change_type, description, \
         committer, attestation) VALUES ($1, $2::timestamptz, $3, $4, $5, $6, $7) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(a.id)
    .bind(&a.time_committed)
    .bind(&a.system_id)
    .bind(&a.change_type)
    .bind(&a.description)
    .bind(&a.committer)
    .bind(&a.attestation)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// One `audit` row into an [`AuditRow`].
fn audit_row_of(r: &sqlx::postgres::PgRow) -> Result<AuditRow, ServiceError> {
    Ok(AuditRow {
        id: r.try_get("id")?,
        time_committed: r
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff()
            .to_string(),
        system_id: r.try_get("system_id")?,
        change_type: r.try_get("change_type")?,
        description: r.try_get("description")?,
        committer: r.try_get("committer")?,
        attestation: r.try_get("attestation")?,
    })
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
    /// Every `vo_attestation` row of the EHR's versioned objects (RM common
    /// master06 §Attestation), in commit order. `default` tolerates archives
    /// dumped before attestations were carried (#1685).
    #[serde(default)]
    attestations: Vec<AttestationRow>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AttestationRow {
    id: Uuid,
    vo_id: VoId,
    sys_version: i32,
    contribution_id: Uuid,
    /// `ATTESTATION.time_committed` as an RFC 3339 instant (the same lossless
    /// `jiff` rendering [`AuditRow::time_committed`] uses).
    time_committed: String,
    /// Whether the ATTESTATION was on the VERSION at committal and is
    /// therefore inside its signed canonical form (RM common master06
    /// §Attestation / §Digital Signature).
    at_committal: bool,
    /// The canonical `ATTESTATION` fragment, verbatim.
    data: Value,
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
    /// `AUDIT_DETAILS.time_committed` as an RFC 3339 instant — `jiff`'s own
    /// rendering of the stored `timestamptz`, so the export can parse it back
    /// losslessly when it reassembles the version's `commit_audit`
    /// (RM common master04 §Audit Details). `PostgreSQL`'s `timestamptz` input
    /// accepts this form and the older `::text` rendering alike
    /// (<https://www.postgresql.org/docs/18/datatype-datetime.html>
    /// §Date/Time Input), so archives written before this projection still
    /// load.
    time_committed: String,
    system_id: String,
    change_type: String,
    /// The canonical `DV_TEXT` fragment of `AUDIT_DETAILS.description` (0..1).
    description: Option<Value>,
    committer: Value,
    /// The `ATTESTATION`-declared attributes when the audit is an `ATTESTATION`
    /// (RM common master06 §Attestation), else absent — so an archive
    /// round-trips the concrete audit class.
    #[serde(default)]
    attestation: Option<Value>,
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
    /// The wrapped `ORIGINAL_VERSION`'s own `{contribution, commit_audit,
    /// signature?}` on an `IMPORTED_VERSION` row (RM common master06 §Committal
    /// and Audits), `None` on a locally created version. `#[serde(default)]` so
    /// archives dumped before this member existed load as local originals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wrapped_original: Option<Value>,
    /// The reassembled canonical openEHR JSON, INLINE — the
    /// `openehr_canonical_json` payload form. `Value::Null` (serialized as an
    /// absent key) for a logically-deleted version, which stores no node rows,
    /// and for every version of an `openehr_canonical_xml` archive, whose
    /// payload lives in [`VersionRecord::body_entry`] instead.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    body: Value,
    /// The `versions/<version_uid>.xml` entry holding this version's complete
    /// `ORIGINAL_VERSION` document — the `openehr_canonical_xml` payload form.
    /// `None` for an inline-JSON archive and for a logically-deleted version
    /// (no content ⇒ no document). `#[serde(default)]` so archives written
    /// before the XML member existed load unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    body_entry: Option<String>,
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

/// The wire message of every filesystem-access failure of these operations.
/// Deliberately opaque: the configured `file_sys_loc` is SERVER deployment
/// layout, and the OS error names the same path — neither belongs in a
/// response body. The path and the OS diagnostic go to the trace record.
const LOCATION_MESSAGE: &str =
    "the configured archive location could not be read or written; see the server log";

/// Build a `file_not_writable` [`SmError`] (the only error `I_ADMIN_DUMP_LOAD`
/// declares) for a failed filesystem access to `path`.
///
/// The body carries [`LOCATION_MESSAGE`]; `path` + `err` are traced.
fn file_not_writable(path: &Path, err: &std::io::Error) -> SmError {
    tracing::error!(
        path = %path.display(),
        error = %err,
        "dump/load: archive location access failed → file_not_writable"
    );
    SmError::new(CallStatusType::FileNotWritable, LOCATION_MESSAGE.to_owned())
}

/// Build a `file_not_writable` [`SmError`] for an archive entry that was read
/// but is not parseable as part of this archive format (a mangled manifest, a
/// truncated or hand-edited segment).
///
/// NOTE (`i_admin_dump_load.adoc` declares `file_not_writable` as the ONE error
/// of both operations; no openEHR spec defines the on-disk archive format —
/// our own design/extension): a corrupt entry and an unreadable entry are the
/// same fact from the operation's point of view — `file_sys_loc` does not hold
/// a readable archive — so they carry the SAME SM error. Reporting a corrupt
/// input as `exception` instead would blame the server for the caller's
/// archive.
///
/// The body NAMES THE ENTRY — that is the caller-actionable fact about the
/// caller's own archive — but carries neither the server path nor the serde
/// diagnostic (offsets and Rust field names of our archive structs); both are
/// traced.
fn unreadable_archive_entry(path: &Path, entry: &str, err: &serde_json::Error) -> SmError {
    tracing::warn!(
        path = %path.display(),
        entry,
        error = %err,
        "dump/load: archive entry is not parseable → file_not_writable"
    );
    SmError::new(
        CallStatusType::FileNotWritable,
        format!("archive entry {entry} is not readable as part of this export archive"),
    )
}

/// The archive manifest entry name (both containers).
const MANIFEST_ENTRY: &str = "manifest.json";
/// The demographic wave's shared entry (contributions + their audits).
const DEMOGRAPHIC_COMMONS_ENTRY: &str = "demographic-commons.json";
/// The packed container's file name inside `file_sys_loc`.
const ZIP_ENTRY_FILE: &str = "archive.zip";
/// The packed 7z container entry (SM `COMPRESSION_FORMAT.7z`).
const SEVENZ_ENTRY_FILE: &str = "archive.7z";
/// The entry-name prefix of an externalized `DV_MULTIMEDIA` blob.
#[cfg(feature = "multimedia")]
const BLOB_PREFIX: &str = "blobs/";
/// The entry-name prefix of an externalized version payload document
/// (`EXPORT_FORMAT.openehr_canonical_xml`).
const VERSIONS_PREFIX: &str = "versions/";
/// The ITS-XML global element a `VERSION` document is written under — the
/// published-element fact is stated once, in the crate owning the schemas
/// (`openehr_its::xml::PUBLISHED_ROOTS`: `version` over the abstract
/// `VERSION`, so the instance names its concrete type with `xsi:type` — the
/// reason the archive serializes through
/// [`openehr_its::xml::to_canonical_xml_declared`]).
const VERSION_ROOT_TAG: &str = "version";
/// That element's declared (abstract) XSD type, from the same table.
const VERSION_ROOT_TYPE: &str = "VERSION";

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
    tracing::error!(
        path = %path.display(),
        operation = what,
        error = %err,
        "dump/load: 7z container fault → file_not_writable"
    );
    SmError::new(CallStatusType::FileNotWritable, LOCATION_MESSAGE.to_owned())
}

fn zip_fault(path: &Path, what: &str, err: &zip::result::ZipError) -> SmError {
    tracing::error!(
        path = %path.display(),
        operation = what,
        error = %err,
        "dump/load: zip container fault → file_not_writable"
    );
    SmError::new(CallStatusType::FileNotWritable, LOCATION_MESSAGE.to_owned())
}

// ── the openehr_canonical_xml payload form ───────────────────────────────────

/// The archive entry name carrying one version's `ORIGINAL_VERSION` document.
///
/// The name is the version's own `OBJECT_VERSION_ID` (BASE
/// `base_types/master05-identification_package.adoc` §Syntaxes:
/// `object_id, '::', creating_system_id, '::', version_tree_id`), so an
/// operator reading the
/// archive sees the openEHR identity of every payload straight off the entry
/// list. No openEHR spec defines an archive layout — our own design/extension.
///
/// # Errors
/// [`SmError`] `exception` when the composed name would escape the flat
/// `versions/` entry space. Only a deployment whose configured
/// `server.system_id` carries a path separator can reach this (the id is
/// server configuration, never client input, and is already validated
/// non-empty and `::`-free), and it would scatter the archive across
/// directories instead of writing the declared entry set.
fn version_entry_name(version_uid: &str) -> Result<String, SmError> {
    if version_uid.contains('/') || version_uid.contains('\\') {
        return Err(SmError::exception(format!(
            "version {version_uid} cannot be externalized: its creating system id carries a \
             path separator, which no archive entry name may contain"
        )));
    }
    Ok(format!("{VERSIONS_PREFIX}{version_uid}.xml"))
}

/// Reassemble one archived version's `ORIGINAL_VERSION` envelope as canonical
/// openEHR JSON, through the SAME builder the served version read uses
/// ([`build_original_version`]) — so an archived document and a served one are
/// the same object, not two renderings of it.
///
/// `audit` is the record's `audit` row the version's `audit_id` names (RM
/// common master06 §Version and its Subtypes: `VERSION.commit_audit` 1..1) —
/// used only for a locally created version; an imported one renders the WRAPPED
/// original's own foreign provenance instead (§Committal and Audits).
/// `attestations` are the version's [`AttestationRow`]s: the at-committal ones
/// render inside the built (signed) form and the after-committal ones append
/// outside it — the same split the served read makes (RM common master06
/// §Attestation / §Digital Signature; #1685).
///
/// # Errors
/// [`ServiceError::Internal`] when the archived audit carries a commit time
/// that is not an RFC 3339 instant, or an imported record's wrapped original is
/// missing a mandatory `VERSION` attribute; the [`AuditInput::canonical`]
/// rejection when the committer is not a canonical `PARTY_PROXY`.
fn original_version_envelope(
    v: &VersionRecord,
    audit: &AuditRow,
    attestations: &[&AttestationRow],
) -> Result<Value, ServiceError> {
    // Same discipline as the version read-back: an archived
    // `other_input_version_uids` that does not decode is a corrupt record, not
    // an absent merge list — restoring the version without its merge inputs
    // would silently rewrite history (RM common master06 §Distributed
    // Versioning).
    let other_input_version_uids: Vec<String> = v
        .other_input_version_uids
        .as_ref()
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|e| {
            ServiceError::internal(
                "archived other_input_version_uids is not a list of version uids",
                e,
            )
        })?
        .unwrap_or_default();
    let tree = TreeId::from_columns(v.trunk_version, v.branch_number, v.branch_version);
    // An IMPORTED_VERSION row's document is the WRAPPED original — its own
    // foreign contribution / commit_audit / signature — because that, not the
    // local wrapper, is what an `<version>` document of this version is
    // (RM common master06 §Committal and Audits); the local act rides the
    // record's own columns and is restored from them on load.
    let (contribution, commit_audit, signature) = if let Some(wrapped) = &v.wrapped_original {
        (
            wrapped.get("contribution").cloned().ok_or_else(|| {
                ServiceError::exception(format!(
                    "version {} of {} is imported but its wrapped ORIGINAL_VERSION carries no \
                     contribution",
                    v.sys_version, v.vo_id
                ))
            })?,
            wrapped.get("commit_audit").cloned().ok_or_else(|| {
                ServiceError::exception(format!(
                    "version {} of {} is imported but its wrapped ORIGINAL_VERSION carries no \
                     commit_audit",
                    v.sys_version, v.vo_id
                ))
            })?,
            wrapped
                .get("signature")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
    } else {
        let time_committed: jiff::Timestamp = audit.time_committed.parse().map_err(|e| {
            ServiceError::internal(
                format!(
                    "audit {} carries an uninterpretable commit time {:?}",
                    audit.id, audit.time_committed
                ),
                e,
            )
        })?;
        let commit_audit = AuditInput {
            system_id: audit.system_id.clone(),
            change_type: audit.change_type.clone(),
            description: audit
                .description
                .as_ref()
                .map(crate::versioning::audit::decode_description)
                .transpose()?,
            committer: crate::versioning::audit::party_proxy(&audit.committer)?,
            attestation: audit
                .attestation
                .as_ref()
                .map(crate::versioning::attestation::AttestationParts::decode)
                .transpose()?
                .map(Box::new),
        }
        .canonical(&time_committed);
        (
            contribution_ref(v.contribution_id),
            commit_audit,
            v.signature.clone(),
        )
    };
    let at_committal: Vec<Value> = attestations
        .iter()
        .filter(|a| a.at_committal)
        .map(|a| a.data.clone())
        .collect();
    let after_committal: Vec<Value> = attestations
        .iter()
        .filter(|a| !a.at_committal)
        .map(|a| a.data.clone())
        .collect();
    let mut ov = build_original_version(&OriginalVersionParts {
        creating_system_id: &v.creating_system_id,
        vo_id: v.vo_id,
        tree,
        preceding_version_uid: v.preceding_version_uid.as_deref(),
        other_input_version_uids: &other_input_version_uids,
        contribution: &contribution,
        commit_audit: &commit_audit,
        lifecycle_state: &v.lifecycle_state,
        data: &v.body,
        attestations: &at_committal,
        signature: signature.as_deref(),
    })?;
    crate::versioning::wire::append_after_committal_attestations(&mut ov, &after_committal);
    Ok(ov)
}

/// Serialize an `ORIGINAL_VERSION` envelope as a canonical-XML document under
/// the published `<version>` root, typed on the versioned object's payload
/// class `T`.
///
/// # Errors
/// [`ServiceError::Internal`] when the envelope does not read back as a typed
/// `ORIGINAL_VERSION<T>`, or the XML writer fails.
fn version_document_of<T: DeserializeOwned + ToXml>(
    envelope: &Value,
) -> Result<String, ServiceError> {
    let typed: OriginalVersion<T> = openehr_its::json::from_canonical_value(envelope)
        .map_err(|e| ServiceError::internal("typing the ORIGINAL_VERSION", e))?;
    openehr_its::xml::to_canonical_xml_declared(
        &typed,
        VERSION_ROOT_TAG,
        VERSION_ROOT_TYPE,
        Namespace::V1,
    )
    .map_err(|e| ServiceError::internal("serializing the ORIGINAL_VERSION to XML", e))
}

/// [`version_document_of`] dispatched on the stored `vo_version.kind` —
/// every versioned-object root the whole-repository archive carries: the
/// EHR-scoped ones (RM ehr master04 §EHR Class) and the ehr-less demographic
/// containers of the demographic wave (RM demographic master02 §Versioning
/// Semantics: PARTY and its descendants are versioned in their own
/// containers).
///
/// # Errors
/// [`ServiceError::Internal`] on an unknown kind or a codec failure.
fn version_document(kind: &str, envelope: &Value) -> Result<String, ServiceError> {
    match kind {
        "COMPOSITION" => version_document_of::<Composition>(envelope),
        "EHR_STATUS" => version_document_of::<EhrStatus>(envelope),
        "EHR_ACCESS" => version_document_of::<EhrAccess>(envelope),
        "FOLDER" => version_document_of::<Folder>(envelope),
        "PERSON" => version_document_of::<Person>(envelope),
        "ORGANISATION" => version_document_of::<Organisation>(envelope),
        "GROUP" => version_document_of::<Group>(envelope),
        "AGENT" => version_document_of::<Agent>(envelope),
        "ROLE" => version_document_of::<Role>(envelope),
        "PARTY_RELATIONSHIP" => version_document_of::<PartyRelationship>(envelope),
        other => Err(ServiceError::exception(format!(
            "no canonical-XML payload type for versioned-object kind {other}"
        ))),
    }
}

/// Why a `versions/*.xml` entry could not be read back into a payload.
#[derive(Debug, thiserror::Error)]
enum VersionPayloadError {
    /// The entry is not a readable canonical-XML `ORIGINAL_VERSION`.
    #[error("{0}")]
    Xml(#[from] openehr_its::xml::runtime::XmlError),
    /// The document parsed, but carries no `data` to restore.
    #[error("the ORIGINAL_VERSION document carries no `data`")]
    NoData,
    /// The record names a versioned-object kind with no canonical-XML type.
    #[error("no canonical-XML payload type for versioned-object kind {0}")]
    UnknownKind(String),
}

/// Read one `versions/*.xml` entry back into the version's canonical-JSON
/// payload — the inverse of [`version_document_of`].
///
/// # Errors
/// [`VersionPayloadError`]; the caller renders it into the record's
/// [`DumpLoadFailReport`].
fn version_payload_of<T: FromXml + Serialize>(xml: &str) -> Result<Value, VersionPayloadError> {
    let typed: OriginalVersion<T> = openehr_its::xml::from_canonical_xml(xml)?;
    let data = typed.data.ok_or(VersionPayloadError::NoData)?;
    Ok(openehr_its::json::to_canonical_value(&data))
}

/// [`version_payload_of`] dispatched on the record's stored kind.
///
/// # Errors
/// [`VersionPayloadError`].
fn version_payload(kind: &str, xml: &str) -> Result<Value, VersionPayloadError> {
    match kind {
        "COMPOSITION" => version_payload_of::<Composition>(xml),
        "EHR_STATUS" => version_payload_of::<EhrStatus>(xml),
        "EHR_ACCESS" => version_payload_of::<EhrAccess>(xml),
        "FOLDER" => version_payload_of::<Folder>(xml),
        "PERSON" => version_payload_of::<Person>(xml),
        "ORGANISATION" => version_payload_of::<Organisation>(xml),
        "GROUP" => version_payload_of::<Group>(xml),
        "AGENT" => version_payload_of::<Agent>(xml),
        "ROLE" => version_payload_of::<Role>(xml),
        "PARTY_RELATIONSHIP" => version_payload_of::<PartyRelationship>(xml),
        other => Err(VersionPayloadError::UnknownKind(other.to_owned())),
    }
}

/// Externalize every version payload of `records` as a canonical-XML
/// `ORIGINAL_VERSION` entry, replacing the record's inline `body` with the
/// entry reference. A logically-deleted version stores no content, so it gets
/// no document (its `body` is already `null` and its `body_entry` stays
/// `None`).
///
/// # Errors
/// [`SmError`] `exception` on a codec failure or a version whose commit audit
/// the record does not carry (a self-inconsistent collection); the container's
/// own `file_not_writable` when an entry cannot be written.
fn externalize_version_documents(
    archive: &mut ArchiveWriter,
    records: &mut [EhrRecord],
) -> Result<(), SmError> {
    for record in records {
        // Disjoint field borrows: the audit index reads `record.audits` while
        // the loop below rewrites `record.versions`.
        let audits: std::collections::HashMap<Uuid, &AuditRow> =
            record.audits.iter().map(|a| (a.id, a)).collect();
        let all_attestations = &record.attestations;
        for v in &mut record.versions {
            if v.body.is_null() {
                continue;
            }
            let attestation_rows: Vec<&AttestationRow> = all_attestations
                .iter()
                .filter(|a| a.vo_id == v.vo_id && a.sys_version == v.sys_version)
                .collect();
            let audit = audits.get(&v.audit_id).ok_or_else(|| {
                SmError::exception(format!(
                    "version {} of {} names commit audit {}, which the exported record does not \
                     carry",
                    v.sys_version, v.vo_id, v.audit_id
                ))
            })?;
            let envelope = original_version_envelope(v, audit, &attestation_rows)?;
            let document = version_document(&v.kind, &envelope)?;
            let name = version_entry_name(&object_version_id(
                v.vo_id,
                &v.creating_system_id,
                TreeId::from_columns(v.trunk_version, v.branch_number, v.branch_version),
            ))?;
            archive.write(&name, document.as_bytes())?;
            v.body_entry = Some(name);
            v.body = Value::Null;
        }
    }
    Ok(())
}

/// [`externalize_version_documents`] for the demographic wave: the same
/// per-version `ORIGINAL_VERSION` documents, with the commit audits read from
/// each container's own record.
///
/// # Errors
/// As [`externalize_version_documents`].
fn externalize_demographic_documents(
    archive: &mut ArchiveWriter,
    records: &mut [DemographicRecord],
) -> Result<(), SmError> {
    for record in records {
        let audits: std::collections::HashMap<Uuid, &AuditRow> =
            record.audits.iter().map(|a| (a.id, a)).collect();
        let all_attestations = &record.attestations;
        for v in &mut record.versions {
            if v.body.is_null() {
                continue;
            }
            let attestation_rows: Vec<&AttestationRow> = all_attestations
                .iter()
                .filter(|a| a.vo_id == v.vo_id && a.sys_version == v.sys_version)
                .collect();
            let audit = audits.get(&v.audit_id).ok_or_else(|| {
                SmError::exception(format!(
                    "version {} of {} names commit audit {}, which the exported record does not \
                     carry",
                    v.sys_version, v.vo_id, v.audit_id
                ))
            })?;
            let envelope = original_version_envelope(v, audit, &attestation_rows)?;
            let document = version_document(&v.kind, &envelope)?;
            let name = version_entry_name(&object_version_id(
                v.vo_id,
                &v.creating_system_id,
                TreeId::from_columns(v.trunk_version, v.branch_number, v.branch_version),
            ))?;
            archive.write(&name, document.as_bytes())?;
            v.body_entry = Some(name);
            v.body = Value::Null;
        }
    }
    Ok(())
}

/// Resolve an `openehr_canonical_xml` record's externalized payloads back into
/// inline canonical JSON, BEFORE anything is written — so a record with an
/// unreadable document is reported and skipped whole rather than half loaded.
///
/// # Errors
/// The failure as a human-readable message; the caller records it in the
/// record's [`DumpLoadFailReport`].
fn resolve_version_documents(
    archive: &mut ArchiveReader,
    record: &mut EhrRecord,
) -> Result<(), String> {
    resolve_versions(archive, &mut record.versions)
}

/// [`resolve_version_documents`] for a demographic record.
///
/// # Errors
/// As [`resolve_version_documents`].
fn resolve_demographic_documents(
    archive: &mut ArchiveReader,
    record: &mut DemographicRecord,
) -> Result<(), String> {
    resolve_versions(archive, &mut record.versions)
}

/// The shared per-version resolution both record shapes use.
///
/// # Errors
/// The failure as a human-readable message.
fn resolve_versions(
    archive: &mut ArchiveReader,
    versions: &mut [VersionRecord],
) -> Result<(), String> {
    for v in versions {
        let Some(entry) = v.body_entry.clone() else {
            // A logically-deleted version legitimately has no document; a live
            // one without an entry reference is a truncated skeleton.
            if v.lifecycle_state == DELETED_LIFECYCLE {
                continue;
            }
            return Err(format!(
                "version {} of {} carries neither an inline body nor a `body_entry`",
                v.sys_version, v.vo_id
            ));
        };
        let bytes = archive.read(&entry).map_err(|e| format!("{entry}: {e}"))?;
        let xml =
            std::str::from_utf8(&bytes).map_err(|e| format!("{entry} is not valid UTF-8: {e}"))?;
        v.body = version_payload(&v.kind, xml).map_err(|e| format!("{entry}: {e}"))?;
    }
    Ok(())
}

impl FerroEhrService {
    /// SM `export_ehrs`: export every EHR to an archive under `file_sys_loc`.
    /// Returns a per-entity report; an empty list means every EHR was dumped
    /// successfully (the report carries only failures).
    ///
    /// Both SM enumerations (`export_format.adoc` / `compression_format.adoc`)
    /// are realized in full. `COMPRESSION_FORMAT` — absent (loose files), `zip`,
    /// and `7z` (`sevenz-rust2`). `EXPORT_FORMAT` — `openehr_canonical_json`
    /// (the default when `logical_format` is absent, and translation-free: the
    /// storage IS verbatim canonical JSON) and `openehr_canonical_xml`, which
    /// externalizes each version payload as an `ORIGINAL_VERSION` document under
    /// the published ITS-XML `<version>` root while the archive's own envelope
    /// stays JSON in both members.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a non-positive
    ///   `segment_split_size`.
    /// - `file_not_writable` — the directory, a segment entry, a version-payload
    ///   entry, a blob entry, or the manifest cannot be created/written.
    /// - `exception` — a database/codec fault while collecting records or
    ///   serializing a version payload, or a blob-store fault while exporting
    ///   referenced multimedia.
    pub async fn export_ehrs(
        &self,
        file_sys_loc: String,
        spec: ExportSpec,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        let dir = Path::new(&file_sys_loc);
        // `EXPORT_SPEC.logical_format` is `[0..1]`; an absent one takes the
        // translation-free member.
        let format = spec
            .logical_format
            .unwrap_or(ExportFormat::OpenehrCanonicalJson);
        if spec.segment_split_size <= 0 {
            return Err(SmError::precondition(
                "segment_split_size must be a positive number of kb",
            ));
        }

        // Opening the container FIRST keeps a refusal from costing a storage
        // read.
        let mut archive = ArchiveWriter::create(dir, spec.compression_format)?;

        let mut records = self.collect_ehr_records().await?;
        // The demographic wave: ehr-less party/relationship containers, so a
        // whole-repository archive really is the whole repository.
        let (demographic_commons, mut demographic_records) =
            self.collect_demographic_wave().await?;

        // Our own extension (no openEHR spec governs multimedia offload): carry
        // every externalized DV_MULTIMEDIA blob the exported versions reference
        // as `blobs/<hex>` entries, so a load into an empty target re-populates
        // the object store. This runs BEFORE the XML externalization below,
        // which moves the payloads the blob scan reads out of the records.
        let blob_keys = self
            .export_referenced_blobs(&mut archive, &records, &demographic_records)
            .await?;
        let archive_version = if blob_keys.is_empty() { 1 } else { 2 };

        if format == ExportFormat::OpenehrCanonicalXml {
            externalize_version_documents(&mut archive, &mut records)?;
            externalize_demographic_documents(&mut archive, &mut demographic_records)?;
        }

        // Serialize each record once; the byte length drives segmenting.
        let mut serialized = Vec::with_capacity(records.len());
        for record in &records {
            serialized.push(serde_json::to_vec(record).map_err(ServiceError::from)?);
        }
        let sizes: Vec<usize> = serialized.iter().map(Vec::len).collect();
        let limit = usize::try_from(spec.segment_split_size.max(0))
            .unwrap_or(0)
            .saturating_mul(1024);
        let ranges = plan_segments(&sizes, limit);

        let mut segment_names = Vec::with_capacity(ranges.len());
        for (seg_no, range) in ranges.iter().enumerate() {
            let name = format!("segment-{seg_no:04}.json");
            // Re-serialize the segment as one JSON array of records. The range
            // was planned over `sizes`, which has the same length as `records`,
            // so it is in bounds; fetched rather than indexed so a planner
            // defect cannot panic the admin route.
            let slice = records.get(range.clone()).unwrap_or_default();
            let bytes = serde_json::to_vec(slice).map_err(ServiceError::from)?;
            archive.write(&name, &bytes)?;
            segment_names.push(name);
        }

        // The demographic wave's entries, only when the repository holds any
        // ehr-less content (an absent wave keeps pre-wave loaders untouched).
        let mut demographic_commons_entry = None;
        let mut demographic_segment_names = Vec::new();
        if !demographic_records.is_empty() {
            let commons_bytes =
                serde_json::to_vec(&demographic_commons).map_err(ServiceError::from)?;
            archive.write(DEMOGRAPHIC_COMMONS_ENTRY, &commons_bytes)?;
            demographic_commons_entry = Some(DEMOGRAPHIC_COMMONS_ENTRY.to_owned());

            let mut serialized = Vec::with_capacity(demographic_records.len());
            for record in &demographic_records {
                serialized.push(serde_json::to_vec(record).map_err(ServiceError::from)?);
            }
            let sizes: Vec<usize> = serialized.iter().map(Vec::len).collect();
            for (seg_no, range) in plan_segments(&sizes, limit).iter().enumerate() {
                let name = format!("demographic-{seg_no:04}.json");
                let slice = demographic_records.get(range.clone()).unwrap_or_default();
                let bytes = serde_json::to_vec(slice).map_err(ServiceError::from)?;
                archive.write(&name, &bytes)?;
                demographic_segment_names.push(name);
            }
        }

        let manifest = Manifest {
            format: format.sm_name().to_owned(),
            archive_version,
            segment_split_size_kb: spec.segment_split_size,
            ehr_count: records.len(),
            segments: segment_names,
            blobs: blob_keys,
            demographic_commons: demographic_commons_entry,
            demographic_segments: demographic_segment_names,
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(ServiceError::from)?;
        archive.write(MANIFEST_ENTRY, &manifest_bytes)?;
        archive.finish()?;

        // Every EHR dumped successfully → no failure entries.
        Ok(Vec::new())
    }

    /// SM `load_ehrs`: populate the repository from an archive under
    /// `file_sys_loc`. Duplicate EHR ids — a subject this repository already
    /// holds under another EHR, and a record whose externalized version
    /// payload will not read — are reported (`dump_status = false`) and
    /// skipped; all other EHRs are re-persisted verbatim.
    ///
    /// The operation is passed no format (`i_admin_dump_load.adoc`:
    /// `load_ehrs(file_sys_loc)`), so BOTH the container (loose / `zip` / `7z`)
    /// and the payload form come from the archive itself: the container from
    /// what the location holds, the payload form from the manifest's own
    /// `EXPORT_FORMAT` member.
    ///
    /// # Errors
    /// - `file_not_writable` — `file_sys_loc` holds no archive container,
    ///   the manifest / a segment entry / a blob entry cannot be read, the
    ///   manifest or a segment is not parseable as part of this archive format
    ///   (a mangled or truncated archive is the same fact as an unreadable
    ///   one — see `unreadable_archive_entry`), or the manifest declares a
    ///   logical format that names no `EXPORT_FORMAT` member.
    /// - `precondition_violation` (`400`) — the archive carries externalized
    ///   multimedia blobs but this server has no multimedia store configured.
    /// - `unprocessable` — an archive record carries overlapping version
    ///   validity periods (a corrupted/hand-crafted archive; the record's
    ///   transaction is rolled back).
    /// - `exception` — a database/codec fault while re-persisting, or a
    ///   blob-store fault while importing.
    pub async fn load_ehrs(
        &self,
        file_sys_loc: String,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        let dir = Path::new(&file_sys_loc);
        // `load_ehrs` is passed no format, so the container is detected from
        // what the location holds (module docs).
        let mut archive = ArchiveReader::open(dir)?;
        let manifest_bytes = archive.read(MANIFEST_ENTRY)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| unreadable_archive_entry(dir, MANIFEST_ENTRY, &e))?;
        // The manifest's own EXPORT_FORMAT member is what tells the format-less
        // operation which payload form the segments carry.
        let format: ExportFormat = manifest.format.parse().map_err(|()| {
            // The manifest's declared value IS the caller's archive content, so
            // the body names it; the server path stays out of the body.
            SmError::new(
                CallStatusType::FileNotWritable,
                format!(
                    "archive entry {MANIFEST_ENTRY} declares logical format {:?}, which names \
                     no EXPORT_FORMAT member",
                    manifest.format
                ),
            )
        })?;

        // Our own extension: re-populate the object store from the archive's
        // `blobs/` entries before loading versions that reference them.
        self.import_blobs(&mut archive, &manifest.blobs).await?;

        let mut reports = Vec::new();
        for segment in &manifest.segments {
            let bytes = archive.read(segment)?;
            let records: Vec<EhrRecord> = serde_json::from_slice(&bytes)
                .map_err(|e| unreadable_archive_entry(dir, segment, &e))?;
            for mut record in records {
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
                // `openehr_canonical_xml`: pull each version's payload out of
                // its `versions/*.xml` entry BEFORE the record's transaction
                // opens, so an unreadable document costs that one record a
                // report and commits nothing (the same per-entity shape the SM
                // gives a duplicate id), instead of aborting the whole load.
                if format == ExportFormat::OpenehrCanonicalXml
                    && let Err(message) = resolve_version_documents(&mut archive, &mut record)
                {
                    reports.push(DumpLoadFailReport {
                        entity_type: "EHR".to_owned(),
                        entity_id: ehr_id.to_string(),
                        dump_status: false,
                        error: Some(format!(
                            "the archive's canonical-XML version payload is unreadable: {message}"
                        )),
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
                    Err(ServiceError::Conflict(e)) => reports.push(DumpLoadFailReport {
                        entity_type: "EHR".to_owned(),
                        entity_id: ehr_id.to_string(),
                        dump_status: false,
                        error: Some(e.message),
                    }),
                    Err(e) => return Err(e.into()),
                }
            }
        }

        // The demographic wave (absent in pre-wave archives): the shared
        // contribution commons first, then each ehr-less container. Duplicate
        // container ids are per-entity reports, never fatal — the same shape
        // the SM gives duplicate EHR ids.
        if let Some(commons_entry) = &manifest.demographic_commons {
            let bytes = archive.read(commons_entry)?;
            let commons: DemographicCommons = serde_json::from_slice(&bytes)
                .map_err(|e| unreadable_archive_entry(dir, commons_entry, &e))?;
            self.load_demographic_commons(&commons).await?;
        }
        for segment in &manifest.demographic_segments {
            let bytes = archive.read(segment)?;
            let records: Vec<DemographicRecord> = serde_json::from_slice(&bytes)
                .map_err(|e| unreadable_archive_entry(dir, segment, &e))?;
            for mut record in records {
                if self.demographic_container_exists(record.vo_id).await? {
                    reports.push(DumpLoadFailReport {
                        entity_type: record.kind.clone(),
                        entity_id: record.vo_id.to_string(),
                        dump_status: false,
                        error: Some(
                            "a demographic container with this id already exists".to_owned(),
                        ),
                    });
                    continue;
                }
                if format == ExportFormat::OpenehrCanonicalXml
                    && let Err(message) = resolve_demographic_documents(&mut archive, &mut record)
                {
                    reports.push(DumpLoadFailReport {
                        entity_type: record.kind.clone(),
                        entity_id: record.vo_id.to_string(),
                        dump_status: false,
                        error: Some(format!(
                            "the archive's canonical-XML version payload is unreadable: {message}"
                        )),
                    });
                    continue;
                }
                self.load_one_demographic(&record).await?;
            }
        }
        Ok(reports)
    }

    /// Fetch every externalized `DV_MULTIMEDIA` blob referenced by the exported
    /// records into `blobs/<hex>` archive entries, returning the blob keys
    /// written (empty when externalization is off). Our own extension — no
    /// openEHR spec governs multimedia offload.
    #[cfg(feature = "multimedia")]
    async fn export_referenced_blobs(
        &self,
        archive: &mut ArchiveWriter,
        records: &[EhrRecord],
        demographic: &[DemographicRecord],
    ) -> Result<Vec<String>, SmError> {
        let Some(engine) = &self.multimedia else {
            return Ok(Vec::new());
        };
        let mut keys: Vec<String> = records
            .iter()
            .flat_map(|r| r.versions.iter())
            .chain(demographic.iter().flat_map(|r| r.versions.iter()))
            .flat_map(|v| engine.referenced_keys(&v.body))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for hex in &keys {
            let bytes = engine.store().get(hex).await.map_err(|e| {
                crate::service::error::internal_fault("export a multimedia blob", &e)
            })?;
            archive.write(&format!("{BLOB_PREFIX}{hex}"), &bytes)?;
        }
        Ok(keys)
    }

    /// The slim twin: externalization is compiled out, so no stored record
    /// references a blob and nothing is exported.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    async fn export_referenced_blobs(
        &self,
        _archive: &mut ArchiveWriter,
        _records: &[EhrRecord],
        _demographic: &[DemographicRecord],
    ) -> Result<Vec<String>, SmError> {
        Ok(Vec::new())
    }

    /// Re-put each archived blob (`blobs/<hex>`) into the object store on load
    /// (idempotent, content-addressed). A no-op when the archive carries no
    /// blobs. Our own extension.
    #[cfg(feature = "multimedia")]
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
                .map_err(|e| {
                    crate::service::error::internal_fault("import a multimedia blob", &e)
                })?;
        }
        Ok(())
    }

    /// The slim twin: a blob-carrying archive cannot be loaded into a binary
    /// built without the `multimedia` feature — refuse loudly rather than
    /// silently dropping content.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    async fn import_blobs(
        &self,
        _archive: &mut ArchiveReader,
        blobs: &[String],
    ) -> Result<(), SmError> {
        if blobs.is_empty() {
            return Ok(());
        }
        Err(SmError::precondition(
            "archive carries externalized multimedia blobs but this binary \
             was built without the `multimedia` cargo feature",
        ))
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

    /// Whether an ehr-less demographic container with `vo_id` already exists.
    async fn demographic_container_exists(&self, vo_id: VoId) -> Result<bool, ServiceError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM vo_version_all WHERE vo_id = $1 AND ehr_id IS NULL)",
        )
        .bind(vo_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Load the demographic wave's shared rows. Audits and contributions may
    /// already exist on a partial load into a non-empty repository (one
    /// demographic contribution can commit several containers), so both
    /// inserts are identity-preserving no-ops on an existing id.
    async fn load_demographic_commons(
        &self,
        commons: &DemographicCommons,
    ) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for a in &commons.audits {
            insert_audit_row(&mut tx, a).await?;
        }
        for c in &commons.contributions {
            sqlx::query(
                "INSERT INTO contribution (id, ehr_id, audit_id) VALUES ($1, NULL, $2) \
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(c.id)
            .bind(c.audit_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Load one ehr-less demographic container: its per-version audits, every
    /// version (`vo_version` + re-decomposed `node` rows, `ehr_id` NULL),
    /// attestations, demographic tags and archive rows — one transaction, so
    /// a failed container commits nothing.
    async fn load_one_demographic(&self, record: &DemographicRecord) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for a in &record.audits {
            insert_audit_row(&mut tx, a).await?;
        }
        for v in &record.versions {
            insert_version(&mut tx, None, v).await?;
        }
        load_attestations(&mut tx, &record.attestations).await?;
        for t in &record.item_tags {
            sqlx::query(
                "INSERT INTO item_tag (id, ehr_id, target_vo_id, target_type, key, value, \
                 target_path, created_at) VALUES ($1, NULL, $2, $3, $4, $5, $6, $7::timestamptz)",
            )
            .bind(t.id)
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
        tx.commit().await?;
        Ok(())
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

    /// Read the demographic wave: the ehr-less contribution commons plus one
    /// record per standalone container (RM demographic master02 §Versioning
    /// Semantics), ordered by container id for a deterministic archive.
    /// NOTE: no openEHR spec governs the archive format — our own
    /// design/extension; the wave keeps the format's archive ⇒ repository
    /// identity over the demographic store.
    async fn collect_demographic_wave(
        &self,
    ) -> Result<(DemographicCommons, Vec<DemographicRecord>), ServiceError> {
        let audit_rows = sqlx::query(
            "SELECT id, time_committed, system_id, change_type, \
             description, committer, attestation FROM audit \
             WHERE id IN (SELECT audit_id FROM contribution WHERE ehr_id IS NULL) \
             ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut commons_audits = Vec::with_capacity(audit_rows.len());
        for r in audit_rows {
            commons_audits.push(audit_row_of(&r)?);
        }
        let contribution_rows =
            sqlx::query("SELECT id, audit_id FROM contribution WHERE ehr_id IS NULL ORDER BY id")
                .fetch_all(&self.pool)
                .await?;
        let mut contributions = Vec::with_capacity(contribution_rows.len());
        for r in contribution_rows {
            contributions.push(ContributionRow {
                id: r.try_get("id")?,
                audit_id: r.try_get("audit_id")?,
            });
        }
        let commons = DemographicCommons {
            audits: commons_audits,
            contributions,
        };

        let container_rows = sqlx::query(
            "SELECT DISTINCT vo_id, kind FROM vo_version_all WHERE ehr_id IS NULL \
             ORDER BY vo_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut records = Vec::with_capacity(container_rows.len());
        for c in container_rows {
            let vo_id: VoId = c.try_get("vo_id")?;
            let kind: String = c.try_get("kind")?;
            records.push(self.collect_one_demographic(vo_id, kind).await?);
        }
        Ok((commons, records))
    }

    /// Read one ehr-less container's versions, per-version audits,
    /// attestations, tags and archive rows.
    async fn collect_one_demographic(
        &self,
        vo_id: VoId,
        kind: String,
    ) -> Result<DemographicRecord, ServiceError> {
        let audit_rows = sqlx::query(
            "SELECT id, time_committed, system_id, change_type, \
             description, committer, attestation FROM audit \
             WHERE id IN (SELECT audit_id FROM vo_version_all \
                          WHERE vo_id = $1 AND ehr_id IS NULL) \
             ORDER BY id",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        let mut audits = Vec::with_capacity(audit_rows.len());
        for r in audit_rows {
            audits.push(audit_row_of(&r)?);
        }

        let version_rows = sqlx::query(
            "SELECT vo_id, kind, sys_version, trunk_version, branch_number, branch_version, \
             preceding_version_uid, other_input_version_uids, lower(sys_period)::text AS lo, \
             upper(sys_period)::text AS hi, lifecycle_state, contribution_id, audit_id, \
             template_id, signature, signature_client_supplied, creating_system_id, \
             wrapped_original \
             FROM vo_version_all WHERE vo_id = $1 AND ehr_id IS NULL \
             ORDER BY sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        let mut versions = Vec::with_capacity(version_rows.len());
        for r in version_rows {
            versions.push(self.version_record_of(&r).await?);
        }

        let attestation_rows = sqlx::query(
            "SELECT id, vo_id, sys_version, contribution_id, time_committed, at_committal, \
             data FROM vo_attestation_all WHERE vo_id = $1 \
             ORDER BY sys_version, time_committed, id",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        let mut attestations = Vec::with_capacity(attestation_rows.len());
        for r in attestation_rows {
            attestations.push(AttestationRow {
                id: r.try_get("id")?,
                vo_id: r.try_get("vo_id")?,
                sys_version: r.try_get("sys_version")?,
                contribution_id: r.try_get("contribution_id")?,
                time_committed: r
                    .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                    .to_jiff()
                    .to_string(),
                at_committal: r.try_get("at_committal")?,
                data: r.try_get("data")?,
            });
        }

        let tag_rows = sqlx::query(
            "SELECT id, target_vo_id, target_type, key, value, target_path, \
             created_at::text AS created_at FROM item_tag \
             WHERE ehr_id IS NULL AND target_vo_id = $1 ORDER BY id",
        )
        .bind(vo_id)
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
             WHERE vo_id = $1 ORDER BY vo_id",
        )
        .bind(vo_id)
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

        Ok(DemographicRecord {
            vo_id,
            kind,
            audits,
            versions,
            attestations,
            item_tags,
            archives,
        })
    }

    /// One `vo_version_all` row into a [`VersionRecord`], its body read
    /// across both storage tiers (deleted versions keep a `null` body).
    async fn version_record_of(
        &self,
        r: &sqlx::postgres::PgRow,
    ) -> Result<VersionRecord, ServiceError> {
        let vo_id: VoId = r.try_get("vo_id")?;
        let sys_version: i32 = r.try_get("sys_version")?;
        let lifecycle_state: String = r.try_get("lifecycle_state")?;
        let body = if lifecycle_state == DELETED_LIFECYCLE {
            Value::Null
        } else {
            node_repo::read_version_canonical_all(&self.pool, vo_id, sys_version).await?
        };
        Ok(VersionRecord {
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
            wrapped_original: r.try_get("wrapped_original")?,
            body,
            body_entry: None,
        })
    }

    /// Read one EHR's `ehr`/audit/contribution/version/tag/archive content. The
    /// per-version canonical body is reassembled through the storage codec
    /// ([`node_repo::read_version_canonical`] — the codec's lossless inverse).
    #[expect(
        clippy::too_many_lines,
        reason = "one linear per-EHR collection pass; splitting it would hide \
                  the segment order"
    )]
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
            "SELECT id, time_committed, system_id, change_type, \
             description, committer, attestation FROM audit \
             WHERE id IN (SELECT audit_id FROM contribution WHERE ehr_id = $1 \
                          UNION SELECT audit_id FROM vo_version_all WHERE ehr_id = $1) \
             ORDER BY id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut audits = Vec::with_capacity(audit_rows.len());
        for r in audit_rows {
            audits.push(AuditRow {
                id: r.try_get("id")?,
                time_committed: r
                    .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                    .to_jiff()
                    .to_string(),
                system_id: r.try_get("system_id")?,
                change_type: r.try_get("change_type")?,
                description: r.try_get("description")?,
                committer: r.try_get("committer")?,
                attestation: r.try_get("attestation")?,
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
             template_id, signature, signature_client_supplied, creating_system_id, \
             wrapped_original \
             FROM vo_version_all WHERE ehr_id = $1 ORDER BY vo_id, sys_version",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut versions = Vec::with_capacity(version_rows.len());
        for r in version_rows {
            let vo_id: VoId = r.try_get("vo_id")?;
            let sys_version: i32 = r.try_get("sys_version")?;
            let lifecycle_state: String = r.try_get("lifecycle_state")?;
            // A deleted version has no node rows; its body stays `null`. An
            // export covers the WHOLE repository, so the content is read across
            // both storage tiers (`crate::storage::version_repo::tier`).
            // NOTE: no openEHR spec governs the `spec_profile` gate — our own
            // design/extension: a dump replicates stored bytes for restore
            // rather than serving an RM body, so it reads ungated.
            let body = if lifecycle_state == DELETED_LIFECYCLE {
                Value::Null
            } else {
                node_repo::read_version_canonical_all(&self.pool, vo_id, sys_version).await?
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
                wrapped_original: r.try_get("wrapped_original")?,
                body,
                // Filled in by `externalize_version_documents` when the export
                // is `openehr_canonical_xml`.
                body_entry: None,
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
             WHERE vo_id IN (SELECT DISTINCT vo_id FROM vo_version_all WHERE ehr_id = $1) \
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

        let attestation_rows = sqlx::query(
            "SELECT id, vo_id, sys_version, contribution_id, time_committed, at_committal, \
             data FROM vo_attestation_all \
             WHERE vo_id IN (SELECT DISTINCT vo_id FROM vo_version_all WHERE ehr_id = $1) \
             ORDER BY vo_id, sys_version, time_committed, id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut attestations = Vec::with_capacity(attestation_rows.len());
        for r in attestation_rows {
            attestations.push(AttestationRow {
                id: r.try_get("id")?,
                vo_id: r.try_get("vo_id")?,
                sys_version: r.try_get("sys_version")?,
                contribution_id: r.try_get("contribution_id")?,
                time_committed: r
                    .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                    .to_jiff()
                    .to_string(),
                at_committal: r.try_get("at_committal")?,
                data: r.try_get("data")?,
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
            attestations,
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
                 committer, attestation) VALUES ($1, $2::timestamptz, $3, $4, $5, $6, $7)",
            )
            .bind(a.id)
            .bind(&a.time_committed)
            .bind(&a.system_id)
            .bind(&a.change_type)
            .bind(&a.description)
            .bind(&a.committer)
            .bind(&a.attestation)
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
            insert_version(&mut tx, Some(ehr_id), v).await?;
        }

        load_attestations(&mut tx, &record.attestations).await?;

        // The load re-decomposed the EHR_STATUS versions directly, so the
        // promoted `ehr` columns are re-derived from the loaded current status
        // — the EHR_STATUS content is the truth, the exported columns only its
        // cached projection. This makes a loaded EHR visible to the subject
        // lookup (SM `I_EHR_SERVICE.get_ehrs_for_subject`) and bound by the
        // one-EHR-per-subject rule (RM ehr master04 §EHR Status), and keeps
        // `is_queryable`/`is_modifiable` matching the loaded state for the AQL
        // full-population gate and the content-write guard (§EHR Active Status).
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
        // rows are one lineage per vo_id; branch rows are per {vo, creating
        // system, fork point, branch number}. The archive is the ONLY path
        // writing explicit historical `sys_period` bounds, so it carries the
        // per-lineage temporal non-overlap invariant check the regular write
        // path holds by construction (RM common master06: one valid version per
        // lineage at any instant). A corrupted archive with overlapping validity
        // fails the whole record before commit.
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
            return Err(ServiceError::content_invalid(
                crate::service::error::Violation::new(format!(
                    "archive for EHR {ehr_id} carries overlapping version validity periods"
                )),
            ));
        }

        // Every loaded row went into the primary tier; the ones the record
        // marks archived belong in the cold tier, so the invariant "a marker
        // means the rows are in `cold`" holds for a loaded EHR exactly as it
        // does for a locally archived one.
        let archived: Vec<VoId> = record.archives.iter().map(|ar| ar.vo_id).collect();
        version_repo::tier::freeze(&mut tx, &archived).await?;

        tx.commit().await?;
        Ok(())
    }
}

/// Re-persist the archived `ehr` root row verbatim (preserved id, immutable
/// `system_id` and `time_created`, plus the archived promoted-column
/// projection, which [`FerroEhrService::resync_promoted_columns`] then re-derives
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
            // NOTE: SM ehr_call_status_type.adoc declares
            // ehr_for_subject_already_exists for the one-EHR-per-subject rule.
            return ServiceError::sm(
                CallStatusType::EhrForSubjectAlreadyExists,
                format!(
                    "EHR {} names subject {}@{}, which another EHR in this repository \
                     already holds (one EHR per subject)",
                    ehr.id,
                    ehr.subject_id.as_deref().unwrap_or("?"),
                    ehr.subject_namespace.as_deref().unwrap_or("?"),
                ),
            );
        }
        ServiceError::Database(e)
    })?;
    Ok(())
}

/// Re-persist the archived `vo_attestation` rows verbatim, once their FK
/// targets (the version and contribution rows) exist — an at-committal
/// attestation is inside the version's signed canonical form (RM common
/// master06 §Digital Signature), so a restore without them would break
/// `verify_on_read` on the restored version (#1685).
///
/// # Errors
/// The underlying insert failure as [`ServiceError::Database`].
async fn load_attestations(
    tx: &mut PgConnection,
    attestations: &[AttestationRow],
) -> Result<(), ServiceError> {
    for a in attestations {
        sqlx::query(
            "INSERT INTO vo_attestation (id, vo_id, sys_version, contribution_id, \
             time_committed, at_committal, data) \
             VALUES ($1, $2, $3, $4, $5::timestamptz, $6, $7)",
        )
        .bind(a.id)
        .bind(a.vo_id)
        .bind(a.sys_version)
        .bind(a.contribution_id)
        .bind(&a.time_committed)
        .bind(a.at_committal)
        .bind(&a.data)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// Insert one version row and its re-decomposed node rows (through the storage
/// codec). The `vo_version` row I/O is delegated to
/// [`crate::storage::version_repo::import::insert_version_verbatim`] (our own design
/// over the greenfield schema — no openEHR spec governs it); the node rows are
/// re-decomposed here through the shared codec.
async fn insert_version(
    tx: &mut PgConnection,
    ehr_id: Option<EhrId>,
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
            wrapped_original: v.wrapped_original.as_ref(),
            body: (!v.body.is_null()).then_some(&v.body),
        },
    )
    .await?;

    // A deleted version (null body) stores no node rows.
    if v.body.is_null() {
        return Ok(());
    }
    let rows = decompose(v.body.clone())?;
    node_repo::write_nodes(tx, v.vo_id, v.sys_version, ehr_id, &rows).await?;
    Ok(())
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
