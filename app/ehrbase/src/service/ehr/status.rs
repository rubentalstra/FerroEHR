//! `I_EHR_STATUS` (`i_ehr_status.adoc`) — the `EHR_STATUS` reads, the five
//! discrete mutators, the versioned-object views, plus two EHR-owned commit
//! hooks: the `is_modifiable` content-write guard and the promoted-subject
//! sync.
//!
//! Spec: RM ehr `ehr_status.adoc` + `master04-ehr_package.adoc` §EHR Status /
//! §EHR Active Status; the versioned-object mechanics are RM common master06,
//! delegated to [`crate::versioning`]. `EHR_STATUS` "is always modifiable"
//! (master04 §EHR Active Status), so its own commits are never gated by the
//! content-write guard — that is how a deactivated EHR is flipped back on.

use crate::ids::{EhrId, VoId};
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::SmError;
use crate::service::version_update::UpdateVersion;
use serde_json::Value;
use sqlx::PgConnection;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::Kind;
use crate::versioning::audit::change_type;
use crate::versioning::change::update;
use crate::versioning::object_version_id::{TreeId, expected_from_if_match, parse_tree_id};
use crate::versioning::read::{read_current, read_version, version_at};
use crate::versioning::wire::{original_version, revision_history, versioned_object};

use super::{ensure_if_match, parse_at_time};

impl EhrbaseService {
    /// The `EHR_STATUS` of an EHR as canonical JSON with its `uid` set — the
    /// current version, or the one current at `at` (time-travel) when given.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR has no current `EHR_STATUS` or
    /// none existed at `at`; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_at(
        &self,
        ehr_id: EhrId,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The **bare** `EHR_STATUS` at a specific version (not the
    /// `ORIGINAL_VERSION` wrapper) — `GET …/ehr_status/{version_uid}`
    ///.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the version does not exist or belongs to
    /// another EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_by_version(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: TreeId,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The one `EHR_STATUS` commit core (`replace_ehr_status` and the discrete
    /// SM mutators both land here): commit a new version from the caller's full
    /// `UPDATE_VERSION` envelope, sync the promoted subject columns, and return
    /// the committed version identity.
    ///
    /// `vo_id` is the `EHR_STATUS` versioned object (resolved once by the
    /// caller's `If-Match` meta pre-read, so the `current_vo` JOIN is not
    /// re-run here); `if_match` is the `OBJECT_VERSION_ID` (or bare version)
    /// the client believes is current. The result is the commit's own
    /// [`Committed`](crate::versioning::change::Committed) (the written version
    /// identity + commit instant, RM common master06 §Committal) — the write
    /// path never re-reads the row it just wrote; a
    /// `Prefer: return=representation` body is read back at the protocol
    /// layer, matching the COMPOSITION/DIRECTORY write paths.
    async fn commit_status(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: UpdateVersion,
        if_match: &str,
    ) -> Result<crate::versioning::change::Committed, ServiceError> {
        let (audit, envelope) = super::resolve_envelope(
            &version,
            change_type::MODIFICATION,
            "EHR_STATUS update",
            &self.effective_system_id(),
        );
        let body = version.data;
        super::validation::validate_ehr_status(&body)?;
        let expected = expected_from_if_match(if_match)?;

        let mut tx = self.pool.begin().await?;
        let committed = update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::EhrStatus,
            body.clone(),
            expected,
            None,
            &audit,
            envelope,
            &self.signing_ctx(),
        )
        .await?;
        // Keep the promoted subject columns in sync (the subject may have
        // changed); the is_queryable promotion (Fix B) rides this same UPDATE.
        self.sync_ehr_subject(&mut tx, ehr_id, &body).await?;
        tx.commit().await?;

        Ok(committed)
    }

