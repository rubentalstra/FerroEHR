//! `I_PARTY` CRUD (`i_party.adoc`) + the `I_DEMOGRAPHIC_SERVICE.create_party`
//! factory (`i_demographic_service.adoc`) — the party create / read / update /
//! delete domain logic, built on the shared [`crate::versioning`]
//! change-control machinery with `ehr_id = None` (no EHR scope — our own
//! design; a party has no owning EHR). ITS-REST 1.0.3 defines no demographic
//! wire contract, so status/`ETag`/`Location`/`If-Match`/deleted-read semantics
//! follow the EHR group by analogy (module PORT NOTE in [`super`]).

use serde_json::Value;
use uuid::Uuid;

use crate::service::demographic::support;
use crate::service::demographic::types::PartyKind;
use crate::service::demographic::validate::validate_party_body;
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::version_update::UpdateAudit;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::change::WriteEnvelope;
use crate::versioning::{
    CommitEnv, TreeId, VersionRead, change_type, create, delete, demographic_current, object_kind,
    object_version_id, update,
};

/// The current version of a demographic party, resolved in ONE lean
/// `vo_version`⋈`audit` read (no node reassembly). Carries the kind-checked
/// identity + commit instant (the `ETag`/`If-Match` parts) and the not-deleted
/// gate, so a write path resolves the target **once** and threads it — the
/// dispatcher never resolves for `If-Match` and the service again for the
/// existence/kind gate. RM common master06 §Version Identification / §Logical
/// Deletion.
pub(super) struct CurrentParty {
    kind: PartyKind,
    vo_id: Uuid,
    tree: TreeId,
    creating_system_id: String,
    time_committed: jiff::Timestamp,
    deleted: bool,
}

impl CurrentParty {
    /// The current version's full `OBJECT_VERSION_ID` `{vo}::{system}::{tree}`
    /// (the `ETag` value / `Location` tail).
    fn ovid(&self) -> String {
        object_version_id(self.vo_id, &self.creating_system_id, self.tree)
    }

    /// The `ResourceMeta` a `412` (`If-Match`) echoes: the current
    /// `OBJECT_VERSION_ID` + its commit instant (empty `ehr_id` — parties are
    /// not EHR-scoped).
    pub(super) fn resource_meta(&self) -> ResourceMeta {
        ResourceMeta::new(String::new(), self.ovid()).with_last_modified(self.time_committed)
    }
}

