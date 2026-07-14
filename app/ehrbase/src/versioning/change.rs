//! The change-set unit: [`Change`], the version-tree placement **decision**,
//! and the shared commit engine `apply_change` (S-12, S-13, S-32, S-33, S-34,
//! S-35).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Version and its
//! Subtypes / §Version Lifecycle / §Distributed Versioning / §Logical Deletion.
//! Versioning owns the *decisions* — where a new version sits in the tree
//! (trunk continue / branch fork / lineage close), which lifecycle transition
//! is legal, and what to sign — and the *builders*; all `sqlx` execution is
//! delegated to `crate::storage::version_repo` (row I/O) and
//! `crate::storage::node_repo` (canonical decompose/reassemble). No openEHR
//! spec governs the SQL — our own design.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::service::ServiceError;
use crate::storage::{decompose, reassemble};
use crate::versioning::attestation::{self, PendingAttest};
use crate::versioning::audit::AuditInput;
use crate::versioning::lifecycle::{self, resolve_lifecycle, validate_transition};
use crate::versioning::object_version_id::{TreeId, eq_composite_id, object_version_id};
use crate::versioning::{Kind, SigningCtx, integrity};

/// The outcome of a versioned-object write: the object id, the new version's
/// tree id, and the provenance carried into the event-outbox envelope.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)] // `time_committed` is the master06 domain term, not a suffix echo
pub(crate) struct Committed {
    pub(crate) vo_id: Uuid,
    /// The per-vo storage commit ordinal of the written row (the node /
    /// attestation key) — NOT the wire version number.
    pub(crate) sys_version: i32,
    /// The new version's `VERSION_TREE_ID` (the wire version identity).
    pub(crate) tree: TreeId,
    /// The `creating_system_id` recorded for the new version.
    pub(crate) creating_system_id: String,
    pub(crate) kind: Kind,
    /// The numeric `audit_change_type` group code recorded for this version.
    pub(crate) change_type: String,
    /// The OPT `template_id` a COMPOSITION was committed against (`None`
    /// otherwise).
    pub(crate) template_id: Option<String>,
    /// The server-computed commit instant (the audit `time_committed`,
    /// master06 §Committal) — the write response's `Last-Modified`, carried
    /// here so the service layer never re-reads the row it just wrote.
    pub(crate) time_committed: jiff::Timestamp,
}

/// One change applied within a CONTRIBUTION (the openEHR change-set unit).
///
/// `signature` carries a **client-supplied** `UPDATE_VERSION.signature`
/// (master06 §Digital Signature): present ⇒ stored verbatim, server does not
/// re-sign; absent ⇒ the server signs the assembled `ORIGINAL_VERSION` if
/// signing is enabled. The direct (non-CONTRIBUTION) endpoints always pass
/// `None`.
///
/// `lifecycle_state` on `Create`/`Modify` carries the client-supplied
/// `version_lifecycle_state` (master06 §Version Lifecycle); `None` defaults to
/// `532|complete|`. `523|deleted|` is reserved to [`Change::Delete`]. Its
/// legality against the preceding version's state is checked in [`apply_change`]
/// (G-01).
pub(crate) enum Change {
    /// Create a new versioned object.
    Create {
        kind: Kind,
        canonical: Value,
        template_id: Option<String>,
        signature: Option<String>,
        lifecycle_state: Option<String>,
        /// Wire `UPDATE_VERSION.attestations` committed with this version
        /// (master06 §Attestation "Signing content at committal").
        attestations: Vec<Value>,
    },
    /// Commit a new version of an existing object.
    Modify {
        vo_id: Uuid,
        kind: Kind,
        canonical: Value,
        expected: Option<TreeId>,
        template_id: Option<String>,
        signature: Option<String>,
        lifecycle_state: Option<String>,
        attestations: Vec<Value>,
        /// Wire `ORIGINAL_VERSION.other_input_version_uids` — the merged-in
        /// version ids for a merge commit (master06 §Version Merging); empty for
        /// a plain modification.
        other_input_version_uids: Vec<String>,
    },
    /// Logically delete an object (a content-less `deleted` version — master06
    /// §Logical Deletion).
    Delete {
        vo_id: Uuid,
        kind: Kind,
        expected: Option<TreeId>,
        signature: Option<String>,
    },
}

