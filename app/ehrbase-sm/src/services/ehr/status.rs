//! `I_EHR_STATUS` (`i_ehr_status.adoc`) — "Interface to `EHR_STATUS` of an
//! EHR, with implicit Contribution creation."

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::common::{SmError, UpdateVersion};

/// `I_EHR_STATUS` — `EHR_STATUS` operations, one Rust method per SM call.
/// Reads return the canonical `EHR_STATUS`/`VERSION`/`VERSIONED_EHR_STATUS`
/// as [`Value`]; every mutating call creates a new `EHR_STATUS` version +
/// `CONTRIBUTION` server-side and returns the new `version_uid`.
#[async_trait]
pub trait EhrStatusService: Send + Sync {
    /// `has_ehr_status_version (an_ehr_id: UUID, a_version_uid: UUID):
    /// Boolean` — pre `has_ehr(an_ehr_id)`. Error `ehr_id_does_not_exist`.
    async fn has_ehr_status_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
    ) -> Result<bool, SmError>;

    /// `get_ehr_status (an_ehr_id: UUID): EHR_STATUS` — "the current version
    /// of the `EHR_STATUS` object for an EHR." Pre `has_ehr(an_ehr_id)`.
    async fn get_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    /// `get_ehr_status_at_time (a_time: Iso8601_date_time [0..1]):
    /// EHR_STATUS` — "the version of the EHR Status extant at time `a_time`.
    /// If no time supplied, get the latest." Pre `has_ehr(an_ehr_id)`.
    ///
    /// PORT NOTE: the SM signature omits `an_ehr_id` while its precondition
    /// references it (spec defect); it is restored here. `a_time` is carried
    /// as an ISO-8601 string to preserve partial-precision semantics.
    async fn get_ehr_status_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError>;

    /// `set_ehr_queryable (an_ehr_id: UUID)` — "Set the EHR `is_queryable`
    /// flag to true; this ensures it is included by the query engine."
    /// Pre `has_ehr`; post `get_ehr_status(an_ehr_id).is_queryable`.
    /// An implicit-CONTRIBUTION `EHR_STATUS` version commit; returns the new
    /// `version_uid`.
    async fn set_ehr_queryable(&self, an_ehr_id: Uuid) -> Result<String, SmError>;

    /// `clear_ehr_queryable (an_ehr_id: UUID)` — "Clear the EHR
    /// `is_queryable` flag; this ensures it is ignored by the query engine."
    /// Pre `has_ehr`; post `not get_ehr_status(an_ehr_id).is_queryable`.
    async fn clear_ehr_queryable(&self, an_ehr_id: Uuid) -> Result<String, SmError>;

    /// `set_ehr_modifiable (an_ehr_id: UUID)` — "Set the EHR `is_modifiable`
    /// flag to true; this ensures it is treated as active and updatable."
    /// Pre `has_ehr`; post `get_ehr_status(an_ehr_id).is_modifiable`.
    async fn set_ehr_modifiable(&self, an_ehr_id: Uuid) -> Result<String, SmError>;

    /// `clear_ehr_modifiable (an_ehr_id: UUID)` — "Clear the EHR
    /// `is_modifiable` flag" (content writes to the EHR are then blocked —
    /// RM `EHR_STATUS.is_modifiable`). Pre `has_ehr`; post
    /// `not get_ehr_status(an_ehr_id).is_modifiable`.
    async fn clear_ehr_modifiable(&self, an_ehr_id: Uuid) -> Result<String, SmError>;

    /// `update_other_details (an_ehr_id: UUID, a_details: ITEM_TREE)` —
    /// "Update `other_details` part of `EHR_STATUS` with new content." Pre
    /// `has_ehr`. `a_details` is the canonical-JSON `ITEM_TREE`.
    async fn update_other_details(
        &self,
        an_ehr_id: Uuid,
        a_details: Value,
    ) -> Result<String, SmError>;

    /// `get_ehr_status_at_version (an_ehr_id: UUID, a_version_uid: UUID):
    /// EHR_STATUS` — the **bare** `EHR_STATUS` at a version (not the
    /// `ORIGINAL_VERSION` wrapper). Pre `has_ehr`.
    ///
    /// `a_version` is the `VERSION_TREE_ID` lexical form
    /// (`trunk_version [ '.' branch_number '.' branch_version ]`, BASE
    /// identification) — branch versions are addressable (RM common master06
    /// §Version tree).
    async fn get_ehr_status_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError>;

    /// `get_versioned_ehr_status (an_ehr_id: UUID): VERSIONED_EHR_STATUS` —
    /// pre `has_ehr`.
    async fn get_versioned_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    // ── ITS-REST wire assembly (adapter-support, not single SM calls) ───────

    /// The ITS-REST `PUT /ehr/{ehr_id}/ehr_status` write: replace the whole
    /// `EHR_STATUS`, returning the new `version_uid`.
    ///
    /// PORT NOTE: the ITS-REST wire replaces the whole object in one call —
    /// the composite of the SM's discrete mutators
    /// ([`set_ehr_queryable`](Self::set_ehr_queryable) …
    /// [`update_other_details`](Self::update_other_details)), which remain
    /// individually callable on the native API (formal equivalence,
    /// `master02-overview.adoc` §Interface Calls). The optimistic-concurrency
    /// `preceding_version_uid` rides in [`UpdateVersion`]; a mismatch →
    /// `version_mismatch`.
    async fn replace_ehr_status(
        &self,
        an_ehr_id: Uuid,
        a_status: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`.
    async fn ehr_status_revision_history(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version` — the
    /// `ORIGINAL_VERSION` extant at `a_time` (or latest), returned with its
    /// `version_uid` for the `200_VERSION_at_time` `ETag`/`Location`.
    async fn ehr_status_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError>;

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}` — the
    /// `ORIGINAL_VERSION` at a specific version (`a_version` = the
    /// `VERSION_TREE_ID` lexical form; trunk or branch).
    async fn ehr_status_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError>;
}