    /// Apply a single in-place mutation to the current `EHR_STATUS` root and
    /// commit it as a new implicit-CONTRIBUTION version — the shared body of
    /// the discrete `I_EHR_STATUS` mutators (`i_ehr_status.adoc`
    /// §`set_ehr_queryable` … §`update_other_details`), formally equivalent to
    /// the whole-object replace (`master02-overview.adoc` §Interface Calls).
    /// Reuses [`Self::commit_status`] with the current version uid as the
    /// preceding version (server-driven optimistic lock).
    ///
    /// `EHR_STATUS` "is always modifiable" (RM ehr master04 §EHR Active
    /// Status), so this is deliberately **not** gated by
    /// [`Self::ensure_content_writable`] — that guard scopes to EHR *contents*,
    /// never to `EHR_STATUS`, which is how `clear_ehr_modifiable` disables an
    /// EHR yet `set_ehr_modifiable` re-enables it.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR has no current `EHR_STATUS`;
    /// [`ServiceError::Unprocessable`] when the mutated status fails structural
    /// validation; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_mutate(
        &self,
        ehr_id: EhrId,
        mutate: impl FnOnce(&mut serde_json::Map<String, Value>),
    ) -> Result<crate::versioning::change::Committed, ServiceError> {
        // Resolve the current EHR_STATUS versioned object once and read its
        // body; the resolved `vo_id` threads into `commit_status` so its commit
        // skips a second `current_vo` resolution.
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = read_current(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let current = self.version_response(ehr_id, vo_id, read);
        let preceding = current
            .meta
            .as_ref()
            .map(|m| m.uid.clone())
            .unwrap_or_default();
        let mut body = current.body;
        if let Value::Object(map) = &mut body {
            // The read injects `uid`; drop it so the re-commit carries only the
            // mutated EHR_STATUS content (the server assigns the new version
            // id).
            map.remove("uid");
            mutate(map);
        }
        self.commit_status(ehr_id, vo_id, UpdateVersion::direct(body), &preceding)
            .await
    }

    /// The `VERSIONED_OBJECT` for an EHR's `EHR_STATUS`.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR has no current `EHR_STATUS`;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn versioned_status(
        &self,
        ehr_id: EhrId,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        versioned_object(&self.pool, vo_id, ehr_id, "VERSIONED_EHR_STATUS").await
    }

    /// The `REVISION_HISTORY` of an EHR's `EHR_STATUS`.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR has no current `EHR_STATUS`;
    /// [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_revision_history(
        &self,
        ehr_id: EhrId,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        revision_history(&self.pool, ehr_id, vo_id).await
    }

    /// An `ORIGINAL_VERSION` of an `EHR_STATUS` at a specific version.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the version does not exist or belongs to
    /// another EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_version(
        &self,
        ehr_id: EhrId,
        vo_id: VoId,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        original_version(&read, self.signer())
    }

    /// The `ORIGINAL_VERSION` of an EHR's `EHR_STATUS` extant at `at`, or the
    /// latest when `at` is `None` (`GET …/versioned_ehr_status/version`,
    /// F-01-05). The metadata carries the `version_uid` for the
    /// `200_VERSION_at_time` `ETag`/`Location`.
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the EHR has no current `EHR_STATUS` or
    /// none existed at `at`; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn status_version_at_time(
        &self,
        ehr_id: EhrId,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => version_at(&self.pool, vo_id, at).await?,
            None => read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR_STATUS version at time for EHR {ehr_id}"))
        })?;
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

    /// The current `EHR_STATUS` version metadata (for a `412`
    /// `ETag`/`Location`).
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the metadata read fails.
    pub(in crate::service) async fn ehr_status_meta(
        &self,
        ehr_id: EhrId,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::EhrStatus).await
    }

    /// The current `EHR_STATUS` versioned-object id **and** its version
    /// metadata, resolved and read in ONE metadata-only statement (no node
    /// reassembly). The `PUT …/ehr_status` pre-read: it supplies both the
    /// full-`OBJECT_VERSION_ID` the `If-Match` compare needs (ITS-REST overview
    /// §Concurrency control) and the `vo_id` threaded into the update, so the
    /// write path resolves the versioned object exactly once. `None` when the
    /// EHR has no current `EHR_STATUS`.
    ///
    /// # Errors
    /// [`ServiceError::Database`] if the metadata read fails.
    pub(in crate::service) async fn ehr_status_meta_with_vo(
        &self,
        ehr_id: EhrId,
    ) -> Result<Option<(VoId, ResourceMeta)>, ServiceError> {
        self.latest_version_meta_with_vo(ehr_id, Kind::EhrStatus)
            .await
    }

    /// Whether the EHR is modifiable (RM ehr `EHR_STATUS.is_modifiable`, 1..1
    /// Boolean, RM ehr master04 §EHR Active Status). Read from the promoted
    /// `ehr.is_modifiable` column, kept in lockstep with the current
    /// `EHR_STATUS` by [`Self::sync_ehr_subject`] and the import/archive-load
    /// backfill; a missing EHR (`None`) is treated as modifiable so the guard
    /// never spuriously blocks.
    ///
    /// The column read is a storage seam
    /// ([`crate::storage::ehr_repo::ehr_is_modifiable`]; no openEHR spec
    /// governs the promoted column — our own storage design).
    async fn ehr_is_modifiable(&self, ehr_id: EhrId) -> Result<bool, ServiceError> {
        Ok(
            crate::storage::ehr_repo::ehr_is_modifiable(&self.pool, ehr_id)
                .await?
                .unwrap_or(true),
        )
    }

    /// Refuse a write to *EHR contents* when the EHR is deactivated
    /// (`EHR_STATUS.is_modifiable = False`). Per RM ehr master04 §EHR Active
    /// Status, `is_modifiable` "is used to indicate whether the contents of an
    /// EHR are modifiable"; "an EHR's 'contents' consist of everything other
    /// than the `EHR_STATUS` object". The `EHR_STATUS` object itself "is
    /// always modifiable", so this guard is applied to COMPOSITION / DIRECTORY
    /// / content-CONTRIBUTION writes only — the
    /// [`crate::versioning::CommitEnv`] `ensure_content_writable` hook
    /// (G-6-adjacent).
    ///
    /// NOTE (wire): ITS-REST 1.0.3 does not enumerate a status code for a
    /// write to a non-modifiable EHR (`composition_create.yaml` lists only
    /// 201/400/404/422; the CNF schedule `master06-func_tc_ehr.adoc` tests the
    /// flag flip, not the write-block outcome), so the code is
    /// underdetermined. We return `409 Conflict` — the write conflicts with
    /// the current state of the target resource (RFC 9110 §15.5.10), the
    /// closest HTTP semantics.
    ///
    /// # Errors
    /// [`ServiceError::Conflict`] when the EHR is not modifiable;
    /// [`ServiceError::Database`] if the flag read fails.
    pub(in crate::service) async fn ensure_content_writable(
        &self,
        ehr_id: EhrId,
    ) -> Result<(), ServiceError> {
        if self.ehr_is_modifiable(ehr_id).await? {
            Ok(())
        } else {
            Err(Self::not_modifiable_error(ehr_id))
        }
    }

    /// The `409 Conflict` for a content write to a deactivated EHR
    /// (`EHR_STATUS.is_modifiable = false`) — see
    /// [`Self::ensure_content_writable`] for the NOTE on the status-code
    /// choice. Shared with the combined
    /// [`Self::ensure_ehr_content_writable`] pre-check so the message stays
    /// single-sourced.
    pub(in crate::service) fn not_modifiable_error(ehr_id: EhrId) -> ServiceError {
        ServiceError::Conflict(format!(
            "EHR {ehr_id} is not modifiable (EHR_STATUS.is_modifiable = false); its \
             contents cannot be created, updated or deleted (RM ehr master04 §EHR Active \
             Status). Set EHR_STATUS.is_modifiable = true to reactivate it."
        ))
    }

    /// Keep the EHR's promoted subject columns (`ehr.subject_id` /
    /// `subject_namespace`) in sync with the `EHR_STATUS` being committed
    /// (`subject.external_ref.id.value` + `.namespace`). The partial unique
    /// index `ehr_subject_uq` enforces **one EHR per subject** at the database
    /// (RM ehr master04 §EHR Status; ITS-REST `409_EHR.yaml`; CNF
    /// `create_ehr-two_ehrs_same_patient`) — a violation maps to
    /// [`ServiceError::Conflict`] (→ 409). A status without an `external_ref`
    /// (e.g. anonymous `PARTY_SELF`) clears the columns and never conflicts.
    ///
    /// The EHR-owned commit hook the versioning layer lifted out of its write
    /// path (RM common master06 §Committal). Called inside the commit
    /// transaction of the EHR-create / EHR_STATUS-update paths, and — for the
    /// CONTRIBUTION path — through the
    /// [`crate::versioning::CommitEnv::post_status_commit`] hook after an
    /// `EHR_STATUS` version. The `UPDATE` stays inline here (not a plain
    /// `ehr_repo` read) because it maps the subject-uniqueness constraint
    /// violation to a service-level [`ServiceError::Conflict`] (→ 409); the
    /// `ehr.subject_*` columns are spec-silent index plumbing (our own
    /// design).
    ///
    /// # Errors
    /// [`ServiceError::Conflict`] when the subject already owns another EHR
    /// (`uq_ehr_subject`); [`ServiceError::Database`] on any other SQL failure.
    pub(in crate::service) async fn sync_ehr_subject(
        &self,
        tx: &mut PgConnection,
        ehr_id: EhrId,
        canonical: &Value,
    ) -> Result<(), ServiceError> {
        // The same promoted-column extraction the EHR-create path folds into
        // its initial INSERT (single-sourced so both promote identical values).
        // The is_modifiable sync (the content-write guard, RM ehr master04
        // §EHR Active Status) rides this same UPDATE — zero extra statements.
        let (subject_id, namespace, is_queryable, is_modifiable) = ehr_promoted_columns(canonical);
        sqlx::query(
            "UPDATE ehr SET subject_id = $2, subject_namespace = $3, is_queryable = $4, \
             is_modifiable = $5 WHERE id = $1",
        )
        .bind(ehr_id)
        .bind(subject_id)
        .bind(namespace)
        .bind(is_queryable)
        .bind(is_modifiable)
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
}