impl Change {
    /// The versioned-object [`Kind`] this change writes.
    pub(crate) fn kind(&self) -> Kind {
        match *self {
            Change::Create { kind, .. }
            | Change::Modify { kind, .. }
            | Change::Delete { kind, .. } => kind,
        }
    }
}

/// One `vo_version` row to insert (validity `[now, ∞)` for a live write; an
/// explicit period for import). The write-side value contract to
/// `crate::storage::version_repo`.
pub(crate) struct NewVersionRow<'a> {
    pub(crate) vo_id: Uuid,
    pub(crate) kind: Kind,
    pub(crate) ehr_id: Option<Uuid>,
    pub(crate) ordinal: i32,
    pub(crate) tree: TreeId,
    pub(crate) lifecycle_state: &'a str,
    pub(crate) creating_system_id: &'a str,
    /// `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first version).
    pub(crate) preceding_version_uid: Option<&'a str>,
    /// `ORIGINAL_VERSION.other_input_version_uids` (empty → stored NULL,
    /// `Is_merged_validity`).
    pub(crate) other_input_version_uids: &'a [String],
    pub(crate) contribution_id: Uuid,
    pub(crate) audit_id: Uuid,
    pub(crate) template_id: Option<&'a str>,
    pub(crate) signature: Option<&'a str>,
}

impl NewVersionRow<'_> {
    /// The plain storage row ([`crate::storage::version_repo::VersionRow`]) —
    /// kind and tree rendered to their column values.
    fn row(&self) -> crate::storage::version_repo::VersionRow<'_> {
        let (trunk_version, branch_number, branch_version) = self.tree.columns();
        crate::storage::version_repo::VersionRow {
            vo_id: self.vo_id,
            kind: self.kind.as_str(),
            ehr_id: self.ehr_id,
            sys_version: self.ordinal,
            trunk_version,
            branch_number,
            branch_version,
            lifecycle_state: self.lifecycle_state,
            creating_system_id: self.creating_system_id,
            preceding_version_uid: self.preceding_version_uid,
            other_input_version_uids: self.other_input_version_uids,
            contribution_id: self.contribution_id,
            audit_id: self.audit_id,
            template_id: self.template_id,
            signature: self.signature,
        }
    }

    /// The imported-row analogue with an explicit `sys_period` `[lower, upper)`
    /// (master06 §Copying — the synthetic local period chain).
    pub(crate) fn imported_row(
        &self,
        lower: jiff::Timestamp,
        upper: Option<jiff::Timestamp>,
    ) -> crate::storage::version_repo::ImportedVersionRow<'_> {
        let (trunk_version, branch_number, branch_version) = self.tree.columns();
        crate::storage::version_repo::ImportedVersionRow {
            vo_id: self.vo_id,
            kind: self.kind.as_str(),
            ehr_id: self.ehr_id,
            sys_version: self.ordinal,
            trunk_version,
            branch_number,
            branch_version,
            lifecycle_state: self.lifecycle_state,
            creating_system_id: self.creating_system_id,
            preceding_version_uid: self.preceding_version_uid,
            other_input_version_uids: self.other_input_version_uids,
            contribution_id: self.contribution_id,
            audit_id: self.audit_id,
            signature: self.signature,
            lower,
            upper,
        }
    }
}

