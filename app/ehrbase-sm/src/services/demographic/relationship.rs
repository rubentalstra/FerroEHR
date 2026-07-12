//! `I_PARTY_RELATIONSHIP` (`i_party_relationship.adoc`: "Interface for
//! `PARTY_RELATIONSHIP` operations") plus the `I_DEMOGRAPHIC_SERVICE`
//! `create_party_relationship` factory (`i_demographic_service.adoc`).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::common::{SmError, UpdateVersion};
use crate::extensions::response::{ResourceMeta, ServiceResponse};

/// `I_PARTY_RELATIONSHIP` — `PARTY_RELATIONSHIP` operations. The SM core
/// calls come first, one Rust method per SM call; the wire-seam methods (our
/// extension by analogy with the party group — module PORT NOTE in
/// [`super`]) follow. The `UV_PARTY_RELATIONSHIP` commit envelope is
/// [`UpdateVersion`]`<PARTY_RELATIONSHIP>` (master03 §Version Update
/// Semantics).
///
/// `I_DEMOGRAPHIC_SERVICE.i_party_relationship (a_versioned_party_rel_id):
/// I_PARTY_RELATIONSHIP` — the SM accessor — is realized by the flat calls
/// below taking the versioned-relationship id directly (formal equivalence,
/// `master02-overview.adoc` §Interface Calls).
#[async_trait]
pub trait PartyRelationshipService: Send + Sync {
    // ── the SM core ─────────────────────────────────────────────────────────

    /// `create_party_relationship (a_version: UV_PARTY_RELATIONSHIP): UUID` —
    /// "Create the first version of a new `PARTY_RELATIONSHIP`. Causes
    /// server-side creation of a new `VERSIONED_OBJECT`, `ORIGINAL_VERSION`
    /// and new `CONTRIBUTION`" (`i_demographic_service.adoc`). Pre
    /// `Pre_content_valid: valid_content (a_version)`. Errors
    /// `definition_unknown`, `content_invalid`.
    async fn create_party_relationship(&self, a_version: UpdateVersion) -> Result<Uuid, SmError>;

    /// `has_party_relationship (a_versioned_party_rel_id: UUID): Boolean` —
    /// "Return True if Party relationship exists in service."
    async fn has_party_relationship(&self, a_versioned_party_rel_id: Uuid)
    -> Result<bool, SmError>;

    /// `get_party_relationship (a_versioned_party_rel_id: UUID):
    /// PARTY_RELATIONSHIP` — "Get the current Version of a Party
    /// relationship." Error `versioned_object_does_not_exist`.
    async fn get_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<Value, SmError>;

    /// `get_party_relationship_at_time (a_versioned_party_rel_id: UUID,
    /// a_time: Iso8601_date_time): PARTY_RELATIONSHIP` — "Get the Version of
    /// a Party relationship current at `a_time`." Error
    /// `versioned_object_does_not_exist`.
    async fn get_party_relationship_at_time(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_time: String,
    ) -> Result<Value, SmError>;

    /// `get_party_relationship_at_version (a_party_rel_version_id: UUID):
    /// PARTY_RELATIONSHIP` — "Get a particular Party relationship Version."
    /// Error `object_version_does_not_exist`.
    async fn get_party_relationship_at_version(
        &self,
        a_party_rel_version_id: String,
    ) -> Result<Value, SmError>;

    /// `update_party_relationship (a_versioned_party_rel_id: UUID, a_version:
    /// UV_PARTY_RELATIONSHIP): UUID` — "Update a `PARTY_RELATIONSHIP` with a
    /// new Version. Causes server-side creation of a new `ORIGINAL_VERSION`
    /// and `CONTRIBUTION`." Pre `definitions_valid` +
    /// `has_party_relationship`. Errors `versioned_object_does_not_exist`,
    /// `object_version_does_not_exist`, `definition_unknown`,
    /// `content_invalid`.
    async fn update_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_version: UpdateVersion,
    ) -> Result<String, SmError>;

    /// `delete_party_relationship (a_versioned_party_rel_id: UUID)` —
    /// "Delete an existing Party relationship." Pre
    /// `has_party_relationship`; post `Post_relationship_deleted: not
    /// has_party_relationship` — realized as logical delete (lifecycle
    /// `523|deleted|`). Error `versioned_object_does_not_exist`.
    async fn delete_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<String, SmError>;

    // ── the relationship wire seam (extension — module PORT NOTE) ───────────

    /// `POST /demographic/party_relationship` — create the first version
    /// (`create_party_relationship`). `201` + `ETag`(version uid)/`Location`;
    /// body per `Prefer`.
    async fn party_relationship_create(&self, body: Value) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/party_relationship/{uid_based_id}` — current,
    /// at-time, or a specific version (`get_party_relationship` /
    /// `…_at_time` jointly). `200` + `ETag`/`Location`; a deleted current
    /// version → `Null` body (→ `204`).
    async fn party_relationship_get(
        &self,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `PUT /demographic/party_relationship/{uid_based_id}` — commit a new
    /// version (`update_party_relationship`). `If-Match` carries the
    /// preceding `OBJECT_VERSION_ID`. `200`/`204` per `Prefer`;
    /// `ETag`/`Location`.
    async fn party_relationship_update(
        &self,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError>;

    /// `DELETE /demographic/party_relationship/{uid_based_id}` — logical
    /// delete (`delete_party_relationship`). The `uid_based_id` MUST be an
    /// `OBJECT_VERSION_ID`. `204` + `ETag`/`Location` of the deleted version.
    async fn party_relationship_delete(
        &self,
        uid_based_id: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}`
    /// — the `VERSIONED_OBJECT` wrapper. `200` (plain).
    async fn versioned_party_relationship_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/revision_history`.
    async fn party_relationship_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/version`
    /// — the VERSION extant at a time (or the latest).
    /// `ETag`(version uid)/`Location`.
    async fn party_relationship_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}`
    /// — `get_party_relationship_at_version`: the `ORIGINAL_VERSION`. `200`
    /// (plain). Error `object_version_does_not_exist` → `404`.
    async fn party_relationship_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError>;

    /// The current relationship version metadata, for the latest
    /// `version_uid` the `412` precondition failure echoes in
    /// `ETag`/`Location`. `None` if the relationship is unknown.
    async fn party_relationship_latest_meta(
        &self,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError>;
}
