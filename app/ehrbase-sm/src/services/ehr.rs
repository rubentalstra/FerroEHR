//! The SM `I_EHR_SERVICE` interface — EHR-level operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    EhrCreateParams, EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams,
    EhrTagsGetParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::types::{EhrSummary, ServiceResponse};

/// The SM `I_EHR_SERVICE` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_service.adoc`): "Primary
/// interface to `EHR_SERVICE` persistent repository." The SM's per-EHR accessor
/// `I_EHR` is flattened — methods carry `ehr_id` (formal equivalence per
/// `master02-overview.adoc` §Interface Calls). `ehr_tags_get` is an ITS-REST
/// item-tag extension (SM-silent).
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait EhrService: Send + Sync {
    // ── EHR ──────────────────────────────────────────────────────────────────

    /// `GET /ehr` — find an EHR by subject. `200_EHR` (no `ETag`/`Location`).
    async fn ehr_get_by_subject(
        &self,
        _params: EhrGetBySubjectParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `POST /ehr` — create an EHR. `201` with `ETag`(`ehr_id`)/`Location`; body
    /// only on `Prefer: return=representation` (`201_EHR.yaml`).
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}` — retrieve an EHR. `200_EHR` (no `ETag`/`Location`).
    async fn ehr_get_by_id(&self, _params: EhrGetByIdParams) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}` — create an EHR with a client id. As `ehr_create`.
    async fn ehr_create_with_id(
        &self,
        _params: EhrCreateWithIdParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/tags` — all item tags in the EHR.
    async fn ehr_tags_get(&self, _params: EhrTagsGetParams) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// SM `I_EHR_SERVICE.get_ehr (an_ehr_id): EHR_SUMMARY`
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_service.adoc`) — the
    /// summary form of the EHR + its `EHR_STATUS`
    /// (`docs/specs/openehr/SM/docs/UML/classes/ehr_summary.adoc`: `ehr_id`,
    /// `system_id`, `ehr_status`, `time_created`, `contribution_count`,
    /// `composition_count`, all mandatory). Pre `has_ehr (an_ehr_id)` → error
    /// `ehr_does_not_exist`.
    ///
    /// Named `get_ehr_summary` (not `ehr_summary`) to realize SM `get_ehr`
    /// without colliding with the wire EHR-body builder the service already
    /// exposes internally (ITS-REST returns the RM `EHR` object for `GET
    /// /ehr/{ehr_id}`, not `EHR_SUMMARY`).
    ///
    /// PORT NOTE: native-API call — no ITS-REST route emits `EHR_SUMMARY`; the
    /// wire `GET /ehr/{ehr_id}` body stays the RM `EHR` object. Defaults to
    /// `NotImplemented`.
    async fn get_ehr_summary(&self, _ehr_id: String) -> Result<EhrSummary, ApiError> {
        Err(ApiError::NotImplemented)
    }
}
