//! [`DemographicService`] on [`EhrbaseService`] — the DEMOGRAPHIC API group.
//!
//! The thin trait adapter that parses the (kind + string) arguments the
//! `ehrbase-rest` [`DemographicService`] seam supplies and delegates to the
//! [`crate::service::demographic`] domain logic. Party ids parse through the
//! same strict BASE decoder the EHR group uses ([`version_id`]).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use ehrbase_sm::SmError;
use ehrbase_sm::services::DemographicService;
use ehrbase_sm::PartyKind;

use crate::service::EhrbaseService;
use crate::service::version_id;

/// Wrap a JSON array of item-tag objects as a plain (header-free) response.
fn tags_response(tags: Vec<Value>) -> ServiceResponse {
    ServiceResponse::plain(Value::Array(tags))
}

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

#[async_trait]
impl DemographicService for EhrbaseService {
    // ── PARTY CRUD ───────────────────────────────────────────────────────────
    async fn party_create(&self, kind: PartyKind, body: Value) -> Result<ServiceResponse, SmError> {
        Ok(self.create_party(kind, body).await?)
    }

    async fn party_get(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
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
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        let expected = version_id::expected_from_if_match(&if_match);
        Ok(self.update_party(kind, vo_id, body, expected).await?)
    }

    async fn party_delete(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        // `delete_party(a_versioned_party_id: UUID)` (our own demographic design,
        // `docs/design/sm-platform/03-demographic-ehr-index-query.md`): the path
        // carries the versioned-party id (bare `HIER_OBJECT_ID` or full
        // `OBJECT_VERSION_ID`). The preceding trunk version for optimistic
        // concurrency comes from `If-Match` when supplied, else the path OVID,
        // else `None` (delete the current version unconditionally).
        let (vo_id, path_version) = version_id::parse_uid_based_id(&uid_based_id)?;
        let expected = if_match
            .as_deref()
            .and_then(version_id::expected_from_if_match)
            .or(path_version);
        Ok(self.delete_party(kind, vo_id, expected).await?)
    }

    // ── VERSIONED_PARTY ──────────────────────────────────────────────────────
    async fn versioned_party_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(self.versioned_party(vo_id).await?))
    }

    async fn versioned_party_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.party_revision_history(vo_id).await?,
        ))
    }

    async fn versioned_party_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&versioned_object_uid)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.party_version_at_time(vo_id, at).await?)
    }

    async fn versioned_party_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
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
    ) -> Result<ServiceResponse, SmError> {
        Ok(self.create_demographic_contribution(body).await?)
    }

    async fn demographic_contribution_get(
        &self,
        contribution_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let id = Uuid::parse_str(&contribution_uid).map_err(|_| {
            SmError::precondition(format!("invalid contribution id: {contribution_uid}"))
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
    ) -> Result<ServiceResponse, SmError> {
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
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(tags_response(self.party_tags(vo_id).await?))
    }

    async fn party_tags_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, SmError> {
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
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        self.delete_party_tag(vo_id, &key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    async fn demographic_latest_meta(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self.party_current_meta(kind, vo_id).await?)
    }
}
