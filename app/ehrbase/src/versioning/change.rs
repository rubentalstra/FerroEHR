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

use crate::service::error::ServiceError;
use crate::storage::codec::{decompose, reassemble};
use crate::storage::row::NodeRow;
use crate::versioning::attestation::{self, PendingAttest};
use crate::versioning::audit::AuditInput;
use crate::versioning::lifecycle::{self, resolve_lifecycle, validate_transition};
use crate::versioning::object_version_id::{TreeId, eq_composite_id, object_version_id};
use crate::versioning::{Kind, SigningCtx, integrity};

/// The outcome of a versioned-object write: the object id, the new version's
/// tree id, and the provenance carried into the event-outbox envelope.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)] // `time_committed` is the master06 domain term, not a suffix echo
pub struct Committed {
    pub vo_id: Uuid,
    /// The per-vo storage commit ordinal of the written row (the node /
    /// attestation key) — NOT the wire version number.
    pub sys_version: i32,
    /// The new version's `VERSION_TREE_ID` (the wire version identity).
    pub tree: TreeId,
    /// The `creating_system_id` recorded for the new version.
    pub creating_system_id: String,
    pub kind: Kind,
    /// The numeric `audit_change_type` group code recorded for this version.
    pub change_type: String,
    /// The OPT `template_id` a COMPOSITION was committed against (`None`
    /// otherwise).
    pub template_id: Option<String>,
    /// The server-computed commit instant (the audit `time_committed`,
    /// master06 §Committal) — the write response's `Last-Modified`, carried
    /// here so the service layer never re-reads the row it just wrote.
    pub time_committed: jiff::Timestamp,
}

impl Committed {
    /// The committed version's full `OBJECT_VERSION_ID` (`ETag`/`Location`
    /// value — RM common master06 §Version Identification).
    #[must_use]
    pub fn version_uid(&self) -> String {
        object_version_id(self.vo_id, &self.creating_system_id, self.tree)
    }

    /// The per-version entry for the PHI-free event-outbox envelope: identity +
    /// provenance metadata only, never clinical content.
    ///
    /// PORT NOTE: no openEHR spec governs eventing — our own extension. The
    /// outbox row is written inside the commit transaction by storage; this
    /// only builds the payload (README cross-ruling: extensions build payloads).
    pub(crate) fn envelope_entry(&self) -> Value {
        serde_json::json!({
            "vo_id": self.vo_id,
            "kind": self.kind.as_str(),
            "sys_version": self.sys_version,
            "version_tree_id": self.tree.to_string(),
            "change_type": self.change_type,
            "template_id": self.template_id,
        })
    }
}

/// One change applied within a CONTRIBUTION — the openEHR change-set unit
/// (RM common master06 §Contributions).
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
///.
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

/// The caller's `UPDATE_VERSION` envelope pieces a direct write threads into
/// the commit (ITS-REST committal-header merge — the attributes "MUST be
/// merged … on commit runtime"): the lifecycle state, a verbatim client
/// signature, and accompanying attestations. `Default` = the plain server
/// commit (532|complete|, server-signed, none).
#[derive(Debug, Default)]
pub(crate) struct WriteEnvelope {
    /// `UPDATE_VERSION.lifecycle_state` (None → 532|complete|, G-01-checked).
    pub(crate) lifecycle_state: Option<String>,
    /// A client-supplied `VERSION.signature`, stored verbatim (master06
    /// §Digital Signature).
    pub(crate) signature: Option<String>,
    /// `UPDATE_VERSION.attestations` committed with the version.
    pub(crate) attestations: Vec<Value>,
}

/// The preceding lineage tip read for a tree-placement decision — mapped from
/// the storage row (`crate::storage::version_repo::placement::next_placement`).
#[derive(Debug, Clone)]
struct PrecedingTip {
    ehr_id: Option<Uuid>,
    kind: Kind,
    ordinal: i32,
    tree: TreeId,
    creating_system_id: String,
    /// The preceding version's lifecycle state — the "from" state of the
    /// transition.
    lifecycle_state: String,
    /// Whether the tip is still open (`upper_inf(sys_period)`).
    open: bool,
}

/// Map a storage tip row onto the versioning value contract ([`PrecedingTip`]).
fn preceding_tip(
    row: crate::storage::version_repo::placement::TipRow,
) -> Result<PrecedingTip, ServiceError> {
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
    /// The transaction timestamp — the commit instant every row of this
    /// transaction stamps, carried by the merged placement read so the
    /// signature is computable before any insert.
    now: jiff::Timestamp,
}

