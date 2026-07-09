//! The SM `I_DEMOGRAPHIC_SERVICE` / `I_PARTY` interfaces — demographic PARTY
//! operations.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{CallStatusType, SmError};

use crate::types::{PartyKind, ResourceMeta, ServiceResponse};

/// The DEMOGRAPHIC API group's application seam.
///
/// Realizes the SM `I_DEMOGRAPHIC_SERVICE`/`I_PARTY` interfaces
/// (`docs/specs/openehr/SM/docs/UML/classes/i_demographic_service.adoc`,
/// `i_party.adoc`).
///
/// ITS-REST 1.0.3 defines **no** demographic wire contract (the SM demographic
/// service is abstract; the CNF demographic schedule — master10 — is all TBD;
/// the CNF profiles table lists demographic as OPTIONS-profile only). This seam
/// is therefore our own design **by analogy with the EHR group** (ADR-008):
/// parties are versioned objects on the same machinery with no EHR scope, and
/// the status codes / `ETag` / `Location` / `Prefer` / `If-Match` behaviour
/// mirrors the EHR group. Each method returns a [`ServiceResponse`] (RM payload
/// + typed [`ResourceMeta`]) from which the HTTP edge derives the headers; a
/// demographic [`ResourceMeta`] carries an **empty** `ehr_id` (parties are not
/// EHR-scoped), so the dispatcher builds a `/demographic/…` `Location` without
/// one. Every method defaults to `NotImplemented`.
#[async_trait]
pub trait DemographicService: Send + Sync {
    /// `POST /demographic/{kind}` — create a party. `201` + `ETag`(version
    /// uid)/`Location`; body per `Prefer`.
    async fn party_create(
        &self,
        _kind: PartyKind,
        _body: Value,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/{kind}/{uid_based_id}` — retrieve a party (current,
    /// at-time, or a specific version). `200` + `ETag`/`Location`; a deleted
    /// current version → `Null` body (→ `204`).
    async fn party_get(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
        _version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `PUT /demographic/{kind}/{uid_based_id}` — commit a new party version.
    /// `If-Match` carries the preceding `OBJECT_VERSION_ID`; `200`/`204` per
    /// `Prefer`; `ETag`/`Location` on both.
    async fn party_update(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
        _if_match: String,
        _body: Value,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `DELETE /demographic/{kind}/{uid_based_id}` — logical delete. The
    /// `uid_based_id` MUST be an `OBJECT_VERSION_ID` (the preceding version).
    /// `204` + `ETag`/`Location` of the deleted version.
    async fn party_delete(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party/{versioned_object_uid}` — the
    /// `VERSIONED_PARTY`. `200` (plain).
    async fn versioned_party_get(
        &self,
        _versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party/{versioned_object_uid}/revision_history`.
    async fn versioned_party_revision_history(
        &self,
        _versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party/{versioned_object_uid}/version` — the
    /// VERSION extant at a time (or the latest). `ETag`(version uid)/`Location`.
    async fn versioned_party_version_get_at_time(
        &self,
        _versioned_object_uid: String,
        _version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/versioned_party/{versioned_object_uid}/version/{version_uid}`
    /// — the `ORIGINAL_VERSION`. `200` (plain).
    async fn versioned_party_version_get_by_id(
        &self,
        _versioned_object_uid: String,
        _version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `POST /demographic/contribution` — commit a demographic CONTRIBUTION
    /// (ehr-less; its versions reference party objects). `201` +
    /// `ETag`(contribution uid)/`Location`; body per `Prefer`.
    async fn demographic_contribution_create(
        &self,
        _body: Value,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/contribution/{contribution_uid}` — a demographic
    /// (ehr-less) CONTRIBUTION. `200` (plain).
    async fn demographic_contribution_get(
        &self,
        _contribution_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/tags` — all demographic item tags (ehr-less).
    async fn demographic_tags_get(
        &self,
        _tag_key: Option<String>,
        _tag_value: Option<String>,
        _tag_target_path: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `GET /demographic/{kind}/{uid_based_id}/tags`.
    async fn party_tags_get(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `PUT /demographic/{kind}/{uid_based_id}/tags`.
    async fn party_tags_update(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
        _body: Vec<Value>,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// `DELETE /demographic/{kind}/{uid_based_id}/tags/{key}`.
    async fn party_tags_delete(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
        _key: String,
    ) -> Result<ServiceResponse, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }

    /// The current party version metadata, for the latest `version_uid` the
    /// spec-analogous `412` precondition failure echoes in `ETag`/`Location`.
    /// `None` if the party is unknown.
    async fn demographic_latest_meta(
        &self,
        _kind: PartyKind,
        _uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        Ok(None)
    }
}
