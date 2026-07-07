//! COMPOSITION domain logic, built on the [`vobject`](super::vobject)
//! versioned-object machinery (the same code path as `EHR_STATUS`).

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use serde_json::Value;
use uuid::Uuid;

use super::codes::change_type;
use super::vobject::{self, Kind};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create a COMPOSITION in an EHR, returning it with its `uid` set and the
    /// version metadata (the `ETag`/`Location` for `201_COMPOSITION`).
    pub(super) async fn create_composition(
        &self,
        ehr_id: Uuid,
        composition: Value,
    ) -> Result<ServiceResponse, ServiceError> {
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
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_composition(ehr_id, committed.vo_id, Some(committed.sys_version))
            .await
    }

    /// Retrieve a COMPOSITION by its versioned-object id, optionally at a
    /// specific version (else the latest).
    ///
    /// A deleted version resolves to `Value::Null`, which the REST layer renders
    /// as `204 No Content` (`composition_get.yaml` `204_because_deleted*`;
    /// finding F-02-01) — never a 404 or 500.
    pub(super) async fn read_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: Option<i32>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match version {
            Some(v) => vobject::read_version(&self.pool, vo_id, v).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == ehr_id)
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;

        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// A COMPOSITION as it was at an instant (time-travel), with its `uid` set.
    /// A deleted version resolves to an empty body (→ `204`, F-02-01).
    pub(super) async fn composition_at_time(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        at: jiff::Timestamp,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::version_at(&self.pool, vo_id, at)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.version_response(ehr_id, vo_id, read))
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
        self.versioned_object(vo_id, read.ehr_id).await
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

    /// The `ORIGINAL_VERSION` of a COMPOSITION extant at `at`, or the latest
    /// when `at` is `None` —
    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version`
    /// (`versioned_composition_version_get_at_time.yaml`; finding F-02-04). The
    /// metadata carries the `version_uid` for
    /// `200_VERSION_of_COMPOSITION_at_time`'s `ETag`/`Location`. A version that
    /// is deleted still returns `200` with the deleted-lifecycle
    /// `ORIGINAL_VERSION` (no `data`).
    pub(super) async fn composition_version_at_time(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id == ehr_id)
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} version at time")))?;
        let meta = self.version_meta(ehr_id, vo_id, read.sys_version, read.time_committed);
        Ok(ServiceResponse::new(self.original_version(&read), meta))
    }

    /// Commit a new version of a COMPOSITION. `expected` (from `If-Match`)
    /// enforces optimistic concurrency.
    pub(super) async fn update_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        composition: Value,
        expected: Option<i32>,
    ) -> Result<ServiceResponse, ServiceError> {
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
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_composition(ehr_id, vo_id, Some(committed.sys_version))
            .await
    }

    /// The current COMPOSITION version metadata (the latest `version_uid` a
    /// `409`/`412` must echo in `ETag`/`Location`), or `None` if unknown/deleted.
    pub(super) async fn composition_current_meta(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        let Some(read) = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
        else {
            return Ok(None);
        };
        Ok(Some(self.version_meta(
            ehr_id,
            vo_id,
            read.sys_version,
            read.time_committed,
        )))
    }

    /// Logically delete a COMPOSITION (a new `523|deleted|` version).
    ///
    /// `expected` is the version tree id carried by the mandatory
    /// `preceding_version_uid` (`composition_delete.yaml`: the `uid_based_id`
    /// MUST be an `OBJECT_VERSION_ID` naming the version to delete). A stale
    /// `preceding_version_uid` → `409 Conflict`
    /// (`409_COMPOSITION_with_uid_based_id.yaml`); an already-deleted target →
    /// `400` (`400_already_deleted.yaml`) — finding F-02-05.
    pub(super) async fn delete_composition(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        expected: i32,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "COMPOSITION {vo_id} is already deleted"
            )));
        }
        if read.sys_version != expected {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                read.sys_version
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "COMPOSITION delete");
        // Pass `expected` to the write too, so a concurrent update between the
        // check above and the commit is caught atomically.
        let committed = vobject::delete(
            &mut tx,
            ehr_id,
            vo_id,
            Kind::Composition,
            Some(expected),
            &audit,
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);
        // 204_COMPOSITION_deleted: the (now deleted) version_uid in ETag/Location.
        Ok(ServiceResponse::deleted(ResourceMeta::new(
            ehr_id.to_string(),
            self.object_version_id(vo_id, committed.sys_version),
        )))
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
        if read.deleted() {
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
    /// `archetype_details/template_id` cannot be *template*-validated. But the
    /// RM class-invariant and RM-mandated terminology passes are
    /// template-independent — they hold for every RM instance — so they run
    /// unconditionally; only the archetype-conformance pass is gated on a
    /// resolved template (finding F-07-02). A declared-but-failing template, or
    /// any RM/terminology violation, is a `422`
    /// (`.../responses/422_COMPOSITION.yaml`); syntactic parse/convert failures
    /// are `400` and are caught earlier at the REST negotiation edge.
    async fn validate_composition_for_commit(
        &self,
        composition: &Value,
    ) -> Result<(), ServiceError> {
        // Always: RM class invariants + RM-mandated openEHR terminology.
        let mut messages = openehr_flat::validate_rm_and_terminology(composition);
        let rm_terminology_failures = messages.len();
        // Additionally: archetype conformance, when a template is declared.
        if let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        {
            let wt = self.web_template_for(template_id).await?;
            messages.extend(openehr_flat::validate_archetype_conformance(
                composition,
                &wt,
            ));
        }
        // §1.2 validation_failures_total{pass}. openEHR-flat groups RM-invariant
        // + terminology into one pass; template (archetype conformance) is the
        // second.
        let template_failures = messages.len() - rm_terminology_failures;
        if rm_terminology_failures > 0 {
            metrics::counter!(
                crate::telemetry::prometheus::VALIDATION_FAILURES,
                "pass" => "rm_terminology",
            )
            .increment(rm_terminology_failures as u64);
        }
        if template_failures > 0 {
            metrics::counter!(
                crate::telemetry::prometheus::VALIDATION_FAILURES,
                "pass" => "template",
            )
            .increment(template_failures as u64);
        }
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

    /// Validate a versioned object about to be committed (direct or via a
    /// CONTRIBUTION): COMPOSITIONs get the full RM + terminology + template
    /// validation; other kinds (`EHR_STATUS` / FOLDER) have no template validator
    /// yet and pass through. Shared by the direct create/update path and the
    /// CONTRIBUTION path so neither can bypass validation (finding F-07-01).
    pub(super) async fn validate_for_commit(
        &self,
        kind: Kind,
        data: &Value,
    ) -> Result<(), ServiceError> {
        match kind {
            Kind::Composition => self.validate_composition_for_commit(data).await,
            Kind::EhrStatus | Kind::EhrAccess | Kind::Folder => Ok(()),
        }
    }
}
