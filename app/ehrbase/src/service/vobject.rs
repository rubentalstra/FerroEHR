//! The shared versioned-object machinery: persist and load COMPOSITION /
//! `EHR_STATUS` / FOLDER uniformly (ADR-008). All writes run inside a caller-owned
//! `sqlx` transaction so a version + its nodes + the contribution + the audit
//! commit atomically.

use crate::signing::Signer;
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, PgPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::storage::{NodeRow, decompose, reassemble};

use super::ServiceError;
use super::codes::lifecycle;
use super::version_id::TreeId;
use super::versioned::build_original_version;

/// The signing context threaded into the shared commit path so every
/// versioned-object write signs its `ORIGINAL_VERSION` (RM common §"Digital
/// Signature"; `docs/design/version-signing.md` §3.3). Borrows the service's
/// system id and configured [`Signer`].
pub(super) struct SigningCtx<'a> {
    /// The effective openEHR `system_id` for this write — the current tenant's
    /// own id when tenancy is on (ADR-015 §1), else the service default. Owned
    /// because it may come from the per-request tenant context, not `&self`.
    pub(super) system_id: String,
    pub(super) signer: &'a Signer,
    /// The optional `DV_MULTIMEDIA` externalization engine (ADR-017). When set,
    /// [`apply_change`] offloads large inline `DV_MULTIMEDIA.data` to object
    /// storage before the canonical body is decomposed (and signed), so the
    /// stored/served/signed form is the externalized one. `None` = inline
    /// behaviour unchanged.
    pub(super) multimedia: Option<&'a crate::multimedia::MultimediaEngine>,
}

/// The kind of versioned object (discriminates `vo_version.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Composition,
    EhrStatus,
    /// The EHR-wide access-control object created with the EHR (RM ehr
    /// §"EHR Creation") and versioned "via the normal mechanism"
    /// (RM ehr §"EHR Access").
    EhrAccess,
    Folder,
    // Demographic party roots (ADR-008). These are versioned objects with no
    // EHR scope: they use the same `vo_version`/`node` machinery with a NULL
    // `ehr_id`.
    Agent,
    Group,
    Organisation,
    Person,
    Role,
    /// A demographic `PARTY_RELATIONSHIP` (RM demographic): a versioned object
    /// with no EHR scope, like the party roots, but *not* a PARTY — it has its
    /// own `versioned_party_relationship` read surface (SM-3,
    /// `i_party_relationship.adoc`).
    PartyRelationship,
}

impl Kind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Kind::Composition => "COMPOSITION",
            Kind::EhrStatus => "EHR_STATUS",
            Kind::EhrAccess => "EHR_ACCESS",
            Kind::Folder => "FOLDER",
            Kind::Agent => "AGENT",
            Kind::Group => "GROUP",
            Kind::Organisation => "ORGANISATION",
            Kind::Person => "PERSON",
            Kind::Role => "ROLE",
            Kind::PartyRelationship => "PARTY_RELATIONSHIP",
        }
    }

    /// Whether this kind is a demographic party root (no EHR scope). This is the
    /// `/versioned_party` read scope — a `PARTY_RELATIONSHIP` is *not* a party.
    pub(super) fn is_party(self) -> bool {
        matches!(
            self,
            Kind::Agent | Kind::Group | Kind::Organisation | Kind::Person | Kind::Role
        )
    }

    /// Whether this kind is a demographic versioned object (no EHR scope): the
    /// five party roots plus `PARTY_RELATIONSHIP`. Gates the ehr-less
    /// contribution scope (`check_kind_scope`) — a demographic CONTRIBUTION may
    /// carry parties and relationships, an EHR one may carry neither.
    pub(super) fn is_demographic(self) -> bool {
        self.is_party() || self == Kind::PartyRelationship
    }

    /// The versioned-object kind for an RM `_type`, if it is a versioned root.
    pub(super) fn from_type(rm_type: &str) -> Option<Self> {
        match rm_type {
            "COMPOSITION" => Some(Kind::Composition),
            "EHR_STATUS" => Some(Kind::EhrStatus),
            "EHR_ACCESS" => Some(Kind::EhrAccess),
            "FOLDER" => Some(Kind::Folder),
            "AGENT" => Some(Kind::Agent),
            "GROUP" => Some(Kind::Group),
            "ORGANISATION" => Some(Kind::Organisation),
            "PERSON" => Some(Kind::Person),
            "ROLE" => Some(Kind::Role),
            "PARTY_RELATIONSHIP" => Some(Kind::PartyRelationship),
            _ => None,
        }
    }
}

/// The kind of the current version of an object, or `None` if it does not exist.
pub(super) async fn object_kind(pool: &PgPool, vo_id: Uuid) -> Result<Option<Kind>, ServiceError> {
    let row = sqlx::query(
        "SELECT kind FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period) \
         AND branch_number = 0",
    )
    .bind(vo_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row {
        Some(r) => Kind::from_type(&r.try_get::<String, _>("kind")?),
        None => None,
    })
}

/// What an audit row records about a committed change.
#[derive(Debug, Clone)]
pub(super) struct AuditInput {
    pub(super) system_id: String,
    /// The numeric `audit_change_type` group code (`249`/`251`/`523`/…) — never
    /// a rubric string (RM `AUDIT_DETAILS.Change_type_valid`).
    pub(super) change_type: String,
    pub(super) description: Option<String>,
    /// Canonical `PARTY_PROXY` of the committer.
    pub(super) committer: serde_json::Value,
}

/// The outcome of a versioned-object write: the object id, the new version's
/// tree id, and the CONTRIBUTION that produced it.
#[derive(Debug, Clone)]
pub(super) struct Committed {
    pub(super) vo_id: Uuid,
    /// The per-vo storage commit ordinal of the written row (the node /
    /// attestation key) — NOT the wire version number.
    pub(super) sys_version: i32,
    /// The new version's `VERSION_TREE_ID` (the wire version identity).
    pub(super) tree: TreeId,
    /// The `creating_system_id` recorded for the new version (the local system
    /// for every non-import write) — the `OBJECT_VERSION_ID` middle part.
    pub(super) creating_system_id: String,
    /// The CONTRIBUTION this write created. Read by `commit_contribution` (to
    /// group versions) and the create-response `Location`; retained as part of
    /// the write result.
    #[allow(dead_code)]
    pub(super) contribution_id: Uuid,
    /// The versioned-object kind of this write — carried for the event-outbox
    /// envelope (ADR-014 §2).
    pub(super) kind: Kind,
    /// The numeric `audit_change_type` group code recorded for this version
    /// (`249`/`251`/`523`/`666`…) — carried for the outbox envelope (ADR-014 §2).
    pub(super) change_type: String,
    /// The OPT `template_id` a COMPOSITION was committed against (`None` for
    /// `EHR_STATUS`/FOLDER/deletes/attestations) — carried for the outbox
    /// envelope + routing key (ADR-014 §2/§5).
    pub(super) template_id: Option<String>,
}

impl Committed {
    /// The per-version entry for the PHI-free event-outbox envelope
    /// (ADR-014 §2): identity + provenance metadata only, never clinical content.
    fn envelope_entry(&self) -> serde_json::Value {
        serde_json::json!({
            "vo_id": self.vo_id,
            "kind": self.kind.as_str(),
            "sys_version": self.sys_version,
            "change_type": self.change_type,
            "template_id": self.template_id,
        })
    }
}

