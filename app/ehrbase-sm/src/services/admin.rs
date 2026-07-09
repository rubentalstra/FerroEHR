//! The ADMIN group's application seam (SM `I_ADMIN_SERVICE` +
//! `I_ADMIN_ARCHIVE`).

use async_trait::async_trait;

use openehr_its::rest::runtime::ApiError;

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
    async fn admin_ehr_delete(&self, _ehr_id: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /admin/ehr/all{?ehr_id*}` — physically delete a **set** of EHRs.
    ///
    /// PORT NOTE: this operation has no spec at all (not in the SM, not in any
    /// OAS; the generated param under-models the RFC 6570 `ehr_id*` list as one
    /// optional string). Our design: `ehr_id` carries a comma-separated id list
    /// and only those EHRs are deleted; an absent/empty list deletes **nothing**
    /// and is a 400 (refusing an implicit delete-everything). Returns the number
    /// of EHRs actually deleted.
    async fn admin_ehr_delete_all(&self, _ehr_ids: Vec<String>) -> Result<u64, ApiError> {
        Err(ApiError::NotImplemented)
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
    ) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `contribution_count` (`i_admin_service.adoc`): the count of all
    /// CONTRIBUTIONs of the named versioned-content service, optionally within a
    /// time range. Scope mapping as [`Self::admin_list_contributions`].
    async fn admin_contribution_count(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
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
    ) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `composition_version_count` (`i_admin_service.adoc`): the count of all
    /// COMPOSITION *versions* (individual version rows), optionally within a time
    /// range. Scope gate as [`Self::versioned_composition_count`].
    async fn composition_version_count(
        &self,
        _a_service: PlatformService,
        _time_range: StatTimeRange,
    ) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `physical_party_delete` (`i_admin_service.adoc`): physically delete a
    /// PARTY "along with related Party relationships". Requirement level 0..1;
    /// error `party_id_does_not_exist` → `404` (the natural REST reading, as for
    /// `physical_ehr_delete`). A cascading physical delete (party VO + every
    /// `PARTY_RELATIONSHIP` VO referencing it, with their versions/nodes/
    /// attestations, orphaned CONTRIBUTIONs/audits, and archive markers) in one
    /// transaction.
    async fn physical_party_delete(&self, _a_party_id: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
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
    async fn archive_ehrs(&self, _ehr_ids: Vec<String>) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
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
    async fn archive_parties(&self, _party_ids: Vec<String>) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }
}
