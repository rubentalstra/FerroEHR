//! DEMOGRAPHIC `PARTY_RELATIONSHIP` domain logic, built on the shared
//! [`vobject`](super::vobject) versioned-object machinery — the same code path
//! as the demographic parties (see [`super::demographic`]), with **no EHR
//! scope** (`ehr_id = None`, ADR-008). A relationship is a versioned object in
//! the demographics repository, but it is *not* a PARTY (it has its own
//! `versioned_party_relationship` read surface).
//!
//! Spec oracle: `SM/docs/UML/classes/i_party_relationship.adoc` (the 6
//! `I_PARTY_RELATIONSHIP` calls) + `i_demographic_service.adoc`
//! (`create_party_relationship(UV_PARTY_RELATIONSHIP): UUID`, pre
//! `valid_content`, server-side `VERSIONED_OBJECT` + `ORIGINAL_VERSION` +
//! `CONTRIBUTION`). ITS-REST 1.0.3 defines no demographic wire contract, so — as
//! for the parties — this behaviour is our own design by analogy with the EHR
//! group (`docs/design/sm-platform/03-demographic-ehr-index-query.md` §5.9).
//!
//! PORT NOTEs on the SM spec asymmetries this module normalizes to the PARTY
//! pattern (design 03 §5.9):
//! - `i_party_relationship.adoc` gives **no** `has_party_relationship`
//!   precondition on `get_party_relationship`, yet lists a
//!   `versioned_object_does_not_exist` error — we treat an unknown id as `404`,
//!   the same has-check the PARTY get performs, so the two demographic families
//!   behave identically.
//! - `update_party_relationship` retains the SM's `definitions_valid`
//!   precondition (structural validity of the new version) rather than the
//!   PARTY's `valid_content`; both reduce to the same structural check here
//!   ([`typed_check`]), so the normalization is behaviour-preserving.

use serde_json::{Value, json};
use uuid::Uuid;

use ehrbase_rest::{ResourceMeta, ServiceResponse};

use super::codes::change_type;
use super::vobject::{self, Kind, VersionRead};
use super::{EhrbaseService, ServiceError};

/// The RM `_type` a `PARTY_RELATIONSHIP` versioned object stores.
const RM_TYPE: &str = "PARTY_RELATIONSHIP";

/// Structurally validate a candidate `PARTY_RELATIONSHIP` body: deserialize
/// into the `openehr_rm` type (a type mismatch → `422`) and enforce that both
/// `source` and `target` `PARTY_REF`s are present. `uid` need not be supplied —
/// the server injects it on read, mirroring the PARTY / COMPOSITION services.
fn typed_check(data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::PartyRelationship;
    // `source`/`target` are mandatory `PARTY_REF`s on the RM type, so a missing
    // one already fails deserialization; the explicit checks below give a
    // relationship-specific `422` message (and guard against a future optionality
    // change in the generated type).
    serde_json::from_value::<PartyRelationship>(data.clone()).map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as PARTY_RELATIONSHIP: {e}"))
    })?;
    for field in ["source", "target"] {
        if data.get(field).is_none_or(Value::is_null) {
            return Err(ServiceError::Unprocessable(format!(
                "PARTY_RELATIONSHIP requires a {field} PARTY_REF"
            )));
        }
    }
    Ok(())
}

/// Validate a relationship body for a direct create/update: its root `_type`
/// must be `PARTY_RELATIONSHIP` (mismatch → `422`), then [`typed_check`].
fn validate_relationship_body(body: &Value) -> Result<(), ServiceError> {
    let declared = body.get("_type").and_then(Value::as_str);
    if declared != Some(RM_TYPE) {
        return Err(ServiceError::Unprocessable(format!(
            "party_relationship _type mismatch: requires {RM_TYPE:?}, got {:?}",
            declared.unwrap_or("<none>"),
        )));
    }
    typed_check(body)
}

/// Validate a relationship version reached through the CONTRIBUTION path (the
/// [`Kind`] was already derived from the payload `_type`, so only the
/// structural check remains). Called from
/// [`validate_for_commit`](EhrbaseService::validate_for_commit).
pub(super) fn validate_relationship_for_commit(data: &Value) -> Result<(), ServiceError> {
    typed_check(data)
}

impl EhrbaseService {
    // ── PARTY_RELATIONSHIP CRUD ──────────────────────────────────────────────