/// Validate an update/delete target (belongs to `ehr_id`, tip is the addressed
/// open lineage tip) and resolve where the new version sits (RM common master06
/// §Version tree / §Distributed Versioning):
///
/// - the preceding version is the current TRUNK tip when `expected` is absent,
/// or exactly the version `expected` names (trunk or branch) — which must be
/// an open lineage tip, else `VersionConflict`;
/// - a preceding version created by THIS system is continued on its lineage
/// (trunk `N` → `N+1`; branch `t.b.v` → `t.b.v+1`), superseding it;
/// - a preceding version created by ANOTHER system (an imported copy) FORKS a
/// new branch `t.(max_branch+1).1` (master06 §Subsequent Local
/// Modifications) — the preceding version stays valid.
///
/// Same-system detection is case-insensitive on `creating_system_id`
/// (composite-identifier equality, G-09; BASE master05 §Composite Identifiers
/// and Case).
///
/// # Errors
/// - [`ServiceError::NotFound`] when the object does not exist, or its stored
/// owner/kind do not match the addressed `(ehr_id, kind)`;
/// - [`ServiceError::VersionConflict`] when `expected` names a version that
/// does not exist or has been superseded (a closed lineage tip).
async fn next_version(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<TreeId>,
    local_system_id: &str,
) -> Result<NextVersion, ServiceError> {
    // Serialize concurrent writers of the same object.
    crate::storage::version_repo::commit::advisory_lock(tx, vo_id).await?;

    // ONE statement: preceding tip + next ordinal + the transaction timestamp
    // (the commit instant every row of this transaction stamps).
    let placement = crate::storage::version_repo::placement::next_placement(
        tx,
        vo_id,
        expected.map(TreeId::columns),
    )
    .await?;
    let ordinal = placement.next_ordinal;
    let now = placement.now;
    let tip = placement.tip.map(preceding_tip).transpose()?;
    let Some(tip) = tip else {
        // The object may exist with the expectation naming no stored version —
        // distinguish "no such object" (404) from "wrong version" (409): the
        // current trunk tip is the lineage tip with no expectation.
        let current = crate::storage::version_repo::placement::next_placement(tx, vo_id, None)
            .await?
            .tip
            .map(preceding_tip)
            .transpose()?;
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
            crate::storage::version_repo::placement::next_branch_number(tx, vo_id, tip.tree.trunk)
                .await?;
        (TreeId::branch(tip.tree.trunk, next_branch, 1), None)
    };
    Ok(NextVersion {
        ordinal,
        tree,
        close_ordinal,
        preceding_uid,
        preceding_lifecycle: tip.lifecycle_state,
        now,
    })
}

/// How the enclosing CONTRIBUTION is supplied to [`apply_change`].
enum ContributionCtx {
    /// A standalone single-object write (create/update/delete): the CONTRIBUTION
    /// is created in the same commit, sharing the version's `commit_audit` (a
    /// direct write is one CONTRIBUTION of one change — master06 §Committal).
    New,
    /// A change within a multi-change CONTRIBUTION already opened by
    /// [`commit_contribution`]; the version's own `commit_audit` is written here,
    /// referencing this existing `contribution_id`.
    Existing(Uuid),
}

/// A [`Change`] resolved to the concrete `vo_version` placement + content — the
/// output of the per-arm decision (offload, lifecycle, decompose, version-tree
/// placement) and the input to the shared write ([`commit_resolved`]).
struct ResolvedWrite {
    kind: Kind,
    vo_id: Uuid,
    ehr_id: Option<Uuid>,
    /// The per-vo storage commit ordinal.
    ordinal: i32,
    tree: TreeId,
    /// The `version_lifecycle_state` code (master06 §Version Lifecycle).
    lifecycle: String,
    /// `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first version).
    preceding_uid: Option<String>,
    other_input_version_uids: Vec<String>,
    template_id: Option<String>,
    /// The lineage tip storage ordinal to supersede at `now()` — `None` for a
    /// first version or a FORK (master06 §Version tree).
    close_ordinal: Option<i32>,
    /// A client-supplied `UPDATE_VERSION.signature`, stored verbatim (master06
    /// §Digital Signature); `None` on the direct endpoints.
    client_signature: Option<String>,
    /// The decomposed node rows (empty for a logical delete — data Void).
    rows: Vec<NodeRow>,
    /// `UPDATE_VERSION.attestations` committed with this version.
    attestations: Vec<Value>,
    /// A newly created FOLDER hierarchy that joins `EHR.folders` (create only).
    is_first_folder: bool,
    /// The transaction timestamp — the commit instant this transaction stamps
    /// on every row, read before any insert so the `VERSION.signature` is
    /// computable up front (RM common master06 §Digital Signature).
    time_committed: jiff::Timestamp,
}

