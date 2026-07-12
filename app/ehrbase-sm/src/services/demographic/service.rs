//! `I_DEMOGRAPHIC_SERVICE` (`i_demographic_service.adoc`: "Primary interface
//! to `DEMOGRAPHIC_SERVICE`") and `I_PARTY` (`i_party.adoc`: "Interface for
//! `PARTY` level operations").

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::common::{SmError, UpdateVersion};
use crate::extensions::response::{ResourceMeta, ServiceResponse};

/// The concrete PARTY resource families of the DEMOGRAPHIC group (the five
/// concrete `ACTOR`/`PARTY` leaves of the RM demographic package the wire
/// routes are keyed by).
///
/// PORT NOTE: the SM addresses parties only by versioned-object id; the
/// per-kind routing is our wire extension (module PORT NOTE) — the RM `_type`
/// of the payload is the authority, `kind` the route key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyKind {
    /// `AGENT` (`/demographic/agent`).
    Agent,
    /// `GROUP` (`/demographic/group`).
    Group,
    /// `ORGANISATION` (`/demographic/organisation`).
    Organisation,
    /// `PERSON` (`/demographic/person`).
    Person,
    /// `ROLE` (`/demographic/role`).
    Role,
}

impl PartyKind {
    /// The RM `_type` this resource family stores (`PERSON`, `ROLE`, …).
    #[must_use]
    pub fn rm_type(self) -> &'static str {
        match self {
            PartyKind::Agent => "AGENT",
            PartyKind::Group => "GROUP",
            PartyKind::Organisation => "ORGANISATION",
            PartyKind::Person => "PERSON",
            PartyKind::Role => "ROLE",
        }
    }

    /// The URL path segment of this resource family (`agent`, `person`, …).
    #[must_use]
    pub fn segment(self) -> &'static str {
        match self {
            PartyKind::Agent => "agent",
            PartyKind::Group => "group",
            PartyKind::Organisation => "organisation",
            PartyKind::Person => "person",
            PartyKind::Role => "role",
        }
    }
}

/// `I_DEMOGRAPHIC_SERVICE` + `I_PARTY` — demographic PARTY operations.
///
/// The SM core calls come first, one Rust method per SM call with verbatim
/// names/preconditions; the wire-seam methods (our extension by analogy with
/// the EHR group — module PORT NOTE) follow, clearly separated. The
/// `UV_PARTY` commit envelope is [`UpdateVersion`]`<PARTY>` (master03
/// §Version Update Semantics).
///
/// `I_DEMOGRAPHIC_SERVICE.i_party (a_versioned_party_id): I_PARTY` — the SM
/// accessor — is realized by the flat `I_PARTY` calls below taking the
/// versioned-party id directly (formal equivalence, `master02-overview.adoc`
/// §Interface Calls); error `versioned_object_does_not_exist` surfaces at
/// each call.
#[async_trait]
pub trait DemographicService: Send + Sync {
    // ── I_DEMOGRAPHIC_SERVICE + I_PARTY (the SM core) ───────────────────────

    /// `create_party (a_version: UV_PARTY): UUID` — "Create the first version
    /// of a new `PARTY` object. Causes server-side creation of a new
    /// `VERSIONED_OBJECT`, `ORIGINAL_VERSION` and new `CONTRIBUTION`."
    /// Pre `Pre_party_definitions_valid: definitions_valid (a_version)` +
    /// `Pre_content_valid: valid_content (a_version)`. Errors
    /// `definition_unknown`, `content_invalid`.
    async fn create_party(&self, a_version: UpdateVersion) -> Result<Uuid, SmError>;

    /// `has_party (a_versioned_party_id: UUID): Boolean` — "Return True if
    /// Party exists" (`i_party.adoc`).
    async fn has_party(&self, a_versioned_party_id: Uuid) -> Result<bool, SmError>;

    /// `has_party_version_id (a_party_version_id: UUID): Boolean` — "True if
    /// a particular version of a Party exists."
    async fn has_party_version_id(&self, a_party_version_id: String) -> Result<bool, SmError>;

    /// `get_party (a_versioned_party_id: UUID): PARTY` — "Get the current
    /// Version of a Party." Pre `has_party`. Error
    /// `versioned_object_does_not_exist`.
    async fn get_party(&self, a_versioned_party_id: Uuid) -> Result<Value, SmError>;

    /// `get_party_at_time (a_versioned_party_id: UUID, a_time:
    /// Iso8601_date_time): PARTY` — "Get the Version of a Party current at
    /// `a_time`." Error `versioned_object_does_not_exist`.
    async fn get_party_at_time(
        &self,
        a_versioned_party_id: Uuid,
        a_time: String,
    ) -> Result<Value, SmError>;

