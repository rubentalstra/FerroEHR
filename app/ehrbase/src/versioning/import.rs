//! Import: replaying received `ORIGINAL_VERSION`s into the local store as
//! `IMPORTED_VERSION`s (S-14, S-36).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Copying / §Committal
//! and Audits. Each received original is committed locally wrapped in an
//! `IMPORTED_VERSION` whose **own** contribution records the local act of
//! committal, while the wrapped original — its 3-part identity, `commit_audit`,
//! `lifecycle_state`, data, `signature`, `attestations` — is preserved verbatim
//! ("the `ORIGINAL_VERSION` instance is never modified", master06 §Copying).
//!
//! PORT NOTE (G-03, master06 §Committal): the greenfield store holds one row per
//! version (identity + `commit_audit` + data), not a distinct
//! `IMPORTED_VERSION` wrapper object; the served form is the `ORIGINAL_VERSION`.
//! The "import" is expressed as (a) the preserved original `commit_audit` +
//! 3-part version identity and (b) a fresh local import CONTRIBUTION recording
//! the local committal — which is exactly what an `IMPORTED_VERSION`'s own
//! contribution/`commit_audit` denote. master06 §Committal sanctions a
//! non-distributed holder keeping only the `ORIGINAL_VERSION` content; the one
//! visible deviation is that the served `ORIGINAL_VERSION.contribution`
//! references the local import contribution, not the (foreign) source
//! contribution.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::service::ServiceError;
use crate::storage::decompose;
use crate::versioning::Kind;
use crate::versioning::audit::AuditInput;
use crate::versioning::change::NewVersionRow;
use crate::versioning::object_version_id::TreeId;

/// A lineage key: the trunk (`("", 0, 0)`) or one specific branch of one system
/// (`(creating_system_id, trunk_version, branch_number)`). Versions on the same
/// lineage supersede each other; distinct lineages coexist.
pub(crate) type Lineage = (String, i32, i32);

/// The current state of a to-be-imported container in the target store —
/// mapped from the storage aggregate read
/// (`crate::storage::version_repo::imported_container_state`).
#[derive(Debug, Clone, Default)]
pub(crate) struct ContainerState {
    /// The stored kind, if the `vo_id` already exists.
    pub(crate) kind: Option<Kind>,
    /// The owning EHR of the existing container.
    pub(crate) owner: Option<Uuid>,
    /// The highest trunk version currently held.
    pub(crate) max_trunk: i32,
    /// The highest storage ordinal currently held.
    pub(crate) max_ordinal: i32,
    /// Whether a still-open current TRUNK version exists.
    pub(crate) trunk_open: bool,
}

/// Read + map the container state through the storage row I/O; an existing
/// container with an unrecognized stored kind is a server fault.
async fn container_state(
    tx: &mut PgConnection,
    vo_id: Uuid,
) -> Result<ContainerState, ServiceError> {
    let row = crate::storage::version_repo::imported_container_state(tx, vo_id).await?;
    let kind = match row.kind {
        None => None,
        Some(text) => Some(Kind::from_type(&text).ok_or_else(|| {
            ServiceError::Internal(format!("unknown versioned-object kind {text:?}"))
        })?),
    };
    Ok(ContainerState {
        kind,
        owner: row.owner,
        max_trunk: row.max_trunk,
        max_ordinal: row.max_ordinal,
        trunk_open: row.trunk_open,
    })
}

/// One received `ORIGINAL_VERSION` to import into the local store (SM
/// `I_EHR_EXTRACT_SERVICE.import_ehr`; master06 §Copying).
#[derive(Debug)]
pub(crate) struct ImportVersion {
    /// The `version_tree_id` of the wrapped original (trunk or branch; branch
    /// import is first-class — master06 §Version tree).
    pub(crate) tree: TreeId,
    /// The wrapped original's `creating_system_id` — per VERSION, not per
    /// container: a copied version tree legitimately mixes systems (master06
    /// §Distributed Versioning).
    pub(crate) creating_system_id: String,
    /// The wrapped original's `preceding_version_uid`, preserved verbatim.
    pub(crate) preceding_version_uid: Option<String>,
    /// The wrapped original's `other_input_version_uids` (merge provenance),
    /// preserved verbatim.
    pub(crate) other_input_version_uids: Vec<String>,
    /// The wrapped original's resolved `version_lifecycle_state` code.
    pub(crate) lifecycle_state: String,
    /// The wrapped original's `commit_audit`, preserved verbatim.
    pub(crate) commit_audit: AuditInput,
    /// The wrapped original's `commit_audit.time_committed`, preserved verbatim.
    pub(crate) commit_time: jiff::Timestamp,
    /// The version data (`Value::Null` for a `523|deleted|` version — no nodes).
    pub(crate) data: Value,
    /// The wrapped original's `VERSION.signature` (preserved, never re-signed).
    pub(crate) signature: Option<String>,
    /// The wrapped original's `ATTESTATION`s (already full RM objects),
    /// preserved.
    pub(crate) attestations: Vec<Value>,
}