/// The core write path shared by single-object writes and CONTRIBUTION commits:
/// resolve one [`Change`] to its version-tree placement + content and commit it
/// under the supplied [`ContributionCtx`], signing the assembled
/// `ORIGINAL_VERSION` (RM common master06 §Digital Signature) and rejecting an
/// illegal lifecycle transition. Returns the [`Committed`] outcome and the
/// enclosing `contribution_id` (for the caller's event-outbox envelope).
///
/// The cross-area pre/post-commit hooks the legacy path ran inline here now
/// belong to other layers and run around this write, driven by the CONTRIBUTION
/// orchestration ([`crate::versioning::CommitEnv`], called from
/// [`super::contribution::commit_version_set`]) and by the direct write paths:
/// - `CommitEnv::pre_composition_modify` — the `VERSIONED_COMPOSITION`
/// cross-version invariants (`Archetype_node_id_valid` / `Persistent_validity`,
/// RM ehr `versioned_composition.adoc`), before a COMPOSITION modify;
/// - `CommitEnv::post_status_commit` — the EHR promoted-subject-column sync,
/// after an `EHR_STATUS` version;
/// - the `compositions_committed_total` metric — a cross-cutting service-layer
/// concern, not a storage write.
///
/// # Errors
/// The [`next_version`] placement errors (`NotFound` / `VersionConflict`) on
/// modify/delete; [`ServiceError::Unprocessable`] for an out-of-group or
/// illegal lifecycle transition; [`ServiceError::Internal`] on a multimedia
/// offload failure; plus the storage/signing errors of [`commit_resolved`].
#[allow(clippy::too_many_lines)] // the three change arms building the resolved write
#[allow(clippy::too_many_arguments)] // the commit scope; grouping is queued for the polish wave
async fn apply_change(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    contribution: ContributionCtx,
    audit: &AuditInput,
    ctx: &SigningCtx<'_>,
    committer_fallback: &Value,
    known_now: Option<jiff::Timestamp>,
    change: Change,
) -> Result<(Committed, Uuid), ServiceError> {
    let resolved = match change {
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
            // a first version can only be `complete`/`incomplete`.
            validate_transition(None, &lifecycle)?;
            let rows = decompose(canonical)?;
            let time_committed = match known_now {
                Some(ts) => ts,
                None => crate::storage::version_repo::placement::tx_now(tx).await?,
            };
            ResolvedWrite {
                kind,
                vo_id: Uuid::now_v7(),
                ehr_id,
                ordinal: 1,
                tree: TreeId::trunk(1),
                lifecycle,
                preceding_uid: None,
                other_input_version_uids: Vec::new(),
                template_id,
                close_ordinal: None,
                client_signature: signature,
                rows,
                attestations,
                is_first_folder: kind == Kind::Folder && ehr_id.is_some(),
                time_committed,
            }
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
            // the transition from the preceding version's state must be
            // legal (master06 §Version Lifecycle state machine).
            validate_transition(Some(&next.preceding_lifecycle), &lifecycle)?;
            ResolvedWrite {
                kind,
                vo_id,
                ehr_id,
                ordinal: next.ordinal,
                tree: next.tree,
                lifecycle,
                preceding_uid: Some(next.preceding_uid),
                other_input_version_uids,
                template_id,
                close_ordinal: next.close_ordinal,
                client_signature: signature,
                rows,
                attestations,
                is_first_folder: false,
                time_committed: known_now.unwrap_or(next.now),
            }
        }
        Change::Delete {
            vo_id,
            kind,
            expected,
            signature,
        } => {
            let next = next_version(tx, ehr_id, vo_id, kind, expected, &ctx.system_id).await?;
            // A deleted version carries no data — its `ORIGINAL_VERSION.data` is
            // Void (master06 §Logical Deletion); the signature is over the
            // data-less version wrapper. Logical deletion is permitted from any
            // live state, so no transition check runs here.
            ResolvedWrite {
                kind,
                vo_id,
                ehr_id,
                ordinal: next.ordinal,
                tree: next.tree,
                lifecycle: lifecycle::state::DELETED.to_owned(),
                preceding_uid: Some(next.preceding_uid),
                other_input_version_uids: Vec::new(),
                template_id: None,
                close_ordinal: next.close_ordinal,
                client_signature: signature,
                rows: Vec::new(),
                attestations: Vec::new(),
                is_first_folder: false,
                time_committed: known_now.unwrap_or(next.now),
            }
        }
    };
    commit_resolved(tx, ctx, audit, contribution, committer_fallback, resolved).await
}