/// Extract the promoted `ehr`-table columns from an `EHR_STATUS` canonical
/// value: the subject `(id, namespace)` — only a COMPLETE `external_ref` pair
/// identifies a subject (RM ehr master04 §EHR Status; an anonymous
/// `PARTY_SELF` yields `(None, None)`) — and the two 1..1 Boolean status flags
/// `is_queryable` (RM ehr master04 §EHR Status) and `is_modifiable` (RM ehr
/// master04 §EHR Active Status), each defaulting to `true` (matching the
/// column default and the default `EHR_STATUS` when a raw path omits them).
/// Shared by the EHR-create path's folded INSERT and the update/contribution
/// [`EhrbaseService::sync_ehr_subject`] hook so both promote identical values.
/// No openEHR spec governs the promoted columns — our own storage design.
pub(in crate::service) fn ehr_promoted_columns(
    canonical: &Value,
) -> (Option<&str>, Option<&str>, bool, bool) {
    let subject_id = canonical
        .pointer("/subject/external_ref/id/value")
        .and_then(Value::as_str);
    let namespace = canonical
        .pointer("/subject/external_ref/namespace")
        .and_then(Value::as_str);
    let (subject_id, namespace) = match (subject_id, namespace) {
        (Some(id), Some(ns)) => (Some(id), Some(ns)),
        _ => (None, None),
    };
    let is_queryable = canonical
        .pointer("/is_queryable")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let is_modifiable = canonical
        .pointer("/is_modifiable")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    (subject_id, namespace, is_queryable, is_modifiable)
}