/// A loaded version: its full provenance metadata and reassembled canonical JSON.
#[derive(Debug, Clone)]
pub(super) struct VersionRead {
    pub(super) vo_id: Uuid,
    /// The owning EHR, or `None` for a demographic party (no EHR scope —
    /// ADR-008). EHR-scoped callers compare against `Some(ehr_id)`.
    pub(super) ehr_id: Option<Uuid>,
    /// The version's `VERSION_TREE_ID` (the wire version identity).
    pub(super) tree: TreeId,
    /// The stored `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first
    /// version).
    pub(super) preceding_version_uid: Option<String>,
    /// The stored `ORIGINAL_VERSION.other_input_version_uids` (merge
    /// provenance; empty when not a merge — `Is_merged_validity`).
    pub(super) other_input_version_uids: Vec<String>,
    /// The version's lifecycle-state numeric code (`version_lifecycle_state`
    /// group: `532` complete, `523` deleted, …).
    pub(super) lifecycle_state: String,
    /// The stored `creating_system_id` — the immutable identity of the system
    /// that created this version (RM common master06 §"Distributed
    /// Versioning"), forming the middle part of its `OBJECT_VERSION_ID`. Empty
    /// for versions written before this column existed (the legacy sentinel);
    /// callers fall back to the service system id only then.
    pub(super) creating_system_id: String,
    pub(super) contribution_id: Uuid,
    /// The version's commit `AUDIT_DETAILS` provenance (mandatory
    /// `VERSION.commit_audit`, 1..1).
    pub(super) audit: AuditInput,
    /// When the version was committed (its audit `time_committed`).
    pub(super) time_committed: jiff::Timestamp,
    /// The stored `vo_version.template_id` (the OPT id a COMPOSITION was
    /// committed against), read back for the ABAC template attribute
    /// (`docs/enterprise/access-control.md` §6.2). `None` for versioned objects
    /// that carry no template (`EHR_STATUS`, FOLDER) or a delete.
    pub(super) template_id: Option<String>,
    /// The stored `VERSION.signature` (RM common §"Digital Signature"), or
    /// `None` for versions committed before signing was enabled (0..1).
    pub(super) signature: Option<String>,
    /// The reassembled canonical JSON, or `Value::Null` for a deleted version
    /// (a logical delete stores no node rows — RM `change_control` §"Logical
    /// Deletion").
    pub(super) canonical: serde_json::Value,
    /// The `ATTESTATION`s attached to this version, in commit order (RM common
    /// master06 §Attestation). Surfaced as `ORIGINAL_VERSION.attestations` on
    /// the read path (appended **after** signature verification — attestations
    /// arrive after committal and are not part of the signed canonical form).
    pub(super) attestations: Vec<serde_json::Value>,
}

impl VersionRead {
    /// Whether this version is logically deleted (`lifecycle_state` `523`).
    pub(super) fn deleted(&self) -> bool {
        self.lifecycle_state == lifecycle::DELETED
    }
}

/// Build a [`VersionRead`] from a `vo_version`⋈`audit` row, resolving the
/// canonical body: a deleted version (lifecycle `523`) carries no node rows, so
/// it is `Value::Null` and reassembly is skipped entirely (this is what stops a
/// deleted read from erroring on an empty node set — finding F-02-01).
async fn version_read(
    pool: &PgPool,
    vo_id: Uuid,
    row: &PgRow,
) -> Result<VersionRead, ServiceError> {
    let sys_version: i32 = row.try_get("sys_version")?;
    let tree = TreeId::from_columns(
        row.try_get("trunk_version")?,
        row.try_get("branch_number")?,
        row.try_get("branch_version")?,
    );
    let other_input_version_uids: Vec<String> = row
        .try_get::<Option<serde_json::Value>, _>("other_input_version_uids")?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let lifecycle_state: String = row.try_get("lifecycle_state")?;
    let canonical = if lifecycle_state == lifecycle::DELETED {
        serde_json::Value::Null
    } else {
        read_nodes(pool, vo_id, sys_version).await?
    };
    let attestations = read_attestations(pool, vo_id, sys_version).await?;
    Ok(VersionRead {
        vo_id,
        ehr_id: row.try_get("ehr_id")?,
        tree,
        preceding_version_uid: row.try_get("preceding_version_uid")?,
        other_input_version_uids,
        lifecycle_state,
        creating_system_id: row.try_get("creating_system_id")?,
        contribution_id: row.try_get("contribution_id")?,
        audit: AuditInput {
            system_id: row.try_get("system_id")?,
            change_type: row.try_get("change_type")?,
            description: row.try_get("description")?,
            committer: row.try_get("committer")?,
        },
        time_committed: row
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff(),
        template_id: row.try_get("template_id")?,
        signature: row.try_get("signature")?,
        canonical,
        attestations,
    })
}

/// Load the `ATTESTATION`s attached to one version, in commit order (RM common
/// master06 §Attestation). Ordered by `time_committed, id`: attestations
/// committed in the same transaction share `now()` (`transaction_timestamp`),
/// so the `uuidv7()` `id` breaks ties in insertion order.
async fn read_attestations(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<Vec<serde_json::Value>, ServiceError> {
    let rows = sqlx::query(
        "SELECT data FROM vo_attestation WHERE vo_id = $1 AND sys_version = $2 \
         ORDER BY time_committed, id",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok(row.try_get::<serde_json::Value, _>("data")?))
        .collect()
}

/// Insert an `audit` row, returning its id **and** the DB-assigned
/// `time_committed`. The timestamp is captured (`RETURNING`) so the commit path
/// can build the exact `ORIGINAL_VERSION` that will later be served — the signed
/// bytes must match the read-time canonical form (design §6.3).
async fn insert_audit(
    tx: &mut PgConnection,
    audit: &AuditInput,
) -> Result<(Uuid, jiff::Timestamp), ServiceError> {
    let row = sqlx::query(
        "INSERT INTO audit (system_id, change_type, description, committer) \
         VALUES ($1, $2, $3, $4) RETURNING id, time_committed",
    )
    .bind(&audit.system_id)
    .bind(&audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .fetch_one(&mut *tx)
    .await?;
    let id: Uuid = row.try_get("id")?;
    let time_committed = row
        .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
        .to_jiff();
    Ok((id, time_committed))
}

/// Insert a `contribution` row referencing its audit, returning its id.
/// `ehr_id` is `None` for a demographic CONTRIBUTION (no EHR scope).
async fn insert_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit_id: Uuid,
) -> Result<Uuid, ServiceError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO contribution (ehr_id, audit_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(ehr_id)
    .bind(audit_id)
    .fetch_one(&mut *tx)
    .await?)
}

/// Insert an `audit` row and its enclosing `contribution`, returning the
/// contribution id, the audit id, and the audit's `time_committed` (for the
/// version's `commit_audit`, which is signed).
async fn write_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    audit: &AuditInput,
) -> Result<(Uuid, Uuid, jiff::Timestamp), ServiceError> {
    let (audit_id, time_committed) = insert_audit(tx, audit).await?;
    let contribution_id = insert_contribution(tx, ehr_id, audit_id).await?;
    Ok((contribution_id, audit_id, time_committed))
}

/// Write the contribution-outbox event row **in the same transaction** as the
/// contribution it announces (ADR-014 §1: "no commit without its event; no
/// event without its commit"). The envelope is PHI-free (ADR-014 §2):
/// contribution id, `ehr_id`, `committed_at`, and one per-version entry of
/// identity + provenance metadata only — never clinical content. Every
/// CONTRIBUTION commit path (single-object writes, `commit_contribution`,
/// `commit_import`) funnels through here so the outbox and the commit are
/// atomic. Publishing is a separate concern (`crate::events`); this only
/// records the intent to publish.
async fn write_outbox(
    tx: &mut PgConnection,
    contribution_id: Uuid,
    ehr_id: Option<Uuid>,
    committed_at: jiff::Timestamp,
    versions: Vec<serde_json::Value>,
) -> Result<(), ServiceError> {
    let envelope = serde_json::json!({
        "contribution_id": contribution_id,
        "ehr_id": ehr_id,
        "committed_at": committed_at.to_string(),
        "versions": versions,
    });
    sqlx::query(
        "INSERT INTO event_outbox (contribution_id, ehr_id, envelope, committed_at) \
         VALUES ($1, $2, $3, $4::timestamptz)",
    )
    .bind(contribution_id)
    .bind(ehr_id)
    .bind(&envelope)
    .bind(committed_at.to_string())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Bulk-insert the decomposed node rows for one version.
async fn insert_nodes(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    ehr_id: Option<Uuid>,
    rows: &[NodeRow],
) -> Result<(), ServiceError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO node (vo_id, sys_version, num, num_cap, parent_num, citem_num, ehr_id, \
         rm_type, archetype, name, path, data) ",
    );
    qb.push_values(rows, |mut b, row| {
        b.push_bind(vo_id)
            .push_bind(sys_version)
            .push_bind(row.num)
            .push_bind(row.num_cap)
            .push_bind(row.parent_num)
            .push_bind(row.citem_num)
            .push_bind(ehr_id)
            .push_bind(&row.rm_type)
            .push_bind(&row.archetype)
            .push_bind(&row.name)
            .push_bind(&row.path)
            .push_bind(&row.data);
    });
    qb.build().execute(&mut *tx).await?;
    Ok(())
}