/// Commit a [`ResolvedWrite`] — close the superseded lineage tip, compute the
/// `VERSION.signature`, then write the `audit` (+ `contribution` for a
/// standalone write) and the `vo_version` row in ONE data-modifying CTE, then
/// the node rows, folder membership and accompanying attestations.
///
/// The signature is computed over the assembled `ORIGINAL_VERSION` (RM common
/// master06 §Digital Signature), which embeds `time_committed` and
/// `contribution_id`. Both are known BEFORE any statement: the commit instant
/// is the transaction timestamp (read by the placement query /
/// [`tx_now`](crate::storage::version_repo::placement::tx_now); every row of the
/// transaction stamps the same `now()`), and a standalone write generates its
/// `contribution_id` here. So audit + contribution + `vo_version` always
/// collapse into the one folded CTE
/// ([`commit_new_version`](crate::storage::version_repo::commit::commit_new_version)
/// / [`commit_version_into`](crate::storage::version_repo::commit::commit_version_into)).
/// The lineage-tip close stays a separate prior statement (the
/// one-open-row-per-lineage partial unique indexes need the old open row gone
/// before the new open row is inserted). No openEHR spec governs statement
/// batching — our own design.
///
/// # Errors
/// [`ServiceError::Signing`] when the canonical form cannot be produced or the
/// signer fails; the storage errors of the close / folded-insert / node /
/// attestation writes; the attestation-completion `Unprocessable` rejections.
async fn commit_resolved(
    tx: &mut PgConnection,
    ctx: &SigningCtx<'_>,
    audit: &AuditInput,
    contribution: ContributionCtx,
    committer_fallback: &Value,
    r: ResolvedWrite,
) -> Result<(Committed, Uuid), ServiceError> {
    let audit_row = audit.row();
    let (trunk_version, branch_number, branch_version) = r.tree.columns();

    // Close the superseded lineage tip FIRST, as its own statement, before the
    // folded insert: the one-open-row-per-lineage partial unique indexes
    // (`uq_vo_version_current` / `uq_vo_version_branch_current`) require the
    // old open row to be gone before the new open row is inserted. `now()` is
    // the transaction timestamp, so the close boundary and the new version's
    // `sys_period` open at the identical instant (master06 §Version tree).
    if let Some(close_ordinal) = r.close_ordinal {
        crate::storage::version_repo::commit::close_ordinal_at_now(tx, r.vo_id, close_ordinal)
            .await?;
    }

    // The enclosing CONTRIBUTION id: pre-existing for a multi-change commit,
    // generated here for a standalone write — known before the signature.
    let contribution_id = match contribution {
        ContributionCtx::New => Uuid::now_v7(),
        ContributionCtx::Existing(cid) => cid,
    };

    let signature = version_signature(ctx, audit, contribution_id, &r)?;

    let folded = crate::storage::version_repo::commit::FoldedVersion {
        vo_id: r.vo_id,
        kind: r.kind.as_str(),
        ehr_id: r.ehr_id,
        sys_version: r.ordinal,
        trunk_version,
        branch_number,
        branch_version,
        lifecycle_state: &r.lifecycle,
        creating_system_id: &ctx.system_id,
        preceding_version_uid: r.preceding_uid.as_deref(),
        other_input_version_uids: &r.other_input_version_uids,
        template_id: r.template_id.as_deref(),
        signature: signature.as_deref(),
    };
    let time_committed = match contribution {
        ContributionCtx::New => {
            let (_cid, _aid, tc) = crate::storage::version_repo::commit::commit_new_version(
                tx,
                &audit_row,
                Some(contribution_id),
                &folded,
            )
            .await?;
            tc
        }
        ContributionCtx::Existing(cid) => {
            let (_aid, tc) = crate::storage::version_repo::commit::commit_version_into(
                tx, &audit_row, cid, &folded,
            )
            .await?;
            tc
        }
    };
    debug_assert_eq!(
        time_committed, r.time_committed,
        "the stored commit instant is the transaction timestamp read up front"
    );

    // The shared commit tail: node rows, folder membership, attestations.
    crate::storage::node_repo::write_nodes(tx, r.vo_id, r.ordinal, r.ehr_id, &r.rows).await?;
    // A newly created FOLDER hierarchy joins `EHR.folders` as a new member (RM
    // ehr master04 §Folders). Recorded only on CREATION.
    if r.is_first_folder
        && let Some(ehr_id) = r.ehr_id
    {
        crate::storage::version_repo::commit::insert_ehr_folder_rank(tx, ehr_id, r.vo_id).await?;
    }
    attestation::insert_accompanying_attestations(
        tx,
        r.vo_id,
        r.ordinal,
        contribution_id,
        &ctx.system_id,
        committer_fallback,
        time_committed,
        &r.attestations,
    )
    .await?;

    Ok((
        Committed {
            vo_id: r.vo_id,
            sys_version: r.ordinal,
            tree: r.tree,
            creating_system_id: ctx.system_id.clone(),
            kind: r.kind,
            change_type: audit.change_type.clone(),
            template_id: r.template_id,
            time_committed,
        },
        contribution_id,
    ))
}