/// The preceding lineage tip read for a tree-placement decision — mapped from
/// the storage row (`crate::storage::version_repo::lineage_tip`).
#[derive(Debug, Clone)]
pub(crate) struct PrecedingTip {
    pub(crate) ehr_id: Option<Uuid>,
    pub(crate) kind: Kind,
    pub(crate) ordinal: i32,
    pub(crate) tree: TreeId,
    pub(crate) creating_system_id: String,
    /// The preceding version's lifecycle state — the "from" state of the
    /// transition (G-01).
    pub(crate) lifecycle_state: String,
    /// Whether the tip is still open (`upper_inf(sys_period)`).
    pub(crate) open: bool,
}

/// The resolved placement of a new version in the version tree.
struct NextVersion {
    ordinal: i32,
    tree: TreeId,
    /// The lineage tip to close on insert; `None` when the commit FORKS a new
    /// branch and the preceding version stays valid.
    close_ordinal: Option<i32>,
    /// The `preceding_version_uid` to store.
    preceding_uid: String,
    /// The preceding version's lifecycle state (the transition "from" state).
    preceding_lifecycle: String,
}

/// Validate an update/delete target (belongs to `ehr_id`, tip is the addressed
/// open lineage tip) and resolve where the new version sits (RM common master06
/// §Version tree / §Distributed Versioning):
///
/// - the preceding version is the current TRUNK tip when `expected` is absent,
///   or exactly the version `expected` names (trunk or branch) — which must be
///   an open lineage tip, else `VersionConflict`;
/// - a preceding version created by THIS system is continued on its lineage
///   (trunk `N` → `N+1`; branch `t.b.v` → `t.b.v+1`), superseding it;
/// - a preceding version created by ANOTHER system (an imported copy) FORKS a
///   new branch `t.(max_branch+1).1` (master06 §Subsequent Local
///   Modifications) — the preceding version stays valid.
///
/// Same-system detection is case-insensitive on `creating_system_id`
/// (composite-identifier equality, G-09; BASE master05 §Composite Identifiers
/// and Case).
/// Read the preceding lineage tip through the storage row I/O, mapped onto the
/// versioning value contract ([`PrecedingTip`]).
async fn lineage_tip(
    tx: &mut PgConnection,
    vo_id: Uuid,
    expected: Option<TreeId>,
) -> Result<Option<PrecedingTip>, ServiceError> {
    let row =
        crate::storage::version_repo::lineage_tip(tx, vo_id, expected.map(TreeId::columns)).await?;
    row.map(|row| {
        let kind = Kind::from_type(&row.kind).ok_or_else(|| {
            ServiceError::Internal(format!("unknown versioned-object kind {:?}", row.kind))
        })?;
        Ok(PrecedingTip {
            ehr_id: row.ehr_id,
            kind,
            ordinal: row.sys_version,
            tree: TreeId::from_columns(row.trunk_version, row.branch_number, row.branch_version),
            creating_system_id: row.creating_system_id,
            lifecycle_state: row.lifecycle_state,
            open: row.open,
        })
    })
    .transpose()
}

