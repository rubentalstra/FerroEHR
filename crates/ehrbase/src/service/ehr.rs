//! EHR + `EHR_STATUS` domain logic, built on the [`vobject`](super::vobject)
//! versioned-object machinery. This is the first fully-implemented vertical of
//! the P12 service; COMPOSITION / DIRECTORY reuse the same machinery.

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes::change_type;
use super::vobject::{self, AuditInput, Change, Kind};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create an EHR (with the given id), its initial `EHR_STATUS`, and its
    /// `EHR_ACCESS`, all committed under **one** CONTRIBUTION — RM ehr §"EHR
    /// Creation": "the result should be a root EHR object, an EHR Status
    /// object, and an EHR Access object … the EHR Status and EHR Access objects
    /// would be created and committed in a Contribution". Shared by `POST /ehr`
    /// and `PUT /ehr/{ehr_id}`. The response carries the EHR body and its
    /// `ehr_id` (the `ETag`/`Location` for `201_EHR`).
    ///
    /// A duplicate subject (`EHR_STATUS.subject.external_ref`) conflicts at the
    /// database (`ehr_subject_uq`) → 409 (`409_EHR.yaml`; CNF
    /// `create_ehr-two_ehrs_same_patient`).
    pub(super) async fn create_ehr(
        &self,
        ehr_id: Uuid,
        status: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        let mut tx = self.pool.begin().await?;

        let inserted = sqlx::query("INSERT INTO ehr (id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(ehr_id)
            .execute(&mut *tx)
            .await?;
        if inserted.rows_affected() == 0 {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already exists"
            )));
        }

        let audit = self.audit(change_type::CREATION, "EHR creation");
        vobject::commit_contribution(
            &mut tx,
            Some(ehr_id),
            &audit,
            vec![
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrStatus,
                        canonical: status,
                        template_id: None,
                        signature: None,
                    },
                ),
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrAccess,
                        canonical: default_ehr_access(),
                        template_id: None,
                        signature: None,
                    },
                ),
            ],
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;

        self.ehr_summary(ehr_id).await
    }

    /// Find an EHR by the subject its current `EHR_STATUS` names (external ref
    /// `id.value` + `namespace`), returning the EHR summary. Served from the
    /// promoted `ehr.subject_*` columns the `EHR_STATUS` writes keep in sync
    /// (unique per subject — `ehr_subject_uq`).
    pub(super) async fn ehr_by_subject(
        &self,
        subject_id: &str,
        namespace: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        let ehr_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM ehr WHERE subject_id = $1 AND subject_namespace = $2",
        )
        .bind(subject_id)
        .bind(namespace)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR for subject {subject_id}@{namespace}"))
        })?;
        self.ehr_summary(ehr_id).await
    }

    /// Build the canonical EHR object for an existing EHR, with its `ehr_id`
    /// metadata (the `ETag`/`Location` for `POST /ehr`).
    pub(super) async fn ehr_summary(&self, ehr_id: Uuid) -> Result<ServiceResponse, ServiceError> {
        let row = sqlx::query("SELECT time_created FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        // timestamptz via the official jiff-sqlx wrapper (sqlx-conventions.md).
        let time_created: jiff::Timestamp = row
            .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
            .to_jiff();

        let (status_vo, status_version) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let status_ovid = self.object_version_id(status_vo, status_version);

        let mut body = json!({
            "_type": "EHR",
            "system_id": { "_type": "HIER_OBJECT_ID", "value": self.system_id },
            "ehr_id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() },
            "ehr_status": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "EHR_STATUS",
                "id": { "_type": "OBJECT_VERSION_ID", "value": status_ovid }
            },
            "time_created": {
                "_type": "DV_DATE_TIME",
                "value": time_created.to_string()
            }
        });
        // EHR.ehr_access (1..1): a reference to the VERSIONED_EHR_ACCESS version
        // container — invariant `Ehr_access_valid:
        // ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")` (RM ehr, EHR class;
        // finding F-06-07). Every EHR this service creates has one; tolerate its
        // absence only for rows inserted outside `create_ehr` (raw fixtures).
        if let Some((access_vo, _)) = self.current_vo(ehr_id, Kind::EhrAccess).await? {
            body["ehr_access"] = json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_ACCESS",
                "id": { "_type": "HIER_OBJECT_ID", "value": access_vo.to_string() }
            });
        }
        // For an EHR the `ETag`/`Location` are keyed by the `ehr_id`
        // (`ETag_EHR.yaml` / `Location_EHR.yaml`).
        let meta = ResourceMeta::new(ehr_id.to_string(), ehr_id.to_string())
            .with_last_modified(time_created);
        Ok(ServiceResponse::new(body, meta))
    }

    /// The `EHR_STATUS` of an EHR as canonical JSON with its `uid` set — the
    /// current version, or the one current at `at` (time-travel) when given.
    pub(super) async fn status_at(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `EHR_STATUS` at a specific version as canonical JSON with its `uid`
    /// set — the **bare** resource (not the `ORIGINAL_VERSION` wrapper) that
    /// `GET /ehr/{ehr_id}/ehr_status/{version_uid}` returns (F-01-03).
    pub(super) async fn status_by_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: i32,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// Update an EHR's `EHR_STATUS`, returning the new version. `if_match` is the
    /// `OBJECT_VERSION_ID` (or bare version) the client believes is current.
    pub(super) async fn status_update(
        &self,
        ehr_id: Uuid,
        body: Value,
        if_match: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let expected = super::version_id::expected_from_if_match(if_match);

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "EHR_STATUS update");
        vobject::update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::EhrStatus,
            body,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;

        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `VERSIONED_OBJECT` for an EHR's `EHR_STATUS`.
    pub(super) async fn versioned_status(&self, ehr_id: Uuid) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        self.versioned_object(vo_id, ehr_id).await
    }

    /// The `REVISION_HISTORY` of an EHR's `EHR_STATUS`.
    pub(super) async fn status_revision_history(
        &self,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        self.revision_history(ehr_id, vo_id).await
    }

    /// An `ORIGINAL_VERSION` of an `EHR_STATUS` at a specific version.
    pub(super) async fn status_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: i32,
    ) -> Result<Value, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        self.original_version(&read)
    }

    /// The `ORIGINAL_VERSION` of an EHR's `EHR_STATUS` extant at `at`, or the
    /// latest when `at` is `None` — `GET /ehr/{ehr_id}/versioned_ehr_status/version`
    /// (`versioned_ehr_status_version_get_at_time.yaml`; finding F-01-05). The
    /// metadata carries the `version_uid` for `200_VERSION_at_time`'s
    /// `ETag`/`Location`.
    pub(super) async fn status_version_at_time(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR_STATUS version at time for EHR {ehr_id}"))
        })?;
        let meta = self.version_meta(ehr_id, vo_id, read.sys_version, read.time_committed);
        let ov = self.original_version(&read)?;
        Ok(ServiceResponse::new(ov, meta))
    }

    /// The current `EHR_STATUS` version metadata (for a `412` `ETag`/`Location`).
    pub(super) async fn ehr_status_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::EhrStatus).await
    }

    /// The current directory FOLDER version metadata (for a `412`
    /// `ETag`/`Location`).
    pub(super) async fn directory_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::Folder).await
    }

    /// The current version row (`vo_id`, `sys_version`) of an EHR's object of a
    /// given kind, if any.
    pub(super) async fn current_vo(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<(Uuid, i32)>, ServiceError> {
        let row = sqlx::query(
            "SELECT vo_id, sys_version FROM vo_version \
             WHERE ehr_id = $1 AND kind = $2 AND upper_inf(sys_period)",
        )
        .bind(ehr_id)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => Ok(Some((r.try_get("vo_id")?, r.try_get("sys_version")?))),
            None => Ok(None),
        }
    }

    pub(super) fn object_version_id(&self, vo_id: Uuid, sys_version: i32) -> String {
        format!("{vo_id}::{}::{sys_version}", self.system_id)
    }

    /// The [`ResourceMeta`] for a versioned resource: the owning EHR plus the
    /// resource `OBJECT_VERSION_ID` (the `ETag` value + `Location` tail) and its
    /// commit time.
    pub(super) fn version_meta(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        sys_version: i32,
        at: jiff::Timestamp,
    ) -> ResourceMeta {
        ResourceMeta::new(
            ehr_id.to_string(),
            self.object_version_id(vo_id, sys_version),
        )
        .with_last_modified(at)
    }

    /// A [`ServiceResponse`] for a loaded versioned object: its canonical body
    /// with the `uid` injected, plus the resource metadata for the wire headers.
    pub(super) fn version_response(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        read: vobject::VersionRead,
    ) -> ServiceResponse {
        let meta = self.version_meta(ehr_id, vo_id, read.sys_version, read.time_committed);
        ServiceResponse::new(self.with_uid(read.canonical, vo_id, read.sys_version), meta)
    }

    /// The current version metadata of an EHR-owned versioned object of `kind`
    /// (for the latest `version_uid` a `409`/`412` must echo). `None` if none.
    pub(super) async fn latest_version_meta(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        let Some((vo_id, _)) = self.current_vo(ehr_id, kind).await? else {
            return Ok(None);
        };
        let Some(read) = vobject::read_current(&self.pool, vo_id).await? else {
            return Ok(None);
        };
        Ok(Some(self.version_meta(
            ehr_id,
            vo_id,
            read.sys_version,
            read.time_committed,
        )))
    }

    /// Inject the `uid` (`OBJECT_VERSION_ID`) into a versioned object's JSON.
    pub(super) fn with_uid(&self, mut canonical: Value, vo_id: Uuid, sys_version: i32) -> Value {
        if let Value::Object(map) = &mut canonical {
            map.insert(
                "uid".to_owned(),
                json!({
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, sys_version)
                }),
            );
        }
        canonical
    }

    pub(super) fn audit(&self, change_type: &str, description: &str) -> AuditInput {
        AuditInput {
            system_id: self.system_id.clone(),
            change_type: change_type.to_owned(),
            description: Some(description.to_owned()),
            committer: committer(),
        }
    }
}

