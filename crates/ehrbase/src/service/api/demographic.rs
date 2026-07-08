//! [`DemographicService`] on [`EhrbaseService`] — the DEMOGRAPHIC API group.
//!
//! The thin trait adapter that parses the (kind + string) arguments the
//! `ehrbase-rest` [`DemographicService`] seam supplies and delegates to the
//! [`crate::service::demographic`] domain logic. Party ids parse through the
//! same strict BASE decoder the EHR group uses ([`version_id`]).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::backend::{DemographicService, PartyKind};
use ehrbase_rest::{ResourceMeta, ServiceResponse};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;
use crate::service::version_id;

/// Wrap a JSON array of item-tag objects as a plain (header-free) response.
fn tags_response(tags: Vec<Value>) -> ServiceResponse {
    ServiceResponse::plain(Value::Array(tags))
}

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, ApiError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| ApiError::BadRequest(format!("invalid version_at_time: {raw}")))
}

#[async_trait]
impl DemographicService for EhrbaseService {
    // ── PARTY CRUD ───────────────────────────────────────────────────────────
    async fn party_create(
        &self,
        kind: PartyKind,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(self.create_party(kind, body).await?)
    }

    async fn party_get(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, version) = version_id::parse_uid_based_id(&uid_based_id)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.read_party(kind, vo_id, version, at).await?)
    }

    async fn party_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        let expected = version_id::expected_from_if_match(&if_match);
        Ok(self.update_party(kind, vo_id, body, expected).await?)
    }

    async fn party_delete(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<ServiceResponse, ApiError> {
        // The uid_based_id MUST be an OBJECT_VERSION_ID (the preceding version);
        // a bare HIER_OBJECT_ID → 400 (mirroring composition_delete).
        let (vo_id, expected) = version_id::parse_version_uid(&uid_based_id)?;
        Ok(self.delete_party(kind, vo_id, expected).await?)
    }

    // ── VERSIONED_PARTY ──────────────────────────────────────────────────────
    async fn versioned_party_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(self.versioned_party(vo_id).await?))
    }

    async fn versioned_party_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.party_revision_history(vo_id).await?,
        ))
    }

    async fn versioned_party_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.party_version_at_time(vo_id, at).await?)
    }

    async fn versioned_party_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        let (_, version) = version_id::parse_version_uid(&version_uid)?;
        Ok(ServiceResponse::plain(
            self.party_version(vo_id, version).await?,
        ))
    }

    // ── demographic CONTRIBUTION ─────────────────────────────────────────────
    async fn demographic_contribution_create(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(self.create_demographic_contribution(body).await?)
    }

    async fn demographic_contribution_get(
        &self,
        contribution_uid: String,
    ) -> Result<ServiceResponse, ApiError> {
        let id = Uuid::parse_str(&contribution_uid).map_err(|_| {
            ApiError::BadRequest(format!("invalid contribution id: {contribution_uid}"))
        })?;
        Ok(ServiceResponse::plain(
            self.demographic_contribution(id).await?,
        ))
    }

    // ── demographic item tags ────────────────────────────────────────────────
    async fn demographic_tags_get(
        &self,
        tag_key: Option<String>,
        tag_value: Option<String>,
        tag_target_path: Option<String>,
    ) -> Result<ServiceResponse, ApiError> {
        let tags = self
            .demographic_tags(
                tag_key.as_deref(),
                tag_value.as_deref(),
                tag_target_path.as_deref(),
            )
            .await?;
        Ok(tags_response(tags))
    }

    async fn party_tags_get(
        &self,
        _kind: PartyKind,
        uid_based_id: String,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(tags_response(self.party_tags(vo_id).await?))
    }

    async fn party_tags_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(tags_response(
            self.replace_party_tags(kind, vo_id, body).await?,
        ))
    }

    async fn party_tags_delete(
        &self,
        _kind: PartyKind,
        uid_based_id: String,
        key: String,
    ) -> Result<ServiceResponse, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        self.delete_party_tag(vo_id, &key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    async fn demographic_latest_meta(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self.party_current_meta(kind, vo_id).await?)
    }
}