async fn next_version(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<TreeId>,
    local_system_id: &str,
) -> Result<NextVersion, ServiceError> {
    // Serialize concurrent writers of the same object.
    crate::storage::version_repo::advisory_lock(tx, vo_id).await?;

    let tip = lineage_tip(tx, vo_id, expected).await?;
    let Some(tip) = tip else {
        // The object may exist with the expectation naming no stored version —
        // distinguish "no such object" (404) from "wrong version" (409): the
        // current trunk tip is the lineage tip with no expectation.
        let current = lineage_tip(tx, vo_id, None).await?;
        return match (expected, current) {
            (Some(tree), Some(current)) => Err(ServiceError::VersionConflict(format!(
                "expected version {tree}, which does not exist (current is {})",
                current.tree
            ))),
            _ => Err(ServiceError::NotFound(format!(
                "{} {vo_id} in EHR {ehr_id:?}",
                kind.as_str()
            ))),
        };
    };
    // For an EHR-scoped object the stored owner must match; for a demographic
    // party both stored and expected owner are `None`, which compares equal.
    if tip.ehr_id != ehr_id || tip.kind != kind {
        return Err(ServiceError::NotFound(format!(
            "{} {vo_id} in EHR {ehr_id:?}",
            kind.as_str()
        )));
    }
    if !tip.open {
        return Err(ServiceError::VersionConflict(format!(
            "expected version {} has been superseded",
            tip.tree
        )));
    }
    let preceding_uid = object_version_id(vo_id, &tip.creating_system_id, tip.tree);
    let ordinal = crate::storage::version_repo::next_ordinal(tx, vo_id).await?;

    let (tree, close_ordinal) = if eq_composite_id(&tip.creating_system_id, local_system_id) {
        // Continue the lineage this system owns; the preceding tip is superseded.
        let tree = match tip.tree.branch {
            None => TreeId::trunk(tip.tree.trunk + 1),
            Some((b, v)) => TreeId::branch(tip.tree.trunk, b, v + 1),
        };
        (tree, Some(tip.ordinal))
    } else {
        // Local modification of a version copied from elsewhere: fork a branch
        // at the preceding version's trunk fork point (master06 §Distributed
        // Versioning); the copied version itself stays valid.
        let next_branch =
            crate::storage::version_repo::next_branch_number(tx, vo_id, tip.tree.trunk).await?;
        (TreeId::branch(tip.tree.trunk, next_branch, 1), None)
    };
    Ok(NextVersion {
        ordinal,
        tree,
        close_ordinal,
        preceding_uid,
        preceding_lifecycle: tip.lifecycle_state,
    })
}

