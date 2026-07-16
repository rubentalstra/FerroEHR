//! The demographic chapter's shared seam onto the [`crate::versioning`]
//! change-control machinery and the version spine in
//! `crate::storage::version_repo`: [`PartyKind`]↔[`Kind`] mapping, the
//! ehr-less version loads, commit-audit construction, and the canonical wire
//! assembly (uid injection, `REVISION_HISTORY`, versioned-object wrappers,
//! `ORIGINAL_VERSION` reads) used by both the party and relationship surfaces.
//!
//! The version-spine reads go through `crate::storage::version_repo` (storage
//! owns the SQL — no openEHR spec governs the storage read, our own design);
//! RM common master04 §Revision History / master06 §Versioned Objects govern
//! the assembled wire shapes. Versioning's `revision_history` /
//! `versioned_object` builders are EHR-scoped, so this chapter maps the
//! ehr-less `VersionMeta` rows into the wire shape itself.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::demographic::types::PartyKind;
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::version_update::UpdateAudit;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::storage::version_repo;
use crate::versioning::{CommitEnv, Kind};
use crate::versioning::audit::{AuditInput, audit_details};
use crate::versioning::change::Committed;
use crate::versioning::object_version_id::{TreeId, object_version_id};
use crate::versioning::read::{VersionRead, read_current, read_version, version_at};
use crate::versioning::wire::original_version;

/// The versioned-object [`Kind`] for a REST [`PartyKind`].
pub(super) fn kind_of(kind: PartyKind) -> Kind {
    match kind {
        PartyKind::Agent => Kind::Agent,
        PartyKind::Group => Kind::Group,
        PartyKind::Organisation => Kind::Organisation,
        PartyKind::Person => Kind::Person,
        PartyKind::Role => Kind::Role,
    }
}

/// The REST [`PartyKind`] for a versioned-object [`Kind`], or `None` for a
/// non-party kind (COMPOSITION / `EHR_STATUS` / … / `PARTY_RELATIONSHIP`).
pub(super) fn party_kind_of(kind: Kind) -> Option<PartyKind> {
    match kind {
        Kind::Agent => Some(PartyKind::Agent),
        Kind::Group => Some(PartyKind::Group),
        Kind::Organisation => Some(PartyKind::Organisation),
        Kind::Person => Some(PartyKind::Person),
        Kind::Role => Some(PartyKind::Role),
        _ => None,
    }
}

/// Inject the `uid` (`OBJECT_VERSION_ID`) into a versioned object's JSON on read
/// — PARTY `Uid_mandatory` (`demographic.party.adoc`), the party's identity
/// copied from its enclosing VERSION.
pub(super) fn inject_uid(
    mut canonical: Value,
    vo_id: Uuid,
    creating_system_id: &str,
    tree: TreeId,
) -> Value {
    if let Value::Object(map) = &mut canonical {
        map.insert(
            "uid".to_owned(),
            serde_json::json!({
                "_type": "OBJECT_VERSION_ID",
                "value": object_version_id(vo_id, creating_system_id, tree)
            }),
        );
    }
    canonical
}

/// Load one **ehr-less** version of a versioned object: a specific `version`,
/// the version extant `at` an instant, or the current one when both are `None`.
/// `None` when the version does not exist or the object is EHR-scoped — a
/// demographic surface never serves an EHR-owned object (`ehr_id = None` is
/// the demographics repository's scope, module PORT NOTE).
///
/// # Errors
/// [`ServiceError`] on a storage/database fault during the version read.
pub(super) async fn load_ehrless(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
    version: Option<TreeId>,
    at: Option<jiff::Timestamp>,
) -> Result<Option<VersionRead>, ServiceError> {
    let read = match (version, at) {
        (Some(v), _) => read_version(pool, vo_id, v).await?,
        (None, Some(at)) => version_at(pool, vo_id, at).await?,
        (None, None) => read_current(pool, vo_id).await?,
    };
    Ok(read.filter(|r| r.ehr_id.is_none()))
}

