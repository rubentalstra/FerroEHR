//! `EhrApi` on [`EhrbaseService`] — EHR, `EHR_STATUS`, COMPOSITION, DIRECTORY and
//! CONTRIBUTION operations. Methods not yet wired (revision history, time-travel
//! reads, item tags, `ehr_get_by_subject`, `contribution_create`) inherit the
//! generated `NotImplemented` default; the write/read/versioning machinery they
//! build on is complete.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrApi, EhrCreateParams,
    EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetByIdParams, VersionedEhrStatusGetParams,
    VersionedEhrStatusRevisionHistoryParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;
use crate::service::ehr::default_ehr_status;

/// The item-tag list shape the contract returns.
type Tags = Vec<std::collections::BTreeMap<String, Value>>;

#[async_trait]
impl EhrApi for EhrbaseService {
    // ── EHR ────────────────────────────────────────────────────────────────
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        body: Option<Value>,
    ) -> Result<Value, ApiError> {
        let status = body.unwrap_or_else(default_ehr_status);
        Ok(self.create_ehr(Uuid::now_v7(), status).await?)
    }

    async fn ehr_create_with_id(
        &self,
        params: EhrCreateWithIdParams,
        body: Option<Value>,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let status = body.unwrap_or_else(default_ehr_status);
        Ok(self.create_ehr(ehr_id, status).await?)
    }

    async fn ehr_get_by_id(&self, params: EhrGetByIdParams) -> Result<Value, ApiError> {
        Ok(self.ehr_summary(parse_ehr_id(&params.ehr_id)?).await?)
    }

    async fn ehr_get_by_subject(&self, params: EhrGetBySubjectParams) -> Result<Value, ApiError> {
        Ok(self
            .ehr_by_subject(&params.subject_id, &params.subject_namespace)
            .await?)
    }

    // ── EHR_STATUS ───────────────────────────────────────────────────────────
    async fn ehr_status_get_at_time(
        &self,
        params: EhrStatusGetAtTimeParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let at = params
            .version_at_time
            .as_deref()
            .map(parse_at_time)
            .transpose()?;
        Ok(self.status_at(ehr_id, at).await?)
    }

    async fn ehr_status_get_by_version_id(
        &self,
        params: EhrStatusGetByVersionIdParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(self.status_version(ehr_id, vo_id, version).await?)
    }

    async fn ehr_status_update(
        &self,
        params: EhrStatusUpdateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.status_update(ehr_id, body, &params.if_match).await?)
    }

    async fn versioned_ehr_status_get(
        &self,
        params: VersionedEhrStatusGetParams,
    ) -> Result<Value, ApiError> {
        Ok(self.versioned_status(parse_ehr_id(&params.ehr_id)?).await?)
    }

    async fn versioned_ehr_status_version_get_by_id(
        &self,
        params: VersionedEhrStatusVersionGetByIdParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(self.status_version(ehr_id, vo_id, version).await?)
    }

    async fn versioned_ehr_status_revision_history(
        &self,
        params: VersionedEhrStatusRevisionHistoryParams,
    ) -> Result<Value, ApiError> {
        Ok(self
            .status_revision_history(parse_ehr_id(&params.ehr_id)?)
            .await?)
    }

    // ── COMPOSITION ──────────────────────────────────────────────────────────
    async fn composition_create(
        &self,
        params: CompositionCreateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_composition(ehr_id, body).await?)
    }

    async fn composition_get(&self, params: CompositionGetParams) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_object_id(&params.uid_based_id)?;
        match (version, params.version_at_time.as_deref()) {
            (Some(v), _) => Ok(self.read_composition(ehr_id, vo_id, Some(v)).await?),
            (None, Some(at)) => Ok(self
                .composition_at_time(ehr_id, vo_id, parse_at_time(at)?)
                .await?),
            (None, None) => Ok(self.read_composition(ehr_id, vo_id, None).await?),
        }
    }

    async fn composition_update(
        &self,
        params: CompositionUpdateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self
            .update_composition(ehr_id, vo_id, body, expected)
            .await?)
    }

    async fn composition_delete(&self, params: CompositionDeleteParams) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        // composition_delete.yaml: the uid_based_id MUST be an OBJECT_VERSION_ID
        // (the preceding_version_uid to delete); a bare HIER_OBJECT_ID → 400.
        let (vo_id, expected) = parse_version_uid(&params.uid_based_id)?;
        Ok(self.delete_composition(ehr_id, vo_id, expected).await?)
    }

    async fn versioned_composition_get(
        &self,
        params: VersionedCompositionGetParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.versioned_object_uid)?;
        Ok(self.versioned_composition(ehr_id, vo_id).await?)
    }

    async fn versioned_composition_version_get_by_id(
        &self,
        params: VersionedCompositionVersionGetByIdParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(self.composition_version(ehr_id, vo_id, version).await?)
    }

    async fn versioned_composition_revision_history(
        &self,
        params: VersionedCompositionRevisionHistoryParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.versioned_object_uid)?;
        Ok(self.revision_history(ehr_id, vo_id).await?)
    }

    // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────────
    async fn directory_create(
        &self,
        params: DirectoryCreateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_directory(ehr_id, body).await?)
    }

    async fn directory_get_at_time(
        &self,
        params: DirectoryGetAtTimeParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let at = params
            .version_at_time
            .as_deref()
            .map(parse_at_time)
            .transpose()?;
        Ok(self
            .directory_at_time(ehr_id, at, params.path.as_deref())
            .await?)
    }

    async fn directory_update(
        &self,
        params: DirectoryUpdateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self.update_directory(ehr_id, body, expected).await?)
    }

    async fn directory_delete(&self, params: DirectoryDeleteParams) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self.delete_directory(ehr_id, expected).await?)
    }

    async fn directory_get_by_version_id(
        &self,
        params: DirectoryGetByVersionIdParams,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(self.directory_version(ehr_id, vo_id, version).await?)
    }

    // ── CONTRIBUTION ─────────────────────────────────────────────────────────
    async fn contribution_create(
        &self,
        params: ContributionCreateParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_contribution(ehr_id, body).await?)
    }

    async fn contribution_get(&self, params: ContributionGetParams) -> Result<Value, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let contribution_id = Uuid::parse_str(&params.contribution_uid).map_err(|_| {
            ApiError::BadRequest(format!(
                "invalid contribution id: {}",
                params.contribution_uid
            ))
        })?;
        Ok(self.get_contribution(ehr_id, contribution_id).await?)
    }

    // ── item tags ────────────────────────────────────────────────────────────
    async fn ehr_tags_get(&self, params: EhrTagsGetParams) -> Result<Tags, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let tags = self
            .ehr_tags(
                ehr_id,
                params.tag_key.as_deref(),
                params.tag_value.as_deref(),
                params.tag_target_path.as_deref(),
            )
            .await?;
        Ok(tag_maps(tags))
    }

    async fn composition_tags_get(
        &self,
        params: CompositionTagsGetParams,
    ) -> Result<Tags, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tag_maps(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn composition_tags_update(
        &self,
        params: CompositionTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<Tags, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tag_maps(
            self.upsert_tags(ehr_id, vo_id, "COMPOSITION", body).await?,
        ))
    }

    async fn composition_tags_delete(
        &self,
        params: CompositionTagsDeleteParams,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(self.delete_tag(ehr_id, vo_id, &params.key).await?)
    }

    async fn ehr_status_tags_get(&self, params: EhrStatusTagsGetParams) -> Result<Tags, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tag_maps(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn ehr_status_tags_update(
        &self,
        params: EhrStatusTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<Tags, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tag_maps(
            self.upsert_tags(ehr_id, vo_id, "EHR_STATUS", body).await?,
        ))
    }

    async fn ehr_status_tags_delete(
        &self,
        params: EhrStatusTagsDeleteParams,
    ) -> Result<(), ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(self.delete_tag(ehr_id, vo_id, &params.key).await?)
    }
}