impl ImportVersion {
    /// The lineage this version sits on.
    fn lineage(&self) -> Lineage {
        match self.tree.branch {
            None => (String::new(), 0, 0),
            Some((b, _)) => (self.creating_system_id.clone(), self.tree.trunk, b),
        }
    }
}

/// One versioned object (a source `VERSIONED_OBJECT`) to import: its cloned
/// `vo_id` (the received `uid.object_id()` — master06 §Copying: "a new
/// `VERSIONED_OBJECT` is created, with its uid set to the same value as the
/// received `VERSION._uid.object_id()`"), its kind, and its versions.
pub(crate) struct ImportContainer {
    pub(crate) vo_id: Uuid,
    pub(crate) kind: Kind,
    pub(crate) versions: Vec<ImportVersion>,
}

/// Replay a set of received `ORIGINAL_VERSION`s into an EHR as
/// `IMPORTED_VERSION`s under **one** local import CONTRIBUTION (master06
/// §Copying, §Committal). The `import_audit` records the local act of committal
/// (`249|creation|`, master06 §Contributions "import of item").
pub(crate) async fn commit_import(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
    outbox_enabled: bool,
) -> Result<Uuid, ServiceError> {
    commit_import_scoped(
        tx,
        Some(ehr_id),
        import_audit,
        containers,
        false,
        outbox_enabled,
    )
    .await
}

/// Land demographics-chapter parties into the demographic repository under their
/// own (ehr-less) import CONTRIBUTION (master09 §Creation Semantics — demographic
/// content is not EHR-owned). A party whose version container already exists
/// locally is SKIPPED — parties are shared continuants across extracts.
pub(crate) async fn commit_demographic_import(
    tx: &mut PgConnection,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
    outbox_enabled: bool,
) -> Result<(), ServiceError> {
    if containers.is_empty() {
        return Ok(());
    }
    let mut fresh = Vec::with_capacity(containers.len());
    for container in containers {
        let state = container_state(tx, container.vo_id).await?;
        if state.kind.is_none() {
            fresh.push(container);
        }
    }
    if fresh.is_empty() {
        return Ok(());
    }
    commit_import_scoped(tx, None, import_audit, fresh, true, outbox_enabled).await?;
    Ok(())
}

