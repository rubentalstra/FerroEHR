//! `I_EHR_INDEX` (`i_ehr_index.adoc`) — "Interface object for the `EHR_INDEX`
//! service."

use async_trait::async_trait;

use crate::common::SmError;

use super::types::{EhrIndexEntry, LocationDesc, ResourceStatus, SubjectRef};

/// `I_EHR_INDEX` — subject↔EHR association management, one Rust method per SM
/// call.
///
/// Index entries are **not** versioned objects — the SM defines no versioning
/// here — so writes emit no CONTRIBUTION (PORT NOTE at the impl). Errors
/// follow `i_ehr_index.adoc` exactly: an unknown EHR →
/// `ehr_id_does_not_exist`, an unknown subject/association →
/// `subject_id_does_not_exist` (the dedicated
/// [`CallStatusType`](crate::CallStatusType) variants, never the generic
/// `versioned_object_does_not_exist`).
///
/// No default method bodies except the two design-filled reads (the SM
/// defines no read operations — PORT NOTE): [`Self::ehr_subjects`] /
/// [`Self::subject_ehrs`] fill that silence and default to empty.
#[async_trait]
pub trait EhrIndexService: Send + Sync {
    /// `add_ehr_subject (an_ehr_id: UUID, a_subject_id: OBJECT_REF, a_status:
    /// RESOURCE_STATUS [0..1], a_loc_desc: LOCATION_DESC [0..1])` — "Add a
    /// subject identifier for the EHR with `an_ehr_id`, with an optional
    /// resource status and location descriptor" (status defaults to a
    /// `Primary` instance).
    async fn add_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: Option<ResourceStatus>,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError>;

    /// `update_ehr_subject_status (an_ehr_id, a_subject_id, a_status:
    /// RESOURCE_STATUS)` — "Update subject resource status for the
    /// association." Errors `subject_id_does_not_exist`,
    /// `ehr_id_does_not_exist`.
    async fn update_ehr_subject_status(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: ResourceStatus,
    ) -> Result<(), SmError>;

    /// `update_ehr_subject_loc_desc (an_ehr_id, a_subject_id, a_loc_desc:
    /// LOCATION_DESC [0..1])` — "Update location descriptor for the
    /// association" (`None` clears it). Errors `subject_id_does_not_exist`,
    /// `ehr_id_does_not_exist`.
    async fn update_ehr_subject_loc_desc(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError>;

    /// `remove_ehr_subject (an_ehr_id, a_subject_id)` — "Remove the subject
    /// identifier association with the EHR with `an_ehr_id` (it might remain
    /// associated with other EHRs however)." Errors
    /// `subject_id_does_not_exist`, `ehr_id_does_not_exist`.
    async fn remove_ehr_subject(&self, ehr_id: String, subject: SubjectRef) -> Result<(), SmError>;

    /// `remove_subject (a_subject_id)` — "Remove all entries for a subject."
    /// Error `subject_id_does_not_exist`.
    async fn remove_subject(&self, subject: SubjectRef) -> Result<(), SmError>;

    /// The subjects associated with `ehr_id` (design-filled read; the SM
    /// defines no reads — PORT NOTE on the trait). Empty for an unknown EHR.
    async fn ehr_subjects(&self, _ehr_id: String) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(Vec::new())
    }

    /// The EHRs associated with `subject` (design-filled read). Empty for an
    /// unknown subject.
    async fn subject_ehrs(&self, _subject: SubjectRef) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(Vec::new())
    }
}
