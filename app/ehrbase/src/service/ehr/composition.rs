//! `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`) — COMPOSITION commit/retrieve
//! with implicit CONTRIBUTION creation. The commit-validation choke point and
//! the `VERSIONED_COMPOSITION` cross-version invariant hook live in the sibling
//! [`composition_validate`](super::composition_validate).
//!
//! Spec: RM ehr `versioned_composition.adoc`, RM composition
//! (`COMPOSITION.category` / `is_persistent`), ITS-REST
//! `responses/422_COMPOSITION.yaml` (a well-formed body that fails template/RM
//! validation is 422, not 400). Versioned-object mechanics are RM common
//! master06, delegated to [`crate::versioning`].

use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::SmError;
use crate::service::version_update::UpdateVersion;
use openehr_base::prelude::ObjectVersionId;
use serde_json::Value;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    Kind, TreeId, change_type, components, create, delete, original_version, parse_tree_id,
    read_current, read_version, revision_history, update, version_at, versioned_object,
};

use super::composition_validate::{
    check_versioned_composition_invariants, composition_template_id,
};

/// The caller's `UPDATE_VERSION` envelope resolved for a direct commit: the
/// commit audit (caller attributes merged with the server rules — ITS-REST
/// overview §"openehr-version and openehr-audit-details" MUST) and the write
/// envelope (lifecycle / verbatim signature / attestations).
fn resolve_envelope(
    version: &crate::service::version_update::UpdateVersion,
    operation_change_type: &str,
    default_description: &str,
    system_id: &str,
) -> (crate::versioning::AuditInput, crate::versioning::change::WriteEnvelope) {
    let audit = crate::versioning::AuditInput::from_update(
        &version.audit,
        operation_change_type,
        default_description,
        system_id,
    );
    let attestations = version
        .attestations
        .as_ref()
        .map(|a| {
            a.iter()
                .filter_map(|x| serde_json::to_value(x).ok())
                .collect()
        })
        .unwrap_or_default();
    let envelope = crate::versioning::change::WriteEnvelope {
        lifecycle_state: Some(version.lifecycle_state.code_string.clone()),
        signature: version.signature.clone(),
        attestations,
    };
    (audit, envelope)
}

impl EhrbaseService {
    /// Create a COMPOSITION in an EHR from the caller's full `UPDATE_VERSION`
    /// envelope, returning the version metadata (the `ETag`/`Location` for
    /// `201_COMPOSITION`). The envelope's audit attributes, lifecycle state,
    /// verbatim signature and attestations are honoured on the persisted
    /// commit (ITS-REST committal-header merge — MUST).
    /// `create_composition` (SM `i_ehr_composition.adoc`): commit the first
    /// version of a COMPOSITION in `ehr_id` from the caller's full
    /// `UPDATE_VERSION` envelope, returning the committed version identity
    /// ([`Committed`] — the `ETag`/`Location`/`Last-Modified` source).
    pub async fn create_composition(
        &self,
        ehr_id: Uuid,
        version: crate::service::version_update::UpdateVersion,
    ) -> Result<crate::versioning::Committed, ServiceError> {
        let (audit, envelope) = resolve_envelope(
            &version,
            change_type::CREATION,
            "COMPOSITION creation",
            &self.effective_system_id(),
        );
        // 553|incomplete| relaxes validation strictness (master06 §Version
        // Lifecycle; blueprint incomplete-lifecycle rule).
        let incomplete = version.lifecycle_state.code_string == "553";
        let composition = version.data;
        // The EHR-existence (404) and content-writability (409) gates in one
        // round trip: a COMPOSITION is EHR content (RM ehr master04 §EHR Creation
        // / §EHR Active Status). Same errors, same order as the separate
        // `ensure_ehr_exists` + `ensure_content_writable` checks.
        self.ensure_ehr_content_writable(ehr_id).await?;
        self.validate_composition_for_commit(&composition, incomplete)
            .await?;
        self.reject_duplicate_persistent(ehr_id, &composition)
            .await?;

        let mut tx = self.pool.begin().await?;
        let committed = create(
            &mut tx,
            Some(ehr_id),
            Kind::Composition,
            composition,
            None,
            &audit,
            envelope,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        // The write result is the committed version identity itself — a
        // representation response re-reads at the protocol layer.
        Ok(committed)
    }

    /// Retrieve a COMPOSITION by its versioned-object id, optionally at a specific
    /// version (else the latest). A deleted version resolves to `Value::Null` (→
    /// `204`, F-02-01) — never 404 or 500.
    async fn read_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match version {
            Some(v) => read_version(&self.pool, vo_id, v).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;

        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// A COMPOSITION as it was at an instant (time-travel), with its `uid` set. A
    /// deleted version resolves to an empty body (→ `204`).
    async fn composition_at_time(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        at: jiff::Timestamp,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = version_at(&self.pool, vo_id, at)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `VERSIONED_OBJECT` for a COMPOSITION (verifies EHR ownership).
    async fn versioned_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let _read = read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        versioned_object(&self.pool, vo_id, ehr_id, "VERSIONED_COMPOSITION").await
    }

    /// The `REVISION_HISTORY` of a COMPOSITION.
    async fn composition_revision_history_value(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        revision_history(&self.pool, ehr_id, vo_id).await
    }

    /// An `ORIGINAL_VERSION` of a COMPOSITION at a specific version.
    async fn composition_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} v{version}")))?;
        original_version(&read, self.signer())
    }

