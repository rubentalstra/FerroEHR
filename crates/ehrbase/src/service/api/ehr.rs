//! [`EhrService`] on [`EhrbaseService`] — the EHR / `EHR_STATUS` / COMPOSITION /
//! DIRECTORY / CONTRIBUTION surface (W2-A).
//!
//! `ehrbase-rest`'s [`EhrService`] seam supersedes the generated `EhrApi`: each
//! method returns a [`ServiceResponse`] (the canonical-JSON RM payload plus the
//! typed [`ResourceMeta`] the HTTP edge turns into `ETag`/`Location`) rather than
//! a bare `Value`. The write/read/versioning machinery lives in the sibling
//! service modules ([`crate::service::ehr`], [`composition`](crate::service),
//! [`directory`](crate::service), [`contribution`](crate::service)); this file is
//! the thin trait adapter that parses params and wraps the result. Every
//! `EhrService` operation is implemented here (the two `*_version_get_at_time`
//! reads landed with F-01-05 / F-02-04).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::{EhrService, ResourceMeta, ServiceResponse};
use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrCreateParams, EhrCreateWithIdParams,
    EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;
use crate::service::ehr::default_ehr_status;

/// Wrap a JSON array of item-tag objects as a plain (header-free) response.
fn tags_response(tags: Vec<Value>) -> ServiceResponse {
    ServiceResponse::plain(Value::Array(tags))
}

