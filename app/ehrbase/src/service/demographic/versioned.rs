//! `VERSIONED_PARTY` read surface — the demographic analogue of the EHR
//! `versioned_composition` reads (`VERSIONED_PARTY`, its `REVISION_HISTORY`, and
//! its `ORIGINAL_VERSION`s). The wire is the Demographic API of ITS-REST
//! Release-1.1.0 (DEVELOPMENT lifecycle within the released spec), which
//! mirrors the EHR group's versioned-object reads by design.
//!
//! The assembly (and its version-spine reads through
//! `crate::storage::version_repo`) is shared with the relationship surface in
//! the `support` module; RM common master04 §Revision History / master06
//! §Versioned Objects govern the wire shapes.

use serde_json::Value;

use crate::ids::VoId;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::service::response::ServiceResponse;
use crate::versioning::object_version_id::TreeId;

impl EhrbaseService {
    /// The `VERSIONED_PARTY` for a party (any of the five kinds). A non-party id
    /// is `404`. `time_created` is the commit time of the earliest held version
    /// (for a locally-created party, v1); the `owner_id` NOTE lives in
    /// `support::versioned_wrapper`.
    pub(super) async fn versioned_party(&self, vo_id: VoId) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        self.versioned_wrapper(vo_id, "VERSIONED_PARTY", "PARTY", "versioned party")
            .await
    }

    /// The `REVISION_HISTORY` of a party: one item per version with its
    /// `OBJECT_VERSION_ID` and the change's `AUDIT_DETAILS` (RM common master04
    /// §Revision History). A non-party id is `404`.
    pub(super) async fn party_revision_history(&self, vo_id: VoId) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        self.demographic_revision_history(vo_id).await
    }

    /// An `ORIGINAL_VERSION` of a party at a specific version. A non-party id is
    /// `404`.
    pub(super) async fn party_version(
        &self,
        vo_id: VoId,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        self.demographic_original_version(vo_id, version, "party")
            .await
    }

    /// The `ORIGINAL_VERSION` of a party extant at `at`, or the latest when `at`
    /// is `None`, with `ETag`/`Location` metadata for the VERSION resource.
    pub(super) async fn party_version_at_time(
        &self,
        vo_id: VoId,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        self.demographic_original_version_at(vo_id, at, "party")
            .await
    }
}
