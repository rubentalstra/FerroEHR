//! The SM EHR-core service interfaces on [`EhrbaseService`] — the EHR /
//! `EHR_STATUS` / COMPOSITION / DIRECTORY / CONTRIBUTION surface (W2-A), split
//! along the SM interface boundaries ([`EhrService`], [`EhrStatusService`],
//! [`EhrCompositionService`], [`EhrDirectoryService`],
//! [`EhrContributionService`]).
//!
//! These seams supersede the generated `EhrApi`: each method returns a
//! [`ServiceResponse`] (the canonical-JSON RM payload plus the typed
//! [`ResourceMeta`] the HTTP edge turns into `ETag`/`Location`) rather than a
//! bare `Value`. The write/read/versioning machinery lives in the sibling
//! service modules ([`crate::service::ehr`], [`composition`](crate::service),
//! [`directory`](crate::service), [`contribution`](crate::service)); this file is
//! the thin trait adapter that parses params and wraps the result. Every
//! operation is implemented here (the two `*_version_get_at_time` reads landed
//! with F-01-05 / F-02-04).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::{
    EhrCompositionService, EhrContributionService, EhrDirectoryService, EhrService,
    EhrStatusService, EhrSummary, Page, ResourceMeta, ServiceResponse,
};
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
use crate::service::version_id;

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

    // ── SM native call: get_ehr → EHR_SUMMARY (no ITS-REST route) ─────────────
    async fn get_ehr_summary(&self, ehr_id: String) -> Result<EhrSummary, ApiError> {
        Ok(self.summarize_ehr(parse_ehr_id(&ehr_id)?).await?)
    }
}

#[async_trait]
impl EhrStatusService for EhrbaseService {
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
        let (vo_id, version) = version_id::parse_version_uid(&params.version_uid)?;
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
        let (vo_id, version) = version_id::parse_version_uid(&params.version_uid)?;
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

    // ── item tags ────────────────────────────────────────────────────────────
    async fn ehr_status_tags_get(
        &self,
        params: EhrStatusTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
        Ok(tags_response(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn ehr_status_tags_update(
        &self,
        params: EhrStatusTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
        Ok(tags_response(
            self.replace_tags(ehr_id, vo_id, "EHR_STATUS", body).await?,
        ))
    }

    async fn ehr_status_tags_delete(
        &self,
        params: EhrStatusTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
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
}

#[async_trait]
impl EhrCompositionService for EhrbaseService {
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
        let (vo_id, version) = version_id::parse_uid_based_id(&params.uid_based_id)?;
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
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
        let expected = version_id::expected_from_if_match(&params.if_match);
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
        let (vo_id, expected) = version_id::parse_version_uid(&params.uid_based_id)?;
        Ok(self.delete_composition(ehr_id, vo_id, expected).await?)
    }

    async fn versioned_composition_get(
        &self,
        params: VersionedCompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.versioned_object_uid)?;
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
        let (vo_id, _) = version_id::parse_uid_based_id(&params.versioned_object_uid)?;
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
        let (vo_id, version) = version_id::parse_version_uid(&params.version_uid)?;
        Ok(ServiceResponse::plain(
            self.composition_version(ehr_id, vo_id, version).await?,
        ))
    }

    async fn versioned_composition_revision_history(
        &self,
        params: VersionedCompositionRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.revision_history(ehr_id, vo_id).await?,
        ))
    }

    // ── item tags ────────────────────────────────────────────────────────────
    async fn composition_tags_get(
        &self,
        params: CompositionTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
        Ok(tags_response(self.target_tags(ehr_id, vo_id).await?))
    }

    async fn composition_tags_update(
        &self,
        params: CompositionTagsUpdateParams,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
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
        let (vo_id, _) = version_id::parse_uid_based_id(&params.uid_based_id)?;
        self.delete_tag(ehr_id, vo_id, &params.key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    // ── conflict-decoration helpers (latest version for 409/412 headers) ──────
    async fn composition_latest_meta(
        &self,
        ehr_id: String,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self.composition_current_meta(ehr_id, vo_id).await?)
    }
}

#[async_trait]
impl EhrDirectoryService for EhrbaseService {
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
        let expected = version_id::expected_from_if_match(&params.if_match);
        Ok(self.update_directory(ehr_id, body, expected).await?)
    }

    async fn directory_delete(
        &self,
        params: DirectoryDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let expected = version_id::expected_from_if_match(&params.if_match);
        Ok(self.delete_directory(ehr_id, expected).await?)
    }

    async fn directory_get_by_version_id(
        &self,
        params: DirectoryGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        let ehr_id = parse_ehr_id(&params.ehr_id)?;
        let (vo_id, version) = version_id::parse_version_uid(&params.version_uid)?;
        Ok(self.directory_version(ehr_id, vo_id, version).await?)
    }

    // ── conflict-decoration helpers (latest version for 409/412 headers) ──────
    async fn directory_latest_meta(
        &self,
        ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(self.directory_meta(parse_ehr_id(&ehr_id)?).await?)
    }
}

#[async_trait]
impl EhrContributionService for EhrbaseService {
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

    // ── SM native calls: list_contributions / contribution_count ──────────────
    // (no ITS-REST route — the wire spec defines none).
    async fn contribution_list(
        &self,
        ehr_id: String,
        time_range: Option<(Option<String>, Option<String>)>,
        page: Page,
    ) -> Result<Vec<String>, ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        let time_range = parse_time_range(time_range)?;
        let ids = self.list_contributions(ehr_id, time_range, page).await?;
        Ok(ids.iter().map(Uuid::to_string).collect())
    }

    async fn contribution_count(
        &self,
        ehr_id: String,
        time_range: Option<(Option<String>, Option<String>)>,
    ) -> Result<i64, ApiError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        let time_range = parse_time_range(time_range)?;
        Ok(self.count_contributions(ehr_id, time_range).await?)
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

/// Parse the optional `(lower, upper)` ISO 8601 bounds of an SM contribution
/// `time_range` into `jiff::Timestamp`s; a malformed bound → `400 BadRequest`.
#[allow(clippy::type_complexity)]
fn parse_time_range(
    raw: Option<(Option<String>, Option<String>)>,
) -> Result<Option<(Option<jiff::Timestamp>, Option<jiff::Timestamp>)>, ApiError> {
    let parse = |b: Option<String>| -> Result<Option<jiff::Timestamp>, ApiError> {
        b.map(|s| {
            s.parse::<jiff::Timestamp>()
                .map_err(|_| ApiError::BadRequest(format!("invalid time_range bound: {s}")))
        })
        .transpose()
    };
    raw.map(|(lo, hi)| Ok((parse(lo)?, parse(hi)?))).transpose()
}