/// PORT NOTE (local temporal periods, master06 §Copying): all versions of an
/// imported container are committed in the single local import act, so they get
/// a synthetic strictly-increasing local `sys_period` chain (base = import time,
/// 1 µs steps) **per lineage** with only each lineage's highest version open.
/// The true source chronology is preserved in each version's
/// `commit_audit.time_committed`.
#[allow(clippy::too_many_lines)] // one linear import transaction; splitting would obscure the replay order
async fn commit_import_scoped(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
    skip_existing: bool,
    outbox_enabled: bool,
) -> Result<Uuid, ServiceError> {
    // One local instant anchors the whole import's temporal chain.
    let base = jiff::Timestamp::now();
    let (contribution_audit_id, import_time) =
        crate::storage::version_repo::insert_audit(tx, &import_audit.row()).await?;
    let contribution_id =
        crate::storage::version_repo::insert_contribution(tx, ehr_id, contribution_audit_id)
            .await?;
    let mut outbox_versions: Vec<Value> = Vec::new();

    for mut container in containers {
        if container.versions.is_empty() {
            continue;
        }
        // Version-tree order; reject a duplicated version identity within one
        // import (the identity tuple is {object_id, creating_system_id,
        // version_tree_id} — master06 §Distributed Versioning).
        container
            .versions
            .sort_by_key(|v| (v.lineage(), v.tree.columns()));
        for pair in container.versions.windows(2) {
            if pair[0].creating_system_id == pair[1].creating_system_id
                && pair[0].tree == pair[1].tree
            {
                return Err(ServiceError::Conflict(format!(
                    "version {}::{}::{} appears more than once in the import",
                    container.vo_id, pair[0].creating_system_id, pair[0].tree
                )));
            }
        }

        let state = container_state(tx, container.vo_id).await?;
        if skip_existing && state.kind.is_some() {
            continue;
        }
        if let Some(existing_kind) = state.kind {
            // First receipt already happened (master06 §Copying Case 3): append
            // to the existing clone — the kind must match, the EHR must own it,
            // and every imported trunk version must be strictly newer.
            if state.owner != ehr_id {
                return Err(ServiceError::Conflict(format!(
                    "versioned object {} already exists in another EHR",
                    container.vo_id
                )));
            }
            if existing_kind != container.kind {
                return Err(ServiceError::Conflict(format!(
                    "versioned object {} is a {}, cannot import a {}",
                    container.vo_id,
                    existing_kind.as_str(),
                    container.kind.as_str()
                )));
            }
            if let Some(first_trunk) = container
                .versions
                .iter()
                .filter(|v| v.tree.is_trunk())
                .map(|v| v.tree.trunk)
                .min()
                && first_trunk <= state.max_trunk
            {
                return Err(ServiceError::Conflict(format!(
                    "versioned object {} already has trunk version {} — cannot \
                     re-import trunk version {first_trunk}",
                    container.vo_id, state.max_trunk
                )));
            }
            if state.trunk_open && container.versions.iter().any(|v| v.tree.is_trunk()) {
                crate::storage::version_repo::close_lineage_at(
                    tx,
                    container.vo_id,
                    &(String::new(), 0, 0),
                    base,
                )
                .await?;
            }
        } else if container.kind == Kind::Folder
            && let Some(ehr_id) = ehr_id
        {
            // A first-received FOLDER container is a new folder hierarchy of the
            // EHR (RM ehr master04 §Folders; master06 §Copying Case 2).
            crate::storage::version_repo::insert_ehr_folder_rank(tx, ehr_id, container.vo_id)
                .await?;
        }

        // Per-lineage period chains: within a lineage each version closes its
        // predecessor; each lineage's last version stays open. Lineages coexist.
        let mut ordinal = state.max_ordinal;
        let versions = container.versions;
        for (i, version) in versions.iter().enumerate() {
            ordinal += 1;
            // PHI-free outbox entry: identity + provenance only; no template_id.
            outbox_versions.push(serde_json::json!({
                "vo_id": container.vo_id,
                "kind": container.kind.as_str(),
                "sys_version": ordinal,
                "version_tree_id": version.tree.to_string(),
                "change_type": version.commit_audit.change_type,
                "template_id": Value::Null,
            }));
            // Synthetic strictly-increasing local period; the next version ON
            // THE SAME LINEAGE (if any) closes this one.
            let lower = base + jiff::SignedDuration::from_micros(i64::try_from(i).unwrap_or(0));
            let upper = versions[i + 1..]
                .iter()
                .position(|later| later.lineage() == version.lineage())
                .map(|offset| {
                    base + jiff::SignedDuration::from_micros(
                        i64::try_from(i + 1 + offset).unwrap_or(0),
                    )
                });
            // Preserves the source commit time verbatim — master06 §Copying
            // ("the ORIGINAL_VERSION instance is never modified").
            let audit_id = crate::storage::version_repo::insert_audit_at(
                tx,
                &version.commit_audit.row(),
                version.commit_time,
            )
            .await?;
            let row = NewVersionRow {
                vo_id: container.vo_id,
                kind: container.kind,
                ehr_id,
                ordinal,
                tree: version.tree,
                lifecycle_state: &version.lifecycle_state,
                creating_system_id: &version.creating_system_id,
                preceding_version_uid: version.preceding_version_uid.as_deref(),
                other_input_version_uids: &version.other_input_version_uids,
                contribution_id,
                audit_id,
                signature: version.signature.as_deref(),
            };
            crate::storage::version_repo::insert_imported_vo_version(
                tx,
                &row.imported_row(lower, upper),
            )
            .await?;
            // A `523|deleted|` version stores no node rows (data is Void).
            if !version.data.is_null() {
                let rows = decompose(version.data.clone())?;
                crate::storage::node_repo::write_nodes(tx, container.vo_id, ordinal, ehr_id, &rows)
                    .await?;
            }
            for attestation in &version.attestations {
                crate::storage::version_repo::insert_attestation(
                    tx,
                    container.vo_id,
                    ordinal,
                    contribution_id,
                    attestation,
                )
                .await?;
            }
        }
    }
    if outbox_enabled && !outbox_versions.is_empty() {
        crate::storage::version_repo::write_outbox(
            tx,
            contribution_id,
            ehr_id,
            import_time,
            outbox_versions,
        )
        .await?;
    }
    Ok(contribution_id)
}
