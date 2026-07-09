//! [`EhrIndexService`] on [`EhrbaseService`] — the SM `I_EHR_INDEX` seam.
//!
//! The thin trait adapter that parses the `ehr_id` UUID and delegates to the
//! [`crate::service::ehr_index`] domain logic. No wire is mounted this phase
//! (EHR Index has no ITS-REST contract — design 08 §7); the service seam exists
//! for the SM native API and future extension routes.

use async_trait::async_trait;
use uuid::Uuid;

use ehrbase_rest::backend::{
    EhrIndexEntry, EhrIndexService, LocationDesc, ResourceStatus, SubjectRef,
};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;

/// Parse an `ehr_id` UUID (`ehr_id_does_not_exist` semantics: an unparseable id
/// is treated as `400`; a well-formed-but-unknown id is `404` at the DB check).
fn parse_ehr_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("invalid ehr id: {raw}")))
}

#[async_trait]
impl EhrIndexService for EhrbaseService {
    async fn add_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: Option<ResourceStatus>,
        loc: Option<LocationDesc>,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self
            .index_add_subject(ehr_id, &subject, status.as_ref(), loc.as_ref())
            .await?)
    }

    async fn update_ehr_subject_status(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: ResourceStatus,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_update_status(ehr_id, &subject, &status).await?)
    }

    async fn update_ehr_subject_loc_desc(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        loc: Option<LocationDesc>,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self
            .index_update_loc_desc(ehr_id, &subject, loc.as_ref())
            .await?)
    }

    async fn remove_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_remove_ehr_subject(ehr_id, &subject).await?)
    }

    async fn remove_subject(&self, subject: SubjectRef) -> Result<(), ApiError> {
        Ok(self.index_remove_subject(&subject).await?)
    }

    async fn ehr_subjects(&self, ehr_id: String) -> Result<Vec<EhrIndexEntry>, ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_ehr_subjects(ehr_id).await?)
    }

    async fn subject_ehrs(&self, subject: SubjectRef) -> Result<Vec<EhrIndexEntry>, ApiError> {
        Ok(self.index_subject_ehrs(&subject).await?)
    }
}
