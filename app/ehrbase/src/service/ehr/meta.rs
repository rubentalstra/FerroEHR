//! Shared version-metadata helpers (G-9): the cross-cutting glue every
//! versioned kind (`EHR_STATUS`, COMPOSITION, DIRECTORY — and the demographic
//! roots) needs to turn a loaded [`VersionRead`] into a wire
//! [`ServiceResponse`] + its [`ResourceMeta`].
//!
//! These were scattered across the flat `ehr.rs`; W-3f hoists them into one
//! place in the EHR chapter. The `OBJECT_VERSION_ID` law is RM common
//! `master06-change_control_package.adoc` §Version Identification + BASE
//! `base_types/master05-identification_package.adoc` §Syntaxes; the
//! `Last-Modified`/`ETag`/`Location` derivation from a version's commit audit is
//! ITS-REST, carried in the [`ResourceMeta`] envelope (no openEHR spec governs
//! that envelope — our own design).
//!
//! The demographic worker (`service/demographic/`) consumes these
//! `pub(in crate::service)` helpers for its versioned party reads; the
//! `current_vo` row read is a storage seam
//! ([`crate::storage::version_repo`]; no openEHR spec governs the SQL — our own
//! design).

use crate::service::response::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::ServiceError;
use crate::versioning::audit::AuditInput;
use crate::versioning::{Kind, TreeId, VersionRead, object_version_id};

impl EhrbaseService {
    /// The current version `(vo_id, VERSION_TREE_ID)` of an EHR's object of a
    /// given [`Kind`], if any — the current trunk row (`upper_inf(sys_period)`,
    /// `branch_number = 0`).
    ///
    /// The `vo_version` current-row read is a storage seam
    /// ([`crate::storage::version_repo::current_vo`]; no openEHR spec governs the
    /// SQL — our own design). The [`crate::versioning::CommitEnv`] `current_vo`
    /// hook adapts this `(Uuid, TreeId)` to its `(Uuid, i32)` (trunk) shape.
    pub(in crate::service) async fn current_vo(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<(Uuid, TreeId)>, ServiceError> {
        Ok(
            crate::storage::version_repo::current_vo(&self.pool, ehr_id, kind.as_str())
                .await?
                .map(|r| {
                    (
                        r.vo_id,
                        TreeId::from_columns(r.trunk_version, r.branch_number, r.branch_version),
                    )
                }),
        )
    }

    /// The `OBJECT_VERSION_ID` wire string `{object_id}::{creating_system_id}::
    /// {version_tree_id}` (RM common master06 §Version Identification).
    /// `creating_system_id` is the stored per-version value — never re-derived
    /// from the live config — so a version's uid and digital signature stay
    /// stable across a later `with_system_id` change (master06 §Distributed
    /// Versioning).
    #[allow(clippy::unused_self)] // call-site ergonomics: every caller holds the service
    pub(in crate::service) fn object_version_id(
        &self,
        vo_id: Uuid,
        creating_system_id: &str,
        version: TreeId,
    ) -> String {
        object_version_id(vo_id, creating_system_id, version)
    }

    /// A metadata-only [`ServiceResponse`] for a write, built entirely from the
    /// commit result — the version identity and the server commit instant are
    /// already in [`Committed`](crate::versioning::Committed), so the write
    /// path never re-reads the row it just wrote (a representation response
    /// re-reads at the protocol layer). The body is `Value::Null` by contract:
    /// every write consumer uses only the metadata.
    pub(in crate::service) fn committed_response(
        &self,
        ehr_id: uuid::Uuid,
        committed: &crate::versioning::Committed,
    ) -> ServiceResponse {
        let meta = self.version_meta(
            ehr_id,
            committed.vo_id,
            &committed.creating_system_id,
            committed.tree,
            committed.time_committed,
        );
        ServiceResponse::new(serde_json::Value::Null, meta)
    }

    /// The [`ResourceMeta`] for a versioned resource: the owning EHR plus the
    /// resource `OBJECT_VERSION_ID` (the `ETag` value + `Location` tail) and its
    /// commit time (the `Last-Modified`).
    pub(in crate::service) fn version_meta(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        creating_system_id: &str,
        version: TreeId,
        at: jiff::Timestamp,
    ) -> ResourceMeta {
        ResourceMeta::new(
            ehr_id.to_string(),
            self.object_version_id(vo_id, creating_system_id, version),
        )
        .with_last_modified(at)
    }

    /// Inject the `uid` (`OBJECT_VERSION_ID`) into a versioned object's canonical
    /// JSON so a bare read carries its wire identity.
    pub(in crate::service) fn with_uid(
        &self,
        mut canonical: Value,
        vo_id: Uuid,
        creating_system_id: &str,
        version: TreeId,
    ) -> Value {
        if let Value::Object(map) = &mut canonical {
            map.insert(
                "uid".to_owned(),
                json!({
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, creating_system_id, version)
                }),
            );
        }
        canonical
    }

