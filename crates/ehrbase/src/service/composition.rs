//! COMPOSITION domain logic, built on the [`vobject`](super::vobject)
//! versioned-object machinery (the same code path as `EHR_STATUS`).

use serde_json::Value;
use uuid::Uuid;

use super::vobject::{self, Kind, change_type};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create a COMPOSITION in an EHR, returning it with its `uid` set.
    pub(super) async fn create_composition(
        &self,
        ehr_id: Uuid,
        composition: Value,
    ) -> Result<Value, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "COMPOSITION creation");
        // template_id stays NULL until template ingestion (P13) populates
        // template_store (the column is an FK to it).
        let committed = vobject::create(
            &mut tx,
            ehr_id,
            Kind::Composition,
            composition,
            None,
            &audit,
        )
        .await?;
        tx.commit().await?;

        self.read_composition(ehr_id, committed.vo_id, Some(committed.sys_version))
            .await
    }

    /// Retrieve a COMPOSITION by its versioned-object id, optionally at a
    /// specific version (else the latest).
    pub(super) async fn read_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: Option<i32>,
    ) -> Result<Value, ServiceError> {
        let read = match version {
            Some(v) => vobject::read_version(&self.pool, vo_id, v).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == ehr_id)
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;

        if read.deleted {
            return Err(ServiceError::NotFound(format!(
                "COMPOSITION {vo_id} is deleted"
            )));
        }
        Ok(self.with_uid(read.canonical, vo_id, read.sys_version))
    }

    /// A COMPOSITION as it was at an instant (time-travel), with its `uid` set.
    pub(super) async fn composition_at_time(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        at: jiff::Timestamp,
    ) -> Result<Value, ServiceError> {
        let read = vobject::version_at(&self.pool, vo_id, at)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted {
            return Err(ServiceError::NotFound(format!(
                "COMPOSITION {vo_id} is deleted"
            )));
        }
        Ok(self.with_uid(read.canonical, vo_id, read.sys_version))
    }

    /// The `VERSIONED_OBJECT` for a COMPOSITION (verifies EHR ownership).
    pub(super) async fn versioned_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        Ok(Self::versioned_object(vo_id, read.ehr_id))
    }

    /// An `ORIGINAL_VERSION` of a COMPOSITION at a specific version.
    pub(super) async fn composition_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: i32,
    ) -> Result<Value, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} v{version}")))?;
        Ok(self.original_version(&read))
    }

    /// Commit a new version of a COMPOSITION. `expected` (from `If-Match`)
    /// enforces optimistic concurrency.
    pub(super) async fn update_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        composition: Value,
        expected: Option<i32>,
    ) -> Result<Value, ServiceError> {
        self.ensure_composition_in_ehr(ehr_id, vo_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "COMPOSITION update");
        let committed = vobject::update(
            &mut tx,
            vo_id,
            Kind::Composition,
            composition,
            expected,
            None,
            &audit,
        )
        .await?;
        tx.commit().await?;

        self.read_composition(ehr_id, vo_id, Some(committed.sys_version))
            .await
    }

    /// Logically delete a COMPOSITION (a new `deleted` version).
    pub(super) async fn delete_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.ensure_composition_in_ehr(ehr_id, vo_id).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "COMPOSITION delete");
        vobject::delete(&mut tx, vo_id, Kind::Composition, None, &audit).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(super) async fn ensure_ehr_exists(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        let exists: bool = sqlx::query_scalar("SELECT exists(SELECT 1 FROM ehr WHERE id = $1)")
            .bind(ehr_id)
            .fetch_one(&self.pool)
            .await?;
        if exists {
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("EHR {ehr_id}")))
        }
    }

    /// Confirm a live COMPOSITION with `vo_id` belongs to `ehr_id`.
    async fn ensure_composition_in_ehr(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<(), ServiceError> {
        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted {
            return Err(ServiceError::NotFound(format!(
                "COMPOSITION {vo_id} is deleted"
            )));
        }
        Ok(())
    }
}
