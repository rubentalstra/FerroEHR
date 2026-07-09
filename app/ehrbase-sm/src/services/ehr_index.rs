//! The SM `I_EHR_INDEX` interface — subject↔EHR association management.

use async_trait::async_trait;

use crate::error::{CallStatusType, SmError};

use crate::types::{EhrIndexEntry, LocationDesc, ResourceStatus, SubjectRef};

/// The EHR Index service seam (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_index.adoc`,
/// `master07-ehr_index_service.adoc`).
///
/// Records N:M associations of subject identifiers with EHR ids, plus the
/// `RESOURCE_STATUS` (instance type + validity period + notes) and
/// `LOCATION_DESC` metadata master07 uses for duplicate management. Index
/// entries are **not** versioned objects — the SM defines no versioning here —
/// so writes emit no CONTRIBUTION (design 08 §4.1; PORT NOTE at the impl).
///
/// PORT NOTE: the SM defines no read operations; the two design-filled getters
/// ([`Self::ehr_subjects`] / [`Self::subject_ehrs`]) fill that silence (design
/// 03 §5.9). Every method defaults to `NotImplemented`/empty.
///
/// Errors follow `i_ehr_index.adoc`: an unknown EHR → `ehr_id_does_not_exist`
/// (`404`), an unknown subject/association → `subject_id_does_not_exist`
/// (`404`).
#[async_trait]
pub trait EhrIndexService: Send + Sync {
    /// `add_ehr_subject` — associate `subject` with `ehr_id`, with an optional
    /// status and location descriptor (defaults to a `Primary` instance).
    /// Errors: `ehr_id_does_not_exist` → `404`.
    async fn add_ehr_subject(
        &self,
        _ehr_id: String,
        _subject: SubjectRef,
        _status: Option<ResourceStatus>,
        _loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `update_ehr_subject_status` — update the `RESOURCE_STATUS` of the
    /// (`ehr_id`, `subject`) association. Errors: `subject_id_does_not_exist` /
    /// `ehr_id_does_not_exist` → `404`.
    async fn update_ehr_subject_status(
        &self,
        _ehr_id: String,
        _subject: SubjectRef,
        _status: ResourceStatus,
    ) -> Result<(), SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `update_ehr_subject_loc_desc` — update the `LOCATION_DESC` of the
    /// (`ehr_id`, `subject`) association (`None` clears it). Errors:
    /// `subject_id_does_not_exist` / `ehr_id_does_not_exist` → `404`.
    async fn update_ehr_subject_loc_desc(
        &self,
        _ehr_id: String,
        _subject: SubjectRef,
        _loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `remove_ehr_subject` — remove the `subject` association with `ehr_id`
    /// (the subject may remain associated with other EHRs). Errors:
    /// `subject_id_does_not_exist` / `ehr_id_does_not_exist` → `404`.
    async fn remove_ehr_subject(
        &self,
        _ehr_id: String,
        _subject: SubjectRef,
    ) -> Result<(), SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `remove_subject` — remove all entries for `subject`. Errors:
    /// `subject_id_does_not_exist` → `404`.
    async fn remove_subject(&self, _subject: SubjectRef) -> Result<(), SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// The subjects associated with `ehr_id` (design-filled read; PORT NOTE on
    /// the trait). Empty for an unknown EHR.
    async fn ehr_subjects(&self, _ehr_id: String) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(Vec::new())
    }

    /// The EHRs associated with `subject` (design-filled read; PORT NOTE on the
    /// trait). Empty for an unknown subject.
    async fn subject_ehrs(&self, _subject: SubjectRef) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(Vec::new())
    }
}