    /// `create_party_relationship` (`i_demographic_service.adoc`): create the
    /// first version of a new `PARTY_RELATIONSHIP` (server-side
    /// `VERSIONED_OBJECT` + `ORIGINAL_VERSION` + `CONTRIBUTION`). Returns it with
    /// its `uid` set and the create-response `ETag`/`Location` metadata.
    pub(super) async fn create_relationship(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        validate_relationship_body(&body)?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "PARTY_RELATIONSHIP creation");
        let committed = vobject::create(
            &mut tx,
            None,
            Kind::PartyRelationship,
            body,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_relationship(committed.vo_id, Some(committed.sys_version), None)
            .await
    }

    /// `get_party_relationship` / `get_party_relationship_at_time`
    /// (`i_party_relationship.adoc`): retrieve a relationship by its
    /// versioned-object id, optionally at a specific version or instant (else
    /// the latest). A deleted current version resolves to `Null` (→ `204`); an
    /// unknown or wrong-kind id is `404`.
    pub(super) async fn read_relationship(
        &self,
        vo_id: Uuid,
        version: Option<i32>,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_relationship_version(vo_id, version, at).await?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.relationship_version_response(vo_id, read))
    }

    /// `update_party_relationship` (`i_party_relationship.adoc`): commit a new
    /// version. `expected` (from `If-Match`) enforces optimistic concurrency (a
    /// stale precondition → `412`). Pre `has_party_relationship` is realized by
    /// [`ensure_relationship`].
    pub(super) async fn update_relationship(
        &self,
        vo_id: Uuid,
        body: Value,
        expected: Option<i32>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_relationship(vo_id).await?;
        validate_relationship_body(&body)?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "PARTY_RELATIONSHIP update");
        let committed = vobject::update(
            &mut tx,
            None,
            vo_id,
            Kind::PartyRelationship,
            body,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_relationship(vo_id, Some(committed.sys_version), None)
            .await
    }

    /// `delete_party_relationship` (`i_party_relationship.adoc`): logically
    /// delete (a `523|deleted|` version). `expected` is the trunk version from
    /// the mandatory `OBJECT_VERSION_ID`; a stale value → `409`, an
    /// already-deleted target → `400` (mirroring the PARTY delete).
    pub(super) async fn delete_relationship(
        &self,
        vo_id: Uuid,
        expected: i32,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_relationship_version(vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "PARTY_RELATIONSHIP {vo_id} is already deleted"
            )));
        }
        if read.sys_version != expected {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                read.sys_version
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "PARTY_RELATIONSHIP delete");
        let committed = vobject::delete(
            &mut tx,
            None,
            vo_id,
            Kind::PartyRelationship,
            Some(expected),
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        Ok(ServiceResponse::deleted(ResourceMeta::new(
            String::new(),
            // Just-created locally → creating_system_id is the service system id
            // (empty → resolved to it).
            self.object_version_id(vo_id, "", committed.sys_version),
        )))
    }

    /// The current relationship version metadata (the latest `version_uid` a
    /// `412` echoes in `ETag`/`Location`), or `None` if unknown/wrong-kind.
    pub(super) async fn relationship_current_meta(
        &self,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        match self.load_relationship_version(vo_id, None, None).await {
            Ok(read) => Ok(Some(ResourceMeta::new(
                String::new(),
                self.object_version_id(vo_id, &read.creating_system_id, read.sys_version),
            ))),
            Err(ServiceError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // ── VERSIONED_PARTY_RELATIONSHIP ─────────────────────────────────────────

    /// The `VERSIONED_OBJECT` wrapper for a relationship. A non-relationship id
    /// is `404`.
    pub(super) async fn versioned_relationship(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let time_created: jiff_sqlx::Timestamp = sqlx::query_scalar(
            "SELECT a.time_committed FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 AND v.sys_version = 1",
        )
        .bind(vo_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned party relationship {vo_id}")))?;
        let time_created = time_created.to_jiff();
        // PORT NOTE: `VERSIONED_OBJECT.owner_id` (1..1) has no EHR owner for a
        // demographic relationship; as for the PARTY versioned object we
        // reference the relationship's own versioned-object id (the demographics
        // repository owns it).
        Ok(json!({
            "_type": "VERSIONED_OBJECT",
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": "PARTY_RELATIONSHIP",
                "id": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        }))
    }

    /// The `REVISION_HISTORY` of a relationship: one item per version with its
    /// `OBJECT_VERSION_ID` and commit `AUDIT_DETAILS`. A non-relationship id is
    /// `404`.
    pub(super) async fn relationship_revision_history(
        &self,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let rows = sqlx::query(
            "SELECT v.sys_version, v.creating_system_id, a.system_id, a.change_type, \
             a.description, a.committer, a.time_committed \
             FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 ORDER BY v.sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            use sqlx::Row;
            let sys_version: i32 = row.try_get("sys_version")?;
            let creating_system_id: String = row.try_get("creating_system_id")?;
            let system_id: String = row.try_get("system_id")?;
            let change_type: String = row.try_get("change_type")?;
            let description: Option<String> = row.try_get("description")?;
            let committer: Value = row.try_get("committer")?;
            let time_committed: jiff::Timestamp = row
                .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                .to_jiff();
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, &creating_system_id, sys_version)
                },
                "audits": [Self::audit_details(
                    &system_id, &change_type, description.as_deref(), &committer, &time_committed,
                )]
            }));
        }
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// `get_party_relationship_at_version` (`i_party_relationship.adoc`): the
    /// `ORIGINAL_VERSION` at a specific version. A non-relationship id is `404`.
    pub(super) async fn relationship_version(
        &self,
        vo_id: Uuid,
        version: i32,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id.is_none())
            .ok_or_else(|| {
                ServiceError::NotFound(format!("party relationship {vo_id} v{version}"))
            })?;
        self.original_version(&read)
    }

    /// The `ORIGINAL_VERSION` extant at `at` (or the latest when `None`), with
    /// `ETag`/`Location` metadata for the VERSION resource.
    pub(super) async fn relationship_version_at_time(
        &self,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_any_relationship(vo_id).await?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| {
            ServiceError::NotFound(format!("party relationship {vo_id} version at time"))
        })?;
        let meta = ResourceMeta::new(
            String::new(),
            self.object_version_id(vo_id, &read.creating_system_id, read.sys_version),
        )
        .with_last_modified(read.time_committed);
        let ov = self.original_version(&read)?;
        Ok(ServiceResponse::new(ov, meta))
    }

    // ── shared helpers ───────────────────────────────────────────────────────

    /// Load a version of a relationship, verifying its kind and ehr-less-ness.
    /// A wrong-kind or unknown id is `404`.
    async fn load_relationship_version(
        &self,
        vo_id: Uuid,
        version: Option<i32>,
        at: Option<jiff::Timestamp>,
    ) -> Result<VersionRead, ServiceError> {
        if vobject::object_kind(&self.pool, vo_id).await? != Some(Kind::PartyRelationship) {
            return Err(ServiceError::NotFound(format!(
                "PARTY_RELATIONSHIP {vo_id}"
            )));
        }
        let read = match (version, at) {
            (Some(v), _) => vobject::read_version(&self.pool, vo_id, v).await?,
            (None, Some(at)) => vobject::version_at(&self.pool, vo_id, at).await?,
            (None, None) => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("PARTY_RELATIONSHIP {vo_id}")))?;
        Ok(read)
    }

    /// Realize the SM `has_party_relationship` precondition: a live (not
    /// deleted) relationship exists.
    async fn ensure_relationship(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        let read = self.load_relationship_version(vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "PARTY_RELATIONSHIP {vo_id} is deleted"
            )));
        }
        Ok(())
    }

    /// Confirm `vo_id` is a relationship (any version) — the check for the
    /// `versioned_party_relationship` reads. A non-relationship id is `404`.
    async fn ensure_any_relationship(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        match vobject::object_kind(&self.pool, vo_id).await? {
            Some(Kind::PartyRelationship) => Ok(()),
            _ => Err(ServiceError::NotFound(format!(
                "versioned party relationship {vo_id}"
            ))),
        }
    }

    /// A [`ServiceResponse`] for a loaded relationship: its canonical body with
    /// the `uid` injected, plus the resource metadata (empty `ehr_id`).
    fn relationship_version_response(&self, vo_id: Uuid, read: VersionRead) -> ServiceResponse {
        let meta = ResourceMeta::new(
            String::new(),
            self.object_version_id(vo_id, &read.creating_system_id, read.sys_version),
        )
        .with_last_modified(read.time_committed);
        ServiceResponse::new(
            self.with_uid(
                read.canonical,
                vo_id,
                &read.creating_system_id,
                read.sys_version,
            ),
            meta,
        )
    }
}
