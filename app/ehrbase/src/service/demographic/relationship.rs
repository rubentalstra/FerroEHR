//! `I_PARTY_RELATIONSHIP` (`i_party_relationship.adoc`) + the
//! `I_DEMOGRAPHIC_SERVICE.create_party_relationship` factory
//! (`i_demographic_service.adoc`) — the demographic `PARTY_RELATIONSHIP` domain
//! logic, built on the shared [`crate::versioning`] machinery with
//! `ehr_id = None` (no EHR scope — our own design). A relationship is a
//! versioned object in the demographics repository, but it is *not* a PARTY (it
//! has its own `versioned_party_relationship` read surface).
//!
//! PORT NOTEs on the SM spec asymmetries this module normalizes to the PARTY
//! pattern (register `docs/design/platform/04-service-demographic-ehr-index.md`):
//! - `i_party_relationship.adoc` gives **no** `has_party_relationship`
//!   precondition on `get_party_relationship`, yet lists a
//!   `versioned_object_does_not_exist` error — we treat an unknown id as `404`,
//!   the same has-check the PARTY get performs, so the two demographic families
//!   behave identically.
//! - `update_party_relationship` retains the SM's `definitions_valid`
//!   precondition (structural validity of the new version) rather than the
//!   PARTY's `valid_content`; both reduce to the same structural check here
//!   ([`typed_check`]), so the normalization is behaviour-preserving.

use crate::service::response::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    CommitEnv, Kind, TreeId, VersionRead, audit_details, change_type, create, delete,
    object_version_id, original_version, read_current, read_version, update, version_at,
};

use super::{inject_uid, validate_party_ref};

/// The RM `_type` a `PARTY_RELATIONSHIP` versioned object stores.
const RM_TYPE: &str = "PARTY_RELATIONSHIP";

/// Structurally validate a candidate `PARTY_RELATIONSHIP` body: deserialize into
/// the `openehr_rm` type (a type mismatch → `422`), enforce that both `source`
/// and `target` `PARTY_REF`s are present continuant refs, and enforce their
/// `PARTY_REF.Type_validity` (G-17). `uid` need not be supplied — the server
/// injects it on read, mirroring the PARTY / COMPOSITION services.
fn typed_check(data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::PartyRelationship;
    // `source`/`target` are mandatory `PARTY_REF`s on the RM type, so a missing
    // one already fails deserialization; the explicit checks below give a
    // relationship-specific `422` message (and guard against a future optionality
    // change in the generated type).
    serde_json::from_value::<PartyRelationship>(data.clone()).map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as PARTY_RELATIONSHIP: {e}"))
    })?;
    for field in ["source", "target"] {
        let Some(reference) = data.get(field).filter(|v| !v.is_null()) else {
            return Err(ServiceError::Unprocessable(format!(
                "PARTY_RELATIONSHIP requires a {field} PARTY_REF"
            )));
        };
        // The refs denote the Version CONTAINER of a Party — an OBJECT_REF
        // carrying a HIER_OBJECT_ID (the continuant), never an
        // OBJECT_VERSION_ID (one particular version) — RM demographic
        // master02 §Modelling of Parties and Relationships.
        if reference
            .pointer("/id/_type")
            .and_then(Value::as_str)
            .is_some_and(|t| t == "OBJECT_VERSION_ID")
        {
            return Err(ServiceError::Unprocessable(format!(
                "PARTY_RELATIONSHIP.{field}.id must identify the party's version \
                 container (HIER_OBJECT_ID), not one version (OBJECT_VERSION_ID) \
                 — RM demographic master02"
            )));
        }
        // G-17: PARTY_REF.Type_validity + OBJECT_REF.namespace (BASE
        // `party_ref.adoc` / `object_ref.adoc`).
        validate_party_ref(reference, &format!("PARTY_RELATIONSHIP.{field}"))?;
    }
    Ok(())
}

/// Validate a relationship body for a direct create/update: its root `_type`
/// must be `PARTY_RELATIONSHIP` (mismatch → `422`), then [`typed_check`].
fn validate_relationship_body(body: &Value) -> Result<(), ServiceError> {
    let declared = body.get("_type").and_then(Value::as_str);
    if declared != Some(RM_TYPE) {
        return Err(ServiceError::Unprocessable(format!(
            "party_relationship _type mismatch: requires {RM_TYPE:?}, got {:?}",
            declared.unwrap_or("<none>"),
        )));
    }
    typed_check(body)
}

/// Validate a relationship version reached through the CONTRIBUTION path (the
/// [`Kind`] was already derived from the payload `_type`, so only the structural
/// check remains). `EhrbaseService::validate_for_commit` dispatches a
/// [`Kind::PartyRelationship`] here.
pub(crate) fn validate_relationship_for_commit(data: &Value) -> Result<(), ServiceError> {
    typed_check(data)
}