/// The core write path shared by single-object writes and CONTRIBUTION commits:
/// apply one [`Change`] under an already-open contribution + version audit,
/// signing the assembled `ORIGINAL_VERSION` (RM common master06 §Digital
/// Signature) and rejecting an illegal lifecycle transition (G-01).
///
/// The cross-area pre/post-commit hooks the legacy path ran inline here now
/// belong to other layers and run around this write, driven by the CONTRIBUTION
/// orchestration ([`crate::versioning::CommitEnv`], called from
/// [`super::contribution::commit_version_set`]) and by the direct write paths:
/// - `CommitEnv::pre_composition_modify` — the `VERSIONED_COMPOSITION`
///   cross-version invariants (`Archetype_node_id_valid` / `Persistent_validity`,
///   RM ehr `versioned_composition.adoc`), before a COMPOSITION modify (G-13);
/// - `CommitEnv::post_status_commit` — the EHR promoted-subject-column sync,
///   after an `EHR_STATUS` version;
/// - the `compositions_committed_total` metric — a cross-cutting service-layer
///   concern, not a storage write.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the three change arms + commit context
async fn apply_change(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    contribution_id: Uuid,
    audit_id: Uuid,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
    ctx: &SigningCtx<'_>,
    committer_fallback: &Value,
    change: Change,
) -> Result<Committed, ServiceError> {
    match change {
        Change::Create {
            kind,
            mut canonical,
            template_id,
            signature,
            lifecycle_state,
            attestations,
        } => {
            // Externalize large inline DV_MULTIMEDIA before decompose/sign, so
            // the stored, served and signed form is the offloaded one.
            if let Some(engine) = ctx.multimedia {
                engine
                    .offload(&mut canonical)
                    .await
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
            }
            let lifecycle = resolve_lifecycle(lifecycle_state)?;
            // G-01: a first version can only be `complete`/`incomplete`.
            validate_transition(None, &lifecycle)?;
            let rows = decompose(canonical)?;
            let vo_id = Uuid::now_v7();
            // Sign the exact data that will be served on read (reassembled from
            // the stored nodes) so the digest recomputes at read time. The
            // reassemble is a full O(nodes) tree rebuild — only performed when
            // a signature will actually be computed (signing enabled and no
            // client-supplied signature short-circuit); with signing off the
            // rebuilt document was discarded on every commit.
            let served = if ctx.signer.enabled() {
                reassemble(&rows)?
            } else {
                Value::Null
            };
            let signature = integrity::sign_version(
                ctx,
                audit,
                time_committed,
                vo_id,
                TreeId::trunk(1),
                None,
                contribution_id,
                &lifecycle,
                &served,
                signature,
            )?;
            crate::storage::version_repo::insert_vo_version(
                tx,
                &NewVersionRow {
                    vo_id,
                    kind,
                    ehr_id,
                    ordinal: 1,
                    tree: TreeId::trunk(1),
                    lifecycle_state: &lifecycle,
                    creating_system_id: &ctx.system_id,
                    preceding_version_uid: None,
                    other_input_version_uids: &[],
                    contribution_id,
                    audit_id,
                    template_id: template_id.as_deref(),
                    signature: signature.as_deref(),
                }
                .row(),
            )
            .await?;
            crate::storage::node_repo::write_nodes(tx, vo_id, 1, ehr_id, &rows).await?;
            // A newly created FOLDER hierarchy joins `EHR.folders` as a new
            // member (RM ehr master04 §Folders). Recorded only on CREATION.
            if kind == Kind::Folder
                && let Some(ehr_id) = ehr_id
            {
                crate::storage::version_repo::insert_ehr_folder_rank(tx, ehr_id, vo_id).await?;
            }
            attestation::insert_accompanying_attestations(
                tx,
                vo_id,
                1,
                contribution_id,
                &ctx.system_id,
                committer_fallback,
                time_committed,
                &attestations,
            )
            .await?;
            Ok(Committed {
                vo_id,
                sys_version: 1,
                tree: TreeId::trunk(1),
                creating_system_id: ctx.system_id.clone(),
                kind,
                change_type: audit.change_type.clone(),
                template_id,
                time_committed,
            })
        }
        Change::Modify {
            vo_id,
            kind,
            mut canonical,
            expected,
            template_id,
            signature,
            lifecycle_state,
            attestations,
            other_input_version_uids,
        } => {
            if let Some(engine) = ctx.multimedia {
                engine
                    .offload(&mut canonical)
                    .await
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
            }
            let lifecycle = resolve_lifecycle(lifecycle_state)?;
            let rows = decompose(canonical)?;
            let next = next_version(tx, ehr_id, vo_id, kind, expected, &ctx.system_id).await?;
            // G-01: the transition from the preceding version's state must be
            // legal (master06 §Version Lifecycle state machine).
            validate_transition(Some(&next.preceding_lifecycle), &lifecycle)?;
            if let Some(close_ordinal) = next.close_ordinal {
                crate::storage::version_repo::close_ordinal_at_now(tx, vo_id, close_ordinal)
                    .await?;
            }
            // See the create arm: the reassemble only pays when a signature
            // will be computed.
            let served = if ctx.signer.enabled() {
                reassemble(&rows)?
            } else {
                Value::Null
            };
            let signature = integrity::sign_version(
                ctx,
                audit,
                time_committed,
                vo_id,
                next.tree,
                Some(&next.preceding_uid),
                contribution_id,
                &lifecycle,
                &served,
                signature,
            )?;
            crate::storage::version_repo::insert_vo_version(
                tx,
                &NewVersionRow {
                    vo_id,
                    kind,
                    ehr_id,
                    ordinal: next.ordinal,
                    tree: next.tree,
                    lifecycle_state: &lifecycle,
                    creating_system_id: &ctx.system_id,
                    preceding_version_uid: Some(&next.preceding_uid),
                    other_input_version_uids: &other_input_version_uids,
                    contribution_id,
                    audit_id,
                    template_id: template_id.as_deref(),
                    signature: signature.as_deref(),
                }
                .row(),
            )
            .await?;
            crate::storage::node_repo::write_nodes(tx, vo_id, next.ordinal, ehr_id, &rows).await?;
            attestation::insert_accompanying_attestations(
                tx,
                vo_id,
                next.ordinal,
                contribution_id,
                &ctx.system_id,
                committer_fallback,
                time_committed,
                &attestations,
            )
            .await?;
            Ok(Committed {
                vo_id,
                sys_version: next.ordinal,
                tree: next.tree,
                creating_system_id: ctx.system_id.clone(),
                kind,
                change_type: audit.change_type.clone(),
                template_id,
                time_committed,
            })
        }
        Change::Delete {
            vo_id,
            kind,
            expected,
            signature,
        } => {
            let next = next_version(tx, ehr_id, vo_id, kind, expected, &ctx.system_id).await?;
            if let Some(close_ordinal) = next.close_ordinal {
                crate::storage::version_repo::close_ordinal_at_now(tx, vo_id, close_ordinal)
                    .await?;
            }
            // A deleted version carries no data — its `ORIGINAL_VERSION.data` is
            // Void (master06 §Logical Deletion); the signature is over the
            // data-less version wrapper. Logical deletion is permitted from any
            // live state, so no transition check runs here.
            let signature = integrity::sign_version(
                ctx,
                audit,
                time_committed,
                vo_id,
                next.tree,
                Some(&next.preceding_uid),
                contribution_id,
                lifecycle::state::DELETED,
                &Value::Null,
                signature,
            )?;
            crate::storage::version_repo::insert_vo_version(
                tx,
                &NewVersionRow {
                    vo_id,
                    kind,
                    ehr_id,
                    ordinal: next.ordinal,
                    tree: next.tree,
                    lifecycle_state: lifecycle::state::DELETED,
                    creating_system_id: &ctx.system_id,
                    preceding_version_uid: Some(&next.preceding_uid),
                    other_input_version_uids: &[],
                    contribution_id,
                    audit_id,
                    template_id: None,
                    signature: signature.as_deref(),
                }
                .row(),
            )
            .await?;
            Ok(Committed {
                vo_id,
                sys_version: next.ordinal,
                tree: next.tree,
                creating_system_id: ctx.system_id.clone(),
                kind,
                change_type: audit.change_type.clone(),
                template_id: None,
                time_committed,
            })
        }
    }
}