// ── The SM I_EHR_STATUS call surface ──────────────────────────────────────────

impl EhrbaseService {
    /// SM `I_EHR_STATUS.has_ehr_status_version` — whether the EHR's
    /// `EHR_STATUS` versioned object is the one named.
    ///
    /// # Errors
    /// [`SmError`] if the current-object resolution fails.
    pub async fn has_ehr_status_version(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: VoId,
    ) -> Result<bool, SmError> {
        // An EHR holds exactly one EHR_STATUS versioned object; the version
        // exists iff that object's `vo_id` matches.
        Ok(self
            .current_vo(an_ehr_id, Kind::EhrStatus)
            .await?
            .is_some_and(|(vo, _)| vo == a_version_uid))
    }

    /// SM `I_EHR_STATUS.get_ehr_status` — the current `EHR_STATUS` (bare, with
    /// its `uid`).
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` (404-equivalent) or
    /// a read fails.
    pub async fn get_ehr_status(&self, an_ehr_id: EhrId) -> Result<Value, SmError> {
        Ok(self.status_at(an_ehr_id, None).await?.body)
    }

    /// SM `I_EHR_STATUS.get_ehr_status_at_time` — the `EHR_STATUS` current at
    /// `a_time` (time-travel), or the current one when `a_time` is `None`.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `a_time` (400-equivalent), a missing status
    /// at that instant (404-equivalent), or a read failure.
    pub async fn get_ehr_status_at_time(
        &self,
        an_ehr_id: EhrId,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_at(an_ehr_id, at).await?.body)
    }

    /// The bare `EHR_STATUS` at a specific version (F-01-03 — not an
    /// `ORIGINAL_VERSION`). `a_version` is the `VERSION_TREE_ID` lexical form.
    ///
    /// # Errors
    /// [`SmError`] for a malformed version id, an unknown version
    /// (404-equivalent), or a read failure.
    pub async fn get_ehr_status_at_version(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: VoId,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = parse_tree_id(a_version)?;
        // the bare EHR_STATUS at that version, not an ORIGINAL_VERSION.
        Ok(self
            .status_by_version(an_ehr_id, a_version_uid, tree)
            .await?
            .body)
    }

    /// SM `I_EHR_STATUS.get_versioned_ehr_status` — the `VERSIONED_EHR_STATUS`
    /// container object.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` (404-equivalent) or
    /// a read fails.
    pub async fn get_versioned_ehr_status(&self, an_ehr_id: EhrId) -> Result<Value, SmError> {
        Ok(self.versioned_status(an_ehr_id).await?)
    }