/// The signature stored with the version: a client-supplied signature is kept
/// verbatim (`VERSION.signature`, master06 §Digital Signature); otherwise,
/// with server signing enabled, the version's canonical form is signed — a
/// logically deleted version has no nodes → Void (master06 §Logical Deletion);
/// a content version signs the reassembled served bytes so the digest
/// recomputes at read time. Reassembly runs only when a signature will
/// actually be computed.
///
/// # Errors
/// [`ServiceError::Signing`] when the canonical form cannot be produced or the
/// signer fails.
fn version_signature(
    ctx: &SigningCtx<'_>,
    audit: &AuditInput,
    contribution_id: Uuid,
    r: &ResolvedWrite,
) -> Result<Option<String>, ServiceError> {
    if r.client_signature.is_some() || !ctx.signer.enabled() {
        return Ok(r.client_signature.clone());
    }
    let served = if r.rows.is_empty() {
        Value::Null
    } else {
        reassemble(&r.rows)?
    };
    integrity::sign_version(
        ctx,
        audit,
        r.time_committed,
        r.vo_id,
        r.tree,
        r.preceding_uid.as_deref(),
        contribution_id,
        &r.lifecycle,
        &served,
        None,
    )
}

/// Write the one PHI-free event-outbox row a single-object commit announces,
/// when eventing is enabled (our own extension; no openEHR spec governs it — the
/// row is skipped entirely, envelope included, when no consumer is configured).
async fn write_single_outbox(
    tx: &mut PgConnection,
    ctx: &SigningCtx<'_>,
    contribution_id: Uuid,
    ehr_id: Option<Uuid>,
    committed: &Committed,
) -> Result<(), ServiceError> {
    if ctx.outbox_enabled {
        crate::storage::version_repo::commit::write_outbox(
            tx,
            contribution_id,
            ehr_id,
            committed.time_committed,
            vec![committed.envelope_entry()],
        )
        .await?;
    }
    Ok(())
}

/// Create the first version of a new versioned object under its own
/// contribution.
///
/// # Errors
/// [`ServiceError::Unprocessable`] for an out-of-group / non-first lifecycle
/// state; the [`commit_resolved`] storage/signing errors; a multimedia offload
/// failure as [`ServiceError::Internal`].
#[allow(clippy::too_many_arguments)] // the write parameters; a struct would not read clearer
pub(crate) async fn create(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    kind: Kind,
    canonical: Value,
    template_id: Option<&str>,
    audit: &AuditInput,
    envelope: WriteEnvelope,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (committed, contribution_id) = apply_change(
        tx,
        ehr_id,
        ContributionCtx::New,
        audit,
        ctx,
        &audit.committer,
        None,
        Change::Create {
            kind,
            canonical,
            template_id: template_id.map(str::to_owned),
            signature: envelope.signature,
            lifecycle_state: envelope.lifecycle_state,
            attestations: envelope.attestations,
        },
    )
    .await?;
    write_single_outbox(tx, ctx, contribution_id, ehr_id, &committed).await?;
    Ok(committed)
}

