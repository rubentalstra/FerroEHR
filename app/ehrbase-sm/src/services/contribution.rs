//! The SM `I_EHR_CONTRIBUTION` interface — CONTRIBUTION operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{ContributionCreateParams, ContributionGetParams};
use openehr_its::rest::runtime::ApiError;

use crate::types::ServiceResponse;

/// The SM `I_EHR_CONTRIBUTION` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`):
/// "Interface for explicit Contribution level operations."
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
///
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait EhrContributionService: Send + Sync {
    /// `POST /ehr/{ehr_id}/contribution` — commit a CONTRIBUTION. `201` +
    /// `ETag`(`contribution_uid`)/`Location`; body per `Prefer`
    /// (`201_CONTRIBUTION.yaml`).
    async fn contribution_create(
        &self,
        _params: ContributionCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/contribution/{contribution_uid}`. `200_CONTRIBUTION`
    /// (no `ETag`/`Location`).
    async fn contribution_get(
        &self,
        _params: ContributionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }
}
