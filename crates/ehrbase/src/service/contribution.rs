//! CONTRIBUTION create + retrieval — the change-set envelope with its
//! `AUDIT_DETAILS` and the versions it produced.
//!
//! `contribution_create` applies a set of VERSIONs atomically under one
//! contribution (via `vobject::commit_contribution`): each version's action is
//! derived from its `commit_audit.change_type` (creation / modification /
//! deleted), the object kind from the payload `_type` (create) or the stored
//! object (modify / delete), all in one transaction.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::vobject::{self, AuditInput, Change, Kind};
use super::{EhrbaseService, ServiceError};

/// The change an incoming VERSION represents, from its `commit_audit.change_type`.
#[derive(Clone, Copy)]
enum Action {
    Create,
    Modify,
    Delete,
}

impl Action {
    /// The stored `change_type` code-string for the audit row.
    fn change_type(self) -> &'static str {
        match self {
            Action::Create => "creation",
            Action::Modify => "modification",
            Action::Delete => "deleted",
        }
    }
}

impl EhrbaseService {
    /// Commit a CONTRIBUTION: apply its set of VERSIONs atomically (one
    /// contribution + audit, each version its own commit audit), then return the
    /// created CONTRIBUTION. Each version's action is taken from its
    /// `commit_audit.change_type` (creation/modification/deleted), with the
    /// object kind from the payload `_type` (create) or the stored object
    /// (modify/delete).
    pub(super) async fn create_contribution(
        &self,
        ehr_id: Uuid,
        body: Value,
    ) -> Result<Value, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        let versions = body
            .get("versions")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                ServiceError::Unprocessable("contribution must contain versions".to_owned())
            })?;

        let contribution_audit = self.parse_audit(body.get("audit"), Action::Create);

        let mut changes: Vec<(AuditInput, Change)> = Vec::with_capacity(versions.len());
        for version in versions {
            let action = version_action(version);
            let version_audit = self.parse_audit(version.get("commit_audit"), action);
            let data = version.get("data").cloned();
            let change = match action {
                Action::Create => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("creation version needs data".to_owned())
                    })?;
                    let kind = data_kind(&data)?;
                    Change::Create {
                        kind,
                        canonical: data,
                        template_id: None,
                    }
                }
                Action::Modify => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("modification version needs data".to_owned())
                    })?;
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    Change::Modify {
                        vo_id,
                        kind,
                        canonical: data,
                        expected: Some(expected),
                        template_id: None,
                    }
                }
                Action::Delete => {
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    Change::Delete {
                        vo_id,
                        kind,
                        expected: Some(expected),
                    }
                }
            };
            changes.push((version_audit, change));
        }

        let mut tx = self.pool.begin().await?;
        let (contribution_id, _) =
            vobject::commit_contribution(&mut tx, ehr_id, &contribution_audit, changes).await?;
        tx.commit().await?;

        self.get_contribution(ehr_id, contribution_id).await
    }

    /// The stored kind of an existing object, or `NotFound`.
    async fn require_kind(&self, vo_id: Uuid) -> Result<Kind, ServiceError> {
        vobject::object_kind(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))
    }

    /// Build an [`AuditInput`] from an ITS-REST audit object (`UpdateAudit`),
    /// defaulting the change type from the version's action and the committer
    /// from the authenticated principal.
    fn parse_audit(&self, audit: Option<&Value>, action: Action) -> AuditInput {
        let change_type = audit
            .and_then(|a| a.get("change_type"))
            .and_then(coded_value)
            .unwrap_or_else(|| action.change_type().to_owned());
        let description = audit
            .and_then(|a| a.get("description"))
            .and_then(|d| d.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let committer = audit
            .and_then(|a| a.get("committer"))
            .cloned()
            .unwrap_or_else(super::ehr::committer);
        let system_id = audit
            .and_then(|a| a.get("system_id"))
            .and_then(Value::as_str)
            .map_or_else(|| self.system_id.clone(), str::to_owned);
        AuditInput {
            system_id,
            change_type,
            description,
            committer,
        }
    }
    /// Retrieve a CONTRIBUTION by id (scoped to the EHR), with its audit and the
    /// `OBJECT_REFs` of the versions it committed.
    pub(super) async fn get_contribution(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let meta = sqlx::query(
            "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
             FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.id = $1 AND c.ehr_id = $2",
        )
        .bind(contribution_id)
        .bind(ehr_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("CONTRIBUTION {contribution_id}")))?;

        let system_id: String = meta.try_get("system_id")?;
        let change_type: String = meta.try_get("change_type")?;
        let description: Option<String> = meta.try_get("description")?;
        let committer: Value = meta.try_get("committer")?;
        let time_committed: jiff::Timestamp = meta
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff();

        let version_rows = sqlx::query(
            "SELECT vo_id, sys_version, kind FROM vo_version WHERE contribution_id = $1 \
             ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;

        let versions: Vec<Value> = version_rows
            .iter()
            .map(|row| -> Result<Value, ServiceError> {
                let vo_id: Uuid = row.try_get("vo_id")?;
                let sys_version: i32 = row.try_get("sys_version")?;
                let kind: String = row.try_get("kind")?;
                Ok(json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": self.object_version_id(vo_id, sys_version)
                    }
                }))
            })
            .collect::<Result<_, _>>()?;

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": Self::audit_details(&system_id, &change_type, description.as_deref(), &committer, &time_committed),
            "versions": versions
        }))
    }

    /// Build an `AUDIT_DETAILS` from stored audit columns.
    pub(super) fn audit_details(
        system_id: &str,
        change_type: &str,
        description: Option<&str>,
        committer: &Value,
        time_committed: &jiff::Timestamp,
    ) -> Value {
        let mut audit = json!({
            "_type": "AUDIT_DETAILS",
            "system_id": system_id,
            "time_committed": { "_type": "DV_DATE_TIME", "value": time_committed.to_string() },
            "change_type": {
                "_type": "DV_CODED_TEXT",
                "value": change_type,
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": change_type
                }
            },
            "committer": committer
        });
        if let (Some(desc), Value::Object(map)) = (description, &mut audit) {
            map.insert(
                "description".to_owned(),
                json!({ "_type": "DV_TEXT", "value": desc }),
            );
        }
        audit
    }
}