    /// The `ORIGINAL_VERSION` of a COMPOSITION extant at `at`, or the latest when
    /// `at` is `None` (`GET …/versioned_composition/{uid}/version`, F-02-04). A
    /// deleted version still returns `200` with the deleted-lifecycle
    /// `ORIGINAL_VERSION` (no `data`).
    async fn composition_version_at_time_read(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} version at time")))?;
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        let ov = original_version(&read, self.signer())?;
        Ok(ServiceResponse::new(ov, meta))
    }

    /// Commit a new version of a COMPOSITION. `expected` (from `If-Match`)
    /// enforces optimistic concurrency. `current` is the trait layer's ONE
    /// merged pre-read (`current_composition_meta`): the ownership gate is
    /// already applied by the caller, and it carries the lifecycle, the stored
    /// template root fragment, and the EHR's `is_modifiable` flag — so this write
    /// runs no further pre-read (the former `If-Match` meta read, modify
    /// pre-read, and `is_modifiable` side-SELECT are one statement now).
    /// `update_composition` (SM `i_ehr_composition.adoc`): commit a new version
    /// of `vo_id` from the caller's full `UPDATE_VERSION` envelope. ONE merged
    /// pre-read carries the whole write pre-check: the owning EHR (ownership →
    /// 404), the full-`OBJECT_VERSION_ID` `If-Match` identity (412), the
    /// lifecycle (deleted → 404), the stored template root fragment (422) and
    /// the EHR's `is_modifiable` flag (409).
    pub async fn update_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: crate::service::version_update::UpdateVersion,
    ) -> Result<crate::versioning::Committed, ServiceError> {
        let Some(current) =
            crate::storage::version_repo::current_composition_meta(&self.pool, vo_id)
                .await?
                .filter(|m| m.ehr_id == Some(ehr_id))
        else {
            return Err(ServiceError::NotFound(format!("COMPOSITION {vo_id}")));
        };
        // The full-`OBJECT_VERSION_ID` `If-Match` compare (F-02-08), built from
        // the same merged read (ITS-REST overview §Concurrency control).
        let tree = TreeId::from_columns(
            current.trunk_version,
            current.branch_number,
            current.branch_version,
        );
        let latest = self.version_meta(
            ehr_id,
            vo_id,
            &current.creating_system_id,
            tree,
            current.time_committed,
        );
        super::ensure_if_match(version.preceding_version_uid.as_ref(), Some(&latest))?;
        let expected = version
            .preceding_version_uid
            .as_ref()
            .map(|o| components(o).map(|(_, v)| v))
            .transpose()?;
        let (audit, envelope) = resolve_envelope(
            &version,
            change_type::MODIFICATION,
            "COMPOSITION update",
            &self.effective_system_id(),
        );
        let incomplete = version.lifecycle_state.code_string == "553";
        let composition = version.data;
        // The lifecycle (deleted → 404, RM common master06 §Logical Deletion)
        // and the content-write guard are checked from the threaded pre-read.
        if current.lifecycle_state == crate::versioning::lifecycle::state::DELETED {
            return Err(ServiceError::NotFound(format!(
                "COMPOSITION {vo_id} is deleted"
            )));
        }
        // is_modifiable = False forbids content writes (RM ehr master04 §EHR
        // Active Status) — folded from the standalone `ensure_content_writable`
        // side-SELECT into the merged pre-read; the 409 outcome and its ordering
        // (after the deleted 404, before the template 422) are unchanged.
        if !current.is_modifiable {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        // Reject an update whose body declares a *different* template than the
        // stored composition it supersedes (CNF master07
        // `update_composition-wrong_template`) — a semantic 422, not 400/412.
        let stored_template = current
            .root_data
            .as_ref()
            .and_then(|d| composition_template_id(d));
        if let (Some(stored), Some(incoming)) =
            (stored_template, composition_template_id(&composition))
            && stored != incoming
        {
            return Err(ServiceError::Unprocessable(format!(
                "update COMPOSITION references template {incoming}, but the stored \
                 composition was committed against template {stored} (template_id mismatch)"
            )));
        }
        self.validate_composition_for_commit(&composition, incomplete)
            .await?;

        let mut tx = self.pool.begin().await?;
        // VERSIONED_COMPOSITION cross-version invariants (RM ehr
        // `versioned_composition.adoc`), lifted out of the versioning write path
        // (G-13) — checked in the same transaction as the commit.
        check_versioned_composition_invariants(&mut tx, vo_id, &composition).await?;
        let committed = update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Composition,
            composition,
            expected,
            None,
            &audit,
            envelope,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        Ok(committed)
    }

    /// The current COMPOSITION version metadata (the latest `version_uid` a
    /// `409`/`412` must echo), or `None` if unknown/deleted.
    async fn composition_current_meta(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        // Lean `vo_version`⋈`audit` read scoped to the EHR: the `ETag`/`If-Match`
        // compare needs only the full `OBJECT_VERSION_ID` + commit instant (RM
        // common master06 §Version Identification / §Committal), never the
        // reassembled document the full `read_current` pays.
        let Some(m) =
            crate::storage::version_repo::current_version_meta_scoped(&self.pool, vo_id, ehr_id)
                .await?
        else {
            return Ok(None);
        };
        let tree = TreeId::from_columns(m.trunk_version, m.branch_number, m.branch_version);
        Ok(Some(self.version_meta(
            ehr_id,
            vo_id,
            &m.creating_system_id,
            tree,
            m.time_committed,
        )))
    }

    /// The OPT `template_id` a COMPOSITION version was committed against, read
    /// back from the version. `version` = the `VERSION_TREE_ID` lexical form
    /// (`N` or `N.B.V`); `None` = the current version. The ABAC template
    /// attribute for the access pre-checks / any per-version resolver.
    ///
    /// PERF(port): goes through the full version read-back for spec fidelity; a
    /// direct `SELECT template_id FROM vo_version` is a cheaper equivalent if
    /// this ever shows on a hot path.
    ///
    /// # Errors
    /// [`ServiceError`] if the version read-back fails.
    pub async fn template_of_version(
        &self,
        vo_id: Uuid,
        version: Option<&str>,
    ) -> Result<Option<String>, ServiceError> {
        let read = match version {
            Some(v) => {
                let tree = parse_tree_id(v)?;
                read_version(&self.pool, vo_id, tree).await?
            }
            None => read_current(&self.pool, vo_id).await?,
        };
        Ok(read.and_then(|r| r.template_id))
    }

    /// Logically delete a COMPOSITION (a new `523|deleted|` version). `expected`
    /// is the version tree id carried by the mandatory `preceding_version_uid`
    /// (`composition_delete.yaml`). A stale precondition → `409`; an
    /// already-deleted target → `400` (F-02-05).
    /// `delete_composition` (SM `i_ehr_composition.adoc`): commit a
    /// `523|deleted|` version of the addressed COMPOSITION (RM common master06
    /// §Logical Deletion). PORT NOTE (G-7): takes the full
    /// `OBJECT_VERSION_ID`, stronger than the SM's `UUID` — the SM is
    /// internally inconsistent (`has_composition` takes OBJECT_VERSION_ID).
    pub async fn delete_composition(
        &self,
        ehr_id: Uuid,
        a_version_uid: &ObjectVersionId,
    ) -> Result<crate::versioning::Committed, ServiceError> {
        let (vo_id, expected) = components(a_version_uid)?;
        // Lean delete pre-read: the pre-checks need only the owning EHR, the
        // lifecycle (already-deleted → 400, F-02-05), and the current
        // `VERSION_TREE_ID` (the `preceding_version_uid` conflict compare) — not
        // a full node reassembly (the deleted version stores no nodes anyway).
        let current = crate::storage::version_repo::current_composition_meta(&self.pool, vo_id)
            .await?
            .filter(|m| m.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if current.lifecycle_state == crate::versioning::lifecycle::state::DELETED {
            return Err(ServiceError::BadRequest(format!(
                "COMPOSITION {vo_id} is already deleted"
            )));
        }
        // is_modifiable = False forbids content writes (RM ehr master04 §EHR
        // Active Status) — folded from the standalone `ensure_content_writable`
        // side-SELECT into the pre-read; the 409 outcome and its ordering (after
        // the already-deleted 400, before the stale-precondition 409) unchanged.
        if !current.is_modifiable {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        let current_tree = TreeId::from_columns(
            current.trunk_version,
            current.branch_number,
            current.branch_version,
        );
        if current_tree != expected {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {current_tree}"
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "COMPOSITION delete");
        let committed = delete(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Composition,
            Some(expected),
            &audit,
            crate::versioning::change::WriteEnvelope::default(),
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);
        // 204_COMPOSITION_deleted: the (now deleted) version identity.
        Ok(committed)
    }

    /// The EHR-existence precheck (SM `ehr_does_not_exist` → `NotFound`); also the
    /// [`crate::versioning::CommitEnv`] `ensure_ehr_exists` hook (G-6). The
    /// existence read is a storage seam
    /// ([`crate::storage::version_repo::ehr_exists`]; no openEHR spec governs the
    /// SQL — our own design).
    async fn ensure_ehr_exists(
        &self,
        ehr_id: Uuid,
    ) -> Result<(), ServiceError> {
        if crate::storage::version_repo::ehr_exists(&self.pool, ehr_id).await? {
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("EHR {ehr_id}")))
        }
    }

    /// The combined EHR-existence + content-writability content-write gate in
    /// ONE round trip — equivalent to [`Self::ensure_ehr_exists`] followed by
    /// [`Self::ensure_content_writable`] (a missing EHR → 404 *before* the
    /// non-modifiable 409, unchanged order), but a single
    /// [`crate::storage::ehr_repo::ehr_writability`] read instead of two pool
    /// round trips. The guarded concepts are RM ehr master04 §EHR Creation
    /// (existence) and §EHR Active Status (`EHR_STATUS.is_modifiable`); no
    /// openEHR spec governs the query shape (our own design).
    async fn ensure_ehr_content_writable(
        &self,
        ehr_id: Uuid,
    ) -> Result<(), ServiceError> {
        let (exists, is_modifiable) =
            crate::storage::ehr_repo::ehr_writability(&self.pool, ehr_id).await?;
        if !exists {
            return Err(ServiceError::NotFound(format!("EHR {ehr_id}")));
        }
        // `None` (no current EHR_STATUS) is treated as modifiable, so the guard
        // never spuriously blocks — identical to `ensure_content_writable`.
        if is_modifiable == Some(false) {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        Ok(())
    }
}

impl EhrbaseService {
    pub async fn has_composition(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        match self.read_composition(an_ehr_id, vo_id, Some(version)).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get_composition_latest(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .read_composition(an_ehr_id, a_versioned_object_uid, None)
            .await?
            .body)
    }

    pub async fn get_composition_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        match a_time.as_deref() {
            None => Ok(self
                .read_composition(an_ehr_id, a_versioned_object_uid, None)
                .await?
                .body),
            Some(raw) => Ok(self
                .composition_at_time(
                    an_ehr_id,
                    a_versioned_object_uid,
                    super::parse_at_time(raw)?,
                )
                .await?
                .body),
        }
    }

    pub async fn get_composition_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        Ok(self
            .read_composition(an_ehr_id, vo_id, Some(version))
            .await?
            .body)
    }

    pub async fn get_versioned_composition(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .versioned_composition(an_ehr_id, a_versioned_object_uid)
            .await?)
    }




    pub async fn composition_revision_history(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .composition_revision_history_value(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    pub async fn composition_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(super::parse_at_time).transpose()?;
        Ok(self
            .composition_version_at_time_read(an_ehr_id, a_versioned_object_uid, at)
            .await?
            .body)
    }

    pub async fn composition_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        Ok(self.composition_version(an_ehr_id, vo_id, version).await?)
    }
}

// ── ITS-REST MultimediaAdapter (adapter-support extension) ────────────────────

impl EhrbaseService {
    pub async fn expand_multimedia(&self, body: Value) -> Result<Value, SmError> {
        // Off by default: no engine ⇒ serve the stored form unchanged.
        let Some(engine) = &self.multimedia else {
            return Ok(body);
        };
        let mut body = body;
        engine
            .expand(&mut body)
            .await
            .map_err(|e| SmError::exception(e.to_string()))?;
        Ok(body)
    }
}