    /// SM `I_EHR_STATUS.set_ehr_queryable` — commit a new `EHR_STATUS` version
    /// with `is_queryable = true`, returning the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` or the commit
    /// fails.
    pub async fn set_ehr_queryable(&self, an_ehr_id: EhrId) -> Result<String, SmError> {
        Ok(self
            .status_mutate(an_ehr_id, |m| {
                m.insert("is_queryable".to_owned(), Value::Bool(true));
            })
            .await?
            .version_uid())
    }

    /// SM `I_EHR_STATUS.clear_ehr_queryable` — commit a new `EHR_STATUS`
    /// version with `is_queryable = false`, returning the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` or the commit
    /// fails.
    pub async fn clear_ehr_queryable(&self, an_ehr_id: EhrId) -> Result<String, SmError> {
        Ok(self
            .status_mutate(an_ehr_id, |m| {
                m.insert("is_queryable".to_owned(), Value::Bool(false));
            })
            .await?
            .version_uid())
    }

    /// SM `I_EHR_STATUS.set_ehr_modifiable` — commit a new `EHR_STATUS` version
    /// with `is_modifiable = true` (reactivating the EHR's contents), returning
    /// the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` or the commit
    /// fails.
    pub async fn set_ehr_modifiable(&self, an_ehr_id: EhrId) -> Result<String, SmError> {
        Ok(self
            .status_mutate(an_ehr_id, |m| {
                m.insert("is_modifiable".to_owned(), Value::Bool(true));
            })
            .await?
            .version_uid())
    }

    /// SM `I_EHR_STATUS.clear_ehr_modifiable` — commit a new `EHR_STATUS`
    /// version with `is_modifiable = false` (deactivating the EHR's contents),
    /// returning the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` or the commit
    /// fails.
    pub async fn clear_ehr_modifiable(&self, an_ehr_id: EhrId) -> Result<String, SmError> {
        // Committable on the EHR it disables: the write guard scopes to EHR
        // *contents*, never to EHR_STATUS (RM ehr master04 §EHR Active Status).
        Ok(self
            .status_mutate(an_ehr_id, |m| {
                m.insert("is_modifiable".to_owned(), Value::Bool(false));
            })
            .await?
            .version_uid())
    }

