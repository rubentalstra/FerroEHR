//! The SM `I_EHR_CONTRIBUTION` interface — CONTRIBUTION operations.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::ehr::{ContributionCreateParams, ContributionGetParams};
use openehr_its::rest::runtime::ApiError;

use crate::types::{Page, ServiceResponse};

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

    /// SM `I_EHR_CONTRIBUTION.list_contributions (an_ehr_id, time_range [0..1],
    /// item_offset [0..1], items_to_fetch [0..1]): List<UUID>` — "Obtain a list
    /// of identifiers of Contributions in EHR"
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`; error
    /// `ehr_does_not_exist`).
    ///
    /// `time_range` realizes the SM `Interval<Iso8601_date_time>` simply as an
    /// optional `(lower, upper)` pair of ISO 8601 bounds, either side open
    /// (`None`) — a `Some((None, None))` and a plain `None` both mean
    /// "unbounded". The `item_offset`/`items_to_fetch` cursor is carried by
    /// [`Page`].
    ///
    /// PORT NOTE: native-API call — no ITS-REST route (the wire spec defines
    /// none); exposed later via extension routes
    /// (`docs/design/sm-platform/08-target-architecture.md` §7). Defaults to
    /// `NotImplemented`.
    async fn contribution_list(
        &self,
        _ehr_id: String,
        _time_range: Option<(Option<String>, Option<String>)>,
        _page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// SM `I_EHR_CONTRIBUTION.contribution_count (ehr_id, time_range [0..1]):
    /// Integer` — "Obtain a count of Contributions in EHR"
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`; error
    /// `ehr_does_not_exist`). `time_range` is the same optional `(lower, upper)`
    /// pair of ISO 8601 bounds as [`Self::contribution_list`].
    ///
    /// PORT NOTE: native-API call — no ITS-REST route (the wire spec defines
    /// none); exposed later via extension routes
    /// (`docs/design/sm-platform/08-target-architecture.md` §7). Defaults to
    /// `NotImplemented`.
    async fn contribution_count(
        &self,
        _ehr_id: String,
        _time_range: Option<(Option<String>, Option<String>)>,
    ) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }
}
