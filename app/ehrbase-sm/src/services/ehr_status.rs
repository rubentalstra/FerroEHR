//! The SM `I_EHR_STATUS` interface — `EHR_STATUS` operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    EhrStatusGetAtTimeParams, EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams,
    EhrStatusTagsGetParams, EhrStatusTagsUpdateParams, EhrStatusUpdateParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::types::{ResourceMeta, ServiceResponse};

/// The SM `I_EHR_STATUS` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_status.adoc`): "Interface to
/// `EHR_STATUS` of an EHR, with implicit Contribution creation."
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait EhrStatusService: Send + Sync {
    /// `GET /ehr/{ehr_id}/ehr_status/{version_uid}` — the **bare** `EHR_STATUS` at
    /// a specific version (not the `ORIGINAL_VERSION` wrapper — F-01-03). `200`
    /// with `ETag`(`version_uid`)/`Location` (`200_EHR_STATUS_retrieved.yaml`).
    async fn ehr_status_get_by_version_id(
        &self,
        _params: EhrStatusGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/ehr_status` — the `EHR_STATUS` (current or at time).
    /// `200` with `ETag`/`Location` (`200_EHR_STATUS_retrieved.yaml`).
    async fn ehr_status_get_at_time(
        &self,
        _params: EhrStatusGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/ehr_status` — update `EHR_STATUS`. Default `204` (no
    /// body); `200` + body on `return=representation`; `ETag`/`Location` on both
    /// (`204_EHR_STATUS.yaml` / `200_EHR_STATUS_updated.yaml`).
    async fn ehr_status_update(
        &self,
        _params: EhrStatusUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status` — the `VERSIONED_EHR_STATUS`.
    /// `200_VERSIONED_EHR_STATUS` (no `ETag`/`Location`).
    async fn versioned_ehr_status_get(
        &self,
        _params: VersionedEhrStatusGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`. `200` (plain).
    async fn versioned_ehr_status_revision_history(
        &self,
        _params: VersionedEhrStatusRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version` — the VERSION extant at a
    /// time. `200_VERSION_at_time`: `ETag`(`version_uid`)/`Location`.
    async fn versioned_ehr_status_version_get_at_time(
        &self,
        _params: VersionedEhrStatusVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}` — the
    /// `ORIGINAL_VERSION`. `200_VERSION` (no `ETag`/`Location`).
    async fn versioned_ehr_status_version_get_by_id(
        &self,
        _params: VersionedEhrStatusVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`.
    async fn ehr_status_tags_get(
        &self,
        _params: EhrStatusTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`.
    async fn ehr_status_tags_update(
        &self,
        _params: EhrStatusTagsUpdateParams,
        _body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}`.
    async fn ehr_status_tags_delete(
        &self,
        _params: EhrStatusTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// The current `EHR_STATUS` version metadata, for the latest `version_uid` the
    /// spec requires in the `ETag`/`Location` of a `412` precondition failure
    /// (`412_EHR_STATUS.yaml`). `None` if the EHR/status is unknown.
    async fn ehr_status_latest_meta(
        &self,
        _ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }
}