    /// SM `I_EHR_STATUS.update_other_details` — commit a new `EHR_STATUS`
    /// version with the given `other_details`, returning the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS`, the mutated status
    /// fails validation (a non-`ITEM_STRUCTURE` `other_details` is
    /// 422-equivalent), or the commit fails.
    pub async fn update_other_details(
        &self,
        an_ehr_id: EhrId,
        a_details: Value,
    ) -> Result<String, SmError> {
        Ok(self
            .status_mutate(an_ehr_id, move |m| {
                m.insert("other_details".to_owned(), a_details);
            })
            .await?
            .version_uid())
    }

    /// Replace the whole `EHR_STATUS` in one commit (`PUT …/ehr_status`),
    /// returning the new version uid.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` (404-equivalent),
    /// the `preceding_version_uid` mismatches the current latest
    /// (412-equivalent), the body fails structural validation
    /// (422-equivalent), or the commit fails.
    pub async fn replace_ehr_status(
        &self,
        an_ehr_id: EhrId,
        a_status: UpdateVersion,
    ) -> Result<String, SmError> {
        // NOTE: the ITS-REST wire replaces the whole EHR_STATUS in one PUT
        // — the aggregate of the five discrete SM mutators above (formal
        // equivalence, `master02-overview.adoc` §Interface Calls). The
        // optimistic `preceding_version_uid` rides in UpdateVersion; a mismatch
        // → 412.
        //
        // The `If-Match` meta pre-read also yields the `vo_id`, threaded into
        // the write so the versioned object is resolved once (no second
        // `current_vo`). No current EHR_STATUS ⇒ NotFound (404), the same
        // outcome the prior `current_vo`-inside-the-write path produced.
        let Some((vo_id, latest)) = self.ehr_status_meta_with_vo(an_ehr_id).await? else {
            return Err(ServiceError::NotFound(format!("EHR_STATUS for EHR {an_ehr_id}")).into());
        };
        ensure_if_match(a_status.preceding_version_uid.as_ref(), Some(&latest))?;
        let if_match = a_status
            .preceding_version_uid
            .as_ref()
            .map(|o| o.value.clone())
            .unwrap_or_default();
        Ok(self
            .commit_status(an_ehr_id, vo_id, a_status, &if_match)
            .await?
            .version_uid())
    }

    /// SM `I_EHR_STATUS.get_revision_history` — the `REVISION_HISTORY` of the
    /// EHR's `EHR_STATUS`.
    ///
    /// # Errors
    /// [`SmError`] when the EHR has no current `EHR_STATUS` (404-equivalent) or
    /// a read fails.
    pub async fn ehr_status_revision_history(&self, an_ehr_id: EhrId) -> Result<Value, SmError> {
        Ok(self.status_revision_history(an_ehr_id).await?)
    }

    /// The `ORIGINAL_VERSION` of the EHR's `EHR_STATUS` extant at `a_time`, or
    /// the latest when `a_time` is `None`
    /// (`GET …/versioned_ehr_status/version`, F-01-05).
    ///
    /// # Errors
    /// [`SmError`] for a malformed `a_time` (400-equivalent), a missing version
    /// at that instant (404-equivalent), or a read failure.
    pub async fn ehr_status_version_at_time(
        &self,
        an_ehr_id: EhrId,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_version_at_time(an_ehr_id, at).await?.body)
    }

    /// The `ORIGINAL_VERSION` of an `EHR_STATUS` at a specific version
    /// (`GET …/versioned_ehr_status/version/{version_uid}`).
    ///
    /// # Errors
    /// [`SmError`] for a malformed version id, an unknown version
    /// (404-equivalent), or a read failure.
    pub async fn ehr_status_original_version(
        &self,
        an_ehr_id: EhrId,
        a_version_uid: VoId,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = parse_tree_id(a_version)?;
        Ok(self.status_version(an_ehr_id, a_version_uid, tree).await?)
    }
}