/// One change applied within a CONTRIBUTION (the openEHR change-set unit).
///
/// `signature` carries a **client-supplied** `UPDATE_VERSION.signature` (RM
/// common §"Digital Signature"; design §3.3): when present it is stored verbatim
/// and the server does not re-sign; when absent the server signs the assembled
/// `ORIGINAL_VERSION` if signing is enabled. The direct (non-CONTRIBUTION)
/// endpoints always pass `None` (server-side signing).
///
/// `lifecycle_state` on `Create`/`Modify` carries the client-supplied
/// `version_lifecycle_state` code from `UPDATE_VERSION.lifecycle_state` (RM
/// common master06 §"Version Lifecycle": `532|complete|`, `553|incomplete|`,
/// `800|inactive|`, `801|abandoned|` — `523|deleted|` is reserved to the
/// [`Change::Delete`] path). `None` defaults to `532|complete|`, so the direct
/// (non-CONTRIBUTION) endpoints — which carry no wire lifecycle — keep
/// committing `532` exactly as before. The value is validated against the
/// terminology group in [`apply_change`] (invalid → 422).
///
/// Merging: `ORIGINAL_VERSION.other_input_version_uids` (with `is_merged`
/// derived per `VERSION.Is_merged_validity`) is accepted on the CONTRIBUTION
/// wire and on import, stored, and served (RM common master06 §"Version
/// Merging"/§"Disjoint Merging"). Branch commits arise per master06
/// §"Distributed Versioning": modifying a version created by another system
/// forks a branch (`Change` carries no explicit branch request — branching is
/// the mandated consequence of the preceding version's provenance, and of a
/// branch-id `expected`/`preceding_version_uid`).
pub(super) enum Change {
    /// Create a new versioned object.
    Create {
        kind: Kind,
        canonical: serde_json::Value,
        template_id: Option<String>,
        signature: Option<String>,
        /// Client-supplied `version_lifecycle_state` code (see the enum doc);
        /// `None` → `532|complete|`.
        lifecycle_state: Option<String>,
        /// Wire `UPDATE_VERSION.attestations` committed with this version
        /// (partial `UPDATE_ATTESTATION`s; completed + stored after the
        /// version write, RM common master06 §Attestation "Signing content
        /// at committal"). Empty for the direct (non-CONTRIBUTION) writes.
        attestations: Vec<serde_json::Value>,
    },
    /// Commit a new version of an existing object.
    Modify {
        vo_id: Uuid,
        kind: Kind,
        canonical: serde_json::Value,
        expected: Option<TreeId>,
        template_id: Option<String>,
        signature: Option<String>,
        /// Client-supplied `version_lifecycle_state` code (see the enum doc);
        /// `None` → `532|complete|`.
        lifecycle_state: Option<String>,
        /// See [`Change::Create::attestations`].
        attestations: Vec<serde_json::Value>,
        /// Wire `ORIGINAL_VERSION.other_input_version_uids` — the merged-in
        /// version ids for a merge commit (master06 §Version Merging); empty
        /// for a plain modification.
        other_input_version_uids: Vec<String>,
    },
    /// Logically delete an object (a content-less `deleted` version).
    Delete {
        vo_id: Uuid,
        kind: Kind,
        expected: Option<TreeId>,
        signature: Option<String>,
    },
}

impl Change {
    /// The versioned-object [`Kind`] this change writes.
    pub(super) fn kind(&self) -> Kind {
        match *self {
            Change::Create { kind, .. }
            | Change::Modify { kind, .. }
            | Change::Delete { kind, .. } => kind,
        }
    }
}

/// Resolve a client-supplied `version_lifecycle_state` token into its canonical
/// numeric code, defaulting to `532|complete|` when absent (RM common master06
/// §"Version Lifecycle"). An out-of-group token is a `422`
/// (`ORIGINAL_VERSION.Lifecycle_state_valid`), naming the terminology group.
fn resolve_lifecycle(state: Option<String>) -> Result<String, ServiceError> {
    match state {
        Some(token) => super::codes::lifecycle_state_code(&token).ok_or_else(|| {
            ServiceError::Unprocessable(format!(
                "lifecycle_state {token:?} is not a code in the openEHR \
                 version_lifecycle_state group (ORIGINAL_VERSION.Lifecycle_state_valid)"
            ))
        }),
        None => Ok(lifecycle::COMPLETE.to_owned()),
    }
}

/// A `666|attestation|` of an **existing** `ORIGINAL_VERSION` committed within
/// a CONTRIBUTION (RM common master06 §Change Control: "a new ATTESTATION is
/// added to the attestations list of an existing `ORIGINAL_VERSION`" — adds no
/// new version). Carried alongside the [`Change`] set so it commits in the
/// same transaction / CONTRIBUTION.
pub(super) struct PendingAttest {
    pub(super) vo_id: Uuid,
    pub(super) kind: Kind,
    /// The target version to attest (from `preceding_version_uid` — trunk or
    /// branch).
    pub(super) expected: TreeId,
    /// The wire `UPDATE_ATTESTATION` partial (the version item's commit audit),
    /// completed into a full RM `ATTESTATION` at commit time.
    pub(super) partial: serde_json::Value,
}

/// Compute the `VERSION.signature` for a version about to be persisted
/// (RM common §"Digital Signature"; design §3.3).
///
/// A **client-supplied** signature (from the CONTRIBUTION `UPDATE_VERSION` path)
/// wins and is stored verbatim (never re-signed, never validated against our
/// canonical form — the author may use another agreed serialization). Otherwise,
/// when signing is enabled, the fully-assembled `ORIGINAL_VERSION` — the *exact*
/// value that will later be served (built by the shared [`build_original_version`]
/// so commit-time and read-time bytes match) — is signed via its
/// `canonical_form()`.
#[allow(clippy::too_many_arguments)] // the parts of an ORIGINAL_VERSION + signing context
fn sign_version(
    ctx: &SigningCtx<'_>,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
    vo_id: Uuid,
    tree: TreeId,
    preceding_uid: Option<&str>,
    contribution_id: Uuid,
    lifecycle_state: &str,
    data: &serde_json::Value,
    client_signature: Option<String>,
) -> Result<Option<String>, ServiceError> {
    if let Some(sig) = client_signature {
        return Ok(Some(sig));
    }
    if !ctx.signer.enabled() {
        return Ok(None);
    }
    let ov = build_original_version(
        &ctx.system_id,
        vo_id,
        tree,
        preceding_uid,
        &[],
        contribution_id,
        audit,
        &time_committed,
        lifecycle_state,
        data,
        None,
    );
    let canonical = openehr_rm::common::change_control::version_impl::canonical_form_of_json(&ov)
        .map_err(|e| ServiceError::Signing(e.to_string()))?;
    let signature = ctx
        .signer
        .sign(&canonical)
        .map_err(|e| ServiceError::Signing(e.to_string()))?;
    Ok(Some(signature))
}

