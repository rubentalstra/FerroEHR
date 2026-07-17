//! Shared version-metadata helpers: the cross-cutting glue every
//! versioned kind (`EHR_STATUS`, COMPOSITION, DIRECTORY) needs to turn a
//! loaded [`VersionRead`] into a wire [`ServiceResponse`] + its
//! [`ResourceMeta`], plus the default commit-audit builder.
//!
//! The `OBJECT_VERSION_ID` law is RM common
//! `master06-change_control_package.adoc` §Version Identification + BASE
//! `base_types/master05-identification_package.adoc` §Syntaxes; the
//! `Last-Modified`/`ETag`/`Location` derivation from a version's commit audit
//! is ITS-REST, carried in the [`ResourceMeta`] envelope (no openEHR spec
//! governs that envelope — our own design).
//!
//! The `current_vo` row read is a storage seam
//! ([`crate::storage::version_repo`]; no openEHR spec governs the SQL — our
//! own design).

use crate::ids::{EhrId, VoId};
use crate::service::response::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::Kind;
use crate::versioning::audit::AuditInput;
use crate::versioning::object_version_id::{TreeId, object_version_id};
use crate::versioning::read::VersionRead;

impl EhrbaseService {
    /// The current version `(vo_id, VERSION_TREE_ID)` of an EHR's object of a
    /// given [`Kind`], if any — the current trunk row (`upper_inf(sys_period)`,
    /// `branch_number = 0`).
    ///
    /// The `vo_version` current-row read is a storage seam
    /// ([`crate::storage::version_repo::meta::current_vo`]; no openEHR spec governs
    /// the SQL — our own design). The [`crate::versioning::CommitEnv`]
    /// `current_vo` hook adapts this `(Uuid, TreeId)` to its `(Uuid, i32)`
    /// (trunk) shape.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the current-row read fails.
    pub(in crate::service) async fn current_vo(
        &self,
        ehr_id: EhrId,
        kind: Kind,
    ) -> Result<Option<(VoId, TreeId)>, ServiceError> {
        Ok(
            crate::storage::version_repo::meta::current_vo(&self.pool, ehr_id, kind.as_str())
                .await?
                .map(|r| {
                    (
                        r.vo_id,
                        TreeId::from_columns(r.trunk_version, r.branch_number, r.branch_version),
                    )
                }),
        )
    }

    /// A metadata-only [`ServiceResponse`] for a write, built entirely from the
    /// commit result — the version identity and the server commit instant are
    /// already in [`Committed`](crate::versioning::change::Committed), so the write
    /// path never re-reads the row it just wrote (a representation response
    /// re-reads at the protocol layer). The body is `Value::Null` by contract:
    /// every write consumer uses only the metadata.
    pub(crate) fn committed_response(
        &self,
        ehr_id: EhrId,
        committed: &crate::versioning::change::Committed,
    ) -> ServiceResponse {
        let meta = self.version_meta(
            ehr_id,
            committed.vo_id,
            &committed.creating_system_id,
            committed.tree,
            committed.time_committed,
        );
        ServiceResponse::new(Value::Null, meta)
    }

    /// The [`ResourceMeta`] for a versioned resource: the owning EHR plus the
    /// resource `OBJECT_VERSION_ID` (the `ETag` value + `Location` tail) and
    /// its commit time (the `Last-Modified`). `creating_system_id` is the
    /// stored per-version value — never re-derived from the live config — so a
    /// version's uid stays stable across a later system-id change (RM common
    /// master06 §Distributed Versioning).
    #[allow(clippy::unused_self)] // call-site ergonomics: every caller holds the service
    pub(crate) fn version_meta(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        creating_system_id: &str,
        version: TreeId,
        at: jiff::Timestamp,
    ) -> ResourceMeta {
        ResourceMeta::new(
            ehr_id.to_string(),
            object_version_id(vo_id, creating_system_id, version),
        )
        .with_last_modified(at)
    }

    /// Inject the `uid` (`OBJECT_VERSION_ID`, RM common master06 §Version
    /// Identification) into a versioned object's canonical JSON so a bare read
    /// carries its wire identity.
    #[allow(clippy::unused_self)] // call-site ergonomics: every caller holds the service
    pub(in crate::service) fn with_uid(
        &self,
        mut canonical: Value,
        vo_id: VoId,
        creating_system_id: &str,
        version: TreeId,
    ) -> Value {
        if let Value::Object(map) = &mut canonical {
            map.insert(
                "uid".to_owned(),
                json!({
                    "_type": "OBJECT_VERSION_ID",
                    "value": object_version_id(vo_id, creating_system_id, version)
                }),
            );
        }
        canonical
    }

