// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`) — COMPOSITION
//! commit/retrieve with implicit CONTRIBUTION creation. The commit-validation
//! choke point and the `VERSIONED_COMPOSITION` cross-version invariant hook
//! live in the sibling [`validation`](super::validation) module.
//!
//! Spec: RM ehr `versioned_composition.adoc`, RM composition
//! (`COMPOSITION.category` / `is_persistent`), ITS-REST
//! `responses/422_COMPOSITION.yaml` (a well-formed body that fails template/RM
//! validation is 422, not 400). Versioned-object mechanics are RM common
//! master06, delegated to [`crate::versioning`].

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the commit interior carries the canonical \
              fragment the seam produced once; stored-content serving"
)]

use openehr_base::prelude::ObjectVersionId;
use openehr_base::v1_3::base_types::identification::lexical::composite_ids_equal;
use serde_json::Value;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::datetime::parse_at_time;
#[cfg(feature = "multimedia")]
use crate::service::error::internal_fault;
use crate::service::error::{ServiceError, Violation};
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::{CallStatusType, SmError};
use crate::versioning::Kind;
use crate::versioning::audit::change_type;
use crate::versioning::change::{create, delete, update};
use crate::versioning::object_version_id::{TreeId, components, parse_tree_id};
use crate::versioning::read::{read_current, read_version, version_at};
use crate::versioning::wire::{revision_history, version_envelope, versioned_object};
use openehr_its::rest::generated::common::UpdateVersion;
use openehr_rm::prelude::Composition;

use super::resolve_envelope;
use super::validation::composition_template_id;

impl FerroEhrService {
    /// `create_composition` (SM `i_ehr_composition.adoc`): commit the first
    /// version of a COMPOSITION in `ehr_id` from the caller's full
    /// `UPDATE_VERSION` envelope, returning the committed version identity
    /// ([`Committed`](crate::versioning::change::Committed) — the
    /// `ETag`/`Location`/`Last-Modified` source). The envelope's audit
    /// attributes, lifecycle state, verbatim signature and attestations are
    /// honoured on the persisted commit (ITS-REST committal-header merge —
    /// MUST).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR does not exist;
    /// [`ServiceError::Conflict`] when the EHR is not modifiable or a live
    /// persistent COMPOSITION for the same template already exists;
    /// [`ServiceError::ValidationFailed`] when RM/terminology/template
    /// validation fails (→ 422); [`ServiceError::Database`] on a storage
    /// failure.
    pub async fn create_composition(
        &self,
        ehr_id: EhrId,
        version: UpdateVersion<Composition>,
    ) -> Result<crate::versioning::change::Committed, ServiceError> {
        // The ONE serialization boundary of this commit, taken before any
        // await so the typed RM value does not ride the whole write
        // transaction (`super::canonicalize`).
        let version = super::canonicalize(version);
        // 553|incomplete| relaxes validation strictness (master06 §Version
        // Lifecycle).
        let super::CommitParts {
            audit,
            envelope,
            incomplete,
            canonical: composition,
        } = resolve_envelope(
            version,
            change_type::CREATION,
            "COMPOSITION creation",
            &self.effective_system_id(),
        )?;
        // The EHR-existence (404) and content-writability (409) gates in one
        // round trip: a COMPOSITION is EHR content (RM ehr master04 §EHR
        // Creation / §EHR Active Status). Same errors, same order as the
        // separate `ensure_ehr_exists` + `ensure_content_writable` checks.
        self.ensure_ehr_content_writable(ehr_id).await?;
        self.validate_composition_for_commit(&composition, incomplete)
            .await?;
        self.reject_duplicate_persistent(ehr_id, &composition)
            .await?;

        // The committed template identity is promoted to `vo_version.template_id`
        // — the ABAC template attribute resolver (`template_of_version`) and the
        // template-delete guard both read that column, so the direct route
        // stamps it exactly like the CONTRIBUTION route.
        let template_id = composition_template_id(&composition).map(str::to_owned);

        let mut tx = self.pool.begin().await?;
        // Boxed: the versioned-write future is by far the widest state this
        // call holds, and inlining it puts the whole node-decomposition
        // machinery on every caller's stack (clippy `large_futures`).
        let committed = Box::pin(create(
            &mut tx,
            Some(ehr_id),
            Kind::Composition,
            composition,
            template_id.as_deref(),
            &audit,
            envelope,
            &self.signing_ctx(),
        ))
        .await?;
        tx.commit().await?;
        crate::telemetry::metrics::metrics()
            .db_transactions
            .add(1, &[opentelemetry::KeyValue::new("outcome", "commit")]);
        crate::versioning::change::meter_committed(&committed);

        // The write result is the committed version identity itself — a
        // representation response re-reads at the protocol layer.
        Ok(committed)
    }

