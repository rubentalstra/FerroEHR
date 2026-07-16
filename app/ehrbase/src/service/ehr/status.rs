//! `I_EHR_STATUS` (`i_ehr_status.adoc`) — the `EHR_STATUS` reads, the five
//! discrete mutators, the versioned-object views, plus two EHR-owned commit
//! hooks: the `is_modifiable` content-write guard and the promoted-subject sync.
//!
//! Spec: RM ehr `ehr_status.adoc` + `master04-ehr_package.adoc` §EHR Status /
//! §EHR Active Status; the versioned-object mechanics are RM common master06,
//! delegated to [`crate::versioning`]. `EHR_STATUS` "is always modifiable"
//! (master04 §EHR Active Status), so its own commits are never gated by the
//! content-write guard — that is how a deactivated EHR is flipped back on.

use crate::service::{ResourceMeta, ServiceResponse, SmError, UpdateVersion};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    Kind, TreeId, change_type, expected_from_if_match, original_version, parse_tree_id,
    read_current, read_version, revision_history, update, version_at, versioned_object,
};

use super::{ensure_if_match, parse_at_time, version_uid};

impl EhrbaseService {
    /// The `EHR_STATUS` of an EHR as canonical JSON with its `uid` set — the
    /// current version, or the one current at `at` (time-travel) when given.
    pub(in crate::service) async fn status_at(
        &self,
        ehr_id: Uuid,
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
    /// `ORIGINAL_VERSION` wrapper) — `GET …/ehr_status/{version_uid}` (F-01-03).
    pub(in crate::service) async fn status_by_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// Update an EHR's `EHR_STATUS`, returning the new version's metadata. `vo_id`
    /// is the `EHR_STATUS` versioned object (resolved once by the caller's
    /// `If-Match` meta pre-read, so the `current_vo` JOIN is not re-run here);
    /// `if_match` is the `OBJECT_VERSION_ID` (or bare version) the client believes
    /// is current.
    ///
    /// The response is metadata-only, built from the commit's own
    /// [`Committed`](crate::versioning::Committed) (the written version identity +
    /// commit instant, RM common master06 §Committal) — the write path never
    /// re-reads the row it just wrote. Every caller uses only the `uid`
    /// (`version_uid(…)`); a `Prefer: return=representation` body is read back at
    /// the protocol layer, matching the COMPOSITION/DIRECTORY write paths.
    pub(in crate::service) async fn status_update(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        body: Value,
        if_match: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        super::validate_ehr_status(&body)?;
        let expected = expected_from_if_match(if_match)?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "EHR_STATUS update");
        let committed = update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::EhrStatus,
            body.clone(),
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        // Keep the promoted subject columns in sync (the subject may have changed);
        // the is_queryable promotion (Fix B) rides this same UPDATE.
        self.sync_ehr_subject(&mut tx, ehr_id, &body).await?;
        tx.commit().await?;

        Ok(self.committed_response(ehr_id, &committed))
    }