/// The core write path shared by single-object writes and CONTRIBUTION commits:
/// apply one [`Change`] under an already-open contribution + version audit,
/// signing the assembled `ORIGINAL_VERSION` (RM common §"Digital Signature").
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the three change arms + commit context
async fn apply_change(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    contribution_id: Uuid,
    audit_id: Uuid,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
    ctx: &SigningCtx<'_>,
    committer_fallback: &serde_json::Value,
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
            if kind == Kind::EhrStatus
                && let Some(ehr_id) = ehr_id
            {
                sync_ehr_subject(&mut *tx, ehr_id, &canonical).await?;
            }
            // Externalize large inline DV_MULTIMEDIA before decompose/sign, so
            // the stored, served and signed form is the offloaded one (ADR-017).
            if let Some(engine) = ctx.multimedia {
                engine
                    .offload(&mut canonical)
                    .await
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
            }
            let lifecycle = resolve_lifecycle(lifecycle_state)?;
            let rows = decompose(canonical)?;
            let vo_id = Uuid::now_v7();
            // Sign the exact data that will be served on read (reassembled from
            // the stored nodes) so the digest recomputes at read time (§6.3).
            let served = reassemble(&rows)?;
            let signature = sign_version(
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
            insert_vo_version(
                tx,
                NewVersionRow {
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
                },
            )
            .await?;
            insert_nodes(tx, vo_id, 1, ehr_id, &rows).await?;
            insert_accompanying_attestations(
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
            record_composition_commit(kind, "creation");
            Ok(Committed {
                vo_id,
                sys_version: 1,
                tree: TreeId::trunk(1),
                creating_system_id: ctx.system_id.clone(),
                contribution_id,
                kind,
                change_type: audit.change_type.clone(),
                template_id,
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
            if kind == Kind::EhrStatus
                && let Some(ehr_id) = ehr_id
            {
                sync_ehr_subject(&mut *tx, ehr_id, &canonical).await?;
            }
            if kind == Kind::Composition {
                check_versioned_composition_invariants(&mut *tx, vo_id, &canonical).await?;
            }
            // Externalize large inline DV_MULTIMEDIA before decompose/sign (ADR-017).
            if let Some(engine) = ctx.multimedia {
                engine
                    .offload(&mut canonical)
                    .await
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
            }
            let lifecycle = resolve_lifecycle(lifecycle_state)?;
            let rows = decompose(canonical)?;
            let next =
                next_version(&mut *tx, ehr_id, vo_id, kind, expected, &ctx.system_id).await?;
            if let Some(close_ordinal) = next.close_ordinal {
                close_ordinal_at_now(&mut *tx, vo_id, close_ordinal).await?;
            }
            let served = reassemble(&rows)?;
            let signature = sign_version(
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
            insert_vo_version(
                tx,
                NewVersionRow {
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
                },
            )
            .await?;
            insert_nodes(tx, vo_id, next.ordinal, ehr_id, &rows).await?;
            insert_accompanying_attestations(
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
            record_composition_commit(kind, "modification");
            Ok(Committed {
                vo_id,
                sys_version: next.ordinal,
                tree: next.tree,
                creating_system_id: ctx.system_id.clone(),
                contribution_id,
                kind,
                change_type: audit.change_type.clone(),
                template_id,
            })
        }
        Change::Delete {
            vo_id,
            kind,
            expected,
            signature,
        } => {
            let next =
                next_version(&mut *tx, ehr_id, vo_id, kind, expected, &ctx.system_id).await?;
            if let Some(close_ordinal) = next.close_ordinal {
                close_ordinal_at_now(&mut *tx, vo_id, close_ordinal).await?;
            }
            // A deleted version carries no data — its `ORIGINAL_VERSION.data` is
            // Void (RM change_control §"Logical Deletion"); the signature is
            // over the data-less version wrapper.
            let signature = sign_version(
                ctx,
                audit,
                time_committed,
                vo_id,
                next.tree,
                Some(&next.preceding_uid),
                contribution_id,
                lifecycle::DELETED,
                &serde_json::Value::Null,
                signature,
            )?;
            insert_vo_version(
                tx,
                NewVersionRow {
                    vo_id,
                    kind,
                    ehr_id,
                    ordinal: next.ordinal,
                    tree: next.tree,
                    lifecycle_state: lifecycle::DELETED,
                    creating_system_id: &ctx.system_id,
                    preceding_version_uid: Some(&next.preceding_uid),
                    other_input_version_uids: &[],
                    contribution_id,
                    audit_id,
                    template_id: None,
                    signature: signature.as_deref(),
                },
            )
            .await?;
            record_composition_commit(kind, "deletion");
            Ok(Committed {
                vo_id,
                sys_version: next.ordinal,
                tree: next.tree,
                creating_system_id: ctx.system_id.clone(),
                contribution_id,
                kind,
                change_type: audit.change_type.clone(),
                template_id: None,
            })
        }
    }
}

/// Insert one `ATTESTATION` row for a version (RM common master06 §Change
/// Control). Stores the completed canonical `ATTESTATION` verbatim in `data`
/// (ADR-008: no synthetic fields); `vo_attestation.time_committed` takes the
/// transaction timestamp (`now()`), which equals the `data.time_committed`
/// stamped by [`super::contribution::complete_attestation`] with the same
/// commit-act time.
async fn insert_attestation(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    contribution_id: Uuid,
    data: &serde_json::Value,
) -> Result<(), ServiceError> {
    sqlx::query(
        "INSERT INTO vo_attestation (vo_id, sys_version, contribution_id, data) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(vo_id)
    .bind(sys_version)
    .bind(contribution_id)
    .bind(data)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Complete + persist the attestations committed together with a NEW version
/// (`UPDATE_VERSION.attestations`; RM common master06 §Attestation "Signing
/// content at committal"). Each partial `UPDATE_ATTESTATION` is completed into
/// a full RM `ATTESTATION` and attached to the just-written version — same
/// transaction, so the version + its attestations commit atomically.
#[allow(clippy::too_many_arguments)] // the parts of an ATTESTATION + its target version
async fn insert_accompanying_attestations(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    contribution_id: Uuid,
    system_id: &str,
    committer_fallback: &serde_json::Value,
    now: jiff::Timestamp,
    partials: &[serde_json::Value],
) -> Result<(), ServiceError> {
    for partial in partials {
        let full =
            super::contribution::complete_attestation(partial, system_id, committer_fallback, now)?;
        insert_attestation(tx, vo_id, sys_version, contribution_id, &full).await?;
    }
    Ok(())
}

/// Attach an `ATTESTATION` to an **existing** `ORIGINAL_VERSION` (a
/// `666|attestation|` version item of a CONTRIBUTION; RM common master06
/// §Change Control: "a new ATTESTATION is added to the attestations list of an
/// existing `ORIGINAL_VERSION`" — no new version, `sys_period` untouched).
///
/// Realizes `VERSIONED_OBJECT.commit_attestation` precondition `has_version_id`
/// (`versioned_object.adoc`): the target `(vo_id, sys_version)` must exist and
/// belong to `ehr_id`, else [`ServiceError::NotFound`] naming the version uid.
/// `attestation` is the already-completed full RM `ATTESTATION`.
async fn attest(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: TreeId,
    attestation: &serde_json::Value,
    contribution_id: Uuid,
) -> Result<Committed, ServiceError> {
    let (t, b, v) = expected.columns();
    let row = sqlx::query(
        "SELECT ehr_id, sys_version, creating_system_id FROM vo_version \
         WHERE vo_id = $1 AND trunk_version = $2 AND branch_number = $3 \
         AND branch_version = $4 AND kind = $5",
    )
    .bind(vo_id)
    .bind(t)
    .bind(b)
    .bind(v)
    .bind(kind.as_str())
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        return Err(ServiceError::NotFound(format!(
            "{} version {vo_id}::{expected}",
            kind.as_str()
        )));
    };
    if row.try_get::<Option<Uuid>, _>("ehr_id")? != ehr_id {
        return Err(ServiceError::NotFound(format!(
            "{} version {vo_id}::{expected} in EHR {ehr_id:?}",
            kind.as_str()
        )));
    }
    let ordinal: i32 = row.try_get("sys_version")?;
    insert_attestation(tx, vo_id, ordinal, contribution_id, attestation).await?;
    Ok(Committed {
        vo_id,
        sys_version: ordinal,
        tree: expected,
        creating_system_id: row.try_get("creating_system_id")?,
        contribution_id,
        kind,
        // A 666 attestation adds no new version; it is announced in the
        // contribution's outbox envelope as a change to the existing version.
        change_type: super::codes::change_type::ATTESTATION.to_owned(),
        template_id: None,
    })
}

/// `VERSIONED_COMPOSITION` cross-version invariants, enforced against the
/// FIRST stored version's root (RM ehr
/// `org.openehr.rm.ehr.versioned_composition.adoc`):
///
/// - `Archetype_node_id_valid`: "all_versions … data.archetype_node_id
///   is_equal (first_version.data.archetype_node_id)" — a versioned
///   composition cannot switch archetype across versions;
/// - `Persistent_validity`: every version's `is_persistent` equals the first
///   version's — the persistence category (`category` `431|persistent|`,
///   openEHR `composition category` group) is fixed for the container's life.
///
/// A violating modification is a 422 naming the invariant.
async fn check_versioned_composition_invariants(
    tx: &mut PgConnection,
    vo_id: Uuid,
    canonical: &serde_json::Value,
) -> Result<(), ServiceError> {
    let Some(first) = sqlx::query(
        "SELECT data->>'archetype_node_id' AS ani, \
                data#>>'{category,defining_code,code_string}' AS category \
         FROM node WHERE vo_id = $1 AND num = 0 ORDER BY sys_version LIMIT 1",
    )
    .bind(vo_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        // No stored content version (e.g. every prior version deleted) — no
        // first-version root to compare against.
        return Ok(());
    };
    let first_ani: Option<String> = first.try_get("ani")?;
    let first_category: Option<String> = first.try_get("category")?;
    let incoming_ani = canonical.get("archetype_node_id").and_then(|v| v.as_str());
    if let (Some(stored), Some(incoming)) = (first_ani.as_deref(), incoming_ani)
        && stored != incoming
    {
        return Err(ServiceError::Unprocessable(format!(
            "COMPOSITION archetype_node_id {incoming:?} differs from the versioned \
             object's first version {stored:?} \
             (VERSIONED_COMPOSITION.Archetype_node_id_valid)"
        )));
    }
    const PERSISTENT: &str = "431";
    let incoming_category = canonical
        .pointer("/category/defining_code/code_string")
        .and_then(|v| v.as_str());
    if let (Some(stored), Some(incoming)) = (first_category.as_deref(), incoming_category)
        && (stored == PERSISTENT) != (incoming == PERSISTENT)
    {
        return Err(ServiceError::Unprocessable(format!(
            "COMPOSITION category {incoming} changes the persistence of the versioned \
             object (first version: {stored}) — is_persistent is fixed across versions \
             (VERSIONED_COMPOSITION.Persistent_validity)"
        )));
    }
    Ok(())
}

/// Record a committed COMPOSITION for the `compositions_committed_total`
/// metric (§1.2). Only COMPOSITIONs are counted; `EHR_STATUS`/FOLDER writes are
/// not this metric's subject.
fn record_composition_commit(kind: Kind, change_type: &'static str) {
    if kind == Kind::Composition {
        metrics::counter!(
            crate::telemetry::prometheus::COMPOSITIONS_COMMITTED,
            "change_type" => change_type,
        )
        .increment(1);
    }
}

/// Keep the EHR's promoted subject columns (`ehr.subject_id` /
/// `subject_namespace`) in sync with the `EHR_STATUS` being committed
/// (`subject.external_ref.id.value` + `.namespace`). The partial unique index
/// `ehr_subject_uq` enforces **one EHR per subject** at the database — the
/// ITS-REST `409_EHR.yaml` conflict ("an already existing EHR with the same
/// subject id, namespace pair") and CNF master06
/// `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient`; a violation maps to
/// [`ServiceError::Conflict`] (→ 409). A status without an `external_ref`
/// (e.g. `PARTY_SELF`) clears the columns and never conflicts.
async fn sync_ehr_subject(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    canonical: &serde_json::Value,
) -> Result<(), ServiceError> {
    use serde_json::Value;
    let subject_id = canonical
        .pointer("/subject/external_ref/id/value")
        .and_then(Value::as_str);
    let namespace = canonical
        .pointer("/subject/external_ref/namespace")
        .and_then(Value::as_str);
    // Only a complete (id, namespace) pair identifies a subject.
    let (subject_id, namespace) = match (subject_id, namespace) {
        (Some(id), Some(ns)) => (Some(id), Some(ns)),
        _ => (None, None),
    };
    sqlx::query("UPDATE ehr SET subject_id = $2, subject_namespace = $3 WHERE id = $1")
        .bind(ehr_id)
        .bind(subject_id)
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e
                && db.constraint() == Some("uq_ehr_subject")
            {
                return ServiceError::Conflict(format!(
                    "an EHR already exists for subject {}@{}",
                    subject_id.unwrap_or("?"),
                    namespace.unwrap_or("?"),
                ));
            }
            ServiceError::Database(e)
        })?;
    Ok(())
}

/// The resolved placement of a new version in the version tree: its storage
/// ordinal, its `VERSION_TREE_ID`, the lineage tip it supersedes (closed on
/// insert; `None` when the commit FORKS a new branch and the preceding version
/// stays valid), and the actual `preceding_version_uid` to store.
struct NextVersion {
    ordinal: i32,
    tree: TreeId,
    close_ordinal: Option<i32>,
    preceding_uid: String,
}

/// Validate an update/delete target (belongs to `ehr_id`, `If-Match` matches)
/// and resolve where the new version sits in the version tree (RM common
/// master06 §Version tree / §Distributed versioning):
///
/// - the preceding version is the current TRUNK tip when `expected` is absent,
///   or exactly the version `expected` names (trunk or branch) — which must be
///   an open lineage tip, else `VersionConflict`;
/// - a preceding version created by THIS system is continued on its lineage
///   (trunk `N` → `N+1`; branch `t.b.v` → `t.b.v+1`), superseding it;
/// - a preceding version created by ANOTHER system (an imported copy) FORKS a
///   new branch `t.(max_branch+1).1` — master06: "branching version
///   identifiers [are required] when local modifications are made to versions
///   copied from elsewhere" — and the preceding version stays valid.
///
/// Serializes concurrent writers of the same object with a per-vo transaction
/// advisory lock (branch writers no longer all contend on one current row).
async fn next_version(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<TreeId>,
    local_system_id: &str,
) -> Result<NextVersion, ServiceError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(vo_id)
        .execute(&mut *tx)
        .await?;
    // The preceding version: the addressed tip, or the current trunk tip.
    let row =
        match expected {
            None => sqlx::query(
                "SELECT ehr_id, kind, sys_version, trunk_version, branch_number, branch_version, \
             creating_system_id, upper_inf(sys_period) AS open \
             FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period) AND branch_number = 0",
            )
            .bind(vo_id)
            .fetch_optional(&mut *tx)
            .await?,
            Some(tree) => {
                let (t, b, v) = tree.columns();
                sqlx::query(
                "SELECT ehr_id, kind, sys_version, trunk_version, branch_number, branch_version, \
                 creating_system_id, upper_inf(sys_period) AS open \
                 FROM vo_version WHERE vo_id = $1 \
                 AND trunk_version = $2 AND branch_number = $3 AND branch_version = $4",
            )
            .bind(vo_id)
            .bind(t)
            .bind(b)
            .bind(v)
            .fetch_optional(&mut *tx)
            .await?
            }
        };
    let Some(row) = row else {
        // The object may exist with the expectation naming no stored version —
        // distinguish "no such object" (404) from "wrong version" (409).
        let current = current_version(&mut *tx, vo_id, kind).await;
        return match (expected, current) {
            (Some(tree), Ok((_, current))) => Err(ServiceError::VersionConflict(format!(
                "expected version {tree}, which does not exist (current is {current})"
            ))),
            _ => Err(ServiceError::NotFound(format!(
                "{} {vo_id} in EHR {ehr_id:?}",
                kind.as_str()
            ))),
        };
    };
    // For an EHR-scoped object, the stored owner must match; for a demographic
    // party both stored and expected owner are `None`, which compares equal.
    if row.try_get::<Option<Uuid>, _>("ehr_id")? != ehr_id
        || Kind::from_type(&row.try_get::<String, _>("kind")?) != Some(kind)
    {
        return Err(ServiceError::NotFound(format!(
            "{} {vo_id} in EHR {ehr_id:?}",
            kind.as_str()
        )));
    }
    let preceding_tree = TreeId::from_columns(
        row.try_get("trunk_version")?,
        row.try_get("branch_number")?,
        row.try_get("branch_version")?,
    );
    if !row.try_get::<bool, _>("open")? {
        return Err(ServiceError::VersionConflict(format!(
            "expected version {preceding_tree} has been superseded"
        )));
    }
    let preceding_ordinal: i32 = row.try_get("sys_version")?;
    let preceding_csid: String = row.try_get("creating_system_id")?;
    let preceding_uid = format!("{vo_id}::{preceding_csid}::{preceding_tree}");

    let ordinal: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sys_version), 0) + 1 FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(&mut *tx)
    .await?;

    // Composite identifiers compare case-insensitively (BASE base_types
    // master05 §"Composite Identifiers and Case") — a creating_system_id
    // differing only by case is the SAME system, so it continues its own
    // lineage rather than forking a branch.
    let (tree, close_ordinal) = if preceding_csid.eq_ignore_ascii_case(local_system_id) {
        // Continue the lineage this system owns; the preceding tip is superseded.
        let tree = match preceding_tree.branch {
            None => TreeId::trunk(preceding_tree.trunk + 1),
            Some((b, v)) => TreeId::branch(preceding_tree.trunk, b, v + 1),
        };
        (tree, Some(preceding_ordinal))
    } else {
        // Local modification of a version copied from elsewhere: fork a branch
        // at the preceding version's trunk fork point (master06 §Distributed
        // versioning); the copied version itself stays valid.
        let next_branch: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(branch_number), 0) + 1 FROM vo_version \
             WHERE vo_id = $1 AND trunk_version = $2",
        )
        .bind(vo_id)
        .bind(preceding_tree.trunk)
        .fetch_one(&mut *tx)
        .await?;
        (TreeId::branch(preceding_tree.trunk, next_branch, 1), None)
    };
    Ok(NextVersion {
        ordinal,
        tree,
        close_ordinal,
        preceding_uid,
    })
}

