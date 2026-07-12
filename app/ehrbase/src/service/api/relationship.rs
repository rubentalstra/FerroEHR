//! [`PartyRelationshipService`] on [`EhrbaseService`] — the demographic
//! `PARTY_RELATIONSHIP` operations.
//!
//! The thin trait adapter that parses the string arguments the `ehrbase-rest`
//! seam supplies and delegates to the [`crate::service::relationship`] domain
//! logic. Relationship ids parse through the same strict BASE decoder the party
//! group uses ([`version_id`]).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use ehrbase_sm::CallStatusType;
use ehrbase_sm::PartyRelationshipService;
use ehrbase_sm::SmError;
use ehrbase_sm::UpdateVersion;

use crate::service::EhrbaseService;
use crate::service::ServiceError;
use crate::service::version_id;

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

/// The `version_uid` a write produced (the new/deleted `OBJECT_VERSION_ID`),
/// pulled from the response metadata.
fn version_uid(resp: ServiceResponse) -> String {
    resp.meta.map(|m| m.uid).unwrap_or_default()
}

#[async_trait]
impl PartyRelationshipService for EhrbaseService {
    // ── I_PARTY_RELATIONSHIP + create factory (the SM core) ─────────────────
    async fn create_party_relationship(&self, a_version: UpdateVersion) -> Result<Uuid, SmError> {
        let resp = self.create_relationship(a_version.data).await?;
        let (vo_id, _) = version_id::parse_version_uid(&version_uid(resp))?;
        Ok(vo_id)
    }

    async fn has_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<bool, SmError> {
        // True iff a *live* relationship exists (a logically deleted one reads
        // `Null`, satisfying the delete post-condition).
        match self
            .read_relationship(a_versioned_party_rel_id, None, None)
            .await
        {
            Ok(resp) => Ok(!resp.is_empty()),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<Value, SmError> {
        let resp = self
            .read_relationship(a_versioned_party_rel_id, None, None)
            .await?;
        if resp.is_empty() {
            return Err(SmError::new(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("party relationship {a_versioned_party_rel_id} has no current version"),
            ));
        }
        Ok(resp.body)
    }

    async fn get_party_relationship_at_time(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_time: String,
    ) -> Result<Value, SmError> {
        let at = parse_at_time(&a_time)?;
        let resp = self
            .read_relationship(a_versioned_party_rel_id, None, Some(at))
            .await?;
        Ok(resp.body)
    }

    async fn get_party_relationship_at_version(
        &self,
        a_party_rel_version_id: String,
    ) -> Result<Value, SmError> {
        let (vo_id, tree) = version_id::parse_version_uid(&a_party_rel_version_id)?;
        match self.relationship_version(vo_id, tree).await {
            Ok(v) => Ok(v),
            Err(ServiceError::NotFound(m)) => {
                Err(SmError::new(CallStatusType::ObjectVersionDoesNotExist, m))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn update_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_version: UpdateVersion,
    ) -> Result<String, SmError> {
        let expected = match &a_version.preceding_version_uid {
            Some(ovid) => Some(version_id::components(ovid)?.1),
            None => None,
        };
        let resp = self
            .update_relationship(a_versioned_party_rel_id, a_version.data, expected)
            .await?;
        Ok(version_uid(resp))
    }

    async fn delete_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<String, SmError> {
        // The SM `delete_party_relationship` has no version argument; the domain
        // delete needs the preceding trunk version, taken from the current one.
        let meta = self
            .relationship_current_meta(a_versioned_party_rel_id)
            .await?
            .ok_or_else(|| {
                SmError::new(
                    CallStatusType::VersionedObjectDoesNotExist,
                    format!("party relationship {a_versioned_party_rel_id}"),
                )
            })?;
        let (_, tree) = version_id::parse_version_uid(&meta.uid)?;
        let resp = self
            .delete_relationship(a_versioned_party_rel_id, tree)
            .await?;
        Ok(version_uid(resp))
    }

    // ── the relationship wire seam ────────────────────────────────────────────
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
