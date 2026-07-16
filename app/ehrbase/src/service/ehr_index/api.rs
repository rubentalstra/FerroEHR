//! [`EhrIndexService`] on [`EhrbaseService`] — the SM `I_EHR_INDEX` seam
//! (`i_ehr_index.adoc`).
//!
//! The thin trait adapter that parses the `ehr_id` UUID and delegates to the
//! [`super::index`] domain logic. No wire is mounted (EHR Index has no ITS-REST
//! contract — G-11: native-API-only, our own extension surface); the service
//! seam exists for the SM native API and future extension routes.
//!
//! G-8/G-9: every domain failure crosses this seam as [`super::IndexError`],
//! whose `From<IndexError> for SmError` maps `ehr_id_does_not_exist` /
//! `subject_id_does_not_exist` onto their dedicated
//! [`CallStatusType`](crate::service::status::CallStatusType) variants — never the generic
//! `versioned_object_does_not_exist` (`i_ehr_index.adoc §Errors`).

use async_trait::async_trait;
use uuid::Uuid;

use crate::service::ehr_index::types::{EhrIndexEntry, LocationDesc, ResourceStatus, SubjectRef};
use crate::service::status::SmError;

use crate::service::EhrbaseService;

/// Parse an `ehr_id` UUID. An unparseable id is a `400` precondition failure;
/// a well-formed-but-unknown id surfaces as `ehr_id_does_not_exist` at the DB
/// check (`i_ehr_index.adoc §Errors`).
fn parse_ehr_id(raw: &str) -> Result<Uuid, SmError> {
    Uuid::parse_str(raw).map_err(|_| SmError::precondition(format!("invalid ehr id: {raw}")))
}

impl EhrbaseService {
    pub async fn add_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: Option<ResourceStatus>,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self
            .index_add_subject(ehr_id, &subject, status.as_ref(), loc.as_ref())
            .await?)
    }

    pub async fn update_ehr_subject_status(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: ResourceStatus,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_update_status(ehr_id, &subject, &status).await?)
    }

    pub async fn update_ehr_subject_loc_desc(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self
            .index_update_loc_desc(ehr_id, &subject, loc.as_ref())
            .await?)
    }

    pub async fn remove_ehr_subject(&self, ehr_id: String, subject: SubjectRef) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_remove_ehr_subject(ehr_id, &subject).await?)
    }

    pub async fn remove_subject(&self, subject: SubjectRef) -> Result<(), SmError> {
        Ok(self.index_remove_subject(&subject).await?)
    }

    pub async fn ehr_subjects(&self, ehr_id: String) -> Result<Vec<EhrIndexEntry>, SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        Ok(self.index_ehr_subjects(ehr_id).await?)
    }

    pub async fn subject_ehrs(&self, subject: SubjectRef) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(self.index_subject_ehrs(&subject).await?)
    }
}
