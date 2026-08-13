// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Archival storage movement (SM `I_ADMIN_ARCHIVE.archive_ehrs` /
//! `archive_parties`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc` —
//! `archive_ehrs` "Move selected EHRs to archival storage", `archive_parties`
//! "Move selected Parties and relationships to archival storage".
//!
//! NOTE (`i_admin_archive.adoc` says "Move … to archival storage" and defines
//! no storage form): the archival tier is spec-silent — no openEHR spec governs
//! storage tiering, so the cold schema and this movement are our own design.
//!
//! Each call writes the `vo_archive` markers AND physically moves the marked
//! objects' `vo_version` / `node` / `vo_attestation` rows into the cold tier
//! ([`crate::storage::version_repo::tier`]), both in one transaction. The move
//! is reversible ([`FerroEhrService::restore_archived_ehrs`] /
//! [`FerroEhrService::restore_archived_parties`]) and invisible on the wire:
//! object-addressed reads fall back to the cold tier on a primary-tier miss, so
//! an archived object stays retrievable, and a write thaws it first so a
//! versioned object is never split across tiers. All-or-nothing — an unknown id
//! aborts the transaction before anything is written or moved.
//!
//! NOTE (no openEHR spec governs storage tiering — our own design): the AQL
//! engine queries the primary tier alone, so archived content leaves the
//! queryable store until it is restored — which is what shedding the query
//! tables' rows and indexes means.

use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};
use crate::storage::version_repo::tier;

impl FerroEhrService {
    /// SM `archive_ehrs`: move every versioned object of each EHR to the cold
    /// archival tier (idempotent).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — any id in the list is not a
    ///   well-formed UUID (the whole request is rejected).
    /// - `versioned_object_does_not_exist` (`404`) — any EHR is unknown
    ///   (`ehr_id_does_not_exist`); nothing is archived.
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn archive_ehrs(&self, ehr_ids: Vec<String>) -> Result<(), SmError> {
        let ids: Vec<EhrId> = super::parse_uuid_list(&ehr_ids, "EHR")?
            .into_iter()
            .map(EhrId)
            .collect();
        Ok(self.archive_ehr_vos(&ids).await?)
    }

    /// SM `archive_parties`: move each party's versioned object to the cold
    /// archival tier (idempotent).
    ///
    /// NOTE (keep — `i_admin_archive.adoc` "Move selected Parties and
    /// relationships"): only the party VO is moved, not the related
    /// `PARTY_RELATIONSHIP`s, which stay independently addressable versioned
    /// objects a caller archives in their own right.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — any id in the list is not a
    ///   well-formed UUID (the whole request is rejected).
    /// - `versioned_object_does_not_exist` (`404`) — any id names no
    ///   demographic PARTY root (`party_id_does_not_exist`); nothing is
    ///   archived.
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn archive_parties(&self, party_ids: Vec<String>) -> Result<(), SmError> {
        let ids = super::parse_uuid_list(&party_ids, "party")?;
        Ok(self.archive_party_vos(&ids).await?)
    }

    /// Bring every archived versioned object of each EHR back from the cold
    /// tier to the primary tier, dropping its archive marker — the exact
    /// reverse of [`Self::archive_ehrs`], and idempotent in the same way (an
    /// EHR with nothing archived restores nothing and succeeds).
    ///
    /// NOTE (`i_admin_archive.adoc` declares only the two archive operations):
    /// the SM has no un-archive call, so this operation and its admin route
    /// (`POST /admin/archive/ehrs/restore`) are both our own extension — the
    /// reverse an archival tier must have to be trustworthy.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — any id is not a well-formed UUID.
    /// - `versioned_object_does_not_exist` (`404`) — any EHR is unknown.
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn restore_archived_ehrs(&self, ehr_ids: Vec<String>) -> Result<(), SmError> {
        let ids: Vec<EhrId> = super::parse_uuid_list(&ehr_ids, "EHR")?
            .into_iter()
            .map(EhrId)
            .collect();
        Ok(self.restore_ehr_vos(&ids).await?)
    }

    /// Bring each archived party's versioned object back from the cold tier to
    /// the primary tier, dropping its archive marker — the exact reverse of
    /// [`Self::archive_parties`].
    ///
    /// NOTE (`i_admin_archive.adoc` declares only the two archive operations):
    /// spec-silent like its EHR twin above; served at
    /// `POST /admin/archive/parties/restore`, our own extension.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — any id is not a well-formed UUID.
    /// - `versioned_object_does_not_exist` (`404`) — any id names no
    ///   demographic PARTY root.
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn restore_archived_parties(&self, party_ids: Vec<String>) -> Result<(), SmError> {
        let ids = super::parse_uuid_list(&party_ids, "party")?;
        Ok(self.restore_party_vos(&ids).await?)
    }