impl EhrbaseService {
    // ── PARTY_RELATIONSHIP CRUD ──────────────────────────────────────────────

    /// `create_party_relationship` (`i_demographic_service.adoc`): create the
    /// first version of a new `PARTY_RELATIONSHIP` (server-side
    /// `VERSIONED_OBJECT` + `ORIGINAL_VERSION` + `CONTRIBUTION`). Returns it with
    /// its `uid` set and the create-response `ETag`/`Location` metadata.
    pub(crate) async fn create_relationship(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        validate_relationship_body(&body)?;

        let audit = self.demographic_audit(change_type::CREATION, "PARTY_RELATIONSHIP creation");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let canonical = body.clone();
        let committed = create(
            &mut tx,
            None,
            Kind::PartyRelationship,
            body,
            None,
            &audit,
            crate::versioning::change::WriteEnvelope::default(),
            &ctx,
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        // The write response is built from the committed identity + the body
        // already in hand — no post-write reassembly read-back (the same
        // metadata discipline as every other write path).
        Ok(Self::party_committed_response(canonical, &committed))
    }

    /// `get_party_relationship` / `get_party_relationship_at_time`
    /// (`i_party_relationship.adoc`): retrieve a relationship by its
    /// versioned-object id, optionally at a specific version or instant (else
    /// the latest). A deleted current version resolves to `Null` (→ `204`); an
    /// unknown or wrong-kind id is `404`.
    pub(crate) async fn read_relationship(
        &self,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_relationship_version(vo_id, version, at).await?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(Self::relationship_version_response(vo_id, read))
    }

    /// `update_party_relationship` (`i_party_relationship.adoc`): commit a new
    /// version. `expected` (from `If-Match`) enforces optimistic concurrency (a
    /// stale precondition → `412`). Pre `has_party_relationship` is realized by
    /// [`ensure_relationship`](EhrbaseService::ensure_relationship).
    pub(crate) async fn update_relationship(
        &self,
        vo_id: Uuid,
        body: Value,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_relationship(vo_id).await?;
        validate_relationship_body(&body)?;

        let audit = self.demographic_audit(change_type::MODIFICATION, "PARTY_RELATIONSHIP update");
        let ctx = CommitEnv::signing_ctx(self);
        let canonical = body.clone();
        let mut tx = self.pool.begin().await?;
        let committed = update(
            &mut tx,
            None,
            vo_id,
            Kind::PartyRelationship,
            body,
            expected,
            None,
            &audit,
            crate::versioning::change::WriteEnvelope::default(),
            &ctx,
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        // Metadata + in-hand body — no post-write reassembly read-back.
        Ok(Self::party_committed_response(canonical, &committed))
    }

    /// `delete_party_relationship` (`i_party_relationship.adoc`): logically
    /// delete (a `523|deleted|` version — RM common master06 §Logical Deletion).
    /// `expected` is the trunk version from the mandatory `OBJECT_VERSION_ID`; a
    /// stale value → `409`, an already-deleted target → `400`.
    pub(crate) async fn delete_relationship(
        &self,
        vo_id: Uuid,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_relationship_version(vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "PARTY_RELATIONSHIP {vo_id} is already deleted"
            )));
        }
        // `None` deletes the current version unconditionally (no precondition
        // supplied — ITS-REST overview §Concurrency control), mirroring the
        // party delete.
        if let Some(expected) = expected
            && read.tree != expected
        {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                read.tree
            )));
        }
        let expected = expected.unwrap_or(read.tree);

        let audit = self.demographic_audit(change_type::DELETED, "PARTY_RELATIONSHIP delete");
        let ctx = CommitEnv::signing_ctx(self);
        let mut tx = self.pool.begin().await?;
        let committed = delete(
            &mut tx,
            None,
            vo_id,
            Kind::PartyRelationship,
            Some(expected),
            &audit,
            crate::versioning::change::WriteEnvelope::default(),
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

    /// The current relationship version metadata (the latest `version_uid` a
    /// `412` echoes in `ETag`/`Location`), or `None` if unknown/wrong-kind.
    pub(crate) async fn relationship_current_meta(
        &self,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        match self.load_relationship_version(vo_id, None, None).await {
            Ok(read) => Ok(Some(ResourceMeta::new(
                String::new(),
                object_version_id(vo_id, &read.creating_system_id, read.tree),
            ))),
            Err(ServiceError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // ── VERSIONED_PARTY_RELATIONSHIP (extension read surface) ─────────────────

    /// The `VERSIONED_OBJECT` wrapper for a relationship. A non-relationship id
    /// is `404`.
    ///
    /// No ITS-REST demographic contract governs this — our own extension by
    /// analogy with the EHR group. The version-spine read goes through
    /// `crate::storage::version_repo` (storage owns the SQL — no openEHR spec
    /// governs the storage read, our own design).
    pub(crate) async fn versioned_relationship(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let time_created = crate::storage::version_repo::time_created(&self.pool, vo_id)
            .await?
            .ok_or_else(|| {
                ServiceError::NotFound(format!("versioned party relationship {vo_id}"))
            })?;
        // PORT NOTE (G-6): `VERSIONED_OBJECT.owner_id` (1..1) has no EHR owner for
        // a demographic relationship (no openEHR spec governs the owner of an
        // ehr-less demographic versioned object — our own design); we reference
        // the relationship's own versioned-object id (the demographics repository
        // owns it).
        Ok(json!({
            "_type": "VERSIONED_OBJECT",
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": "PARTY_RELATIONSHIP",
                "id": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        }))
    }

    /// The `REVISION_HISTORY` of a relationship: one item per version with its
    /// `OBJECT_VERSION_ID` and commit `AUDIT_DETAILS` (RM common master04
    /// §Revision History). A non-relationship id is `404`.
    ///
    /// As [`versioned_relationship`](Self::versioned_relationship), the ehr-less
    /// version-spine read goes through `crate::storage::version_repo`.
    pub(crate) async fn relationship_revision_history(
        &self,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let metas = crate::storage::version_repo::all_version_meta(&self.pool, vo_id).await?;

        let mut items = Vec::with_capacity(metas.len());
        for meta in &metas {
            let tree =
                TreeId::from_columns(meta.trunk_version, meta.branch_number, meta.branch_version);
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": object_version_id(vo_id, &meta.creating_system_id, tree)
                },
                "audits": [audit_details(
                    &meta.audit_system_id,
                    &meta.audit_change_type,
                    meta.audit_description.as_deref(),
                    &meta.audit_committer,
                    &meta.time_committed,
                )]
            }));
        }
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// `get_party_relationship_at_version` (`i_party_relationship.adoc`): the
    /// `ORIGINAL_VERSION` at a specific version. A non-relationship id is `404`.
    pub(crate) async fn relationship_version(
        &self,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id.is_none())
            .ok_or_else(|| {
                ServiceError::NotFound(format!("party relationship {vo_id} v{version}"))
            })?;
        let signer = CommitEnv::signing_ctx(self).signer;
        original_version(&read, signer)
    }

    /// The `ORIGINAL_VERSION` extant at `at` (or the latest when `None`), with
    /// `ETag`/`Location` metadata for the VERSION resource.
    pub(crate) async fn relationship_version_at_time(
        &self,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| {
            ServiceError::NotFound(format!("party relationship {vo_id} version at time"))
        })?;
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        let signer = CommitEnv::signing_ctx(self).signer;
        let ov = original_version(&read, signer)?;
        Ok(ServiceResponse::new(ov, meta))
    }

    // ── shared helpers ───────────────────────────────────────────────────────

    /// Load a version of a relationship, verifying its kind and ehr-less-ness. A
    /// wrong-kind or unknown id is `404`.
    async fn load_relationship_version(
        &self,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<VersionRead, ServiceError> {
        if crate::versioning::object_kind(&self.pool, vo_id).await? != Some(Kind::PartyRelationship)
        {
            return Err(ServiceError::NotFound(format!(
                "PARTY_RELATIONSHIP {vo_id}"
            )));
        }
        let read = match (version, at) {
            (Some(v), _) => read_version(&self.pool, vo_id, v).await?,
            (None, Some(at)) => version_at(&self.pool, vo_id, at).await?,
            (None, None) => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("PARTY_RELATIONSHIP {vo_id}")))?;
        Ok(read)
    }

    /// Realize the SM `has_party_relationship` precondition: a live (not deleted)
    /// relationship exists.
    async fn ensure_relationship(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        let read = self.load_relationship_version(vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "PARTY_RELATIONSHIP {vo_id} is deleted"
            )));
        }
        Ok(())
    }

    /// Confirm `vo_id` is a relationship (any version) — the check for the
    /// `versioned_party_relationship` reads. A non-relationship id is `404`.
    async fn ensure_any_relationship(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        match crate::versioning::object_kind(&self.pool, vo_id).await? {
            Some(Kind::PartyRelationship) => Ok(()),
            _ => Err(ServiceError::NotFound(format!(
                "versioned party relationship {vo_id}"
            ))),
        }
    }

    /// A [`ServiceResponse`] for a loaded relationship: its canonical body with
    /// the `uid` injected, plus the resource metadata (empty `ehr_id`).
    fn relationship_version_response(vo_id: Uuid, read: VersionRead) -> ServiceResponse {
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        ServiceResponse::new(
            inject_uid(read.canonical, vo_id, &read.creating_system_id, read.tree),
            meta,
        )
    }
}