/// The default `EHR_STATUS` for a new EHR (queryable, modifiable, `PARTY_SELF`).
pub(super) fn default_ehr_status() -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": true
    })
}

/// The default `EHR_ACCESS` created with every EHR (RM ehr §"EHR Creation";
/// finding F-06-07). `EHR_ACCESS` is a LOCATABLE with only the optional
/// `settings` attribute; with no access-control scheme configured (Stage 1 has
/// no RBAC — Stage 2), it is committed with none.
pub(super) fn default_ehr_access() -> Value {
    json!({
        "_type": "EHR_ACCESS",
        "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Access" }
    })
}

/// The committer `PARTY_PROXY` for an audit, taken from the authenticated
/// principal of the current request (published by the auth middleware). Writes
/// with no authenticated principal (auth disabled, or internal/system writes)
/// are attributed to the system identity.
pub(super) fn committer() -> Value {
    match ehrbase_rest::auth::current_principal() {
        Some(principal) => {
            let id_type = match principal.method {
                ehrbase_rest::AuthMethod::Basic => "basic",
                ehrbase_rest::AuthMethod::Bearer => "oauth2",
            };
            json!({
                "_type": "PARTY_IDENTIFIED",
                "name": principal.subject.clone(),
                "identifiers": [{
                    "_type": "DV_IDENTIFIER",
                    "id": principal.subject,
                    "issuer": "ehrbase-rs",
                    "type": id_type
                }]
            })
        }
        None => json!({ "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_decomposes() {
        // The default EHR_STATUS must be a valid structure root for the codec.
        let rows = crate::storage::decompose(default_ehr_status()).expect("decompose");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rm_type, "EHR_STATUS");
    }
}