    /// `get_party_at_version (a_party_version_id: UUID): PARTY` — "Get a
    /// particular Party Version." Pre `has_party_version`. Error
    /// `object_version_does_not_exist`.
    async fn get_party_at_version(&self, a_party_version_id: String) -> Result<Value, SmError>;

    /// `update_party (a_versioned_party_id: UUID, a_version: UV_PARTY): UUID`
    /// — "Update a `PARTY` with a new Version. Causes server-side creation of
    /// a new `ORIGINAL_VERSION` and `CONTRIBUTION`." Pre
    /// `definitions_valid` + `has_party`. Errors
    /// `versioned_object_does_not_exist`, `object_version_does_not_exist`,
    /// `definition_unknown`, `content_invalid`.
    async fn update_party(
        &self,
        a_versioned_party_id: Uuid,
        a_version: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `delete_party (a_versioned_party_id: UUID)` — "Delete an existing
    /// Party." Pre `has_party`; post `Post_party_deleted: not has_party` —
    /// realized as logical delete (a new version with lifecycle
    /// `523|deleted|`, satisfying the post-condition at the read surface).
    /// Error `versioned_object_does_not_exist`.
    async fn delete_party(&self, a_versioned_party_id: Uuid) -> Result<String, SmError>;

    // ── the demographic wire seam (extension — module PORT NOTE) ────────────

    /// `POST /demographic/{kind}` — create a party. `201` + `ETag`(version
    /// uid)/`Location`; body per `Prefer`. The wire body is the bare RM
    /// PARTY; the adapter wraps it into the `UV_PARTY` envelope with a
    /// server-default audit (documented wire adaptation of `create_party`).
    async fn party_create(&self, kind: PartyKind, body: Value) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/{kind}/{uid_based_id}` — retrieve a party (current,
    /// at-time, or a specific version — `get_party` /
    /// `get_party_at_time` / `get_party_at_version` jointly). `200` +
    /// `ETag`/`Location`; a deleted current version → `Null` body (→ `204`).
    async fn party_get(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `PUT /demographic/{kind}/{uid_based_id}` — commit a new party version
    /// (`update_party`). `If-Match` carries the preceding
    /// `OBJECT_VERSION_ID`; `200`/`204` per `Prefer`; `ETag`/`Location`.
    async fn party_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError>;

    /// `DELETE /demographic/{kind}/{uid_based_id}` — logical delete
    /// (`delete_party`). The `uid_based_id` is the versioned-party id —
    /// either a bare `HIER_OBJECT_ID` or an `OBJECT_VERSION_ID`; the
    /// preceding trunk version is taken from `If-Match` when present. `204` +
    /// `ETag`/`Location` of the deleted version.
    async fn party_delete(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party/{versioned_object_uid}` — the
    /// `VERSIONED_PARTY`. `200` (plain).
    async fn versioned_party_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party/{versioned_object_uid}/revision_history`.
    async fn versioned_party_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party/{versioned_object_uid}/version` —
    /// the VERSION extant at a time (or the latest).
    /// `ETag`(version uid)/`Location`.
    async fn versioned_party_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party/{versioned_object_uid}/version/{version_uid}`
    /// — the `ORIGINAL_VERSION`. `200` (plain).
    async fn versioned_party_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `POST /demographic/contribution` — commit a demographic CONTRIBUTION
    /// (ehr-less; its versions reference party objects). `201` +
    /// `ETag`(contribution uid)/`Location`; body per `Prefer`.
    async fn demographic_contribution_create(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/contribution/{contribution_uid}` — a demographic
    /// (ehr-less) CONTRIBUTION. `200` (plain).
    async fn demographic_contribution_get(
        &self,
        contribution_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/tags` — all demographic item tags (ehr-less; the
    /// item-tag extension applied to parties).
    async fn demographic_tags_get(
        &self,
        tag_key: Option<String>,
        tag_value: Option<String>,
        tag_target_path: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/{kind}/{uid_based_id}/tags`.
    async fn party_tags_get(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `PUT /demographic/{kind}/{uid_based_id}/tags`.
    async fn party_tags_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, SmError>;

    /// `DELETE /demographic/{kind}/{uid_based_id}/tags/{key}`.
    async fn party_tags_delete(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        key: String,
    ) -> Result<ServiceResponse, SmError>;

    /// The current party version metadata, for the latest `version_uid` the
    /// `412` precondition failure echoes in `ETag`/`Location`. `None` if the
    /// party is unknown.
    async fn demographic_latest_meta(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError>;
}