    /// A [`ServiceResponse`] for a loaded versioned object: its canonical body
    /// with the `uid` injected, plus the resource metadata for the wire headers.
    pub(in crate::service) fn version_response(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        read: VersionRead,
    ) -> ServiceResponse {
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        ServiceResponse::new(
            self.with_uid(read.canonical, vo_id, &read.creating_system_id, read.tree),
            meta,
        )
    }

    /// The current version metadata of an EHR-owned object of `kind` (the latest
    /// `version_uid` a `409`/`412` must echo in `ETag`/`Location`), or `None`.
    ///
    /// Resolved and read in ONE metadata-only `vo_version`⋈`audit` statement
    /// (`current_version_meta_by_kind`): the `409`/`412` path needs only the
    /// full `OBJECT_VERSION_ID` + commit instant, never the reassembled document
    /// or the attestations, so this avoids the node reassembly + attestation read
    /// the full version read pays. The full-`OBJECT_VERSION_ID` `If-Match`
    /// compare (ITS-REST overview §Concurrency control) is unchanged — the
    /// emitted `ETag` is still `object_id::creating_system_id::version_tree_id`.
    pub(in crate::service) async fn latest_version_meta(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        Ok(self
            .latest_version_meta_with_vo(ehr_id, kind)
            .await?
            .map(|(_, m)| m))
    }

    /// The current version's `vo_id` **and** its [`ResourceMeta`] for an
    /// EHR-owned object of `kind`, resolved and read in the same ONE
    /// metadata-only `vo_version`⋈`audit` statement `latest_version_meta` uses.
    /// Threading the `vo_id` back to the caller lets a following write skip
    /// re-resolving `(ehr_id, kind) → vo_id`, so the `If-Match` pre-read and the
    /// write share one `current_vo` resolution. The full-`OBJECT_VERSION_ID`
    /// compare (ITS-REST overview §Concurrency control) is unchanged.
    pub(in crate::service) async fn latest_version_meta_with_vo(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<(Uuid, ResourceMeta)>, ServiceError> {
        let Some(m) = crate::storage::version_repo::current_version_meta_by_kind(
            &self.pool,
            ehr_id,
            kind.as_str(),
        )
        .await?
        else {
            return Ok(None);
        };
        let tree = TreeId::from_columns(m.trunk_version, m.branch_number, m.branch_version);
        let meta = self.version_meta(
            ehr_id,
            m.vo_id,
            &m.creating_system_id,
            tree,
            m.time_committed,
        );
        Ok(Some((m.vo_id, meta)))
    }

    /// Build an [`AuditInput`] for a direct (single-object) write: the effective
    /// system id, the numeric `audit_change_type` code, a description, and the
    /// request's authenticated committer (RM common master04 §Audit Details).
    pub(in crate::service) fn audit(&self, change_type: &str, description: &str) -> AuditInput {
        AuditInput {
            system_id: self.effective_system_id(),
            change_type: change_type.to_owned(),
            description: Some(description.to_owned()),
            committer: committer(),
        }
    }
}

/// The committer `PARTY_PROXY` for an audit, from the authenticated committer of
/// the current request (published by the protocol adapter into the
/// [`crate::service::committer`] context). A write with no
/// authenticated principal (auth disabled, or an internal/system write) is
/// attributed to the system identity (RM common master04 `AUDIT_DETAILS.committer`
/// 1..1).
pub(in crate::service) fn committer() -> Value {
    match crate::service::committer::current_committer() {
        Some(identity) => json!({
            "_type": "PARTY_IDENTIFIED",
            "name": identity.subject.clone(),
            "identifiers": [{
                "_type": "DV_IDENTIFIER",
                "id": identity.subject,
                "issuer": "ehrbase-rs",
                "type": identity.id_type
            }]
        }),
        None => json!({ "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }),
    }
}

// ── ITS-REST VersionMetaAdapter (adapter-support extension) ───────────────────

impl EhrbaseService {
    pub async fn composition_latest_meta(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self
            .composition_current_meta(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    pub async fn ehr_status_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self.ehr_status_meta(an_ehr_id).await?)
    }

    pub async fn directory_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self.directory_meta(an_ehr_id).await?)
    }
}