#[async_trait]
impl EhrService for EhrbaseService {
    // ── EHR ────────────────────────────────────────────────────────────────
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let status = body.unwrap_or_else(default_ehr_status);
        Ok(self.create_ehr(Uuid::now_v7(), status).await?)
    }

    async fn ehr_create_with_id(
        &self,
        params: EhrCreateWithIdParams,
        body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let status = body.unwrap_or_else(default_ehr_status);
        Ok(self.create_ehr(ehr_id, status).await?)
    }

    async fn ehr_get_by_id(&self, params: EhrGetByIdParams) -> Result<ServiceResponse, ApiError> {
        Ok(self.ehr_summary(parse_ehr_id(&params.ehr_id)?).await?)
    }

    async fn ehr_get_by_subject(
        &self,
        params: EhrGetBySubjectParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(self
            .ehr_by_subject(&params.subject_id, &params.subject_namespace)
            .await?)
    }

    // ── EHR_STATUS ───────────────────────────────────────────────────────────
    async fn ehr_status_get_at_time(
        &self,
        params: EhrStatusGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
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
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        // F-01-03: the bare EHR_STATUS at that version, not an ORIGINAL_VERSION.
        Ok(self.status_by_version(ehr_id, vo_id, version).await?)
    }

    async fn ehr_status_update(
        &self,
        params: EhrStatusUpdateParams,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.status_update(ehr_id, body, &params.if_match).await?)
    }

    async fn versioned_ehr_status_get(
        &self,
        params: VersionedEhrStatusGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::plain(
            self.versioned_status(parse_ehr_id(&params.ehr_id)?).await?,
        ))
    }

    async fn versioned_ehr_status_version_get_at_time(
        &self,
        params: VersionedEhrStatusVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        // F-01-05: the VERSION extant at `version_at_time` (or the latest) —
        // an ORIGINAL_VERSION with `200_VERSION_at_time` ETag/Location meta.
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let at = params
            .version_at_time
            .as_deref()
            .map(parse_at_time)
            .transpose()?;
        Ok(self.status_version_at_time(ehr_id, at).await?)
    }

    async fn versioned_ehr_status_version_get_by_id(
        &self,
        params: VersionedEhrStatusVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(ServiceResponse::plain(
            self.status_version(ehr_id, vo_id, version).await?,
        ))
    }

    async fn versioned_ehr_status_revision_history(
        &self,
        params: VersionedEhrStatusRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        Ok(ServiceResponse::plain(
            self.status_revision_history(parse_ehr_id(&params.ehr_id)?)
                .await?,
        ))
    }

    // ── COMPOSITION ──────────────────────────────────────────────────────────
    async fn composition_create(
        &self,
        params: CompositionCreateParams,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_composition(ehr_id, body).await?)
    }

    async fn composition_get(
        &self,
        params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
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
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self
            .update_composition(ehr_id, vo_id, body, expected)
            .await?)
    }

    async fn composition_delete(
        &self,
        params: CompositionDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        // composition_delete.yaml: the uid_based_id MUST be an OBJECT_VERSION_ID
        // (the preceding_version_uid to delete); a bare HIER_OBJECT_ID → 400.
        let (vo_id, expected) = parse_version_uid(&params.uid_based_id)?;
        Ok(self.delete_composition(ehr_id, vo_id, expected).await?)
    }

    async fn versioned_composition_get(
        &self,
        params: VersionedCompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.versioned_composition(ehr_id, vo_id).await?,
        ))
    }

    async fn versioned_composition_version_get_at_time(
        &self,
        params: VersionedCompositionVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        // F-02-04: the VERSION extant at `version_at_time` (or the latest) —
        // an ORIGINAL_VERSION with `200_VERSION_of_COMPOSITION_at_time`
        // ETag/Location meta.
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.versioned_object_uid)?;
        let at = params
            .version_at_time
            .as_deref()
            .map(parse_at_time)
            .transpose()?;
        Ok(self.composition_version_at_time(ehr_id, vo_id, at).await?)
    }

    async fn versioned_composition_version_get_by_id(
        &self,
        params: VersionedCompositionVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(ServiceResponse::plain(
            self.composition_version(ehr_id, vo_id, version).await?,
        ))
    }

    async fn versioned_composition_revision_history(
        &self,
        params: VersionedCompositionRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.revision_history(ehr_id, vo_id).await?,
        ))
    }

    // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────────
    async fn directory_create(
        &self,
        params: DirectoryCreateParams,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_directory(ehr_id, body).await?)
    }

    async fn directory_get_at_time(
        &self,
        params: DirectoryGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
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
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self.update_directory(ehr_id, body, expected).await?)
    }

    async fn directory_delete(
        &self,
        params: DirectoryDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let expected = expected_from_if_match(&params.if_match);
        Ok(self.delete_directory(ehr_id, expected).await?)
    }

    async fn directory_get_by_version_id(
        &self,
        params: DirectoryGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = parse_version_uid(&params.version_uid)?;
        Ok(self.directory_version(ehr_id, vo_id, version).await?)
    }

    // ── CONTRIBUTION ─────────────────────────────────────────────────────────
    async fn contribution_create(
        &self,
        params: ContributionCreateParams,
        body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        Ok(self.create_contribution(ehr_id, body).await?)
    }

    async fn contribution_get(
        &self,
        params: ContributionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let contribution_id = Uuid::parse_str(&params.contribution_uid).map_err(|_| {
            ApiError::BadRequest(format!(
                "invalid contribution id: {}",
                params.contribution_uid
            ))
        })?;
        Ok(ServiceResponse::plain(
            self.get_contribution(ehr_id, contribution_id).await?,
        ))
    }

    // ── item tags ────────────────────────────────────────────────────────────
    async fn ehr_tags_get(&self, params: EhrTagsGetParams) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let tags = self
            .ehr_tags(
                ehr_id,
                params.tag_key.as_deref(),
                params.tag_value.as_deref(),
                params.tag_target_path.as_deref(),
            )
            .await?;
        Ok(tags_response(tags))
    }

    async fn composition_tags_get(
        &self,
        params: CompositionTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tags_response(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn composition_tags_update(
        &self,
        params: CompositionTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tags_response(
            self.replace_tags(ehr_id, vo_id, "COMPOSITION", body)
                .await?,
        ))
    }

    async fn composition_tags_delete(
        &self,
        params: CompositionTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        self.delete_tag(ehr_id, vo_id, &params.key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    async fn ehr_status_tags_get(
        &self,
        params: EhrStatusTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tags_response(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn ehr_status_tags_update(
        &self,
        params: EhrStatusTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        Ok(tags_response(
            self.replace_tags(ehr_id, vo_id, "EHR_STATUS", body).await?,
        ))
    }

    async fn ehr_status_tags_delete(
        &self,
        params: EhrStatusTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = parse_object_id(&params.uid_based_id)?;
        self.delete_tag(ehr_id, vo_id, &params.key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    // ── conflict-decoration helpers (latest version for 409/412 headers) ──────
    async fn ehr_status_latest_meta(
        &self,
        ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(self.ehr_status_meta(parse_ehr_id(&ehr_id)?).await?)
    }

    async fn composition_latest_meta(
        &self,
        ehr_id: String,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        let (vo_id, _) = parse_object_id(&uid_based_id)?;
        Ok(self.composition_current_meta(ehr_id, vo_id).await?)
    }

    async fn directory_latest_meta(
        &self,
        ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(self.directory_meta(parse_ehr_id(&ehr_id)?).await?)
    }
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
