//! [`PartyRelationshipService`] on [`EhrbaseService`] — the demographic
//! `PARTY_RELATIONSHIP` operations.
//!
//! The thin trait adapter that parses the string arguments the `ehrbase-rest`
//! seam supplies and delegates to the [`crate::service::relationship`] domain
//! logic. Relationship ids parse through the same strict BASE decoder the party
//! group uses ([`version_id`]).

use async_trait::async_trait;
use serde_json::Value;

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use ehrbase_sm::SmError;
use ehrbase_sm::services::PartyRelationshipService;

use crate::service::EhrbaseService;
use crate::service::version_id;

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

#[async_trait]
impl PartyRelationshipService for EhrbaseService {
    async fn party_relationship_create(&self, body: Value) -> Result<ServiceResponse, SmError> {
        Ok(self.create_relationship(body).await?)
    }

    async fn party_relationship_get(
        &self,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, version) = version_id::parse_uid_based_id(&uid_based_id)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.read_relationship(vo_id, version, at).await?)
    }

    async fn party_relationship_update(
        &self,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        let expected = version_id::expected_from_if_match(&if_match);
        Ok(self.update_relationship(vo_id, body, expected).await?)
    }

    async fn party_relationship_delete(
        &self,
        uid_based_id: String,
    ) -> Result<ServiceResponse, SmError> {
        // The uid_based_id MUST be an OBJECT_VERSION_ID (the preceding version);
        // a bare HIER_OBJECT_ID → 400 (mirroring the party delete).
        let (vo_id, expected) = version_id::parse_version_uid(&uid_based_id)?;
        Ok(self.delete_relationship(vo_id, expected).await?)
    }

    async fn versioned_party_relationship_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.versioned_relationship(vo_id).await?,
        ))
    }

    async fn party_relationship_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.relationship_revision_history(vo_id).await?,
        ))
    }

    async fn party_relationship_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.relationship_version_at_time(vo_id, at).await?)
    }

    async fn party_relationship_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        let (_, version) = version_id::parse_version_uid(&version_uid)?;
        Ok(ServiceResponse::plain(
            self.relationship_version(vo_id, version).await?,
        ))
    }

    async fn party_relationship_latest_meta(
        &self,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self.relationship_current_meta(vo_id).await?)
    }
}