impl EhrbaseService {
    /// Load a version of a party, verifying it is of the expected [`PartyKind`]
    /// and ehr-less. A wrong-kind or unknown id is `404`.
    async fn load_party_version(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<VersionRead, ServiceError> {
        // The stored kind (constant per versioned object) must match the route.
        let stored = object_kind(&self.pool, vo_id).await?;
        if stored != Some(support::kind_of(kind)) {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id}",
                kind.rm_type()
            )));
        }
        support::load_ehrless(&self.pool, vo_id, version, at)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))
    }

    /// Confirm a live party of the expected kind exists (not deleted) — the
    /// SM `has_party` precondition (`i_party.adoc`).
    pub(super) async fn ensure_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
    ) -> Result<(), ServiceError> {
        let read = self.load_party_version(kind, vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id} is deleted",
                kind.rm_type()
            )));
        }
        Ok(())
    }

    /// The stored [`PartyKind`] of a versioned object, for the kind-agnostic SM
    /// `I_PARTY` calls (which address parties by versioned-object id only). A
    /// non-party id (COMPOSITION, `PARTY_RELATIONSHIP`, …) or unknown id is `404`
    /// (`versioned_object_does_not_exist`).
    pub(super) async fn party_kind_at(&self, vo_id: Uuid) -> Result<PartyKind, ServiceError> {
        object_kind(&self.pool, vo_id)
            .await?
            .and_then(support::party_kind_of)
            .ok_or_else(|| ServiceError::NotFound(format!("versioned party {vo_id}")))
    }

    /// Confirm `vo_id` is some party (any of the five kinds) — the check for the
    /// kind-agnostic `versioned_party` reads. A non-party id (COMPOSITION, …) or
    /// unknown id is `404` (`versioned_object_does_not_exist`).
    pub(super) async fn ensure_any_party(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        match object_kind(&self.pool, vo_id).await? {
            Some(k) if k.is_party() => Ok(()),
            _ => Err(ServiceError::NotFound(format!("versioned party {vo_id}"))),
        }
    }

    /// `create_party` (`i_demographic_service.adoc`): create the first version
    /// of a new PARTY (server-side `VERSIONED_OBJECT` + `ORIGINAL_VERSION` +
    /// `CONTRIBUTION`). Returns it with its `uid` set and the create-response
    /// `ETag`/`Location` metadata.
    ///
    /// The representation is built **from the commit result** (never a
    /// post-commit re-read): the served body is the just-written canonical with
    /// the `uid` injected, byte-identical to a fresh [`Self::read_party`] (the
    /// node codec round-trips losslessly; pinned by a test). When the
    /// `DV_MULTIMEDIA` externalization engine is engaged the stored/served form
    /// is the offloaded body, which the in-memory input does not reflect, so the
    /// fresh read is kept for byte-fidelity (no openEHR spec governs media
    /// externalization — our own extension).
    pub(super) async fn commit_new_party(
        &self,
        kind: PartyKind,
        body: Value,
        update_audit: Option<&UpdateAudit>,
    ) -> Result<ServiceResponse, ServiceError> {
        validate_party_body(kind, &body)?;

        // The caller's UPDATE_VERSION audit attributes merge with the server
        // rules (ITS-REST committal MUST); the wire party seam passes them
        // when the request carried committal headers.
        let audit = self.demographic_audit(update_audit, change_type::CREATION, "PARTY creation");
        let ctx = CommitEnv::signing_ctx(self);
        // Keep the served bytes for the in-memory representation, unless media
        // externalization is on (then the fresh read reflects the offloaded form).
        let repr_body = self.multimedia.is_none().then(|| body.clone());
        let mut tx = self.pool.begin().await?;
        let committed = create(
            &mut tx,
            None,
            support::kind_of(kind),
            body,
            None,
            &audit,
            WriteEnvelope::default(),
            &ctx,
        )
        .await?;
        tx.commit().await?;
        support::record_commit();

        match repr_body {
            Some(canonical) => Ok(support::committed_response(canonical, &committed)),
            None => {
                self.read_party(kind, committed.vo_id, Some(committed.tree), None)
                    .await
            }
        }
    }

    /// `get_party` / `get_party_at_time` (`i_party.adoc`): retrieve a party by
    /// its versioned-object id, optionally at a specific version (`version`) or
    /// instant (`at`; else the latest). A deleted current version resolves to
    /// `Value::Null` (→ `204`, mirroring COMPOSITION). A wrong-kind object (a
    /// PERSON under the `agent` route, or a COMPOSITION) is `404`.
    pub(super) async fn read_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_party_version(kind, vo_id, version, at).await?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(support::version_response(vo_id, read))
    }

    /// Resolve the current version of a party of the routed [`PartyKind`] in ONE
    /// lean read ([`demographic_current`]) — kind-checked and ehr-less. `None`
    /// when there is no current version, the stored kind differs (a PERSON under
    /// the `agent` route), or the id is a non-demographic / EHR-scoped object.
    /// The write paths never reassemble nodes just to gate the write.
    pub(super) async fn party_current(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
    ) -> Result<Option<CurrentParty>, ServiceError> {
        let Some(current) = demographic_current(&self.pool, vo_id).await? else {
            return Ok(None);
        };
        if current.kind != support::kind_of(kind) {
            return Ok(None);
        }
        Ok(Some(CurrentParty {
            kind,
            vo_id,
            tree: current.tree,
            creating_system_id: current.creating_system_id,
            time_committed: current.time_committed,
            deleted: current.deleted,
        }))
    }

    /// `update_party` (`i_party.adoc`): commit a new party version. Resolves the
    /// current version once (kind + existence gate) and delegates to
    /// [`Self::commit_party_update`]. The wire seam (the `api` module) resolves
    /// for `If-Match` and calls `commit_party_update` directly, threading its
    /// handle so the target is resolved only once across the request.
    /// `expected` (from `If-Match`) enforces optimistic concurrency.
    pub(super) async fn update_party_version(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        body: Value,
        expected: Option<TreeId>,
        update_audit: Option<&UpdateAudit>,
    ) -> Result<ServiceResponse, ServiceError> {
        let current = self
            .party_current(kind, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        self.commit_party_update(current, body, expected, update_audit)
            .await
    }

    /// Commit a new version of an already-resolved party. `current` is the lean
    /// current-version handle (threaded from the dispatcher's `If-Match` resolve
    /// or from [`Self::update_party_version`]); a deleted current is `404` (pre
    /// `has_party`, `i_party.adoc §update_party`). The write response is built
    /// from the commit result (see [`Self::commit_new_party`]).
    pub(super) async fn commit_party_update(
        &self,
        current: CurrentParty,
        body: Value,
        expected: Option<TreeId>,
        update_audit: Option<&UpdateAudit>,
    ) -> Result<ServiceResponse, ServiceError> {
        let kind = current.kind;
        if current.deleted {
            return Err(ServiceError::NotFound(format!(
                "{} {} is deleted",
                kind.rm_type(),
                current.vo_id
            )));
        }
        validate_party_body(kind, &body)?;

        let audit = self.demographic_audit(update_audit, change_type::MODIFICATION, "PARTY update");
        let ctx = CommitEnv::signing_ctx(self);
        let repr_body = self.multimedia.is_none().then(|| body.clone());
        let mut tx = self.pool.begin().await?;
        let committed = update(
            &mut tx,
            None,
            current.vo_id,
            support::kind_of(kind),
            body,
            expected,
            None,
            &audit,
            WriteEnvelope::default(),
            &ctx,
        )
        .await?;
        tx.commit().await?;
        support::record_commit();

        match repr_body {
            Some(canonical) => Ok(support::committed_response(canonical, &committed)),
            None => {
                self.read_party(kind, committed.vo_id, Some(committed.tree), None)
                    .await
            }
        }
    }

    /// `delete_party` (`i_party.adoc`): logically delete a party (a new
    /// `523|deleted|` version — RM common master06 §Logical Deletion; post
    /// `not has_party` holds, a deleted party reads `Null`). Resolves the target
    /// once and delegates to [`Self::commit_party_delete`]; the wire seam
    /// threads its `If-Match` resolve instead. `expected` is the
    /// caller-supplied trunk version (from `If-Match` or the path
    /// `OBJECT_VERSION_ID`); when `Some`, a mismatch with the current version →
    /// `409`; when `None`, the current version is deleted unconditionally (SM
    /// `delete_party` has no version argument). An already-deleted target → `400`.
    pub(super) async fn delete_party_version(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        expected: Option<TreeId>,
        update_audit: Option<&UpdateAudit>,
    ) -> Result<ServiceResponse, ServiceError> {
        let current = self
            .party_current(kind, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        self.commit_party_delete(current, expected, update_audit)
            .await
    }

    /// Commit the logical delete of an already-resolved party (see
    /// [`Self::delete_party_version`] for the `expected` semantics). Returns
    /// the deleted version's `ETag`/`Location` metadata.
    pub(super) async fn commit_party_delete(
        &self,
        current: CurrentParty,
        expected: Option<TreeId>,
        update_audit: Option<&UpdateAudit>,
    ) -> Result<ServiceResponse, ServiceError> {
        if current.deleted {
            return Err(ServiceError::BadRequest(format!(
                "{} {} is already deleted",
                current.kind.rm_type(),
                current.vo_id
            )));
        }
        if let Some(expected) = expected
            && current.tree != expected
        {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                current.tree
            )));
        }

        let audit = self.demographic_audit(update_audit, change_type::DELETED, "PARTY delete");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let committed = delete(
            &mut tx,
            None,
            current.vo_id,
            support::kind_of(current.kind),
            Some(current.tree),
            &audit,
            WriteEnvelope::default(),
            &ctx,
        )
        .await?;
        tx.commit().await?;
        support::record_commit();

        Ok(ServiceResponse::deleted(ResourceMeta::new(
            String::new(),
            object_version_id(current.vo_id, &committed.creating_system_id, committed.tree),
        )))
    }

    /// The current party version metadata (the latest `version_uid` a `412`
    /// echoes in `ETag`/`Location`), or `None` if unknown/wrong-kind — the lean
    /// resolve, no node reassembly.
    pub(super) async fn party_current_meta(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        Ok(self
            .party_current(kind, vo_id)
            .await?
            .map(|c| c.resource_meta()))
    }
}
