//! The ADMIN group's application seam (SM `I_ADMIN_SERVICE`).

use async_trait::async_trait;

use openehr_its::rest::runtime::ApiError;

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
}
