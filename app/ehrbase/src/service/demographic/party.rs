//! `I_PARTY` CRUD (`i_party.adoc`) + the `I_DEMOGRAPHIC_SERVICE.create_party`
//! factory (`i_demographic_service.adoc`) — the party create / read / update /
//! delete domain logic, built on the shared [`crate::versioning`]
//! change-control machinery with `ehr_id = None` (no EHR scope — our own
//! design; a party has no owning EHR). ITS-REST 1.0.3 defines no demographic
//! wire contract, so status/`ETag`/`Location`/`If-Match`/deleted-read semantics
//! follow the EHR group by analogy (module PORT NOTE in [`super`]).

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use ehrbase_sm::PartyKind;
use serde_json::Value;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    CommitEnv, TreeId, change_type, create, delete, object_version_id, update,
};

use super::{kind_of, validate_party_body};

impl EhrbaseService {
    /// `create_party` (`i_demographic_service.adoc`): create the first version
    /// of a new PARTY (server-side `VERSIONED_OBJECT` + `ORIGINAL_VERSION` +
    /// `CONTRIBUTION`). Returns it with its `uid` set and the create-response
    /// `ETag`/`Location` metadata.
    pub(crate) async fn create_party(
        &self,
        kind: PartyKind,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        validate_party_body(kind, &body)?;

        let audit = self.demographic_audit(change_type::CREATION, "PARTY creation");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let committed = create(&mut tx, None, kind_of(kind), body, None, &audit, &ctx).await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_party(kind, committed.vo_id, Some(committed.tree), None)
            .await
    }

    /// `get_party` / `get_party_at_time` (`i_party.adoc`): retrieve a party by
    /// its versioned-object id, optionally at a specific version (`version`) or
    /// instant (`at`; else the latest). A deleted current version resolves to
    /// `Value::Null` (→ `204`, mirroring COMPOSITION). A wrong-kind object (a
    /// PERSON under the `agent` route, or a COMPOSITION) is `404`.
    pub(crate) async fn read_party(
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
        Ok(Self::party_version_response(vo_id, read))
    }

    /// `update_party` (`i_party.adoc`): commit a new party version. `expected`
    /// (from `If-Match`) enforces optimistic concurrency (a stale precondition →
    /// version conflict → `412`). Pre `has_party` is realized by
    /// [`ensure_party`](EhrbaseService::ensure_party).
    pub(crate) async fn update_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        body: Value,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_party(kind, vo_id).await?;
        validate_party_body(kind, &body)?;

        let audit = self.demographic_audit(change_type::MODIFICATION, "PARTY update");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let committed = update(
            &mut tx,
            None,
            vo_id,
            kind_of(kind),
            body,
            expected,
            None,
            &audit,
            &ctx,
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_party(kind, vo_id, Some(committed.tree), None)
            .await
    }

    /// `delete_party` (`i_party.adoc`): logically delete a party (a new
    /// `523|deleted|` version — RM common master06 §Logical Deletion; post
    /// `not has_party` holds, a deleted party reads `Null`). `expected` is the
    /// caller-supplied trunk version (from `If-Match` or the path
    /// `OBJECT_VERSION_ID`); when `Some`, a mismatch with the current version →
    /// `409`; when `None`, the current version is deleted unconditionally (SM
    /// `delete_party` has no version argument). An already-deleted target → `400`.
    pub(crate) async fn delete_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_party_version(kind, vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "{} {vo_id} is already deleted",
                kind.rm_type()
            )));
        }
        if let Some(expected) = expected
            && read.tree != expected
        {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                read.tree
            )));
        }

        let audit = self.demographic_audit(change_type::DELETED, "PARTY delete");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let committed = delete(
            &mut tx,
            None,
            vo_id,
            kind_of(kind),
            Some(read.tree),
            &audit,
            &ctx,
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        Ok(ServiceResponse::deleted(ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &committed.creating_system_id, committed.tree),
        )))
    }

    /// The current party version metadata (the latest `version_uid` a `412`
    /// echoes in `ETag`/`Location`), or `None` if unknown/wrong-kind.
    pub(crate) async fn party_current_meta(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        match self.load_party_version(kind, vo_id, None, None).await {
            Ok(read) => Ok(Some(ResourceMeta::new(
                String::new(),
                object_version_id(vo_id, &read.creating_system_id, read.tree),
            ))),
            Err(ServiceError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