    /// A [`ServiceResponse`] for a loaded versioned object: its canonical body
    /// with the `uid` injected, plus the resource metadata for the wire
    /// headers.
    pub(in crate::service) fn version_response(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
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

    /// The current version metadata of an EHR-owned object of `kind` (the
    /// latest `version_uid` a `409`/`412` must echo in `ETag`/`Location`), or
    /// `None`.
    ///
    /// Resolved and read in ONE metadata-only `vo_version`⋈`audit` statement
    /// (`current_version_meta_by_kind`): the `409`/`412` path needs only the
    /// full `OBJECT_VERSION_ID` + commit instant, never the reassembled
    /// document or the attestations, so this avoids the node reassembly +
    /// attestation read the full version read pays. The
    /// full-`OBJECT_VERSION_ID` `If-Match` compare (ITS-REST overview
    /// §Concurrency control) is unchanged — the emitted `ETag` is still
    /// `object_id::creating_system_id::version_tree_id`.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the metadata read fails.
    pub(in crate::service) async fn latest_version_meta(
        &self,
        ehr_id: EhrId,
        kind: Kind,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        Ok(self
            .latest_version_meta_with_vo(ehr_id, kind)
            .await?
            .map(|(_, m)| m))
    }

    /// The current version's `vo_id` **and** its [`ResourceMeta`] for an
    /// EHR-owned object of `kind`, resolved and read in the same ONE
    /// metadata-only `vo_version`⋈`audit` statement [`Self::latest_version_meta`]
    /// uses. Threading the `vo_id` back to the caller lets a following write
    /// skip re-resolving `(ehr_id, kind) → vo_id`, so the `If-Match` pre-read
    /// and the write share one `current_vo` resolution. The
    /// full-`OBJECT_VERSION_ID` compare (ITS-REST overview §Concurrency
    /// control) is unchanged.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the metadata read fails.
    pub(in crate::service) async fn latest_version_meta_with_vo(
        &self,
        ehr_id: EhrId,
        kind: Kind,
    ) -> Result<Option<(VoId, ResourceMeta)>, ServiceError> {
        let Some(m) = crate::storage::version_repo::meta::current_version_meta_by_kind(
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

    /// Build an [`AuditInput`] for a direct (single-object) write: the
    /// effective system id, the numeric `audit_change_type` code, a
    /// description, and the request's authenticated committer (RM common
    /// master04 §Audit Details).
    pub(in crate::service) fn audit(&self, change_type: &str, description: &str) -> AuditInput {
        AuditInput {
            system_id: self.effective_system_id(),
            change_type: change_type.to_owned(),
            description: Some(description.to_owned()),
            committer: committer(),
        }
    }
}

/// The committer `PARTY_PROXY` for an audit, from the authenticated committer
/// of the current request (published by the protocol adapter into the
/// [`crate::service::committer`] context). A write with no authenticated
/// principal (auth disabled, or an internal/system write) is attributed to the
/// system identity (RM common master04 `AUDIT_DETAILS.committer` 1..1).
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

// ── ITS-REST version-metadata adapter (adapter-support extension) ─────────────
//
// The protocol adapter's `412`/`ETag` pre-reads. No openEHR spec governs this
// adapter seam — our own extension over the SM-governed version identities.

impl EhrbaseService {
    /// The current COMPOSITION version metadata (the latest `version_uid` a
    /// `409`/`412` must echo), or `None` if unknown or outside this EHR.
    ///
    /// # Errors
    /// [`SmError`](crate::service::status::SmError) if the metadata read fails.
    pub async fn composition_latest_meta(
        &self,
        an_ehr_id: EhrId,
        a_versioned_object_uid: VoId,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self
            .composition_current_meta(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    /// The current `EHR_STATUS` version metadata of an EHR, or `None` when the
    /// EHR has no current `EHR_STATUS`.
    ///
    /// # Errors
    /// [`SmError`](crate::service::status::SmError) if the metadata read fails.
    pub async fn ehr_status_latest_meta(
        &self,
        an_ehr_id: EhrId,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self.ehr_status_meta(an_ehr_id).await?)
    }

    /// The current directory FOLDER version metadata of an EHR, or `None` when
    /// the EHR indexes no directory hierarchy.
    ///
    /// # Errors
    /// [`SmError`](crate::service::status::SmError) if the metadata read fails.
    pub async fn directory_latest_meta(
        &self,
        an_ehr_id: EhrId,
    ) -> Result<Option<ResourceMeta>, crate::service::status::SmError> {
        Ok(self.directory_meta(an_ehr_id).await?)
    }
}
