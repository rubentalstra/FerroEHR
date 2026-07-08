//! The SM `I_EHR_DIRECTORY` interface — DIRECTORY (FOLDER) operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::types::{ResourceMeta, ServiceResponse};

/// The SM `I_EHR_DIRECTORY` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_directory.adoc`): "Operations
/// on EHR directory, with implicit Contribution creation."
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait EhrDirectoryService: Send + Sync {
    /// `GET /ehr/{ehr_id}/directory` — the directory FOLDER (current or at time),
    /// or a deleted read → empty body (→ `204`). `200_FOLDER_retrieved` (no
    /// `ETag`/`Location`).
    async fn directory_get_at_time(
        &self,
        _params: DirectoryGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/directory` — update. Default `204`; `200` + body on
    /// `return=representation`; `ETag`/`Location` on both
    /// (`204_directory_updated.yaml` / `200_directory_updated.yaml`).
    async fn directory_update(
        &self,
        _params: DirectoryUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `POST /ehr/{ehr_id}/directory` — create the directory FOLDER. `201` +
    /// `ETag`/`Location`; body per `Prefer` (`201_directory.yaml`).
    async fn directory_create(
        &self,
        _params: DirectoryCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/directory` — logical delete. `204_because_deleted`
    /// (no headers).
    async fn directory_delete(
        &self,
        _params: DirectoryDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/directory/{version_uid}` — a specific version, or a
    /// deleted read → empty body (→ `204`). `200_FOLDER_retrieved` (no headers).
    async fn directory_get_by_version_id(
        &self,
        _params: DirectoryGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// The current directory FOLDER version metadata, for the latest
    /// `version_uid` in the `ETag`/`Location` of a `412` (`412_directory.yaml`).
    async fn directory_latest_meta(
        &self,
        _ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }
}