/// Insert an `audit` row + its enclosing `contribution`, returning
/// `(contribution_id, audit_id, time_committed)` — the version `commit_audit`'s
/// server-computed time (master06 §Committal: `time_committed` "computed on the
/// server", S-22). One round trip (the storage layer merges the two inserts —
/// [`crate::storage::version_repo::write_contribution`]).
async fn write_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit: &AuditInput,
) -> Result<(Uuid, Uuid, jiff::Timestamp), ServiceError> {
    Ok(crate::storage::version_repo::write_contribution(tx, ehr_id, &audit.row(), None).await?)
}

/// Create the first version of a new versioned object under its own
/// contribution.
pub(crate) async fn create(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    kind: Kind,
    canonical: Value,
    template_id: Option<&str>,
    audit: &AuditInput,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id, time_committed) = write_contribution(tx, ehr_id, audit).await?;
    let committed = apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        audit,
        time_committed,
        ctx,
        &audit.committer,
        Change::Create {
            kind,
            canonical,
            template_id: template_id.map(str::to_owned),
            signature: None,
            lifecycle_state: None,
            attestations: Vec::new(),
        },
    )
    .await?;
    crate::storage::version_repo::write_outbox(
        tx,
        contribution_id,
        ehr_id,
        time_committed,
        vec![committed.envelope_entry()],
    )
    .await?;
    Ok(committed)
}

