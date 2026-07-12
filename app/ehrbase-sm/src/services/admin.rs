//! The ADMIN group's application seam (SM `I_ADMIN_SERVICE` +
//! `I_ADMIN_ARCHIVE`).

use async_trait::async_trait;

use crate::error::{CallStatusType, SmError};

use crate::types::PlatformService;

/// A statistics time filter: an optional `(lower, upper)` pair of ISO 8601
/// date-time bounds, each independently optional (open bounds allowed). Realizes
/// the SM `time_interval: Interval<Iso8601_date_time> [0..1]` parameter of the
/// four `i_admin_service.adoc` statistics calls.
///
/// PORT NOTE: the SM `Interval` is treated as **closed** `[lower, upper]` — the
/// default openEHR `Interval` bound inclusivity — matched against each
/// CONTRIBUTION/version audit `time_committed`. An invalid ISO bound is a `400`
/// (rejected at the adapter before the query runs).
pub type StatTimeRange = Option<(Option<String>, Option<String>)>;

/// The ADMIN group's application seam (SM `I_ADMIN_SERVICE.physical_ehr_delete`,
/// requirement level 0..1 — an optional platform capability).
///
/// PORT NOTE: the ADMIN API is dev-branch only in ITS-REST (no vendored OAS;
/// CNF master12 is all TBD). The normative core is SM
/// `i_admin_service.adoc`: **physical** deletion of an EHR (precondition
/// `has_ehr`, error `ehr_id_does_not_exist`); the CNF Robot prior art
/// (`I_ADMIN_SERVICE/001-EHR.robot`) expects `204` and a full cascade (EHR,
/// `EHR_STATUS`, `EHR_ACCESS`, compositions, directory, contributions, audits all
/// physically gone). Unknown EHR → 404 (inferred HTTP mapping of
/// `ehr_id_does_not_exist`).
///
/// Every method defaults to `NotImplemented`, so [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait AdminService: Send + Sync {
    /// `DELETE /admin/ehr/{ehr_id}` — physically delete one EHR and every trace
    /// of it. `204`; unknown EHR → 404.
    async fn admin_ehr_delete(&self, _ehr_id: String) -> Result<(), SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `DELETE /admin/ehr/all{?ehr_id*}` — physically delete a **set** of EHRs.
    ///
    /// PORT NOTE: this operation has no spec at all (not in the SM, not in any
    /// OAS; the generated param under-models the RFC 6570 `ehr_id*` list as one
    /// optional string). Our design: `ehr_id` carries a comma-separated id list
    /// and only those EHRs are deleted; an absent/empty list deletes **nothing**
    /// and is a 400 (refusing an implicit delete-everything). Returns the number
    /// of EHRs actually deleted.
    async fn admin_ehr_delete_all(&self, _ehr_ids: Vec<String>) -> Result<u64, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `list_contributions` (`i_admin_service.adoc`): the ids of all
    /// CONTRIBUTIONs of the named versioned-content service, optionally within a
    /// time range. Requirement level 0..1 (optional platform capability).
    ///
    /// PORT NOTE: `a_service` selects the scope — `Ehr` → EHR-scoped
    /// contributions (`ehr_id IS NOT NULL`), `Demographic` → ehr-less
    /// (`ehr_id IS NULL`); every other member is not a versioned-content service
    /// and yields the empty list (`platform_service.adoc`; design fixed-decision
    /// SM-4). No ITS-REST wire (ADMIN dev-branch only) — native seam.
    async fn admin_list_contributions(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<Vec<String>, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `contribution_count` (`i_admin_service.adoc`): the count of all
    /// CONTRIBUTIONs of the named versioned-content service, optionally within a
    /// time range. Scope mapping as [`Self::admin_list_contributions`].
    async fn admin_contribution_count(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `versioned_composition_count` (`i_admin_service.adoc`): the count of all
    /// `VERSIONED_COMPOSITION`s (distinct COMPOSITION versioned objects), optionally
    /// filtered to those with a version committed within the time range.
    ///
    /// PORT NOTE: COMPOSITIONs are EHR-scoped, so only `a_service = Ehr` yields a
    /// non-zero count; every other member yields 0 (COMPOSITIONs are not part of
    /// their scope).
    async fn versioned_composition_count(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `composition_version_count` (`i_admin_service.adoc`): the count of all
    /// COMPOSITION *versions* (individual version rows), optionally within a time
    /// range. Scope gate as [`Self::versioned_composition_count`].
    async fn composition_version_count(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `physical_party_delete` (`i_admin_service.adoc`): physically delete a
    /// PARTY "along with related Party relationships". Requirement level 0..1;
    /// error `party_id_does_not_exist` → `404` (the natural REST reading, as for
    /// `physical_ehr_delete`). A cascading physical delete (party VO + every
    /// `PARTY_RELATIONSHIP` VO referencing it, with their versions/nodes/
    /// attestations, orphaned CONTRIBUTIONs/audits, and archive markers) in one
    /// transaction.
    async fn physical_party_delete(&self, _a_party_id: String) -> Result<(), SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }
}

/// The ARCHIVE group's application seam (SM `I_ADMIN_ARCHIVE`,
/// `docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc`).
///
/// "Move selected EHRs/Parties to archival storage." Both calls are requirement
/// level 0..1.
///
/// PORT NOTE: the SM defines no storage form for the archival tier. This phase
/// (SM-4) implements archival as a **marker** (`vo_archive`): the operations
/// record which versioned objects are archived; the actual storage movement to a
/// tier is P20 optimization. **Serving reads are unchanged** — nothing on the
/// read path consults `vo_archive` — so archival introduces zero wire drift
/// (design fixed-decision SM-4). Unknown id → `404`
/// (`ehr_id_does_not_exist` / `party_id_does_not_exist`), all-or-nothing.
///
/// Every method defaults to `NotImplemented`, so [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait AdminArchive: Send + Sync {
    /// `archive_ehrs(ehr_ids [0..1])` — move the selected EHRs to archival
    /// storage. All-or-nothing: any unknown id → `404` (`ehr_id_does_not_exist`)
    /// and nothing is archived. Idempotent (re-archiving an already-marked VO is
    /// a no-op). An empty/absent list archives nothing.
    async fn archive_ehrs(&self, _ehr_ids: Vec<String>) -> Result<(), SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `archive_parties(party_ids [0..1])` — move the selected Parties to
    /// archival storage. All-or-nothing: any unknown id → `404`
    /// (`party_id_does_not_exist`). Idempotent; empty/absent list archives
    /// nothing.
    ///
    /// PORT NOTE: `i_admin_archive.adoc` says "Move selected Parties **and
    /// relationships**"; this phase marks only the party VOs (design
    /// fixed-decision SM-4). Because archival is a marker with no read-path
    /// effect, not marking the relationship VOs has no observable consequence
    /// this phase; a storage-tier implementation (P20) would extend the marker
    /// set to the related relationships.
    async fn archive_parties(&self, _party_ids: Vec<String>) -> Result<(), SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }
}

