//! [`AdminService`] on [`EhrbaseService`] — physical EHR deletion
//! (SM `I_ADMIN_SERVICE.physical_ehr_delete`).
//!
//! Thin trait adapter: parse the id(s) and delegate to the physical-delete
//! machinery in [`crate::service::admin`]. The config gate (whether the admin
//! surface is reachable at all) lives at the REST edge (`dispatch::admin`).

use async_trait::async_trait;
use uuid::Uuid;

use ehrbase_rest::AdminService;
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;

#[async_trait]
impl AdminService for EhrbaseService {
    async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), ApiError> {
        Ok(self.physical_ehr_delete(parse_ehr_id(&ehr_id)?).await?)
    }

    async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, ApiError> {
        // Any malformed id in the list → 400 (the whole bulk request is
        // rejected before any deletion runs).
        let ids = ehr_ids
            .iter()
            .map(|s| parse_ehr_id(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.physical_ehr_delete_all(&ids).await?)
    }
}

fn parse_ehr_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("invalid EHR id: {raw}")))
}
