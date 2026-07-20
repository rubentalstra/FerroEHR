//! `I_EHR_SERVICE` (`i_ehr_service.adoc`) + `EHR_SUMMARY` (`ehr_summary.adoc`):
//! EHR create (4 variants), `has_ehr`(`_for_subject`), `get_ehr(s)`, and the
//! folder-hierarchy reads the `EHR` wire body needs.
//!
//! Spec: arch-overview `master06-design_of_the_ehr.adoc` §The EHR (EHR root,
//! `system_id`, `EHR_ACCESS`, `EHR_STATUS`, directory, folders,
//! `time_created`) and RM ehr `master04-ehr_package.adoc` §EHR Creation /
//! §Folders. The EHR-table and folder-membership SQL is a storage seam
//! (no openEHR spec governs the schema — our own design).

use crate::ids::EhrId;
use crate::service::ehr::handle::EhrSummary;
use crate::service::ehr_index::types::SubjectRef;
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::SmError;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::Kind;
use crate::versioning::audit::change_type;
use crate::versioning::change::{Change, commit_contribution};
use crate::versioning::object_version_id::{TreeId, object_version_id};

use super::status_for_subject;

impl EhrbaseService {
    /// Create an EHR (with the given id), its initial `EHR_STATUS`, and its
    /// `EHR_ACCESS`, all committed under **one** CONTRIBUTION — RM ehr master04
    /// §EHR Creation: "the result should be a root EHR object, an EHR Status
    /// object, and an EHR Access object … created and committed in a
    /// Contribution". Shared by `POST /ehr` and `PUT /ehr/{ehr_id}`.
    ///
    /// # Errors
    /// [`ServiceError::Unprocessable`] when the supplied `EHR_STATUS` is
    /// structurally invalid; [`ServiceError::Conflict`] when the EHR already
    /// exists or the subject already owns another EHR (`ehr_subject_uq`, kept
    /// in sync by [`Self::sync_ehr_subject`] → 409, ITS-REST `409_EHR.yaml`;
    /// CNF `create_ehr-two_ehrs_same_patient`); [`ServiceError::Database`] on
    /// a storage failure.
    pub(in crate::service) async fn commit_new_ehr(
        &self,
        ehr_id: EhrId,
        status: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        // The supplied EHR_STATUS must be a structurally valid RM instance
        // before the EHR is created (CNF master06 §Test Data Sets INVALID
        // class 2).
        super::validation::validate_ehr_status(&status)?;

        let mut tx = self.pool.begin().await?;

        // EHR.system_id is recorded at creation, immutable thereafter (arch
        // master06 §System Identity — a stored value, not the live config).
        // `time_created` (arch master06 §The EHR) comes back from the INSERT so
        // the create response is built without a follow-up `ehr` header read.
        // The promoted subject / is_queryable columns are set in this same
        // INSERT (the values are known from the incoming EHR_STATUS), so the
        // create path never runs the separate `sync_ehr_subject` UPDATE the
        // update/contribution paths use. A subject already owned by another EHR
        // is a distinct 409 (RM ehr master04 §EHR Status; ITS-REST
        // `409_EHR.yaml`).
        let (subject_id, subject_namespace, is_queryable, is_modifiable) =
            super::status::ehr_promoted_columns(&status);
        let time_created = match crate::storage::ehr_repo::insert_ehr(
            &mut tx,
            ehr_id,
            &self.effective_system_id(),
            subject_id,
            subject_namespace,
            is_queryable,
            is_modifiable,
        )
        .await
        {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Err(ServiceError::Conflict(format!(
                    "EHR {ehr_id} already exists"
                )));
            }
            Err(crate::storage::error::StorageError::SubjectInUse(id, ns)) => {
                return Err(ServiceError::Conflict(format!(
                    "an EHR already exists for subject {id}@{ns}"
                )));
            }
            Err(e) => return Err(e.into()),
        };