/// One `vo_version` row to insert (validity `[now, ∞)`).
struct NewVersionRow<'a> {
    vo_id: Uuid,
    kind: Kind,
    ehr_id: Option<Uuid>,
    ordinal: i32,
    tree: TreeId,
    lifecycle_state: &'a str,
    creating_system_id: &'a str,
    /// The stored `ORIGINAL_VERSION.preceding_version_uid` (`None` for a first
    /// version).
    preceding_version_uid: Option<&'a str>,
    /// `ORIGINAL_VERSION.other_input_version_uids` (merge provenance; empty →
    /// stored NULL, `Is_merged_validity`).
    other_input_version_uids: &'a [String],
    contribution_id: Uuid,
    audit_id: Uuid,
    template_id: Option<&'a str>,
    signature: Option<&'a str>,
}

/// Insert one `vo_version` row (validity `[now, ∞)`).
async fn insert_vo_version(
    tx: &mut PgConnection,
    row: NewVersionRow<'_>,
) -> Result<(), ServiceError> {
    let (trunk_version, branch_number, branch_version) = row.tree.columns();
    let other_input = if row.other_input_version_uids.is_empty() {
        None
    } else {
        Some(serde_json::json!(row.other_input_version_uids))
    };
    sqlx::query(
        "INSERT INTO vo_version \
         (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
          sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
          other_input_version_uids, contribution_id, audit_id, template_id, signature) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, tstzrange(now(), NULL, '[)'), $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(row.vo_id)
    .bind(row.kind.as_str())
    .bind(row.ehr_id)
    .bind(row.ordinal)
    .bind(trunk_version)
    .bind(branch_number)
    .bind(branch_version)
    .bind(row.lifecycle_state)
    .bind(row.creating_system_id)
    .bind(row.preceding_version_uid)
    .bind(other_input)
    .bind(row.contribution_id)
    .bind(row.audit_id)
    .bind(row.template_id)
    .bind(row.signature)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Create the first version of a new versioned object under its own contribution.
#[allow(clippy::too_many_arguments)] // the write parameters; a struct would not read clearer
pub(super) async fn create(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    kind: Kind,
    canonical: serde_json::Value,
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
            // The direct (non-CONTRIBUTION) create carries no wire lifecycle;
            // defaults to `532|complete|` (unchanged wire behaviour).
            lifecycle_state: None,
            attestations: Vec::new(),
        },
    )
    .await?;
    write_outbox(
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
pub(super) async fn update(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    canonical: serde_json::Value,
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
            // The direct (non-CONTRIBUTION) update carries no wire lifecycle;
            // defaults to `532|complete|` (unchanged wire behaviour).
            lifecycle_state: None,
            attestations: Vec::new(),
            other_input_version_uids: Vec::new(),
        },
    )
    .await?;
    write_outbox(
        tx,
        contribution_id,
        ehr_id,
        time_committed,
        vec![committed.envelope_entry()],
    )
    .await?;
    Ok(committed)
}

