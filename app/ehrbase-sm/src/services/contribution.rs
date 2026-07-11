//! The SM `I_EHR_CONTRIBUTION` interface — the literal openEHR Platform Service
//! Model call set
//! (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`; digest
//! `docs/design/sm-platform/02-ehr-service.md` §6). "Interface for explicit
//! Contribution level operations."

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SmError;
use crate::types::{Page, UpdateAudit, UpdateVersion};

/// The optional inclusive `(lower, upper)` ISO-8601 bounds of an SM
/// `Interval<Iso8601_date_time>` — either side open (`None`) means unbounded.
pub type TimeRange = Option<(Option<String>, Option<String>)>;

/// `I_EHR_CONTRIBUTION` — explicit CONTRIBUTION-level operations, one Rust
/// method per SM call.
#[async_trait]
pub trait EhrContributionService: Send + Sync {
    /// `has_contribution (an_ehr_id: UUID, a_contrib_id: UUID): Boolean` — pre
    /// `has_ehr`. Error `ehr_id_does_not_exist`.
    async fn has_contribution(&self, an_ehr_id: Uuid, a_contrib_id: Uuid) -> Result<bool, SmError>;

    /// `get_contribution` with `OBJECT_REF` resolution: the CONTRIBUTION's
    /// `versions` carry the full `ORIGINAL_VERSION` objects instead of
    /// `OBJECT_REF`s. Backs the ITS-REST `Prefer: resolve_refs` negotiation
    /// (`Requests_and_responses` §Representation details negotiation — no SM
    /// operation defines this; it is the REST adapter's negotiation surface).
    async fn get_contribution_resolved(
        &self,
        _an_ehr_id: Uuid,
        _a_contrib_id: Uuid,
    ) -> Result<serde_json::Value, SmError> {
        Err(SmError::new(
            crate::CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `get_contribution (an_ehr_id: UUID, a_contrib_id: UUID): CONTRIBUTION` —
    /// pre `has_ehr` + `has_contribution`. Error `contribution_does_not_exist`.
    async fn get_contribution(&self, an_ehr_id: Uuid, a_contrib_id: Uuid)
    -> Result<Value, SmError>;

    /// `commit_contribution (an_ehr_id: UUID, versions: List<UPDATE_VERSION>,
    /// an_audit: UPDATE_AUDIT): UUID` — "Commit a `CONTRIBUTION` containing any
    /// number of `UPDATE_VERSION` objects" (the explicit multi-version atomic
    /// commit). Pre `has_ehr`; post `has_contribution(Result)`. Returns the new
    /// `contribution_uid`.
    async fn commit_contribution(
        &self,
        an_ehr_id: Uuid,
        versions: Vec<UpdateVersion>,
        an_audit: UpdateAudit,
    ) -> Result<String, SmError>;

    /// `list_contributions (an_ehr_id: UUID, time_range:
    /// Interval<Iso8601_date_time> [0..1], item_offset [0..1], items_to_fetch
    /// [0..1]): List<UUID>` — Contribution ids in the EHR. Error
    /// `ehr_does_not_exist`.
    async fn list_contributions(
        &self,
        an_ehr_id: Uuid,
        time_range: TimeRange,
        page: Page,
    ) -> Result<Vec<String>, SmError>;

    /// `contribution_count (ehr_id: UUID, time_range [0..1]): Integer` — the
    /// count of Contributions in the EHR. Error `ehr_does_not_exist`.
    async fn contribution_count(
        &self,
        an_ehr_id: Uuid,
        time_range: TimeRange,
    ) -> Result<i64, SmError>;
}