/// Commit a new version of an existing object under its own contribution.
///
/// # Errors
/// [`ServiceError::NotFound`] when `(ehr_id, kind, vo_id)` does not address a
/// stored object; [`ServiceError::VersionConflict`] when `expected` is not the
/// open lineage tip; [`ServiceError::Unprocessable`] for an illegal lifecycle
/// transition; plus the [`commit_resolved`] storage/signing errors.
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
    envelope: WriteEnvelope,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (committed, contribution_id) = apply_change(
        tx,
        ehr_id,
        ContributionCtx::New,
        audit,
        ctx,
        &audit.committer,
        None,
        Change::Modify {
            vo_id,
            kind,
            canonical,
            expected,
            template_id: template_id.map(str::to_owned),
            signature: envelope.signature,
            lifecycle_state: envelope.lifecycle_state,
            attestations: envelope.attestations,
            other_input_version_uids: Vec::new(),
        },
    )
    .await?;
    write_single_outbox(tx, ctx, contribution_id, ehr_id, &committed).await?;
    Ok(committed)
}

/// Logically delete an object under its own contribution (master06 §Logical
/// Deletion).
///
/// # Errors
/// [`ServiceError::NotFound`] when `(ehr_id, kind, vo_id)` does not address a
/// stored object; [`ServiceError::VersionConflict`] when `expected` is not the
/// open lineage tip; plus the [`commit_resolved`] storage/signing errors.
#[allow(clippy::too_many_arguments)] // the write parameters; a struct would not read clearer
pub(crate) async fn delete(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<TreeId>,
    audit: &AuditInput,
    envelope: WriteEnvelope,
    ctx: &SigningCtx<'_>,
) -> Result<Committed, ServiceError> {
    let (committed, contribution_id) = apply_change(
        tx,
        ehr_id,
        ContributionCtx::New,
        audit,
        ctx,
        &audit.committer,
        None,
        Change::Delete {
            vo_id,
            kind,
            expected,
            signature: envelope.signature,
        },
    )
    .await?;
    write_single_outbox(tx, ctx, contribution_id, ehr_id, &committed).await?;
    Ok(committed)
}

/// Commit a set of changes atomically under one CONTRIBUTION (RM common
/// master06 §Committal and Audits — "similar to nested transactions", S-17).
/// `contribution_audit` is the CONTRIBUTION's own audit; each change carries its
/// VERSION `commit_audit`. `attests` are `666|attestation|` items — new
/// `ATTESTATION`s attached to **existing** versions, committed in the
/// same transaction but adding no new version.
///
/// master06 §Committal (m4): a version item that omits `committer`/`system_id`
/// inherits them from the CONTRIBUTION audit — realized by the callers
/// building each version `AuditInput`; the attestation committer likewise
/// defaults to the CONTRIBUTION committer here.
///
/// # Errors
/// The per-change [`apply_change`] errors; the attestation-completion
/// `Unprocessable` rejections and the `NotFound` of a missing attestation
/// target; the storage errors of the contribution/outbox writes.
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
        crate::storage::version_repo::commit::write_contribution(
            tx,
            ehr_id,
            &contribution_audit.row(),
            supplied_uid,
        )
        .await?;
    let committer_fallback = &contribution_audit.committer;
    let mut committed = Vec::with_capacity(changes.len() + attests.len());
    for (version_audit, change) in changes {
        // Each change writes its own `commit_audit` + `vo_version` under the
        // shared contribution, always through the folded CTE — the commit
        // instant is the contribution's transaction timestamp (one `now()`
        // for the whole set), so the signature is computable up front.
        let (change_committed, _cid) = apply_change(
            tx,
            ehr_id,
            ContributionCtx::Existing(contribution_id),
            &version_audit,
            ctx,
            committer_fallback,
            Some(contribution_time),
            change,
        )
        .await?;
        committed.push(change_committed);
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
    // One PHI-free outbox event for the whole CONTRIBUTION (same transaction),
    // skipped entirely — including the envelope collection — when no eventing
    // consumer is configured (our own extension; no openEHR spec governs it).
    if ctx.outbox_enabled {
        let versions = committed.iter().map(Committed::envelope_entry).collect();
        crate::storage::version_repo::commit::write_outbox(
            tx,
            contribution_id,
            ehr_id,
            contribution_time,
            versions,
        )
        .await?;
    }
    Ok((contribution_id, committed))
}