/// Logically delete an object under its own contribution.
pub(super) async fn delete(
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
    write_outbox(
        tx,
        contribution_id,
        ehr_id,
        time_committed,
        vec![committed.envelope_entry()],
    )
    .await?;
    Ok(committed)
}

/// Commit a set of changes atomically under one CONTRIBUTION. `contribution_audit`
/// is the CONTRIBUTION's own audit; each change carries its VERSION `commit_audit`.
///
/// `attests` are `666|attestation|` items — new `ATTESTATION`s attached to
/// **existing** versions (RM common master06 §Change Control), committed in the
/// same transaction / CONTRIBUTION as the version changes but adding no new
/// version (and therefore no version audit row: the attestation *is* an
/// `AUDIT_DETAILS` subtype, stored verbatim in `vo_attestation.data`). The
/// committer of each attestation defaults to the CONTRIBUTION's committer when
/// the wire partial omits one (master06 §Committal: `system_id`/`committer`/
/// `time_committed` copied from the contribution act).
pub(super) async fn commit_contribution(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    contribution_audit: &AuditInput,
    changes: Vec<(AuditInput, Change)>,
    attests: Vec<PendingAttest>,
    ctx: &SigningCtx<'_>,
) -> Result<(Uuid, Vec<Committed>), ServiceError> {
    let (contribution_audit_id, contribution_time) = insert_audit(tx, contribution_audit).await?;
    let contribution_id = insert_contribution(tx, ehr_id, contribution_audit_id).await?;
    let committer_fallback = &contribution_audit.committer;
    let mut committed = Vec::with_capacity(changes.len() + attests.len());
    for (version_audit, change) in changes {
        let (audit_id, time_committed) = insert_audit(tx, &version_audit).await?;
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
    // Standalone 666 attestations of existing versions (no new version, no
    // version audit row) — completed with the contribution's commit-act time.
    for attest_item in attests {
        let full = super::contribution::complete_attestation(
            &attest_item.partial,
            &ctx.system_id,
            committer_fallback,
            contribution_time,
        )?;
        committed.push(
            attest(
                tx,
                ehr_id,
                attest_item.vo_id,
                attest_item.kind,
                attest_item.expected,
                &full,
                contribution_id,
            )
            .await?,
        );
    }
    // One PHI-free outbox event for the whole CONTRIBUTION, same transaction
    // (ADR-014 §1/§2), carrying every committed version + attestation.
    let versions = committed.iter().map(Committed::envelope_entry).collect();
    write_outbox(tx, contribution_id, ehr_id, contribution_time, versions).await?;
    Ok((contribution_id, committed))
}

/// One received `ORIGINAL_VERSION` to import into the local store (SM
/// `I_EHR_EXTRACT_SERVICE.import_ehr`/`import_ehr_extract`; RM common master06
/// §Copying). Each is committed locally wrapped in an `IMPORTED_VERSION` whose
/// **own** contribution records the local act of committal, while the wrapped
/// original — its identity (`object_id` → `vo_id`, `creating_system_id`, trunk
/// `version_tree_id` → `sys_version`), its `commit_audit`, `lifecycle_state`,
/// data, `signature` and `attestations` — is preserved verbatim ("the
/// `ORIGINAL_VERSION` instance is never modified", master06 §Copying).
#[derive(Debug)]
pub(super) struct ImportVersion {
    /// The `version_tree_id` of the wrapped original (trunk or branch — RM
    /// common master06 §Version tree; branch import is first-class).
    pub(super) tree: TreeId,
    /// The wrapped original's `creating_system_id` — per VERSION, not per
    /// container: a copied version tree legitimately mixes systems (source
    /// trunk versions + branch modifications made elsewhere; master06
    /// §"Distributed Versioning").
    pub(super) creating_system_id: String,
    /// The wrapped original's `preceding_version_uid`, preserved verbatim
    /// (`None` for a first version).
    pub(super) preceding_version_uid: Option<String>,
    /// The wrapped original's `other_input_version_uids` (merge provenance,
    /// master06 §Version Merging), preserved verbatim.
    pub(super) other_input_version_uids: Vec<String>,
    /// The wrapped original's resolved `version_lifecycle_state` code.
    pub(super) lifecycle_state: String,
    /// The wrapped original's `commit_audit`, preserved verbatim.
    pub(super) commit_audit: AuditInput,
    /// The wrapped original's `commit_audit.time_committed`, preserved verbatim.
    pub(super) commit_time: jiff::Timestamp,
    /// The version data (`Value::Null` for a `523|deleted|` version — no nodes).
    pub(super) data: serde_json::Value,
    /// The wrapped original's `VERSION.signature` (preserved, never re-signed).
    pub(super) signature: Option<String>,
    /// The wrapped original's `ATTESTATION`s (already full RM objects), preserved.
    pub(super) attestations: Vec<serde_json::Value>,
}

impl ImportVersion {
    /// The lineage this version sits on: the trunk (`branch_number` 0), or one
    /// specific branch of one system. Versions on the same lineage supersede
    /// each other; distinct lineages coexist.
    fn lineage(&self) -> (String, i32, i32) {
        match self.tree.branch {
            None => (String::new(), 0, 0),
            Some((b, _)) => (self.creating_system_id.clone(), self.tree.trunk, b),
        }
    }
}

/// One versioned object (a source `VERSIONED_OBJECT`) to import: its cloned
/// `vo_id` (the received `uid.object_id()`), its kind, and its versions.
pub(super) struct ImportContainer {
    /// The received `object_id` — the local `VERSIONED_OBJECT` is a clone with
    /// this uid (master06 §Copying: "a new `VERSIONED_OBJECT` is created, with
    /// its uid set to the same value as the received `VERSION._uid.object_id()`").
    pub(super) vo_id: Uuid,
    pub(super) kind: Kind,
    pub(super) versions: Vec<ImportVersion>,
}

/// Insert one `audit` row with an **explicit** `time_committed`, returning its
/// id — used to preserve an imported `ORIGINAL_VERSION`'s original
/// `commit_audit.time_committed` (the wrapped original is never modified,
/// master06 §Copying). Unlike [`insert_audit`], which stamps the local commit
/// time, this carries the source system's committal time verbatim.
async fn insert_audit_at(
    tx: &mut PgConnection,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
) -> Result<Uuid, ServiceError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO audit (system_id, change_type, description, committer, time_committed) \
         VALUES ($1, $2, $3, $4, $5::timestamptz) RETURNING id",
    )
    .bind(&audit.system_id)
    .bind(&audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .bind(time_committed.to_string())
    .fetch_one(&mut *tx)
    .await?)
}