    /// Retrieve a COMPOSITION by its versioned-object id, optionally at a
    /// specific version (else the latest). A deleted version resolves to
    /// `Value::Null` (→ `204`) — never 404 or 500.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the version does not exist or belongs
    /// to another EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn read_composition(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match version {
            Some(v) => read_version(&self.pool, self.spec_profile, vo_id, v).await?,
            None => read_current(&self.pool, self.spec_profile, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::CompositionDoesNotExist,
                format!("COMPOSITION {vo_id}"),
            )
        })?;

        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read)?)
    }

    /// A COMPOSITION as it was at an instant (time-travel), with its `uid`
    /// set. A deleted version resolves to an empty body (→ `204`).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when no version existed at `at` or the
    /// object belongs to another EHR; [`ServiceError::Database`] on a storage
    /// failure.
    pub(in crate::service) async fn composition_at_time(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        at: jiff::Timestamp,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = version_at(&self.pool, self.spec_profile, vo_id, at)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::CompositionDoesNotExist,
                    format!("COMPOSITION {vo_id}"),
                )
            })?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read)?)
    }

    /// The `VERSIONED_OBJECT` for a COMPOSITION (verifies EHR ownership).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the object does not exist or belongs to
    /// another EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn versioned_composition(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
    ) -> Result<ServiceResponse, ServiceError> {
        // Ownership gate only — one scalar read (`vo_version.ehr_id`), never
        // the full current-version reassembly this metadata-shaped response
        // would immediately discard.
        crate::storage::version_repo::meta::vo_owner(&self.pool, vo_id)
            .await?
            .filter(|owner| *owner == Some(ehr_id))
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::CompositionDoesNotExist,
                    format!("COMPOSITION {vo_id}"),
                )
            })?;
        let (body, last_modified) =
            versioned_object(&self.pool, vo_id, ehr_id, "VERSIONED_COMPOSITION").await?;
        Ok(ServiceResponse::new(
            body,
            super::meta::container_meta(ehr_id, vo_id, last_modified),
        ))
    }

    /// The `REVISION_HISTORY` of a COMPOSITION.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the object does not exist in this EHR;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn composition_revision_history_value(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
    ) -> Result<ServiceResponse, ServiceError> {
        let (body, last_modified) = revision_history(&self.pool, ehr_id, vo_id).await?;
        Ok(ServiceResponse::new(
            body,
            super::meta::container_meta(ehr_id, vo_id, last_modified),
        ))
    }

    /// The VERSION envelope of a COMPOSITION at a specific version — an
    /// `ORIGINAL_VERSION`, or an `IMPORTED_VERSION` when the version was
    /// received from another system (RM common master06 §Version and its
    /// Subtypes).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the version does not exist or belongs
    /// to another EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn composition_version(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        let read = read_version(&self.pool, self.spec_profile, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| {
                ServiceError::sm(
                    CallStatusType::ObjectVersionDoesNotExist,
                    format!("COMPOSITION {vo_id} v{version}"),
                )
            })?;
        version_envelope(&read, self.signer())
    }

    /// The VERSION envelope of a COMPOSITION extant at `at`, or the latest
    /// when `at` is `None` (`GET …/versioned_composition/{uid}/version`). A deleted version still returns `200` with the
    /// deleted-lifecycle envelope (no `data`).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when no version existed at `at` or the
    /// object belongs to another EHR; [`ServiceError::Database`] on a storage
    /// failure.
    pub(in crate::service) async fn composition_version_at_time_read(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match at {
            Some(at) => version_at(&self.pool, self.spec_profile, vo_id, at).await?,
            None => read_current(&self.pool, self.spec_profile, vo_id).await?,
        }
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::ObjectVersionDoesNotExist,
                format!("COMPOSITION {vo_id} version at time"),
            )
        })?;
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        let ov = version_envelope(&read, self.signer())?;
        Ok(ServiceResponse::new(ov, meta))
    }

    /// `update_composition` (SM `i_ehr_composition.adoc`): commit a new
    /// version of `vo_id` from the caller's full `UPDATE_VERSION` envelope,
    /// returning the committed version identity. ONE merged pre-read
    /// (`current_composition_meta`) carries the whole write pre-check: the
    /// owning EHR (ownership → 404), the full-`OBJECT_VERSION_ID` `If-Match`
    /// identity (412 — ITS-REST overview §Concurrency control), the
    /// lifecycle (deleted → 404), the stored template root fragment (422) and
    /// the EHR's `is_modifiable` flag (409) — the former `If-Match` meta read,
    /// modify pre-read, and `is_modifiable` side-SELECT are one statement.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the COMPOSITION does not exist in this
    /// EHR or is already deleted; [`ServiceError::VersionConflict`] on an
    /// `If-Match` mismatch (→ 412); [`ServiceError::Conflict`] when the EHR is
    /// not modifiable (→ 409); [`ServiceError::Unprocessable`] on a
    /// template-id mismatch with the stored composition or a
    /// `VERSIONED_COMPOSITION` invariant violation;
    /// [`ServiceError::ValidationFailed`] when RM/terminology/template
    /// validation fails (→ 422); [`ServiceError::Database`] on a storage
    /// failure.
    pub async fn update_composition(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: UpdateVersion<Composition>,
    ) -> Result<crate::versioning::change::Committed, ServiceError> {
        // The ONE serialization boundary of this commit, taken before any
        // await so the typed RM value does not ride the whole write
        // transaction (`super::canonicalize`).
        let version = super::canonicalize(version);
        let Some(current) =
            crate::storage::version_repo::meta::current_composition_meta(&self.pool, vo_id)
                .await?
                .filter(|m| m.ehr_id == Some(ehr_id))
        else {
            return Err(ServiceError::sm(
                CallStatusType::CompositionDoesNotExist,
                format!("COMPOSITION {vo_id}"),
            ));
        };
        // The full-`OBJECT_VERSION_ID` `If-Match` compare, built from
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
        let super::CommitParts {
            audit,
            envelope,
            incomplete,
            canonical: composition,
        } = resolve_envelope(
            version,
            change_type::MODIFICATION,
            "COMPOSITION update",
            &self.effective_system_id(),
        )?;
        // The lifecycle (deleted → 404, RM common master06 §Logical Deletion)
        // and the content-write guard are checked from the threaded pre-read.
        if current.lifecycle_state == crate::versioning::lifecycle::state::DELETED {
            return Err(ServiceError::sm(
                CallStatusType::CompositionDoesNotExist,
                format!("COMPOSITION {vo_id} is deleted"),
            ));
        }
        // is_modifiable = False forbids content writes (RM ehr master04 §EHR
        // Active Status) — folded from the standalone `ensure_content_writable`
        // side-SELECT into the merged pre-read; the 409 outcome and its
        // ordering (after the deleted 404, before the template 422) are
        // unchanged.
        if !current.is_modifiable {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        // Reject an update whose body declares a *different* template than the
        // stored composition it supersedes — a semantic 422. NOTE: no openEHR
        // spec governs template stability across versions (RM ehr
        // `versioned_composition.adoc` pins archetype_node_id and
        // is_persistent across versions but not
        // archetype_details.template_id) — our own design convention,
        // consistent with those container invariants.
        let stored_template = current
            .root_data
            .as_ref()
            .and_then(|d| composition_template_id(d));
        if let (Some(stored), Some(incoming)) =
            (stored_template, composition_template_id(&composition))
            && stored != incoming
        {
            return Err(ServiceError::content_invalid(
                Violation::new(format!(
                    "is {incoming} on the update, but the stored composition was \
                     committed against template {stored} (template_id mismatch)"
                ))
                .with_path("COMPOSITION.archetype_details.template_id"),
            ));
        }
        self.validate_composition_for_commit(&composition, incomplete)
            .await?;

        // Same template stamping as the create arm — every version row carries
        // the template it was committed against.
        let template_id = composition_template_id(&composition).map(str::to_owned);

        let mut tx = self.pool.begin().await?;
        // VERSIONED_COMPOSITION cross-version invariants (RM ehr
        // `versioned_composition.adoc`), lifted out of the versioning write
        // path — checked in the same transaction as the commit.
        super::validation::check_versioned_composition_invariants(&mut tx, vo_id, &composition)
            .await?;
        let committed = update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Composition,
            composition,
            expected,
            template_id.as_deref(),
            &audit,
            envelope,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        crate::telemetry::metrics::metrics()
            .db_transactions
            .add(1, &[opentelemetry::KeyValue::new("outcome", "commit")]);
        crate::versioning::change::meter_committed(&committed);

        Ok(committed)
    }

    /// The current COMPOSITION version metadata (the latest `version_uid` a
    /// `409`/`412` must echo), or `None` if unknown/deleted.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the metadata read fails.
    pub(in crate::service) async fn composition_current_meta(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        // Lean `vo_version`⋈`audit` read scoped to the EHR: the
        // `ETag`/`If-Match` compare needs only the full `OBJECT_VERSION_ID` +
        // commit instant (RM common master06 §Version Identification /
        // §Committal), never the reassembled document the full `read_current`
        // pays.
        let Some(m) = crate::storage::version_repo::meta::current_version_meta_scoped(
            &self.pool, vo_id, ehr_id,
        )
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
    /// NOTE (settled shape): this resolves through the promoted
    /// `vo_version.template_id` column — one scalar `SELECT`, no node
    /// reassembly — because it runs per authorization check. No openEHR spec
    /// governs the storage mechanics; the promoted column is our own design.
    ///
    /// # Errors
    /// [`ServiceError`] for a malformed `version` string or a failing version
    /// read-back.
    pub async fn template_of_version(
        &self,
        vo_id: VoId,
        version: Option<&str>,
    ) -> Result<Option<String>, ServiceError> {
        // One scalar read of the promoted `vo_version.template_id` column —
        // this resolver runs per authorization check, so it must never pay a
        // node reassembly.
        let tree = version.map(parse_tree_id).transpose()?;
        Ok(crate::storage::version_repo::meta::template_id_of(
            &self.pool,
            vo_id,
            tree.map(TreeId::columns),
        )
        .await?
        .flatten())
    }

    /// `delete_composition` (SM `i_ehr_composition.adoc`): commit a
    /// `523|deleted|` version of the addressed COMPOSITION (RM common master06
    /// §Logical Deletion), returning the (now deleted) version identity
    /// (`204_COMPOSITION_deleted`). NOTE: takes the full
    /// `OBJECT_VERSION_ID` — the mandatory `preceding_version_uid`
    /// (`composition_delete.yaml`) — stronger than the SM's `UUID`; the SM is
    /// internally inconsistent (`has_composition` takes `OBJECT_VERSION_ID`).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the COMPOSITION does not exist in this
    /// EHR; [`ServiceError::BadRequest`] when it is already deleted;
    /// [`ServiceError::Conflict`] when the EHR is not modifiable or the
    /// `preceding_version_uid` is stale (→ 409); [`ServiceError::Database`] on
    /// a storage failure.
    pub async fn delete_composition(
        &self,
        ehr_id: EhrId,
        a_version_uid: &ObjectVersionId,
        update_audit: Option<&openehr_its::rest::generated::common::UpdateAudit>,
    ) -> Result<crate::versioning::change::Committed, ServiceError> {
        let (vo_id, expected) = components(a_version_uid)?;
        // Lean delete pre-read: the pre-checks need only the owning EHR, the
        // lifecycle (already-deleted → 400), and the current
        // `VERSION_TREE_ID` (the `preceding_version_uid` conflict compare) —
        // not a full node reassembly (the deleted version stores no nodes
        // anyway).
        let current =
            crate::storage::version_repo::meta::current_composition_meta(&self.pool, vo_id)
                .await?
                .filter(|m| m.ehr_id == Some(ehr_id))
                .ok_or_else(|| {
                    ServiceError::sm(
                        CallStatusType::CompositionDoesNotExist,
                        format!("COMPOSITION {vo_id}"),
                    )
                })?;
        if current.lifecycle_state == crate::versioning::lifecycle::state::DELETED {
            return Err(ServiceError::precondition(format!(
                "COMPOSITION {vo_id} is already deleted"
            )));
        }
        // is_modifiable = False forbids content writes (RM ehr master04 §EHR
        // Active Status) — folded from the standalone `ensure_content_writable`
        // side-SELECT into the pre-read; the 409 outcome and its ordering
        // (after the already-deleted 400, before the stale-precondition 409)
        // unchanged.
        if !current.is_modifiable {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        let current_tree = TreeId::from_columns(
            current.trunk_version,
            current.branch_number,
            current.branch_version,
        );
        // The addressed preceding_version_uid must identify THE latest
        // VERSION by its full three-part identity — object_id ::
        // creating_system_id :: version_tree_id (ITS-REST overview
        // Resources.md §Identifier types: the version_uid "uniquely
        // identifies a VERSION"; RM common master06 §Distributed
        // Versioning) — compared case-insensitively (BASE master05
        // §Composite Identifiers and Case). Comparing the tree id alone
        // let a fabricated creating_system_id delete the latest version.
        let latest_uid = crate::versioning::object_version_id::object_version_id(
            vo_id,
            &current.creating_system_id,
            current_tree,
        );
        if !composite_ids_equal(&latest_uid, a_version_uid.value()) {
            return Err(ServiceError::conflict(format!(
                "preceding_version_uid names version {}, latest is {latest_uid}",
                a_version_uid.value()
            )));
        }
        let _ = expected;

        let mut tx = self.pool.begin().await?;
        // A DELETE is a commit on a change-controlled resource, so the
        // committal request headers merge here too (ITS-REST overview
        // §"openehr-version and openehr-audit-details": the headers MUST be
        // accepted on PUT, POST and DELETE).
        let audit = match update_audit {
            Some(u) => crate::versioning::audit::AuditInput::from_update(
                u,
                change_type::DELETED,
                "COMPOSITION delete",
                &self.effective_system_id(),
            )?,
            None => self.audit(change_type::DELETED, "COMPOSITION delete"),
        };
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
        crate::telemetry::metrics::metrics()
            .db_transactions
            .add(1, &[opentelemetry::KeyValue::new("outcome", "commit")]);
        crate::versioning::change::meter_committed(&committed);
        // 204_COMPOSITION_deleted: the (now deleted) version identity.
        Ok(committed)
    }

    /// The EHR-existence precheck (SM `ehr_does_not_exist` → `NotFound`); also
    /// the [`crate::versioning::CommitEnv`] `ensure_ehr_exists` hook.
    /// The existence read is a storage seam
    /// ([`crate::storage::version_repo::meta::ehr_exists`]; no openEHR spec governs
    /// the SQL — our own design).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR does not exist;
    /// [`ServiceError::Database`] if the existence read fails.
    #[expect(
        clippy::same_name_method,
        reason = "the `CommitEnv` seam (service/commit_env.rs) deliberately \
                  mirrors these chapter method names so the versioning layer \
                  calls them by their own vocabulary; that impl disambiguates \
                  explicitly with `FerroEhrService::<name>(self, …)`"
    )]
    pub(in crate::service) async fn ensure_ehr_exists(
        &self,
        ehr_id: EhrId,
    ) -> Result<(), ServiceError> {
        if crate::storage::version_repo::meta::ehr_exists(&self.pool, ehr_id).await? {
            Ok(())
        } else {
            Err(ServiceError::sm(
                CallStatusType::EhrIdDoesNotExist,
                format!("EHR {ehr_id}"),
            ))
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
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR does not exist;
    /// [`ServiceError::Conflict`] when it is not modifiable;
    /// [`ServiceError::Database`] if the read fails.
    pub(in crate::service) async fn ensure_ehr_content_writable(
        &self,
        ehr_id: EhrId,
    ) -> Result<(), ServiceError> {
        let (exists, is_modifiable) =
            crate::storage::ehr_repo::ehr_writability(&self.pool, ehr_id).await?;
        if !exists {
            return Err(ServiceError::sm(
                CallStatusType::EhrIdDoesNotExist,
                format!("EHR {ehr_id}"),
            ));
        }
        // `None` (no current EHR_STATUS) is treated as modifiable, so the guard
        // never spuriously blocks — identical to `ensure_content_writable`.
        if is_modifiable == Some(false) {
            return Err(Self::not_modifiable_error(ehr_id));
        }
        Ok(())
    }
}

// ── The SM I_EHR_COMPOSITION call surface ─────────────────────────────────────

impl FerroEhrService {
    /// SM `I_EHR_COMPOSITION.has_composition` — whether the named version
    /// exists in the EHR.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `OBJECT_VERSION_ID` or a failing read (a
    /// missing version is `Ok(false)`).
    pub async fn has_composition(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        match self.read_composition(an_ehr_id, vo_id, Some(version)).await {
            Ok(read) => Ok(match read.meta.as_ref() {
                Some(meta) => super::ensure_addressed_version(&a_version_uid, &meta.uid).is_ok(),
                // A deleted version serves no representation/meta; the
                // fetched tree row existing is the answer.
                None => true,
            }),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// The latest COMPOSITION of a versioned object (bare, with its `uid`); a
    /// deleted latest version yields `Value::Null` (→ 204).
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn get_composition_latest(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<Value, SmError> {
        Ok(self
            .composition_latest_response(an_ehr_id, a_versioned_object_uid)
            .await?
            .body)
    }

    /// SM `I_EHR_COMPOSITION.get_composition_at_time` — the COMPOSITION
    /// current at `a_time`, or the latest when `a_time` is `None`.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `a_time` (400-equivalent), a missing
    /// version at that instant (404-equivalent), or a read failure.
    pub async fn get_composition_at_time(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        Ok(self
            .composition_at_time_response(an_ehr_id, a_versioned_object_uid, a_time)
            .await?
            .body)
    }

    /// SM `I_EHR_COMPOSITION.get_composition_at_version` — the bare
    /// COMPOSITION at the named version.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `OBJECT_VERSION_ID`, an unknown version
    /// (404-equivalent), or a read failure.
    pub async fn get_composition_at_version(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        Ok(self
            .composition_at_version_response(an_ehr_id, a_version_uid)
            .await?
            .body)
    }

    /// SM `I_EHR_COMPOSITION.get_versioned_composition` — the
    /// `VERSIONED_COMPOSITION` container object.
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn get_versioned_composition(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<Value, SmError> {
        Ok(self
            .versioned_composition(an_ehr_id, a_versioned_object_uid)
            .await?
            .body)
    }

    /// [`Self::get_versioned_composition`] with the container metadata the
    /// wire's `ETag`/`Last-Modified` need: the container uid identity plus the
    /// newest held version's commit instant (ITS-REST overview
    /// `Requests_and_responses.md` §"`ETag` and Last-Modified" — both headers
    /// SHOULD accompany a `VERSIONED_OBJECT` response).
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn versioned_composition_response(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<ServiceResponse, SmError> {
        Ok(self
            .versioned_composition(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    /// SM `I_EHR_COMPOSITION.get_revision_history` — the `REVISION_HISTORY`
    /// of a COMPOSITION.
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn composition_revision_history(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<Value, SmError> {
        Ok(self
            .composition_revision_history_value(an_ehr_id, a_versioned_object_uid)
            .await?
            .body)
    }

    /// [`Self::composition_revision_history`] with the container metadata the
    /// wire's `ETag`/`Last-Modified` need (container uid + newest commit
    /// instant — same derivation as
    /// [`Self::versioned_composition_response`]).
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn composition_revision_history_response(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<ServiceResponse, SmError> {
        Ok(self
            .composition_revision_history_value(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    /// The VERSION envelope of a COMPOSITION extant at `a_time`, or the
    /// latest when `a_time` is `None`
    /// (`GET …/versioned_composition/{uid}/version`).
    ///
    /// # Errors
    /// [`SmError`] for a malformed `a_time` (400-equivalent), a missing
    /// version at that instant (404-equivalent), or a read failure.
    pub async fn composition_version_at_time(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self
            .composition_version_at_time_read(an_ehr_id, a_versioned_object_uid, at)
            .await?
            .body)
    }

    /// The VERSION envelope of a COMPOSITION at the named version
    /// (`GET …/versioned_composition/{uid}/version/{version_uid}`).
    ///
    /// # Errors
    /// [`SmError`] for a malformed `OBJECT_VERSION_ID`, an unknown version
    /// (404-equivalent), or a read failure.
    pub async fn composition_version_envelope(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        let body = self.composition_version(an_ehr_id, vo_id, version).await?;
        // The served VERSION.uid is the stored full identity; the
        // addressed uid must equal it (Resources.md §Identifier types; BASE
        // master05 case rule) — a fabricated creating_system_id names no
        // VERSION here.
        if let Some(served) = body
            .get("uid")
            .and_then(|uid| uid.get("value"))
            .and_then(Value::as_str)
        {
            super::ensure_addressed_version(&a_version_uid, served)?;
        }
        Ok(body)
    }
}

// ── ITS-REST read-response adapter (adapter-support extension) ────────────────
//
// The SM `I_EHR_COMPOSITION` reads above return the bare COMPOSITION, which
// carries no commit audit — yet ITS-REST requires `Last-Modified` "derived from
// `VERSION.commit_audit.time_committed.value`" (`Requests_and_responses.md`
// §"`ETag` and Last-Modified"). These siblings therefore hand the adapter the
// same read PLUS its [`ResourceMeta`] (version uid + commit instant). No
// openEHR spec governs this envelope — our own design.

impl FerroEhrService {
    /// [`Self::get_composition_latest`] with the version metadata the wire's
    /// `ETag`/`Last-Modified` need. A deleted latest version yields a null body
    /// and no metadata (→ `204`).
    ///
    /// # Errors
    /// [`SmError`] when the object does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn composition_latest_response(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<ServiceResponse, SmError> {
        Ok(self
            .read_composition(an_ehr_id, a_versioned_object_uid, None)
            .await?)
    }

    /// [`Self::get_composition_at_time`] with the version metadata the wire's
    /// `ETag`/`Last-Modified` need.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `a_time` (400-equivalent), a missing
    /// version at that instant (404-equivalent), or a read failure.
    pub async fn composition_at_time_response(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
        a_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        match a_time.as_deref() {
            None => Ok(self
                .read_composition(an_ehr_id, a_versioned_object_uid, None)
                .await?),
            Some(raw) => Ok(self
                .composition_at_time(an_ehr_id, a_versioned_object_uid, parse_at_time(raw)?)
                .await?),
        }
    }

    /// [`Self::get_composition_at_version`] with the version metadata the
    /// wire's `ETag`/`Last-Modified` need.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `OBJECT_VERSION_ID`, an unknown version
    /// (404-equivalent), or a read failure.
    pub async fn composition_at_version_response(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: ObjectVersionId,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, version) = components(&a_version_uid)?;
        let read = self
            .read_composition(an_ehr_id, vo_id, Some(version))
            .await?;
        if let Some(meta) = read.meta.as_ref() {
            super::ensure_addressed_version(&a_version_uid, &meta.uid)?;
        }
        Ok(read)
    }
}

// ── ITS-REST MultimediaAdapter (adapter-support extension) ────────────────────

impl FerroEhrService {
    /// Re-inline externalized multimedia content into a canonical body before
    /// it is served (the S3/object-store externalization extension — no
    /// openEHR spec governs storage movement; the served canonical form is
    /// unchanged). With no engine configured the stored form is served
    /// unchanged.
    ///
    /// # Errors
    /// [`SmError`] when the configured multimedia engine fails to expand a
    /// reference (e.g. the external object store is unreachable).
    #[cfg(feature = "multimedia")]
    pub async fn expand_multimedia(&self, body: Value) -> Result<Value, SmError> {
        let Some(engine) = &self.multimedia else {
            // No store is reachable. Serving the stored form is correct only
            // when there is nothing to expand; a record that DOES reference a
            // blob is clinical content this server externalized, and answering
            // 200 with the compact reference would drop the caller's request
            // on the floor without saying so.
            if ferroehr_ext::multimedia::references_external_blob(&body) {
                return Err(internal_fault(
                    "expand a multimedia reference",
                    &"this record references externalized multimedia but no object store is \
                      configured, so the content cannot be re-inlined (configure \
                      multimedia.endpoint, or re-enable multimedia, to serve it)",
                ));
            }
            return Ok(body);
        };
        let mut body = body;
        engine
            .expand(&mut body)
            .await
            .map_err(|e| internal_fault("expand a multimedia reference", &e))?;
        Ok(body)
    }

    /// The slim twin: externalization is compiled out, so the stored canonical
    /// form is always served unchanged.
    ///
    /// # Errors
    /// Infallible in this configuration; the `Result` mirrors the multimedia
    /// twin so callers are configuration-independent.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    pub async fn expand_multimedia(&self, body: Value) -> Result<Value, SmError> {
        Ok(body)
    }
}