/// Commit a new version of an existing object under its own contribution.
#[allow(clippy::too_many_arguments)] // the write parameters; a struct would not read clearer
pub(crate) async fn update(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    canonical: Value,
    expected: Option<TreeId>,
    template_id: Option<&str>,
    audit: &AuditInput,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id, time_committed) = write_contribution(tx, ehr_id, audit).await?;
    let committed = apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        audit,
        time_committed,
        ctx,
        &audit.committer,
        Change::Modify {
            vo_id,
            kind,
            canonical,
            expected,
            template_id: template_id.map(str::to_owned),
            signature: None,
            lifecycle_state: None,
            attestations: Vec::new(),
            other_input_version_uids: Vec::new(),
        },
    )
    .await?;
    crate::storage::version_repo::write_outbox(
        tx,
        contribution_id,
        ehr_id,
        time_committed,
        vec![committed.envelope_entry()],
    )
    .await?;
    Ok(committed)
}

/// Logically delete an object under its own contribution (master06 §Logical
/// Deletion).
pub(crate) async fn delete(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<TreeId>,
    audit: &AuditInput,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id, time_committed) = write_contribution(tx, ehr_id, audit).await?;
    let committed = apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        audit,
        time_committed,
        ctx,
        &audit.committer,
        Change::Delete {
            vo_id,
            kind,
            expected,
            signature: None,
        },
    )
    .await?;
    crate::storage::version_repo::write_outbox(
        tx,
        contribution_id,
        ehr_id,
        time_committed,
        vec![committed.envelope_entry()],
    )
    .await?;
    Ok(committed)
}

/// Commit a set of changes atomically under one CONTRIBUTION (RM common
/// master06 §Committal and Audits — "similar to nested transactions", S-17).
/// `contribution_audit` is the CONTRIBUTION's own audit; each change carries its
/// VERSION `commit_audit`. `attests` are `666|attestation|` items — new
/// `ATTESTATION`s attached to **existing** versions (S-25), committed in the
/// same transaction but adding no new version.
///
/// master06 §Committal (m4): a version item that omits `committer`/`system_id`
/// inherits them from the CONTRIBUTION audit (S-21) — realized by the callers
/// building each version `AuditInput`; the attestation committer likewise
/// defaults to the CONTRIBUTION committer here.
pub(crate) async fn commit_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    supplied_uid: Option<Uuid>,
    contribution_audit: &AuditInput,
    changes: Vec<(AuditInput, Change)>,
    attests: Vec<PendingAttest>,
    ctx: &SigningCtx<'_>,
) -> Result<(Uuid, Vec<Committed>), ServiceError> {
    // The CONTRIBUTION's own audit + contribution rows in one round trip (the
    // per-version `commit_audit`s are inserted per change below).
    let (contribution_id, _contribution_audit_id, contribution_time) =
        crate::storage::version_repo::write_contribution(
            tx,
            ehr_id,
            &contribution_audit.row(),
            supplied_uid,
        )
        .await?;
    let committer_fallback = &contribution_audit.committer;
    let mut committed = Vec::with_capacity(changes.len() + attests.len());
    for (version_audit, change) in changes {
        let (audit_id, time_committed) =
            crate::storage::version_repo::insert_audit(tx, &version_audit.row()).await?;
        committed.push(
            apply_change(
                tx,
                ehr_id,
                contribution_id,
                audit_id,
                &version_audit,
                time_committed,
                ctx,
                committer_fallback,
                change,
            )
            .await?,
        );
    }
    // Standalone 666 attestations of existing versions (no new version) —
    // completed with the contribution's commit-act time.
    for item in attests {
        let full = attestation::complete_attestation(
            &item.partial,
            &ctx.system_id,
            committer_fallback,
            contribution_time,
        )?;
        committed.push(
            attestation::attest(
                tx,
                ehr_id,
                item.vo_id,
                item.kind,
                item.expected,
                &full,
                contribution_id,
                contribution_time,
            )
            .await?,
        );
    }
    // One PHI-free outbox event for the whole CONTRIBUTION (same transaction).
    let versions = committed.iter().map(Committed::envelope_entry).collect();
    crate::storage::version_repo::write_outbox(
        tx,
        contribution_id,
        ehr_id,
        contribution_time,
        versions,
    )
    .await?;
    Ok((contribution_id, committed))
}
