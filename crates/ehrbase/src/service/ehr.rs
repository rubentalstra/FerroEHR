//! EHR + `EHR_STATUS` domain logic, built on the [`vobject`](super::vobject)
//! versioned-object machinery. This is the first fully-implemented vertical of
//! the P12 service; COMPOSITION / DIRECTORY reuse the same machinery.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::vobject::{self, AuditInput, Kind, change_type};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create an EHR (with the given id) and its initial `EHR_STATUS`. Shared by
    /// `POST /ehr` and `PUT /ehr/{ehr_id}`.
    pub(super) async fn create_ehr(
        &self,
        ehr_id: Uuid,
        status: Value,
    ) -> Result<Value, ServiceError> {
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
        vobject::create(&mut tx, ehr_id, Kind::EhrStatus, status, None, &audit).await?;
        tx.commit().await?;

        self.ehr_summary(ehr_id).await
    }

    /// Build the canonical EHR object for an existing EHR.
    pub(super) async fn ehr_summary(&self, ehr_id: Uuid) -> Result<Value, ServiceError> {
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

        Ok(json!({
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
        }))
    }

    /// The `EHR_STATUS` of an EHR as canonical JSON with its `uid` set — the
    /// current version, or the one current at `at` (time-travel) when given.
    pub(super) async fn status_at(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.with_uid(read.canonical, vo_id, read.sys_version))
    }

    /// Update an EHR's `EHR_STATUS`, returning the new version. `if_match` is the
    /// `OBJECT_VERSION_ID` (or bare version) the client believes is current.
    pub(super) async fn status_update(
        &self,
        ehr_id: Uuid,
        body: Value,
        if_match: &str,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let expected = parse_expected_version(if_match);

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "EHR_STATUS update");
        let committed = vobject::update(
            &mut tx,
            ehr_id,
            vo_id,
            Kind::EhrStatus,
            body,
            expected,
            None,
            &audit,
        )
        .await?;
        tx.commit().await?;

        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.with_uid(read.canonical, vo_id, committed.sys_version))
    }

    /// The `VERSIONED_OBJECT` for an EHR's `EHR_STATUS`.
    pub(super) async fn versioned_status(&self, ehr_id: Uuid) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(Self::versioned_object(vo_id, ehr_id))
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
            .filter(|r| r.ehr_id == ehr_id)
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        Ok(self.original_version(&read))
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

/// Extract the expected version number from an `If-Match` header value: either a
/// bare integer or the `version_tree_id` tail of an `OBJECT_VERSION_ID`
/// (`uuid::system::N`). Returns `None` when it cannot be parsed (no precondition
/// enforced).
fn parse_expected_version(if_match: &str) -> Option<i32> {
    let token = if_match.trim().trim_matches('"');
    token
        .rsplit("::")
        .next()
        .and_then(|v| v.parse::<i32>().ok())
        .or_else(|| token.parse::<i32>().ok())
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

    #[test]
    fn expected_version_parsing() {
        assert_eq!(parse_expected_version("\"abc::sys::3\""), Some(3));
        assert_eq!(parse_expected_version("2"), Some(2));
        assert_eq!(parse_expected_version("garbage"), None);
    }
}
