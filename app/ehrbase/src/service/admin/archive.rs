//! Archive markers (SM `I_ADMIN_ARCHIVE.archive_ehrs` / `archive_parties`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc` —
//! `archive_ehrs` "Move selected EHRs to archival storage", `archive_parties`
//! "Move selected Parties and relationships to archival storage".
//!
//! PORT NOTE (re-verify — `i_admin_archive.adoc` says "Move … to archival
//! storage"; G-A2): the *archival storage tier* is spec-silent — openEHR
//! defines no storage mechanics — so this is our own design. This wave realises
//! the "move" as a `vo_archive` marker only: serving reads are unchanged (zero
//! wire drift), and the physical storage movement to a cold tier is deferred.
//! All-or-nothing — an unknown id aborts the transaction before any marker is
//! written.
//!
//! PERF(port): the physical movement of `vo_archive`-marked rows to a cold
//! storage tier (and any read-path effect) is deferred to P20 optimization —
//! no openEHR spec governs storage mechanics, so the tiering is our own design
//! and purely a performance concern, not a conformance one (register 02
//! `storage/`).

use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// `archive_ehrs`: mark every versioned object of each EHR as archived
    /// (idempotent). Any unknown EHR → `ehr_id_does_not_exist`
    /// ([`ServiceError::NotFound`], → `404`) and nothing is archived.
    pub(super) async fn archive_ehr_vos(&self, ehr_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &ehr_id in ehr_ids {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&mut *tx)
                .await?;
            if !exists {
                return Err(ServiceError::NotFound(format!("EHR {ehr_id}")));
            }
        }
        for &ehr_id in ehr_ids {
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, reason) \
                 SELECT DISTINCT vo_id, 'archive_ehrs' FROM vo_version WHERE ehr_id = $1 \
                 ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(ehr_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// `archive_parties`: mark each party's versioned object as archived
    /// (idempotent). Any unknown/non-party id → `party_id_does_not_exist`
    /// ([`ServiceError::NotFound`], → `404`) and nothing is archived.
    ///
    /// PORT NOTE (keep — `i_admin_archive.adoc` "Move selected Parties and
    /// relationships"; G-A3): only the party VO is marked this wave, not the
    /// related `PARTY_RELATIONSHIP`s. While archival is a read-neutral marker
    /// (G-A2) this has no observable effect; the relationship marker set is
    /// extended when the storage-tier movement (G-A2) is realised.
    pub(super) async fn archive_party_vos(&self, party_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &party_id in party_ids {
            let kind: Option<String> = sqlx::query_scalar(
                "SELECT kind FROM vo_version WHERE vo_id = $1 AND ehr_id IS NULL",
            )
            .bind(party_id)
            .fetch_optional(&mut *tx)
            .await?;
            if !kind.as_deref().is_some_and(super::is_party_kind) {
                return Err(ServiceError::NotFound(format!("party {party_id}")));
            }
        }
        for &party_id in party_ids {
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, reason) VALUES ($1, 'archive_parties') \
                 ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(party_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