        let audit = self.audit(change_type::CREATION, "EHR creation");
        let (_contribution_id, committed) = commit_contribution(
            &mut tx,
            Some(ehr_id),
            None,
            &audit,
            vec![
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrStatus,
                        canonical: status.clone(),
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrAccess,
                        canonical: super::access::default_ehr_access(),
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
            ],
            Vec::new(),
            &self.signing_ctx(),
        )
        .await?;
        // The promoted subject / is_queryable columns were set in the initial
        // `insert_ehr` (folded, no separate UPDATE) — one EHR per subject (RM
        // ehr master04 §EHR Status).
        tx.commit().await?;

        // The EHR is created with the settings-less default EHR_ACCESS
        // (default-open); seed the access cache so the first EHR-scoped request
        // is a hit, not a DB miss (the access gate runs on every such request).
        self.prewarm_ehr_access_open(ehr_id).await;

        // Build the RM `EHR` wire body straight from the commit results — the
        // status/access version identities are already in `Committed`,
        // `time_created` came back from the row INSERT, and a fresh EHR indexes
        // no folder hierarchy — so the create path never re-reads via
        // `ehr_summary` (its five header/version/folder reads). The body is
        // byte-identical to `ehr_summary` for a new EHR (pinned by a test); it
        // is stashed so `ehr_created_object` serves a
        // `Prefer: return=representation` response without a re-read.
        let body = self.ehr_object_from_committed(ehr_id, time_created, &committed);
        self.created_ehr_repr.insert(ehr_id, body.clone()).await;
        let meta = ResourceMeta::new(ehr_id.to_string(), ehr_id.to_string())
            .with_last_modified(time_created);
        Ok(ServiceResponse::new(body, meta))
    }

    /// Assemble the RM `EHR` wire body for a just-created EHR straight from the
    /// CONTRIBUTION commit results — no storage reads. The status/access
    /// version identities come from the
    /// [`Committed`](crate::versioning::change::Committed) rows (`EHR_STATUS` then
    /// `EHR_ACCESS`, RM ehr master04 §EHR Creation), the status ref carries its
    /// `OBJECT_VERSION_ID` (the stored per-version `creating_system_id`,
    /// master06 §Distributed Versioning), and a fresh EHR has no
    /// `directory`/`folders` (RM ehr master04 §Folders, 0..1). Byte-identical
    /// to [`Self::ehr_summary`] for a newly created EHR (pinned by a test); the
    /// key order mirrors `ehr_summary`.
    fn ehr_object_from_committed(
        &self,
        ehr_id: EhrId,
        time_created: jiff::Timestamp,
        committed: &[crate::versioning::change::Committed],
    ) -> Value {
        let status_ref = committed
            .iter()
            .find(|c| c.kind == Kind::EhrStatus)
            .map(|c| {
                json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_EHR_STATUS",
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": object_version_id(c.vo_id, &c.creating_system_id, c.tree)
                    }
                })
            });
        let mut body = json!({
            "_type": "EHR",
            "system_id": { "_type": "HIER_OBJECT_ID", "value": self.effective_system_id() },
            "ehr_id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() },
            "ehr_status": status_ref,
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        });
        if let Some(access) = committed.iter().find(|c| c.kind == Kind::EhrAccess) {
            body["ehr_access"] = json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_ACCESS",
                "id": { "_type": "HIER_OBJECT_ID", "value": access.vo_id.to_string() }
            });
        }
        body
    }

    /// Find an EHR by the subject its current `EHR_STATUS` names (external ref
    /// `id.value` + `namespace`). Served from the promoted `ehr.subject_*`
    /// columns (unique per subject — `ehr_subject_uq`).
    ///
    /// NOTE (`i_ehr_service.adoc` §`get_ehrs_for_subject`): the DB
    /// constraint narrows the SM `List<EHR_SUMMARY>` to ≤1. CNF
    /// `create_ehr-two_ehrs_same_patient` expects **409** on a second EHR for
    /// the same subject, which supports the one-EHR-per-subject rule (RM ehr
    /// master04 §EHR Status: the subject is 0..1 and identifies the EHR);
    /// kept, cited.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when no EHR names the subject;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn ehr_by_subject(
        &self,
        subject_id: &str,
        namespace: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        let ehr_id = crate::storage::ehr_repo::ehr_id_by_subject(&self.pool, subject_id, namespace)
            .await?
            .ok_or_else(|| {
                ServiceError::NotFound(format!("EHR for subject {subject_id}@{namespace}"))
            })?;
        self.ehr_summary(ehr_id).await
    }

    /// Build the canonical RM `EHR` object for an existing EHR, with its
    /// `ehr_id` metadata (the `ETag`/`Location` for `POST /ehr`). ITS-REST
    /// extension: the wire `GET /ehr/{id}` returns the RM `EHR`, not the SM
    /// `EHR_SUMMARY`.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR or its current `EHR_STATUS` does
    /// not exist; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn ehr_summary(
        &self,
        ehr_id: EhrId,
    ) -> Result<ServiceResponse, ServiceError> {
        // ONE statement for the whole representation (the former four serial
        // reads — header, EHR_STATUS identity, EHR_ACCESS ref, folder
        // hierarchies — merged; read batching is spec-silent, our own design).
        // EHR.system_id is IMMUTABLE after creation (arch master06 §System
        // Identity) — the stored per-EHR value, never the live config.
        let read = crate::storage::ehr_repo::ehr_summary_read(&self.pool, ehr_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        let stored_system_id = read.system_id;
        let time_created = read.time_created;
        let status = read
            .status
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let status_version = TreeId::from_columns(
            status.trunk_version,
            status.branch_number,
            status.branch_version,
        );
        let status_ovid =
            object_version_id(status.vo_id, &status.creating_system_id, status_version);

        let mut body = json!({
            "_type": "EHR",
            "system_id": { "_type": "HIER_OBJECT_ID", "value": stored_system_id },
            "ehr_id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() },
            // Ehr_status_valid: ehr_status.type.is_equal("VERSIONED_EHR_STATUS")
            // (RM ehr `ehr.adoc` invariants — normative). NOTE (spec
            // defect): the non-normative ITS-REST example shows `type:
            // EHR_STATUS`; the RM invariant wins for `type`, the id keeps the
            // OBJECT_VERSION_ID form clients use to address the current version.
            "ehr_status": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_STATUS",
                "id": { "_type": "OBJECT_VERSION_ID", "value": status_ovid }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        });
        // EHR.ehr_access (1..1): a reference to the VERSIONED_EHR_ACCESS
        // container (invariant Ehr_access_valid — RM ehr, EHR class). Every EHR
        // this service creates has one; tolerate absence only for raw fixtures.
        if let Some(access_vo) = read.access_vo {
            body["ehr_access"] = json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_ACCESS",
                "id": { "_type": "HIER_OBJECT_ID", "value": access_vo.to_string() }
            });
        }
        // EHR.folders (0..1) + EHR.directory (0..1): the LIVE hierarchies in
        // rank order, each an OBJECT_REF to a VERSIONED_FOLDER (invariant
        // Folders_valid; directory = folders.item(1), Directory_in_folders —
        // RM ehr, EHR class).
        let refs: Vec<Value> = read
            .folders
            .iter()
            .map(|vo| {
                json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_FOLDER",
                    "id": { "_type": "HIER_OBJECT_ID", "value": vo.to_string() }
                })
            })
            .collect();
        if let Some(first) = refs.first() {
            body["directory"] = first.clone();
            body["folders"] = Value::Array(refs.clone());
        }
        let meta = ResourceMeta::new(ehr_id.to_string(), ehr_id.to_string())
            .with_last_modified(time_created);
        Ok(ServiceResponse::new(body, meta))
    }

    /// SM `EHR_SUMMARY` (`ehr_summary.adoc`) — all six mandatory attributes.
    /// `system_id` is the stored `EHR.system_id`; `ehr_status` is the current
    /// bare `EHR_STATUS`; `composition_count` is the number of "(versioned)
    /// Compositions" — distinct versioned objects (`vo_id`), not versions.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR or its current `EHR_STATUS` does
    /// not exist; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn summarize_ehr(
        &self,
        ehr_id: EhrId,
    ) -> Result<EhrSummary, ServiceError> {
        let (stored_system_id, time_created) =
            crate::storage::ehr_repo::ehr_header(&self.pool, ehr_id)
                .await?
                .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;

        // Copy of EHR.ehr_status: the current EHR_STATUS (bare, with its uid).
        let ehr_status = self.status_at(ehr_id, None).await?.body;

        let contribution_count =
            crate::storage::version_repo::contribution::ehr_contribution_count(&self.pool, ehr_id)
                .await?;
        let composition_count =
            crate::storage::version_repo::meta::composition_count(&self.pool, ehr_id).await?;

        Ok(EhrSummary {
            ehr_id: ehr_id.to_string(),
            system_id: stored_system_id,
            ehr_status,
            time_created: time_created.to_string(),
            contribution_count,
            composition_count,
        })
    }
}