/// Convert JSON tag objects into the `Vec<BTreeMap>` the contract returns.
fn tag_maps(tags: Vec<Value>) -> Tags {
    tags.into_iter()
        .map(|tag| match tag {
            Value::Object(map) => map.into_iter().collect(),
            _ => std::collections::BTreeMap::new(),
        })
        .collect()
}

fn parse_ehr_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("invalid EHR id: {raw}")))
}

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, ApiError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| ApiError::BadRequest(format!("invalid version_at_time: {raw}")))
}

/// Parse a `uid_based_id`/`versioned_object_uid`: a bare `HIER_OBJECT_ID`
/// (`{uuid}`) or an `OBJECT_VERSION_ID` (`{uuid}::{system}::{version}`) → the
/// object id plus an optional version.
fn parse_object_id(raw: &str) -> Result<(Uuid, Option<i32>), ApiError> {
    let head = raw.split("::").next().unwrap_or(raw);
    let vo_id = Uuid::parse_str(head)
        .map_err(|_| ApiError::BadRequest(format!("invalid object id: {raw}")))?;
    let version = if raw.contains("::") {
        raw.rsplit("::").next().and_then(|v| v.parse::<i32>().ok())
    } else {
        None
    };
    Ok((vo_id, version))
}

/// Parse a `version_uid` (`OBJECT_VERSION_ID`), which must carry a version.
fn parse_version_uid(raw: &str) -> Result<(Uuid, i32), ApiError> {
    match parse_object_id(raw)? {
        (vo_id, Some(version)) => Ok((vo_id, version)),
        (_, None) => Err(ApiError::BadRequest(format!(
            "expected an OBJECT_VERSION_ID (uuid::system::version), got {raw}"
        ))),
    }
}

/// The expected version from an `If-Match` header (the version tail of an
/// `OBJECT_VERSION_ID`, or a bare integer); `None` if unparseable.
fn expected_from_if_match(if_match: &str) -> Option<i32> {
    let token = if_match.trim().trim_matches('"');
    token
        .rsplit("::")
        .next()
        .and_then(|v| v.parse::<i32>().ok())
}
