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
        self.validate_composition_for_commit(&composition).await?;

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
        self.validate_composition_for_commit(&composition).await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "COMPOSITION update");
        let committed = vobject::update(
            &mut tx,
            ehr_id,
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
        vobject::delete(&mut tx, ehr_id, vo_id, Kind::Composition, None, &audit).await?;
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

    /// Validate an incoming COMPOSITION against its operational template before
    /// it is persisted (the single choke point for the JSON dispatch path and
    /// the FLAT path, both of which reach `create_composition`/`update_composition`).
    ///
    /// PORT NOTE: openEHR ITS-REST 1.0.3 —
    /// `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`:
    /// a well-formed COMPOSITION that references an unknown template, or that
    /// the template "is not validating", is `422 Unprocessable Entity` — not
    /// `400`. Syntactic parse/convert failures are `400`
    /// (`.../responses/400_COMPOSITION.yaml`) and are caught earlier at the REST
    /// negotiation edge, before the service sees the value. `EHRbase`'s CNF Robot
    /// suite asserts `400` for some *structurally* invalid bodies (rejected by a
    /// JSON/XML schema pass before OPT validation, per
    /// `docs/specs/openehr/CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`
    /// `== Test Environment`); lacking that schema pass we surface such cases
    /// through the validator as `422` ("converts, but does not validate"),
    /// following the 422 spec text rather than `EHRbase`'s schema-layer split.
    ///
    /// PORT NOTE: `ARCHETYPED.template_id` is optional in the openEHR RM
    /// (`docs/specs/openehr/RM/docs/common/`), so a COMPOSITION that declares no
    /// `archetype_details/template_id` cannot be template-validated and is
    /// committed without template-conformance checks (its RM class invariants
    /// still apply). Only a *declared* template drives a `422` here. This
    /// narrows the "absent template → 422" reading: the existing storage-
    /// lifecycle tests commit templateless skeleton COMPOSITIONs, which the RM
    /// permits and which have no template to validate against.
    async fn validate_composition_for_commit(
        &self,
        composition: &Value,
    ) -> Result<(), ServiceError> {
        let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        let wt = self.web_template_for(template_id).await?;
        let messages = openehr_flat::validate_composition(composition, &wt);
        if messages.is_empty() {
            return Ok(());
        }
        let errors = messages
            .into_iter()
            .map(|m| openehr_its::rest::runtime::ValidationError {
                path: m.path,
                message: m.message,
            })
            .collect();
        Err(ServiceError::ValidationFailed(errors))
    }
}
