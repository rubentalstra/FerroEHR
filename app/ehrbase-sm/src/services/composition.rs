//! The SM `I_EHR_COMPOSITION` interface — COMPOSITION operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, VersionedCompositionGetParams,
    VersionedCompositionRevisionHistoryParams, VersionedCompositionVersionGetAtTimeParams,
    VersionedCompositionVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::types::{ResourceMeta, ServiceResponse};

/// The SM `I_EHR_COMPOSITION` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_composition.adoc`): "Interface
/// for commit and retrieve of Compositions, with implicit Contribution
/// creation."
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait EhrCompositionService: Send + Sync {
    /// `POST /ehr/{ehr_id}/composition` — create. `201` + `ETag`/`Location`;
    /// body per `Prefer` (`201_COMPOSITION.yaml`).
    async fn composition_create(
        &self,
        _params: CompositionCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/composition/{uid_based_id}` — retrieve. `200` +
    /// `ETag`/`Location`, or a deleted read → empty body (→ `204`).
    async fn composition_get(
        &self,
        _params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/composition/{uid_based_id}` — update. `200` +
    /// `ETag`/`Location`; body per `Prefer` (`200_COMPOSITION_updated.yaml`).
    async fn composition_update(
        &self,
        _params: CompositionUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/composition/{uid_based_id}` — logical delete. `204`
    /// + `ETag`/`Location` of the deleted version (`204_COMPOSITION_deleted.yaml`).
    async fn composition_delete(
        &self,
        _params: CompositionDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}`.
    /// `200_VERSIONED_COMPOSITION` (no `ETag`/`Location`).
    async fn versioned_composition_get(
        &self,
        _params: VersionedCompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`.
    async fn versioned_composition_revision_history(
        &self,
        _params: VersionedCompositionRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version` —
    /// the VERSION extant at a time. `200_VERSION_of_COMPOSITION_at_time`:
    /// `ETag`/`Location`.
    async fn versioned_composition_version_get_at_time(
        &self,
        _params: VersionedCompositionVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}`
    /// — the `ORIGINAL_VERSION`. `200_VERSION` (no `ETag`/`Location`).
    async fn versioned_composition_version_get_by_id(
        &self,
        _params: VersionedCompositionVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/composition/{uid_based_id}/tags`.
    async fn composition_tags_get(
        &self,
        _params: CompositionTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/composition/{uid_based_id}/tags`.
    async fn composition_tags_update(
        &self,
        _params: CompositionTagsUpdateParams,
        _body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}`.
    async fn composition_tags_delete(
        &self,
        _params: CompositionTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// The current COMPOSITION version metadata, for the latest `version_uid` in
    /// the `ETag`/`Location` of a `409`/`412`
    /// (`409_COMPOSITION_with_uid_based_id.yaml` / `412_COMPOSITION.yaml`).
    async fn composition_latest_meta(
        &self,
        _ehr_id: String,
        _uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }
}
