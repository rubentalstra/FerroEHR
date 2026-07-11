//! The SM `I_EHR_STATUS` interface — the literal openEHR Platform Service Model
//! call set (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_status.adoc`; digest
//! `docs/design/sm-platform/02-ehr-service.md` §3). "Interface to `EHR_STATUS`
//! of an EHR, with implicit Contribution creation" — every mutating call
//! creates a new `EHR_STATUS` version + `CONTRIBUTION` server-side.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SmError;
use crate::types::UpdateVersion;

/// `I_EHR_STATUS` — `EHR_STATUS` operations, one Rust method per SM call.
/// Reads return the canonical `EHR_STATUS`/`VERSION`/`VERSIONED_EHR_STATUS` as
/// [`Value`]; the implicit-Contribution write returns the new `version_uid`.
#[async_trait]
pub trait EhrStatusService: Send + Sync {
    /// `has_ehr_status_version (an_ehr_id: UUID, a_version_uid: UUID): Boolean`
    /// — pre `has_ehr(an_ehr_id)`.
    async fn has_ehr_status_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
    ) -> Result<bool, SmError>;

    /// `get_ehr_status (an_ehr_id: UUID): EHR_STATUS` — the current version.
    /// Pre `has_ehr(an_ehr_id)`.
    async fn get_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    /// `get_ehr_status_at_time (an_ehr_id, a_time: Iso8601_date_time [0..1]):
    /// EHR_STATUS` — no time ⇒ latest. Pre `has_ehr(an_ehr_id)`.
    ///
    /// PORT NOTE: the SM signature omits `an_ehr_id` (spec defect, digest 2
    /// §9); it is restored here. `a_time` is carried as an ISO-8601 string to
    /// preserve partial-precision semantics (consistent with the codebase's
    /// `version_at_time` handling).
    async fn get_ehr_status_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError>;

    /// `get_ehr_status_at_version (an_ehr_id: UUID, a_version_uid: UUID):
    /// EHR_STATUS` — the **bare** `EHR_STATUS` at a version (not the
    /// `ORIGINAL_VERSION` wrapper; F-01-03). Pre `has_ehr`.
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
    /// PORT NOTE: the SM decomposes `EHR_STATUS` mutation into
    /// `set_/clear_ehr_queryable`, `set_/clear_ehr_modifiable`, and
    /// `update_other_details`; the ITS-REST wire replaces the whole object in
    /// one call, so this composite adapter method realizes those SM calls
    /// jointly (formal equivalence, `master02-overview.adoc` §Interface Calls).
    /// The optimistic-concurrency `preceding_version_uid` rides in
    /// [`UpdateVersion`]; a mismatch → `version_mismatch`.
    async fn replace_ehr_status(
        &self,
        an_ehr_id: Uuid,
        a_status: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`.
    async fn ehr_status_revision_history(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version` — the `ORIGINAL_VERSION`
    /// extant at `a_time` (or latest), returned with its `version_uid` for the
    /// `200_VERSION_at_time` `ETag`/`Location`.
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