/// A [`ServiceResponse`] for a loaded party / relationship version: its
/// canonical body with the `uid` injected (PARTY `Uid_mandatory`) plus the
/// resource metadata (an empty `ehr_id` — demographic objects are not
/// EHR-scoped).
pub(super) fn version_response(vo_id: Uuid, read: VersionRead) -> ServiceResponse {
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

/// The create/update representation built **from the commit result**, never a
/// post-commit re-read: the served body is the just-written `canonical` with
/// the `uid` injected, and the identity + commit instant come straight from
/// [`Committed`] (RM common master06 §Committal — the written version
/// identity). Byte-identical to a fresh read: the served form is
/// `inject_uid(reassemble(decompose(canonical)))`, and the node codec
/// round-trips `canonical` losslessly (pinned by a test). The caller passes
/// the pre-write `canonical`; the multimedia-externalization fallback (where
/// the stored form is offloaded and the in-memory body would diverge) stays in
/// the party write paths.
pub(super) fn committed_response(canonical: Value, committed: &Committed) -> ServiceResponse {
    let vo_id = committed.vo_id;
    let meta = ResourceMeta::new(
        String::new(),
        object_version_id(vo_id, &committed.creating_system_id, committed.tree),
    )
    .with_last_modified(committed.time_committed);
    ServiceResponse::new(
        inject_uid(
            canonical,
            vo_id,
            &committed.creating_system_id,
            committed.tree,
        ),
        meta,
    )
}

/// Count one committed write transaction — the `db_transactions` outcome
/// metric every service write path records (no openEHR spec governs this —
/// our own telemetry).
pub(super) fn record_commit() {
    metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
        .increment(1);
}

/// One `REVISION_HISTORY_ITEM` for a stored version row: its
/// `OBJECT_VERSION_ID` and the change's `AUDIT_DETAILS` (RM common master04
/// §Revision History).
fn revision_history_item(vo_id: Uuid, meta: &version_repo::meta::VersionMeta) -> Value {
    let tree = TreeId::from_columns(meta.trunk_version, meta.branch_number, meta.branch_version);
    json!({
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
    })
}

impl EhrbaseService {
    /// The version `commit_audit` for a demographic write (RM common master04
    /// §Audit Details): when the request carried committal headers,
    /// `update_audit`'s `UPDATE_VERSION` audit attributes merge with the
    /// server rules (ITS-REST overview §"openehr-version and
    /// openehr-audit-details", a MUST); otherwise the server defaults apply —
    /// the effective `system_id`, the numeric `audit_change_type` group code,
    /// and the request's default committer (the authenticated principal's
    /// `PARTY_PROXY`, from [`CommitEnv::default_committer`]).
    pub(super) fn demographic_audit(
        &self,
        update_audit: Option<&UpdateAudit>,
        change_type: &str,
        description: &str,
    ) -> AuditInput {
        match update_audit {
            Some(u) => {
                AuditInput::from_update(u, change_type, description, &self.effective_system_id())
            }
            None => AuditInput {
                system_id: self.effective_system_id(),
                change_type: change_type.to_owned(),
                description: Some(description.to_owned()),
                committer: CommitEnv::default_committer(self),
            },
        }
    }

    /// The versioned-object wrapper (`VERSIONED_PARTY` / `VERSIONED_OBJECT`)
    /// for an ehr-less demographic object. `type_name` is the wire `_type`,
    /// `ref_type` the `owner_id` ref's `type`, and `label` names the family
    /// for the `404`. `VERSIONED_OBJECT.time_created` is the commit time of
    /// the earliest held version (for a locally-created object, v1).
    ///
    /// PORT NOTE (G-6): `VERSIONED_OBJECT.owner_id` (1..1) has no EHR owner
    /// for a demographic versioned object (no openEHR spec governs the owner
    /// of an ehr-less demographic versioned object — our own design); we
    /// reference the object's own versioned-object id (the demographics
    /// repository owns it).
    ///
    /// # Errors
    /// - [`ServiceError::NotFound`] — the object holds no versions (`label`
    ///   names the family in the message).
    /// - [`ServiceError`] on a storage/database fault reading `time_created`.
    pub(super) async fn versioned_wrapper(
        &self,
        vo_id: Uuid,
        type_name: &str,
        ref_type: &str,
        label: &str,
    ) -> Result<Value, ServiceError> {
        let time_created = version_repo::meta::time_created(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{label} {vo_id}")))?;
        Ok(json!({
            "_type": type_name,
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": ref_type,
                "id": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        }))
    }

    /// The `REVISION_HISTORY` of an ehr-less demographic object: one item per
    /// version with its `OBJECT_VERSION_ID` and commit `AUDIT_DETAILS` (RM
    /// common master04 §Revision History). The caller has already confirmed
    /// the object belongs to its family (party / relationship).
    ///
    /// # Errors
    /// [`ServiceError`] on a storage/database fault reading the version spine.
    pub(super) async fn demographic_revision_history(
        &self,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let metas = version_repo::meta::all_version_meta(&self.pool, vo_id).await?;
        let items: Vec<Value> = metas
            .iter()
            .map(|meta| revision_history_item(vo_id, meta))
            .collect();
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// The `ORIGINAL_VERSION` of an ehr-less demographic object at a specific
    /// `version`, signed per the server signing context (`label` names the
    /// family for the `404`).
    ///
    /// # Errors
    /// - [`ServiceError::NotFound`] — no ehr-less version `version` of `vo_id`
    ///   exists.
    /// - [`ServiceError`] on a storage/database fault, or when the
    ///   `ORIGINAL_VERSION` assembly/signing fails.
    pub(super) async fn demographic_original_version(
        &self,
        vo_id: Uuid,
        version: TreeId,
        label: &str,
    ) -> Result<Value, ServiceError> {
        let read = load_ehrless(&self.pool, vo_id, Some(version), None)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{label} {vo_id} v{version}")))?;
        let signer = CommitEnv::signing_ctx(self).signer;
        original_version(&read, signer)
    }

    /// The `ORIGINAL_VERSION` of an ehr-less demographic object extant at `at`
    /// (or the latest when `None`), with `ETag`/`Location` metadata for the
    /// VERSION resource (`label` names the family for the `404`).
    ///
    /// # Errors
    /// - [`ServiceError::NotFound`] — no ehr-less version of `vo_id` existed
    ///   at `at` (or no current version when `at` is `None`).
    /// - [`ServiceError`] on a storage/database fault, or when the
    ///   `ORIGINAL_VERSION` assembly/signing fails.
    pub(super) async fn demographic_original_version_at(
        &self,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
        label: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = load_ehrless(&self.pool, vo_id, None, at)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("{label} {vo_id} version at time")))?;
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        let signer = CommitEnv::signing_ctx(self).signer;
        let ov = original_version(&read, signer)?;
        Ok(ServiceResponse::new(ov, meta))
    }
}