    /// Mark and move every versioned object of each EHR, all-or-nothing: every
    /// EHR is existence-checked before anything is written.
    async fn archive_ehr_vos(&self, ehr_ids: &[EhrId]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &ehr_id in ehr_ids {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&mut *tx)
                .await?;
            if !exists {
                return Err(ServiceError::sm(
                    CallStatusType::EhrIdDoesNotExist,
                    format!("EHR {ehr_id}"),
                ));
            }
        }
        for &ehr_id in ehr_ids {
            // The still-live objects of this EHR: what the marker set and the
            // move both address. Already-archived objects have no primary rows
            // left, which is what makes re-archiving a no-op.
            let vo_ids: Vec<VoId> =
                sqlx::query_scalar("SELECT DISTINCT vo_id FROM vo_version WHERE ehr_id = $1")
                    .bind(ehr_id)
                    .fetch_all(&mut *tx)
                    .await?;
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, reason) \
                 SELECT unnest($1::uuid[]), 'archive_ehrs' \
                 ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(&vo_ids)
            .execute(&mut *tx)
            .await?;
            tier::freeze(&mut tx, &vo_ids).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Mark and move each party's versioned object, all-or-nothing: every id is
    /// checked to name a demographic PARTY root before anything is written. An
    /// unknown or non-party id (e.g. a `PARTY_RELATIONSHIP`) is
    /// `party_id_does_not_exist`.
    async fn archive_party_vos(&self, party_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        let mut live: Vec<VoId> = Vec::new();
        for &party_id in party_ids {
            let kind = party_kind_any_tier(&mut tx, party_id).await?;
            if !kind.as_deref().is_some_and(super::is_party_kind) {
                return Err(ServiceError::sm(
                    CallStatusType::PartyIdDoesNotExist,
                    format!("party {party_id}"),
                ));
            }
            live.push(VoId(party_id));
        }
        sqlx::query(
            "INSERT INTO vo_archive (vo_id, reason) \
             SELECT unnest($1::uuid[]), 'archive_parties' \
             ON CONFLICT (vo_id) DO NOTHING",
        )
        .bind(&live)
        .execute(&mut *tx)
        .await?;
        tier::freeze(&mut tx, &live).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Restore every archived versioned object of each EHR, all-or-nothing.
    async fn restore_ehr_vos(&self, ehr_ids: &[EhrId]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &ehr_id in ehr_ids {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&mut *tx)
                .await?;
            if !exists {
                return Err(ServiceError::sm(
                    CallStatusType::EhrIdDoesNotExist,
                    format!("EHR {ehr_id}"),
                ));
            }
        }
        for &ehr_id in ehr_ids {
            let vo_ids: Vec<VoId> =
                sqlx::query_scalar("SELECT DISTINCT vo_id FROM cold.vo_version WHERE ehr_id = $1")
                    .bind(ehr_id)
                    .fetch_all(&mut *tx)
                    .await?;
            tier::thaw(&mut tx, &vo_ids).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Restore each archived party's versioned object, all-or-nothing.
    async fn restore_party_vos(&self, party_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        let mut ids: Vec<VoId> = Vec::new();
        for &party_id in party_ids {
            let kind = party_kind_any_tier(&mut tx, party_id).await?;
            if !kind.as_deref().is_some_and(super::is_party_kind) {
                return Err(ServiceError::sm(
                    CallStatusType::PartyIdDoesNotExist,
                    format!("party {party_id}"),
                ));
            }
            ids.push(VoId(party_id));
        }
        tier::thaw(&mut tx, &ids).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// The `vo_version.kind` of a demographic (ehr-less) versioned object in EITHER
/// storage tier — the guard both the archive and the restore call must answer
/// the same way whichever tier the party currently lives in.
async fn party_kind_any_tier(
    tx: &mut sqlx::PgConnection,
    party_id: Uuid,
) -> Result<Option<String>, ServiceError> {
    Ok(sqlx::query_scalar(
        "SELECT kind FROM vo_version_all WHERE vo_id = $1 AND ehr_id IS NULL LIMIT 1",
    )
    .bind(party_id)
    .fetch_optional(&mut *tx)
    .await?)
}