    /// Apply a single in-place mutation to the current `EHR_STATUS` root and
    /// commit it as a new implicit-CONTRIBUTION version — the shared body of the
    /// discrete `I_EHR_STATUS` mutators (`i_ehr_status.adoc` §`set_ehr_queryable` …
    /// §`update_other_details`), formally equivalent to the whole-object replace
    /// (`master02-overview.adoc` §Interface Calls). Reuses [`Self::status_update`]
    /// with the current version uid as the preceding version (server-driven
    /// optimistic lock).
    ///
    /// `EHR_STATUS` "is always modifiable" (RM ehr master04 §EHR Active Status),
    /// so this is deliberately **not** gated by [`Self::ensure_content_writable`]
    /// — that guard scopes to EHR *contents*, never to `EHR_STATUS`, which is how
    /// `clear_ehr_modifiable` disables an EHR yet `set_ehr_modifiable` re-enables
    /// it.
    pub(in crate::service) async fn status_mutate(
        &self,
        ehr_id: Uuid,
        mutate: impl FnOnce(&mut serde_json::Map<String, Value>),
    ) -> Result<ServiceResponse, ServiceError> {
        // Resolve the current EHR_STATUS versioned object once and read its body;
        // the resolved `vo_id` threads into `status_update` so its commit skips a
        // second `current_vo` resolution.
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
            // mutated EHR_STATUS content (the server assigns the new version id).
            map.remove("uid");
            mutate(map);
        }
        self.status_update(ehr_id, vo_id, body, &preceding).await
    }

    /// The `VERSIONED_OBJECT` for an EHR's `EHR_STATUS`.
    pub(in crate::service) async fn versioned_status(
        &self,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        versioned_object(&self.pool, vo_id, ehr_id, "VERSIONED_EHR_STATUS").await
    }

    /// The `REVISION_HISTORY` of an EHR's `EHR_STATUS`.
    pub(in crate::service) async fn status_revision_history(
        &self,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        revision_history(&self.pool, ehr_id, vo_id).await
    }

    /// An `ORIGINAL_VERSION` of an `EHR_STATUS` at a specific version.
    pub(in crate::service) async fn status_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
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
    pub(in crate::service) async fn status_version_at_time(
        &self,
        ehr_id: Uuid,
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

    /// The current `EHR_STATUS` version metadata (for a `412` `ETag`/`Location`).
    pub(in crate::service) async fn ehr_status_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::EhrStatus).await
    }

    /// The current `EHR_STATUS` versioned-object id **and** its version metadata,
    /// resolved and read in ONE metadata-only statement (no node reassembly). The
    /// `PUT …/ehr_status` pre-read: it supplies both the full-`OBJECT_VERSION_ID`
    /// the `If-Match` compare needs (ITS-REST overview §Concurrency control) and
    /// the `vo_id` threaded into [`Self::status_update`], so the update path
    /// resolves the versioned object exactly once. `None` when the EHR has no
    /// current `EHR_STATUS`.
    pub(in crate::service) async fn ehr_status_meta_with_vo(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<(Uuid, ResourceMeta)>, ServiceError> {
        self.latest_version_meta_with_vo(ehr_id, Kind::EhrStatus)
            .await
    }

    /// Whether the EHR is modifiable (RM ehr `EHR_STATUS.is_modifiable`, 1..1
    /// Boolean, RM ehr master04 §EHR Active Status). Read from the promoted
    /// `ehr.is_modifiable` column, kept in lockstep with the current `EHR_STATUS`
    /// by [`Self::sync_ehr_subject`] and the import/archive-load backfill; a
    /// missing EHR (`None`) is treated as modifiable so the guard never
    /// spuriously blocks.
    ///
    /// The column read is a storage seam
    /// ([`crate::storage::ehr_repo::ehr_is_modifiable`]; no openEHR spec governs
    /// the promoted column — our own storage design).
    async fn ehr_is_modifiable(&self, ehr_id: Uuid) -> Result<bool, ServiceError> {
        Ok(
            crate::storage::ehr_repo::ehr_is_modifiable(&self.pool, ehr_id)
                .await?
                .unwrap_or(true),
        )
    }

    /// Refuse a write to *EHR contents* when the EHR is deactivated
    /// (`EHR_STATUS.is_modifiable = False`). Per RM ehr master04 §EHR Active
    /// Status, `is_modifiable` "is used to indicate whether the contents of an
    /// EHR are modifiable"; "an EHR's 'contents' consist of everything other than
    /// the `EHR_STATUS` object". The `EHR_STATUS` object itself "is always
    /// modifiable", so this guard is applied to COMPOSITION / DIRECTORY /
    /// content-CONTRIBUTION writes only — the [`crate::versioning::CommitEnv`]
    /// `ensure_content_writable` hook (G-6-adjacent).
    ///
    /// PORT NOTE (wire): ITS-REST 1.0.3 does not enumerate a status code for a
    /// write to a non-modifiable EHR (`composition_create.yaml` lists only
    /// 201/400/404/422; the CNF schedule `master06-func_tc_ehr.adoc` tests the
    /// flag flip, not the write-block outcome), so the code is underdetermined.
    /// We return `409 Conflict` — the write conflicts with the current state of
    /// the target resource (RFC 9110 §15.5.10), the closest HTTP semantics.
    pub(in crate::service) async fn ensure_content_writable(
        &self,
        ehr_id: Uuid,
    ) -> Result<(), ServiceError> {
        if self.ehr_is_modifiable(ehr_id).await? {
            Ok(())
        } else {
            Err(Self::not_modifiable_error(ehr_id))
        }
    }

    /// The `409 Conflict` for a content write to a deactivated EHR
    /// (`EHR_STATUS.is_modifiable = false`) — see [`Self::ensure_content_writable`]
    /// for the PORT NOTE on the status-code choice. Shared with the combined
    /// [`Self::ensure_ehr_content_writable`] pre-check so the message stays
    /// single-sourced.
    pub(in crate::service) fn not_modifiable_error(ehr_id: Uuid) -> ServiceError {
        ServiceError::Conflict(format!(
            "EHR {ehr_id} is not modifiable (EHR_STATUS.is_modifiable = false); its \
             contents cannot be created, updated or deleted (RM ehr master04 §EHR Active \
             Status). Set EHR_STATUS.is_modifiable = true to reactivate it."
        ))
    }

    /// Keep the EHR's promoted subject columns (`ehr.subject_id` /
    /// `subject_namespace`) in sync with the `EHR_STATUS` being committed
    /// (`subject.external_ref.id.value` + `.namespace`). The partial unique index
    /// `ehr_subject_uq` enforces **one EHR per subject** at the database (RM ehr
    /// master04 §EHR Status; ITS-REST `409_EHR.yaml`; CNF
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
    /// `ehr.subject_*` columns are spec-silent index plumbing (our own design).
    pub(in crate::service) async fn sync_ehr_subject(
        &self,
        tx: &mut PgConnection,
        ehr_id: Uuid,
        canonical: &Value,
    ) -> Result<(), ServiceError> {
        // The same promoted-column extraction the EHR-create path folds into its
        // initial INSERT (single-sourced so both promote identical values). The
        // is_modifiable sync (the content-write guard, RM ehr master04 §EHR
        // Active Status) rides this same UPDATE — zero extra statements.
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
/// identifies a subject (RM ehr master04 §EHR Status; an anonymous `PARTY_SELF`
/// yields `(None, None)`) — and the two 1..1 Boolean status flags `is_queryable`
/// (RM ehr master04 §EHR Status) and `is_modifiable` (RM ehr master04 §EHR
/// Active Status), each defaulting to `true` (matching the column default and
/// the default `EHR_STATUS` when a raw path omits them). Shared by the
/// EHR-create path's folded INSERT and the update/contribution
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

impl EhrbaseService {
    pub async fn has_ehr_status_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
    ) -> Result<bool, SmError> {
        // An EHR holds exactly one EHR_STATUS versioned object; the version
        // exists iff that object's `vo_id` matches.
        Ok(self
            .current_vo(an_ehr_id, Kind::EhrStatus)
            .await?
            .is_some_and(|(vo, _)| vo == a_version_uid))
    }

    pub async fn get_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.status_at(an_ehr_id, None).await?.body)
    }

    pub async fn get_ehr_status_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_at(an_ehr_id, at).await?.body)
    }

    pub async fn get_ehr_status_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = parse_tree_id(a_version)?;
        // F-01-03: the bare EHR_STATUS at that version, not an ORIGINAL_VERSION.
        Ok(self
            .status_by_version(an_ehr_id, a_version_uid, tree)
            .await?
            .body)
    }

    pub async fn get_versioned_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.versioned_status(an_ehr_id).await?)
    }

    pub async fn set_ehr_queryable(&self, an_ehr_id: Uuid) -> Result<String, SmError> {
        version_uid(
            self.status_mutate(an_ehr_id, |m| {
                m.insert("is_queryable".to_owned(), Value::Bool(true));
            })
            .await?,
        )
    }

    pub async fn clear_ehr_queryable(&self, an_ehr_id: Uuid) -> Result<String, SmError> {
        version_uid(
            self.status_mutate(an_ehr_id, |m| {
                m.insert("is_queryable".to_owned(), Value::Bool(false));
            })
            .await?,
        )
    }

    pub async fn set_ehr_modifiable(&self, an_ehr_id: Uuid) -> Result<String, SmError> {
        version_uid(
            self.status_mutate(an_ehr_id, |m| {
                m.insert("is_modifiable".to_owned(), Value::Bool(true));
            })
            .await?,
        )
    }

    pub async fn clear_ehr_modifiable(&self, an_ehr_id: Uuid) -> Result<String, SmError> {
        // Committable on the EHR it disables: the write guard scopes to EHR
        // *contents*, never to EHR_STATUS (RM ehr master04 §EHR Active Status).
        version_uid(
            self.status_mutate(an_ehr_id, |m| {
                m.insert("is_modifiable".to_owned(), Value::Bool(false));
            })
            .await?,
        )
    }

    pub async fn update_other_details(
        &self,
        an_ehr_id: Uuid,
        a_details: Value,
    ) -> Result<String, SmError> {
        version_uid(
            self.status_mutate(an_ehr_id, move |m| {
                m.insert("other_details".to_owned(), a_details);
            })
            .await?,
        )
    }

    pub async fn replace_ehr_status(
        &self,
        an_ehr_id: Uuid,
        a_status: UpdateVersion,
    ) -> Result<String, SmError> {
        // PORT NOTE: the ITS-REST wire replaces the whole EHR_STATUS in one PUT —
        // the aggregate of the five discrete SM mutators above (formal
        // equivalence, `master02-overview.adoc` §Interface Calls). The optimistic
        // `preceding_version_uid` rides in UpdateVersion; a mismatch → 412.
        //
        // The `If-Match` meta pre-read also yields the `vo_id`, threaded into the
        // write so the versioned object is resolved once (no second `current_vo`).
        // No current EHR_STATUS ⇒ NotFound (404), the same outcome the prior
        // `current_vo`-inside-the-write path produced.
        let Some((vo_id, latest)) = self.ehr_status_meta_with_vo(an_ehr_id).await? else {
            return Err(ServiceError::NotFound(format!("EHR_STATUS for EHR {an_ehr_id}")).into());
        };
        ensure_if_match(a_status.preceding_version_uid.as_ref(), Some(&latest))?;
        let if_match = a_status
            .preceding_version_uid
            .map(|o| o.value)
            .unwrap_or_default();
        version_uid(
            self.status_update(an_ehr_id, vo_id, a_status.data, &if_match)
                .await?,
        )
    }

    pub async fn ehr_status_revision_history(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.status_revision_history(an_ehr_id).await?)
    }

    pub async fn ehr_status_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_version_at_time(an_ehr_id, at).await?.body)
    }

    pub async fn ehr_status_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = parse_tree_id(a_version)?;
        Ok(self.status_version(an_ehr_id, a_version_uid, tree).await?)
    }
}