/// The default `EHR_STATUS` for a new EHR (queryable, modifiable,
/// `PARTY_SELF`) — RM ehr master04 §EHR Creation.
pub(in crate::service) fn default_ehr_status() -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": true
    })
}

// ── The SM I_EHR_SERVICE call surface ─────────────────────────────────────────

impl EhrbaseService {
    /// SM `I_EHR_SERVICE.has_ehr` — whether the EHR exists.
    ///
    /// # Errors
    /// [`SmError`] if the existence read fails (a missing EHR is `Ok(false)`).
    pub async fn has_ehr(&self, ehr_id: EhrId) -> Result<bool, SmError> {
        match self.ensure_ehr_exists(ehr_id).await {
            Ok(()) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// SM `I_EHR_SERVICE.has_ehr_for_subject` — whether an EHR exists whose
    /// current `EHR_STATUS` names the subject.
    ///
    /// # Errors
    /// [`SmError`] if the subject lookup fails (no matching EHR is
    /// `Ok(false)`).
    pub async fn has_ehr_for_subject(&self, a_subject_id: SubjectRef) -> Result<bool, SmError> {
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// SM `I_EHR_SERVICE.create_ehr` — create an EHR with a server-assigned id
    /// and the given (or default) `EHR_STATUS`, returning the new `ehr_id`.
    ///
    /// # Errors
    /// [`SmError`] when the status is structurally invalid (422-equivalent),
    /// the subject already owns an EHR (409-equivalent), or storage fails.
    pub async fn create_ehr(&self, an_ehr_status: Option<Value>) -> Result<EhrId, SmError> {
        // NOTE (`i_ehr_service.adoc` §create_ehr `Pre_no_subject`):
        // the SM precondition `an_ehr_status.subject = Void` is NOT enforced on
        // the id-only create paths. `POST /ehr` intentionally accepts a
        // subject-bearing status (the ITS-REST `ehr` schema carries an optional
        // `ehr_status`, and the sync hook records the subject as the EHR's
        // patient), an accepted SM-vs-ITS-REST divergence: the subject slot is
        // a 0..1 `PARTY_SELF` (RM ehr master04 §EHR Status), and the CDR treats
        // a supplied anonymous-or-identified subject as the EHR's subject
        // rather than rejecting it. Recorded, not silently guessed.
        let ehr_id = EhrId::new();
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.commit_new_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    /// SM `I_EHR_SERVICE.create_ehr_with_id` — create an EHR under the
    /// caller-supplied id (`PUT /ehr/{ehr_id}`).
    ///
    /// # Errors
    /// [`SmError`] when the EHR already exists (409-equivalent), the status is
    /// invalid, the subject already owns an EHR, or storage fails.
    pub async fn create_ehr_with_id(
        &self,
        an_ehr_id: EhrId,
        an_ehr_status: Option<Value>,
    ) -> Result<EhrId, SmError> {
        // see `create_ehr` — `Pre_no_subject` deliberately not enforced.
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.commit_new_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    /// SM `I_EHR_SERVICE.create_ehr_for_subject` — create an EHR whose
    /// `EHR_STATUS.subject` names the given subject.
    ///
    /// # Errors
    /// [`SmError`] when the subject already owns an EHR (409-equivalent), the
    /// status is invalid, or storage fails.
    pub async fn create_ehr_for_subject(
        &self,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<EhrId, SmError> {
        let ehr_id = EhrId::new();
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.commit_new_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    /// SM `I_EHR_SERVICE.create_ehr_for_subject_with_id` — subject-scoped
    /// creation under a caller-supplied EHR id.
    ///
    /// # Errors
    /// [`SmError`] when the EHR already exists, the subject already owns an
    /// EHR, the status is invalid, or storage fails.
    pub async fn create_ehr_for_subject_with_id(
        &self,
        an_ehr_id: EhrId,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<EhrId, SmError> {
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.commit_new_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    /// SM `I_EHR_SERVICE.get_ehr` — the `EHR_SUMMARY` of an EHR.
    ///
    /// # Errors
    /// [`SmError`] when the EHR does not exist (404-equivalent) or a read
    /// fails.
    pub async fn get_ehr(&self, an_ehr_id: EhrId) -> Result<EhrSummary, SmError> {
        Ok(self.summarize_ehr(an_ehr_id).await?)
    }

    /// SM `I_EHR_SERVICE.get_ehrs_for_subject` — the `EHR_SUMMARY` list for a
    /// subject (≤1 under the one-EHR-per-subject rule; see the note on
    /// [`Self::ehr_by_subject`]).
    ///
    /// # Errors
    /// [`SmError`] if a read fails, or when a found EHR body carries no
    /// `ehr_id` (an internal invariant violation).
    pub async fn get_ehrs_for_subject(
        &self,
        a_subject_id: SubjectRef,
    ) -> Result<Vec<EhrSummary>, SmError> {
        // one EHR per subject narrows the List to ≤1 (see `ehr_by_subject`).
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(resp) => {
                let ehr_id = resp
                    .body
                    .pointer("/ehr_id/value")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok())
                    .map(EhrId)
                    .ok_or_else(|| SmError::exception("EHR body carries no ehr_id"))?;
                Ok(vec![self.summarize_ehr(ehr_id).await?])
            }
            Err(ServiceError::NotFound(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// The canonical RM `EHR` wire object (`GET /ehr/{ehr_id}` — an ITS-REST
    /// shape, not the SM `EHR_SUMMARY`).
    ///
    /// # Errors
    /// [`SmError`] when the EHR or its current `EHR_STATUS` does not exist, or
    /// a read fails.
    pub async fn ehr_object(&self, an_ehr_id: EhrId) -> Result<Value, SmError> {
        Ok(self.ehr_summary(an_ehr_id).await?.body)
    }

    /// The RM `EHR` wire object for a just-created EHR — the
    /// `Prefer: return=representation` body of `POST /ehr` / `PUT /ehr/{id}`.
    ///
    /// # Errors
    /// [`SmError`] when the fallback full read finds no such EHR, or a read
    /// fails.
    pub async fn ehr_created_object(&self, an_ehr_id: EhrId) -> Result<Value, SmError> {
        // Serve the create-time representation from the stash the commit path
        // populated (built from `Committed`, no re-read); a popped entry cannot
        // be reused. Fall back to a full read when the entry has been evicted
        // (short TTL) or the EHR was created off this path (import/clone).
        if let Some(body) = self.created_ehr_repr.remove(&an_ehr_id).await {
            return Ok(body);
        }
        self.ehr_object(an_ehr_id).await
    }

    /// The canonical RM `EHR` wire object located by subject
    /// (`GET /ehr?subject_id=…&subject_namespace=…`).
    ///
    /// # Errors
    /// [`SmError`] when no EHR names the subject (404-equivalent) or a read
    /// fails.
    pub async fn ehr_object_for_subject(
        &self,
        subject_id: &str,
        subject_namespace: &str,
    ) -> Result<Value, SmError> {
        Ok(self
            .ehr_by_subject(subject_id, subject_namespace)
            .await?
            .body)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::default_ehr_status;

    /// The default `EHR_STATUS` must be a valid structure root for the storage
    /// codec (one root node — the decomposition granularity of
    /// `crate::storage::codec::decompose`).
    #[test]
    fn default_status_decomposes() {
        let rows = crate::storage::codec::decompose(default_ehr_status()).expect("decompose");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rm_type, "EHR_STATUS");
    }
}
