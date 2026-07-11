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
        // EHR_STATUS.is_modifiable = False forbids content writes (ehr/master04
        // §"EHR Active Status"); a COMPOSITION is EHR content.
        self.ensure_content_writable(ehr_id).await?;
        // The direct COMPOSITION create/update endpoints carry no
        // `lifecycle_state` (it is an `ORIGINAL_VERSION` attribute, set only via
        // a CONTRIBUTION `UPDATE_VERSION`), so a direct commit is always
        // `532|complete|` → full-strictness validation (`incomplete = false`).
        self.validate_composition_for_commit(&composition, false)
            .await?;
        self.reject_duplicate_persistent(ehr_id, &composition)
            .await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "COMPOSITION creation");
        // template_id stays NULL until template ingestion (P13) populates
        // template_store (the column is an FK to it).
        let committed = vobject::create(
            &mut tx,
            Some(ehr_id),
            Kind::Composition,
            composition,
            None,
            &audit,
            &self.signing_ctx(),
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
        .filter(|r| r.ehr_id == Some(ehr_id))
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
            .filter(|r| r.ehr_id == Some(ehr_id))
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
        let _read = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        self.versioned_object(vo_id, ehr_id).await
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
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} v{version}")))?;
        self.original_version(&read)
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
        .filter(|r| r.ehr_id == Some(ehr_id))
        .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id} version at time")))?;
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.sys_version,
            read.time_committed,
        );
        let ov = self.original_version(&read)?;
        Ok(ServiceResponse::new(ov, meta))
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
        let current = vobject::read_current(&self.pool, vo_id)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if current.deleted() {
            return Err(ServiceError::NotFound(format!(
                "COMPOSITION {vo_id} is deleted"
            )));
        }
        // EHR_STATUS.is_modifiable = False forbids content writes (ehr/master04
        // §"EHR Active Status").
        self.ensure_content_writable(ehr_id).await?;
        // Reject an update whose body declares a *different* template than the
        // stored composition it supersedes (CNF master07
        // `update_composition-wrong_template`). ITS-REST `422_COMPOSITION`
        // ("could be converted … but there are semantic validation errors") is
        // the fit — a template change is not a syntactic (400) or
        // precondition (412) failure.
        if let (Some(stored), Some(incoming)) = (
            composition_template_id(&current.canonical),
            composition_template_id(&composition),
        ) && stored != incoming
        {
            return Err(ServiceError::Unprocessable(format!(
                "update COMPOSITION references template {incoming}, but the stored \
                 composition was committed against template {stored} (template_id mismatch)"
            )));
        }
        // Direct update carries no lifecycle_state (see `create_composition`).
        self.validate_composition_for_commit(&composition, false)
            .await?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "COMPOSITION update");
        let committed = vobject::update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::Composition,
            composition,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
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
            .filter(|r| r.ehr_id == Some(ehr_id))
        else {
            return Ok(None);
        };
        Ok(Some(self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.sys_version,
            read.time_committed,
        )))
    }

    /// The OPT `template_id` a COMPOSITION version was committed against, read
    /// back from `vo_version.template_id` (`docs/enterprise/access-control.md`
    /// §6.2) — the ABAC template attribute for the `composition_delete`
    /// pre-check (the template of the preceding, still-current version) and any
    /// resolver over a specific version. `version` = `None` reads the current
    /// version. `Ok(None)` when the object is unknown or carries no template.
    ///
    /// PERF(port): this goes through the full version read-back (node
    /// reassembly) for spec fidelity; a direct `SELECT template_id FROM
    /// vo_version` is a cheaper equivalent if this ever shows on a hot path.
    ///
    /// # Errors
    /// [`ServiceError`] if the version read-back query fails.
    pub async fn template_of_version(
        &self,
        vo_id: Uuid,
        version: Option<i32>,
    ) -> Result<Option<String>, ServiceError> {
        let read = match version {
            Some(v) => vobject::read_version(&self.pool, vo_id, v).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        };
        Ok(read.and_then(|r| r.template_id))
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
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("COMPOSITION {vo_id}")))?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "COMPOSITION {vo_id} is already deleted"
            )));
        }
        // EHR_STATUS.is_modifiable = False forbids content writes, incl. logical
        // delete (a delete is a new `523|deleted|` version — ehr/master04
        // §"EHR Active Status").
        self.ensure_content_writable(ehr_id).await?;
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
            Some(ehr_id),
            vo_id,
            Kind::Composition,
            Some(expected),
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);
        // 204_COMPOSITION_deleted: the (now deleted) version_uid in ETag/Location.
        // The version was just created locally, so its creating_system_id is the
        // service system id (passed empty → resolved to it).
        Ok(ServiceResponse::deleted(ResourceMeta::new(
            ehr_id.to_string(),
            self.object_version_id(vo_id, "", committed.sys_version),
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

    /// Enforce the CNF persistent-COMPOSITION uniqueness convention: an EHR may
    /// hold only one *live* persistent COMPOSITION per template, so a second
    /// `create` for the same persistent OPT must be rejected (CNF master07
    /// `create_composition-same_opt_twice`: "only one 'create' is allowed for
    /// persistent COMPOSITIONs, the next operations … should be modifications").
    ///
    /// PORT NOTE: the openEHR RM does **not** define this cardinality — the CNF
    /// schedule itself records it as "under debate in the openEHR SEC … due to
    /// the lack of information in the openEHR specifications"
    /// (`docs/specs/openehr/CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`).
    /// We adopt the CNF criterion. Only persistent COMPOSITIONs with a declared
    /// template are constrained; event/episodic COMPOSITIONs are unbounded.
    async fn reject_duplicate_persistent(
        &self,
        ehr_id: Uuid,
        composition: &Value,
    ) -> Result<(), ServiceError> {
        if !is_persistent(composition) {
            return Ok(());
        }
        let Some(template_id) = composition_template_id(composition) else {
            return Ok(());
        };
        // PERF(port): the OPT `template_id` is not promoted onto `vo_version`, so
        // this scans the EHR's live COMPOSITIONs and reassembles each to read its
        // category + template. An EHR holds few persistent compositions; if this
        // ever shows on a hot path, populate `vo_version.template_id` and filter
        // in SQL.
        let vo_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT vo_id FROM vo_version WHERE ehr_id = $1 AND kind = 'COMPOSITION' \
             AND upper_inf(sys_period) AND lifecycle_state <> $2",
        )
        .bind(ehr_id)
        .bind(super::codes::lifecycle::DELETED)
        .fetch_all(&self.pool)
        .await?;
        for vo_id in vo_ids {
            if let Some(read) = vobject::read_current(&self.pool, vo_id).await?
                && is_persistent(&read.canonical)
                && composition_template_id(&read.canonical) == Some(template_id)
            {
                return Err(ServiceError::Conflict(format!(
                    "EHR {ehr_id} already has a persistent COMPOSITION for template \
                     {template_id}; only one create is allowed (subsequent commits must \
                     be modifications)"
                )));
            }
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
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        // Always: RM class invariants + RM-mandated openEHR terminology. These
        // are properties of the RM instance ("wrongness" checks) and hold even
        // for a `553|incomplete|` commit — the master06 relaxation only zeroes
        // archetype/template existence & cardinality lower bounds, it does not
        // permit *wrong* data.
        let mut messages = openehr_flat::validate_rm_and_terminology(composition);
        let rm_terminology_failures = messages.len();
        // Additionally: archetype conformance, when a template is declared.
        // A `553|incomplete|` commit uses the relaxed pass (existence/occurrences/
        // cardinality lower limits treated as zero — RM common master06
        // §"Incomplete Content").
        if let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        {
            let wt = self.web_template_for(template_id).await?;
            messages.extend(if incomplete {
                openehr_flat::validate_archetype_conformance_incomplete(composition, &wt)
            } else {
                openehr_flat::validate_archetype_conformance(composition, &wt)
            });
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
    ///
    /// `incomplete` (a `553|incomplete|` CONTRIBUTION version, RM common master06
    /// §"Incomplete Content") relaxes the archetype/template existence &
    /// cardinality **lower** limits to zero for COMPOSITIONs; RM invariants and
    /// terminology stay at full strictness ("data may be missing, but it may not
    /// be wrong"). The direct endpoints have no `lifecycle_state` and always pass
    /// `false`.
    pub(super) async fn validate_for_commit(
        &self,
        kind: Kind,
        data: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        match kind {
            Kind::Composition => self.validate_composition_for_commit(data, incomplete).await,
            // An EHR_STATUS committed via a CONTRIBUTION is validated exactly as
            // one supplied on EHR create (CNF master08
            // `commit_contribution-invalid_ehr_status`).
            Kind::EhrStatus => super::ehr::validate_ehr_status(data),
            Kind::EhrAccess | Kind::Folder => Ok(()),
            // Demographic party roots validate structurally (typed deserialize +
            // PARTY invariants) via the demographic module.
            Kind::Agent | Kind::Group | Kind::Organisation | Kind::Person | Kind::Role => {
                Self::validate_party_kind_for_commit(kind, data)
            }
            // PARTY_RELATIONSHIP validates structurally (typed deserialize +
            // source/target present) via the relationship module.
            Kind::PartyRelationship => super::relationship::validate_relationship_for_commit(data),
        }
    }
}

/// The OPT `template_id` a COMPOSITION declares
/// (`archetype_details.template_id.value`), if any.
fn composition_template_id(composition: &Value) -> Option<&str> {
    composition
        .pointer("/archetype_details/template_id/value")
        .and_then(Value::as_str)
}

/// Whether a COMPOSITION is `431|persistent|` (RM composition, COMPOSITION.category
/// / `is_persistent()`), read from its `category.defining_code.code_string`.
fn is_persistent(composition: &Value) -> bool {
    composition
        .pointer("/category/defining_code/code_string")
        .and_then(Value::as_str)
        == Some("431")
}
