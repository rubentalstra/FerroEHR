//! The SM EHR Index service (`master07-ehr_index_service.adoc`):
//! `I_EHR_INDEX` plus `RESOURCE_STATUS` / `RESOURCE_INSTANCE_TYPE` /
//! `LOCATION_DESC`.
//!
//! "The primary function of the EHR Index service is to enable the recording
//! of associations of subject identifiers … with EHR identifiers. In a
//! privacy-supporting environment, this enables EHRs to be persisted with
//! only an EHR id" (master07 §Overview). Associations are N:M; the two
//! multiple-association cases are error states the `RESOURCE_STATUS`
//! meta-data exists to "detect and rectify".

pub mod service;
pub mod types;

pub use service::EhrIndexService;
pub use types::{EhrIndexEntry, LocationDesc, ResourceInstanceType, ResourceStatus, SubjectRef};
