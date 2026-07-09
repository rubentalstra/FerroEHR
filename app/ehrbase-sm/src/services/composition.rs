//! The SM `I_EHR_COMPOSITION` interface — the literal openEHR Platform Service
//! Model call set
//! (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_composition.adoc`; digest
//! `docs/design/sm-platform/02-ehr-service.md` §5). "Interface for commit and
//! retrieve of Compositions, with implicit Contribution creation."

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use openehr_base::prelude::ObjectVersionId;

use crate::error::SmError;
use crate::types::UpdateVersion;

/// `I_EHR_COMPOSITION` — COMPOSITION operations, one Rust method per SM call.
/// Reads return the canonical `COMPOSITION`/`VERSION`/`VERSIONED_COMPOSITION`
/// as [`Value`]; the implicit-Contribution writes return the new `version_uid`
/// (SM `create_composition`/`update_composition` → `UUID`).
#[async_trait]
pub trait EhrCompositionService: Send + Sync {
    /// `has_composition (an_ehr_id: UUID, a_version_uid: OBJECT_VERSION_ID):
    /// Boolean` — pre `has_ehr`. Error `ehr_id_does_not_exist`.
    async fn has_composition(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError>;

    /// `get_composition_latest (an_ehr_id: UUID, a_versioned_object_uid: UUID):
    /// COMPOSITION` — pre `has_ehr` + `has_composition`. Error
    /// `composition_does_not_exist`. A logically deleted composition resolves
    /// to `Value::Null` (→ wire `204`).
    async fn get_composition_latest(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError>;

    /// `get_composition_at_time (an_ehr_id, a_versioned_object_uid: UUID,
    /// a_time: Iso8601_date_time [0..1]): COMPOSITION` — no time ⇒ latest.
    async fn get_composition_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError>;

    /// `get_composition_at_version (an_ehr_id, a_version_uid:
    /// OBJECT_VERSION_ID): COMPOSITION` — errors `ehr_does_not_exist`,
    /// `object_version_does_not_exist`.
    async fn get_composition_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError>;

    /// `get_versioned_composition (an_ehr_id, a_versioned_object_uid: UUID):
    /// VERSIONED_COMPOSITION` — pre `has_ehr`. Error
    /// `versioned_composition_does_not_exist`.
    async fn get_versioned_composition(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError>;

    /// `create_composition (an_ehr_id: UUID, a_comp: UV_COMPOSITION): UUID` —
    /// pre `has_ehr` + `definitions_valid` + `valid_content`; post
    /// `has_composition(an_ehr_id, Result)`. Errors
    /// `composition_already_exists`, `definition_unknown`, `content_invalid`.
    /// Creates a `VERSIONED_OBJECT` + `ORIGINAL_VERSION` + CONTRIBUTION.
    async fn create_composition(
        &self,
        an_ehr_id: Uuid,
        a_comp: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `update_composition (an_ehr_id: UUID, a_comp: UV_COMPOSITION): UUID` —
    /// pre `has_ehr` + `definitions_valid` + `valid_content`;
    /// `a_comp.preceding_version_uid` must match the current version
    /// (optimistic lock → `version_mismatch`). New `ORIGINAL_VERSION` +
    /// CONTRIBUTION. Error `composition_does_not_exist`.
    async fn update_composition(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_comp: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `delete_composition (an_ehr_id: UUID, a_version_uid: OBJECT_VERSION_ID)`
    /// — logical delete: a new version with content removed, lifecycle
    /// `523|deleted|`. Returns the deleted `version_uid` (for the wire
    /// `204_COMPOSITION_deleted` `ETag`/`Location`).
    async fn delete_composition(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<String, SmError>;

    // ── ITS-REST wire assembly (adapter-support, not single SM calls) ───────

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`.
    async fn composition_revision_history(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError>;

    /// `GET …/versioned_composition/{versioned_object_uid}/version` — the
    /// `ORIGINAL_VERSION` extant at `a_time` (or latest).
    async fn composition_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError>;

    /// `GET …/versioned_composition/{versioned_object_uid}/version/{version_uid}`
    /// — the `ORIGINAL_VERSION` at a specific version.
    async fn composition_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError>;
}