/// Insert one `vo_version` row with an **explicit** `sys_period` (`[lower,
/// upper)`, `upper = None` ⇒ the still-open current version) — the import
/// analogue of [`insert_vo_version`], which always opens at `now()`. The import
/// path builds a synthetic strictly-increasing local period chain so a whole
/// imported version history lands as one contiguous, non-overlapping tree
/// (temporal `WITHOUT OVERLAPS` PK; ADR-008).
#[allow(clippy::too_many_arguments)] // one row's columns; a struct would not read clearer
#[allow(clippy::too_many_arguments)] // one row's columns beyond the shared struct
async fn insert_imported_vo_version(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    vo_id: Uuid,
    kind: Kind,
    version: &ImportVersion,
    ordinal: i32,
    lower: jiff::Timestamp,
    upper: Option<jiff::Timestamp>,
    contribution_id: Uuid,
    audit_id: Uuid,
) -> Result<(), ServiceError> {
    let (trunk_version, branch_number, branch_version) = version.tree.columns();
    let other_input = if version.other_input_version_uids.is_empty() {
        None
    } else {
        Some(serde_json::json!(version.other_input_version_uids))
    };
    sqlx::query(
        "INSERT INTO vo_version \
         (vo_id, kind, ehr_id, sys_version, trunk_version, branch_number, branch_version, \
          sys_period, lifecycle_state, creating_system_id, preceding_version_uid, \
          other_input_version_uids, contribution_id, audit_id, template_id, signature) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, \
                 tstzrange($8::timestamptz, $9::timestamptz, '[)'), $10, $11, $12, $13, $14, \
                 $15, NULL, $16)",
    )
    .bind(vo_id)
    .bind(kind.as_str())
    .bind(ehr_id)
    .bind(ordinal)
    .bind(trunk_version)
    .bind(branch_number)
    .bind(branch_version)
    .bind(lower.to_string())
    .bind(upper.map(|t| t.to_string()))
    .bind(&version.lifecycle_state)
    .bind(&version.creating_system_id)
    .bind(&version.preceding_version_uid)
    .bind(other_input)
    .bind(contribution_id)
    .bind(audit_id)
    .bind(version.signature.as_deref())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Close the open (`upper_inf`) version of one LINEAGE of `vo_id` at an
/// explicit instant (the import base time). The trunk lineage is
/// `branch_number = 0`; a branch lineage is one `(creating_system_id,
/// trunk_version, branch_number)`. Used when importing further versions into
/// an existing container (master06 §Copying "previous copies have been made
/// for the item"), so the new imported chain opens cleanly after the closed one.
async fn close_lineage_at(
    tx: &mut PgConnection,
    vo_id: Uuid,
    lineage: &(String, i32, i32),
    at: jiff::Timestamp,
) -> Result<(), ServiceError> {
    let (csid, trunk, branch) = lineage;
    if *branch == 0 {
        sqlx::query(
            "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), $2::timestamptz, '[)') \
             WHERE vo_id = $1 AND upper_inf(sys_period) AND branch_number = 0",
        )
        .bind(vo_id)
        .bind(at.to_string())
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), $2::timestamptz, '[)') \
             WHERE vo_id = $1 AND upper_inf(sys_period) \
             AND creating_system_id = $3 AND trunk_version = $4 AND branch_number = $5",
        )
        .bind(vo_id)
        .bind(at.to_string())
        .bind(csid)
        .bind(trunk)
        .bind(branch)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// The current state of a to-be-imported container in the target EHR: the