/// `EXPORT_FORMAT` enumeration
/// (`docs/specs/openehr/SM/docs/UML/classes/export_format.adoc`): the logical
/// serialization flavour a dump is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// `openehr_canonical_xml`.
    OpenehrCanonicalXml,
    /// `openehr_canonical_json`.
    OpenehrCanonicalJson,
}

impl ExportFormat {
    /// The SM enumeration literal, exactly as the spec spells it.
    #[must_use]
    pub fn sm_name(self) -> &'static str {
        match self {
            Self::OpenehrCanonicalXml => "openehr_canonical_xml",
            Self::OpenehrCanonicalJson => "openehr_canonical_json",
        }
    }
}

/// `COMPRESSION_FORMAT` enumeration
/// (`docs/specs/openehr/SM/docs/UML/classes/compression_format.adoc`): the
/// compression to apply while dumping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// `zip`.
    Zip,
    /// `7z` (not a valid Rust identifier — the SM literal is `7z`).
    SevenZip,
}

impl CompressionFormat {
    /// The SM enumeration literal, exactly as the spec spells it.
    #[must_use]
    pub fn sm_name(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::SevenZip => "7z",
        }
    }
}

/// `EXPORT_SPEC` class
/// (`docs/specs/openehr/SM/docs/UML/classes/export_spec.adoc`): "the details for
/// an export operation".
///
/// PORT NOTE: `EXPORT_SPEC` carries `logical_format [0..1]`,
/// `compression_format [0..1]`, `encoding: ENCODING_FORMAT [0..1]`, and
/// `segment_split_size: Integer [1..1]` (kb). The `I_ADMIN_DUMP_LOAD.export_ehrs`
/// signature instead passes the three format enums *loose* and omits
/// `segment_split_size` entirely — `EXPORT_SPEC` is the SM's own richer bundle
/// for exactly this operation, so [`AdminDumpLoad::export_ehrs`] takes an
/// `ExportSpec` (the strictly more expressive form) and the loose params map
/// onto its fields. `ENCODING_FORMAT` is an **empty enumeration** (no values in
/// `encoding_format.adoc`), so the SM `encoding` attribute has no representable
/// value and is dropped here (`docs/design/sm-platform/
/// 04-message-subject-proxy-terminology-admin.md` §4.3).
#[derive(Debug, Clone)]
pub struct ExportSpec {
    /// Logical format to use, i.e. flavour of XML, JSON etc.
    pub logical_format: Option<ExportFormat>,
    /// Compression format to use during dump.
    pub compression_format: Option<CompressionFormat>,
    /// Size in kb of segment size on file system to split the export into
    /// (`segment_split_size`, 1..1).
    pub segment_split_size: i32,
}