/// The action a VERSION represents, from its `commit_audit.change_type`
/// (openEHR audit change-type group: 249 creation, 251 modification, 523
/// deleted). Falls back to create/modify based on `preceding_version_uid`.
fn version_action(version: &Value) -> Action {
    let code = version
        .get("commit_audit")
        .and_then(|a| a.get("change_type"))
        .and_then(coded_value);
    match code.as_deref() {
        Some("249" | "creation") => Action::Create,
        Some("523" | "deleted") => Action::Delete,
        Some(_) => Action::Modify,
        None if version.get("preceding_version_uid").is_some() => Action::Modify,
        None => Action::Create,
    }
}

/// The change-type code of a `DV_CODED_TEXT`: its `defining_code.code_string`
/// if present, else its `value`.
fn coded_value(dv: &Value) -> Option<String> {
    dv.get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str)
        .or_else(|| dv.get("value").and_then(Value::as_str))
        .map(str::to_owned)
}

/// The versioned-object kind of a VERSION's `data`, from its `_type`.
fn data_kind(data: &Value) -> Result<Kind, ServiceError> {
    let rm_type = data
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Kind::from_type(rm_type).ok_or_else(|| {
        ServiceError::Unprocessable(format!("not a versioned root type: {rm_type:?}"))
    })
}

/// Parse a VERSION's `preceding_version_uid` (`OBJECT_VERSION_ID`, as a string or
/// `{value}`) into the object id and the version it must currently be at.
fn parse_preceding(version: &Value) -> Result<(Uuid, i32), ServiceError> {
    let raw = version
        .get("preceding_version_uid")
        .and_then(|p| {
            p.as_str()
                .or_else(|| p.get("value").and_then(Value::as_str))
        })
        .ok_or_else(|| {
            ServiceError::Unprocessable(
                "preceding_version_uid required for modify/delete".to_owned(),
            )
        })?;
    let mut parts = raw.split("::");
    let vo_id = parts
        .next()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            ServiceError::Unprocessable(format!("invalid preceding_version_uid: {raw}"))
        })?;
    let version = parts
        .nth(1)
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| {
            ServiceError::Unprocessable(format!("preceding_version_uid needs a version: {raw}"))
        })?;
    Ok((vo_id, version))
}
