//! The SM `I_PARTY_RELATIONSHIP` interface (+ the `I_DEMOGRAPHIC_SERVICE`
//! `create_party_relationship` factory) — demographic `PARTY_RELATIONSHIP`
//! operations.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{CallStatusType, SmError};

use crate::types::{ResourceMeta, ServiceResponse};

/// The `PARTY_RELATIONSHIP` slice of the DEMOGRAPHIC group's application seam.
///
/// Realizes `I_PARTY_RELATIONSHIP`
/// (`docs/specs/openehr/SM/docs/UML/classes/i_party_relationship.adoc`) plus
/// the `create_party_relationship(UV_PARTY_RELATIONSHIP): UUID` factory of
/// `I_DEMOGRAPHIC_SERVICE` (`i_demographic_service.adoc`).
///
/// As with [`DemographicService`](super::DemographicService), ITS-REST 1.0.3
/// defines **no** demographic wire contract, so this seam is our own design by
/// analogy with the EHR group (ADR-008): relationships are versioned objects on
/// the same machinery with no EHR scope, and the status codes / `ETag` /
/// `Location` / `Prefer` / `If-Match` behaviour mirror the party group. Each
/// method returns a [`ServiceResponse`]; a relationship [`ResourceMeta`] carries
/// an **empty** `ehr_id`. Every method defaults to `NotImplemented`.
#[async_trait]
pub trait PartyRelationshipService: Send + Sync {
    /// `POST /demographic/party_relationship` — `I_DEMOGRAPHIC_SERVICE`
    /// `create_party_relationship` (pre `valid_content`): create the first
    /// version, server-side `VERSIONED_OBJECT` + `ORIGINAL_VERSION` + `CONTRIBUTION`.
    /// `201` + `ETag`(version uid)/`Location`; body per `Prefer`.
    async fn party_relationship_create(&self, _body: Value) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/party_relationship/{uid_based_id}` —
    /// `I_PARTY_RELATIONSHIP` `get_party_relationship` /
    /// `get_party_relationship_at_time` (current, at-time, or a specific
    /// version). `200` + `ETag`/`Location`; a deleted current version → `Null`
    /// body (→ `204`). Errors: `versioned_object_does_not_exist` → `404`.
    async fn party_relationship_get(
        &self,
        _uid_based_id: String,
        _version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `PUT /demographic/party_relationship/{uid_based_id}` —
    /// `I_PARTY_RELATIONSHIP` `update_party_relationship` (pre
    /// `definitions_valid` + `has_party_relationship`): a new
    /// `ORIGINAL_VERSION` + `CONTRIBUTION`. `If-Match` carries the preceding
    /// `OBJECT_VERSION_ID`. `200`/`204` per `Prefer`; `ETag`/`Location`.
    /// Errors: `versioned_object_does_not_exist`/`object_version_does_not_exist`
    /// → `404`, `content_invalid` → `422`.
    async fn party_relationship_update(
        &self,
        _uid_based_id: String,
        _if_match: String,
        _body: Value,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `DELETE /demographic/party_relationship/{uid_based_id}` —
    /// `I_PARTY_RELATIONSHIP` `delete_party_relationship` (pre
    /// `has_party_relationship`, post `not has_party_relationship`). The
    /// `uid_based_id` MUST be an `OBJECT_VERSION_ID`. `204` + `ETag`/`Location`
    /// of the deleted version.
    async fn party_relationship_delete(
        &self,
        _uid_based_id: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}`
    /// — the `VERSIONED_OBJECT` wrapper. `200` (plain).
    async fn versioned_party_relationship_get(
        &self,
        _versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/revision_history`.
    async fn party_relationship_revision_history(
        &self,
        _versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/version`
    /// — the VERSION extant at a time (or the latest).
    /// `ETag`(version uid)/`Location`.
    async fn party_relationship_version_get_at_time(
        &self,
        _versioned_object_uid: String,
        _version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}`
    /// — `I_PARTY_RELATIONSHIP` `get_party_relationship_at_version`: the
    /// `ORIGINAL_VERSION`. `200` (plain). Errors:
    /// `object_version_does_not_exist` → `404`.
    async fn party_relationship_version_get_by_id(
        &self,
        _versioned_object_uid: String,
        _version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// The current relationship version metadata, for the latest `version_uid`
    /// the spec-analogous `412` precondition failure echoes in
    /// `ETag`/`Location`. `None` if the relationship is unknown.
    async fn party_relationship_latest_meta(
        &self,
        _uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        Ok(None)
    }
}