impl ExportSpec {
    /// An uncompressed canonical-JSON export split into `segment_split_size_kb`
    /// segments — the format the greenfield storage exports natively (a deliberate design decision:
    /// `node.data` is verbatim canonical openEHR JSON).
    #[must_use]
    pub fn canonical_json(segment_split_size_kb: i32) -> Self {
        Self {
            logical_format: Some(ExportFormat::OpenehrCanonicalJson),
            compression_format: None,
            segment_split_size: segment_split_size_kb,
        }
    }
}

/// `DUMP_LOAD_FAIL_REPORT` class
/// (`docs/specs/openehr/SM/docs/UML/classes/dump_load_fail_report.adoc`):
/// "Dump or Load fail report for a single entity, e.g. `EHR`, `PARTY` etc."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpLoadFailReport {
    /// Type name of entity (`entity_type`, 1..1).
    pub entity_type: String,
    /// Identifier of entity (`entity_id`, 1..1).
    pub entity_id: String,
    /// Status of entity in the dump/load operation (`dump_status`, 1..1):
    /// `true` = successfully dumped/loaded; `false` = failed for this entity.
    pub dump_status: bool,
    /// Detailed error information, if available (`error`, 0..1).
    pub error: Option<String>,
}

/// The DUMP/LOAD group's application seam (SM `I_ADMIN_DUMP_LOAD`,
/// `docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc`).
///
/// "Interface to dump/load facilities." Both calls are requirement level 0..1
/// (optional platform capabilities). Each reports per-entity outcomes as
/// [`DumpLoadFailReport`]s; the only declared error for either is
/// `file_not_writable` ([`CallStatusType::FileNotWritable`]).
///
/// PORT NOTE (not on `Platform`): dump/load has **no ITS-REST wire** — the
/// ADMIN API is dev-branch only (no vendored OAS; CNF master12 is TBD) and no
/// dump/load route is defined anywhere. It is therefore a native-API-only,
/// CLI/ops-invoked capability and is deliberately **not** part of the
/// [`Platform`](crate::Platform) union (which is defined as "everything the
/// ITS-REST surface dispatches to"). The concrete service implements it
/// directly.
///
/// PORT NOTE (losslessness): a repository dump/load must be a *verbatim*
/// migration of an indelible versioned store (RM common master06 §Overview: a
/// versioned repository "is by definition indelible"). Load therefore
/// re-persists the exported versions with their original `OBJECT_VERSION_ID`s,
/// audit provenance, and commit times preserved — it does **not** replay them
/// through the ordinary create/update path (which would mint fresh version ids
/// and audit timestamps, i.e. lose information). "Re-commit" here means
/// re-insert the stored change-control state, not re-run the commit act.
///
/// Every method defaults to `NotImplemented` (a `501`) until the real service
/// overrides it.
#[async_trait]
pub trait AdminDumpLoad: Send + Sync {
    /// `export_ehrs` — export all EHRs to a file-system location in the format
    /// described by `spec`. Returns a per-entity report list (a failed entity
    /// carries `dump_status = false` + an `error`). Error `file_not_writable`
    /// when `file_sys_loc` cannot be written.
    async fn export_ehrs(
        &self,
        file_sys_loc: String,
        spec: ExportSpec,
    ) -> Result<Vec<DumpLoadFailReport>, SmError>;

    /// `load_ehrs` — populate the EHR repository from an export archive on the
    /// file system. "Repository need not be empty, but import EHRs with
    /// duplicate EHR ids will fail" — a duplicate is reported as a
    /// [`DumpLoadFailReport`] (`dump_status = false`), not a hard error, so a
    /// partial import proceeds. Error `file_not_writable` when the archive
    /// cannot be read.
    async fn load_ehrs(&self, file_sys_loc: String) -> Result<Vec<DumpLoadFailReport>, SmError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_enum_sm_names_match_the_spec_literals() {
        assert_eq!(
            ExportFormat::OpenehrCanonicalXml.sm_name(),
            "openehr_canonical_xml"
        );
        assert_eq!(
            ExportFormat::OpenehrCanonicalJson.sm_name(),
            "openehr_canonical_json"
        );
        assert_eq!(CompressionFormat::Zip.sm_name(), "zip");
        assert_eq!(CompressionFormat::SevenZip.sm_name(), "7z");
    }

    #[test]
    fn canonical_json_spec_defaults_to_uncompressed_json() {
        let spec = ExportSpec::canonical_json(1024);
        assert_eq!(
            spec.logical_format,
            Some(ExportFormat::OpenehrCanonicalJson)
        );
        assert_eq!(spec.compression_format, None);
        assert_eq!(spec.segment_split_size, 1024);
    }
}