/// stored kind (if the `vo_id` already exists), the highest trunk version, the
/// highest storage ordinal, and whether a still-open current TRUNK version
/// exists. `(None, None, 0, 0, false)` when the container is not present
/// (first receipt of this item — master06 §Copying Case 2). A `vo_id` owned by
/// a *different* EHR is an error at the caller.
async fn imported_container_state(
    tx: &mut PgConnection,
    vo_id: Uuid,
) -> Result<(Option<Kind>, Option<Uuid>, i32, i32, bool), ServiceError> {
    let row = sqlx::query(
        "SELECT max(trunk_version) FILTER (WHERE branch_number = 0) AS max_trunk, \
                max(sys_version) AS max_ordinal, \
                bool_or(upper_inf(sys_period) AND branch_number = 0) AS trunk_open, \
                (array_agg(kind))[1] AS kind, \
                (array_agg(ehr_id))[1] AS owner \
         FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(&mut *tx)
    .await?;
    let max_ordinal: Option<i32> = row.try_get("max_ordinal")?;
    let Some(max_ordinal) = max_ordinal else {
        return Ok((None, None, 0, 0, false));
    };
    let max_trunk: i32 = row.try_get::<Option<i32>, _>("max_trunk")?.unwrap_or(0);
    let trunk_open: bool = row
        .try_get::<Option<bool>, _>("trunk_open")?
        .unwrap_or(false);
    let kind: Option<String> = row.try_get("kind")?;
    let owner: Option<Uuid> = row.try_get("owner")?;
    Ok((
        kind.as_deref().and_then(Kind::from_type),
        owner,
        max_trunk,
        max_ordinal,
        trunk_open,
    ))
}

/// Replay a set of received `ORIGINAL_VERSION`s into the local store as
/// `IMPORTED_VERSION`s under **one** local import CONTRIBUTION (SM
/// `import_ehr`/`import_ehr_extract`; RM common master06 §Copying, §Committal).
///
/// The `import_audit` records the local act of committal (`249|creation|`,
/// master06 §Contributions "import of item"); it becomes the CONTRIBUTION's
/// audit and every version row's `contribution_id`. Each imported version keeps
/// the wrapped original's identity and `commit_audit` verbatim (stored as the
/// row's own `audit_id`), so a re-export serves back a byte-identical
/// `ORIGINAL_VERSION`.
///
/// PORT NOTE (`IMPORTED_VERSION` representation, master06 §Committal): the
/// greenfield store holds one row per version (identity + `commit_audit` + data),
/// not a distinct `IMPORTED_VERSION` wrapper object. The "import" is expressed
/// as (a) the preserved original `commit_audit` + 3-part version identity and
/// (b) a fresh local import CONTRIBUTION recording the local committal act —
/// which is exactly what an `IMPORTED_VERSION`'s own contribution/`commit_audit`
/// denote. master06 §Committal sanctions a non-distributed holder keeping only
/// the `ORIGINAL_VERSION` content; the one visible deviation is that the served
/// `ORIGINAL_VERSION.contribution` references the local import contribution
/// rather than the (foreign, un-imported) source contribution.
///
/// PORT NOTE (local temporal periods, master06 §Copying): all versions of an
/// imported container are committed in the single local import act, so they get
/// a synthetic strictly-increasing local `sys_period` chain (base = import time,
/// 1 µs steps) **per lineage** (trunk / each branch — lineages coexist in time)
/// with only each lineage's highest version open. The true source chronology is
/// preserved in each version's `commit_audit.time_committed`.
#[allow(clippy::too_many_lines)] // the import replay + its per-version outbox entry
pub(super) async fn commit_import(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
) -> Result<Uuid, ServiceError> {
    commit_import_scoped(tx, Some(ehr_id), import_audit, containers, false).await
}

/// Land demographics-chapter parties (`X_VERSIONED_PARTY`) into the
/// demographic repository under their own (ehr-less) import CONTRIBUTION
/// (master09 §Creation Semantics demographics chapter; RM demographic content
/// is not EHR-owned). A party whose version container already exists locally
/// is SKIPPED — parties are shared continuants across extracts, and the copy
/// already held is authoritative here.
pub(super) async fn commit_demographic_import(
    tx: &mut PgConnection,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
) -> Result<(), ServiceError> {
    if containers.is_empty() {
        return Ok(());
    }
    let mut fresh = Vec::with_capacity(containers.len());
    for container in containers {
        let (existing_kind, _, _, _, _) = imported_container_state(tx, container.vo_id).await?;
        if existing_kind.is_none() {
            fresh.push(container);
        }
    }
    if fresh.is_empty() {
        return Ok(());
    }
    commit_import_scoped(tx, None, import_audit, fresh, true).await?;
    Ok(())
}

async fn commit_import_scoped(
    tx: &mut PgConnection,
    ehr_id: Option<Uuid>,
    import_audit: &AuditInput,
    containers: Vec<ImportContainer>,
    skip_existing: bool,
) -> Result<Uuid, ServiceError> {
    // One local instant anchors the whole import's temporal chain.
    let base = jiff::Timestamp::now();
    let (contribution_id, _audit_id, import_time) =
        write_contribution(tx, ehr_id, import_audit).await?;
    // Per-version entries for the single import-contribution outbox event.
    let mut outbox_versions: Vec<serde_json::Value> = Vec::new();

    for mut container in containers {
        if container.versions.is_empty() {
            continue;
        }
        // Version-tree order (trunk versions first within a lineage grouping);
        // reject a duplicated version identity within one import (the identity
        // tuple is {object_id, creating_system_id, version_tree_id} — master06
        // §Distributed versioning).
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

        let (existing_kind, owner, max_trunk, max_ordinal, trunk_open) =
            imported_container_state(tx, container.vo_id).await?;
        if skip_existing && existing_kind.is_some() {
            continue;
        }
        if let Some(existing_kind) = existing_kind {
            // First receipt of *this item* has already happened (master06 §Copying
            // Case 3): append to the existing clone — the kind must match, the EHR
            // must own it, and every imported trunk version must be strictly newer
            // (branch versions land on their own lineages; the uq_vo_version_tree
            // constraint rejects a duplicate identity).
            if owner != ehr_id {
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
                && first_trunk <= max_trunk
            {
                return Err(ServiceError::Conflict(format!(
                    "versioned object {} already has trunk version {max_trunk} — cannot \
                     re-import trunk version {first_trunk}",
                    container.vo_id
                )));
            }
            if trunk_open && container.versions.iter().any(|v| v.tree.is_trunk()) {
                close_lineage_at(tx, container.vo_id, &(String::new(), 0, 0), base).await?;
            }
        }

        // Per-lineage period chains: within a lineage each version closes its
        // predecessor; each lineage's last version stays open. Lineages coexist.
        let mut ordinal = max_ordinal;
        let versions = container.versions;
        for (i, version) in versions.iter().enumerate() {
            ordinal += 1;
            // PHI-free outbox entry for this imported version (ADR-014 §2):
            // identity + provenance only; imports carry no template_id.
            outbox_versions.push(serde_json::json!({
                "vo_id": container.vo_id,
                "kind": container.kind.as_str(),
                "sys_version": ordinal,
                "version_tree_id": version.tree.to_string(),
                "change_type": version.commit_audit.change_type,
                "template_id": serde_json::Value::Null,
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
            let audit_id = insert_audit_at(tx, &version.commit_audit, version.commit_time).await?;
            insert_imported_vo_version(
                tx,
                ehr_id,
                container.vo_id,
                container.kind,
                version,
                ordinal,
                lower,
                upper,
                contribution_id,
                audit_id,
            )
            .await?;
            // A `523|deleted|` version stores no node rows (data is Void).
            if !version.data.is_null() {
                let rows = decompose(version.data.clone())?;
                insert_nodes(tx, container.vo_id, ordinal, ehr_id, &rows).await?;
            }
            for attestation in &version.attestations {
                insert_attestation(tx, container.vo_id, ordinal, contribution_id, attestation)
                    .await?;
            }
        }
    }
    // One PHI-free outbox event for the whole import CONTRIBUTION, same
    // transaction (ADR-014 §1/§2). An empty import (no versions) writes none.
    if !outbox_versions.is_empty() {
        write_outbox(tx, contribution_id, ehr_id, import_time, outbox_versions).await?;
    }
    Ok(contribution_id)
}

/// The current trunk version number of an object (`upper_inf` on the trunk
/// lineage — RM common master06 `latest_trunk_version`), plus its `ehr_id`.
/// Locks the row `FOR UPDATE` so concurrent updates serialize.
async fn current_version(
    tx: &mut PgConnection,
    vo_id: Uuid,
    kind: Kind,
) -> Result<(Option<Uuid>, i32), ServiceError> {
    let row = sqlx::query(
        "SELECT ehr_id, trunk_version FROM vo_version \
         WHERE vo_id = $1 AND kind = $2 AND upper_inf(sys_period) AND branch_number = 0 \
         FOR UPDATE",
    )
    .bind(vo_id)
    .bind(kind.as_str())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.as_str())))?;
    Ok((row.try_get("ehr_id")?, row.try_get("trunk_version")?))
}

/// Close (supersede) one specific version row — the lineage tip a new version
/// replaces — at `now()`. Lineage-precise: a branch commit closes its branch
/// tip, a trunk commit the trunk tip; a FORK closes nothing.
async fn close_ordinal_at_now(
    tx: &mut PgConnection,
    vo_id: Uuid,
    ordinal: i32,
) -> Result<(), ServiceError> {
    sqlx::query(
        "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), now(), '[)') \
         WHERE vo_id = $1 AND sys_version = $2 AND upper_inf(sys_period)",
    )
    .bind(vo_id)
    .bind(ordinal)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Read the current version of an object by id (any kind). `None` if it never
/// existed; a deleted current version is returned with `canonical = Null` and a
/// `523` lifecycle so callers can distinguish 404 (never existed) from a
/// deleted read (spec 204 / lifecycle `deleted`).
pub(super) async fn read_current(
    pool: &PgPool,
    vo_id: Uuid,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(row) = sqlx::query(
        "SELECT v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, \
         a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 AND upper_inf(v.sys_period) AND v.branch_number = 0",
    )
    .bind(vo_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(version_read(pool, vo_id, &row).await?))
}

/// Read a specific version of an object by its STORAGE ORDINAL (`sys_version`)
/// — for internal callers that key rows by ordinal (the FHIR mapping table,
/// extract export iteration), never for wire version ids (those are
/// `VERSION_TREE_ID`s — use [`read_version`]).
pub(super) async fn read_version_by_ordinal(
    pool: &PgPool,
    vo_id: Uuid,
    ordinal: i32,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(row) = sqlx::query(
        "SELECT v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, \
         a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 AND v.sys_version = $2",
    )
    .bind(vo_id)
    .bind(ordinal)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(version_read(pool, vo_id, &row).await?))
}

/// Read a specific version of an object by its `VERSION_TREE_ID` (for
/// `.../version/{version_uid}` — trunk or branch).
pub(super) async fn read_version(
    pool: &PgPool,
    vo_id: Uuid,
    tree: TreeId,
) -> Result<Option<VersionRead>, ServiceError> {
    let (t, b, v) = tree.columns();
    let Some(row) = sqlx::query(
        "SELECT v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, \
         a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 AND v.trunk_version = $2 AND v.branch_number = $3 \
         AND v.branch_version = $4",
    )
    .bind(vo_id)
    .bind(t)
    .bind(b)
    .bind(v)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(version_read(pool, vo_id, &row).await?))
}

/// Read the version of an object that was current at a given instant
/// (time-travel): the row whose `sys_period` contains `at`. `None` if the
/// object did not exist at that time.
pub(super) async fn version_at(
    pool: &PgPool,
    vo_id: Uuid,
    at: jiff::Timestamp,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(row) = sqlx::query(
        "SELECT v.ehr_id, v.sys_version, v.trunk_version, v.branch_number, v.branch_version, v.lifecycle_state, v.creating_system_id, v.preceding_version_uid, v.other_input_version_uids, v.contribution_id, v.template_id, v.signature, \
         a.system_id, a.change_type, a.description, a.committer, a.time_committed \
         FROM vo_version v JOIN audit a ON a.id = v.audit_id \
         WHERE v.vo_id = $1 AND v.sys_period @> $2::timestamptz AND v.branch_number = 0",
    )
    .bind(vo_id)
    .bind(at.to_string())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(version_read(pool, vo_id, &row).await?))
}

/// Reassemble the canonical JSON of one stored version from its `node` rows.
async fn read_nodes(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<serde_json::Value, ServiceError> {
    let rows = sqlx::query(
        "SELECT num, num_cap, parent_num, citem_num, rm_type, archetype, name, path, data \
         FROM node WHERE vo_id = $1 AND sys_version = $2 ORDER BY num",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_all(pool)
    .await?;

    let mut node_rows = Vec::with_capacity(rows.len());
    for row in rows {
        node_rows.push(NodeRow {
            num: row.try_get("num")?,
            num_cap: row.try_get("num_cap")?,
            parent_num: row.try_get("parent_num")?,
            citem_num: row.try_get("citem_num")?,
            rm_type: row.try_get("rm_type")?,
            archetype: row.try_get("archetype")?,
            name: row.try_get("name")?,
            path: row.try_get("path")?,
            data: row.try_get("data")?,
        });
    }
    Ok(reassemble(&node_rows)?)
}
