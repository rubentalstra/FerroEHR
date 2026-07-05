//! The shared versioned-object machinery: persist and load COMPOSITION /
//! `EHR_STATUS` / FOLDER uniformly (ADR-008). All writes run inside a caller-owned
//! `sqlx` transaction so a version + its nodes + the contribution + the audit
//! commit atomically.

use sqlx::{PgConnection, PgPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::storage::{NodeRow, decompose, reassemble};

use super::ServiceError;

/// The kind of versioned object (discriminates `vo_version.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Composition,
    EhrStatus,
    Folder,
}

impl Kind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Kind::Composition => "COMPOSITION",
            Kind::EhrStatus => "EHR_STATUS",
            Kind::Folder => "FOLDER",
        }
    }

    /// The versioned-object kind for an RM `_type`, if it is a versioned root.
    pub(super) fn from_type(rm_type: &str) -> Option<Self> {
        match rm_type {
            "COMPOSITION" => Some(Kind::Composition),
            "EHR_STATUS" => Some(Kind::EhrStatus),
            "FOLDER" => Some(Kind::Folder),
            _ => None,
        }
    }
}

/// The kind of the current version of an object, or `None` if it does not exist.
pub(super) async fn object_kind(pool: &PgPool, vo_id: Uuid) -> Result<Option<Kind>, ServiceError> {
    let row = sqlx::query("SELECT kind FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period)")
        .bind(vo_id)
        .fetch_optional(pool)
        .await?;
    Ok(match row {
        Some(r) => Kind::from_type(&r.try_get::<String, _>("kind")?),
        None => None,
    })
}

/// openEHR audit `change_type` code strings (openEHR Terminology group 249..).
pub(super) mod change_type {
    pub(crate) const CREATION: &str = "creation";
    pub(crate) const MODIFICATION: &str = "modification";
    pub(crate) const DELETED: &str = "deleted";
}

/// What an audit row records about a committed change.
#[derive(Debug, Clone)]
pub(super) struct AuditInput {
    pub(super) system_id: String,
    pub(super) change_type: String,
    pub(super) description: Option<String>,
    /// Canonical `PARTY_PROXY` of the committer.
    pub(super) committer: serde_json::Value,
}

/// The outcome of a versioned-object write: the object id, the new version
/// number, and the CONTRIBUTION that produced it.
#[derive(Debug, Clone)]
pub(super) struct Committed {
    pub(super) vo_id: Uuid,
    pub(super) sys_version: i32,
    /// The CONTRIBUTION this write created. Read by `commit_contribution` (to
    /// group versions) and the create-response `Location`; retained as part of
    /// the write result.
    #[allow(dead_code)]
    pub(super) contribution_id: Uuid,
}

/// A loaded version: its full provenance metadata and reassembled canonical JSON.
#[derive(Debug, Clone)]
pub(super) struct VersionRead {
    pub(super) vo_id: Uuid,
    pub(super) ehr_id: Uuid,
    pub(super) sys_version: i32,
    pub(super) deleted: bool,
    pub(super) contribution_id: Uuid,
    pub(super) canonical: serde_json::Value,
}

/// Insert an `audit` row, returning its id.
async fn insert_audit(tx: &mut PgConnection, audit: &AuditInput) -> Result<Uuid, ServiceError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO audit (system_id, change_type, description, committer) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&audit.system_id)
    .bind(&audit.change_type)
    .bind(&audit.description)
    .bind(&audit.committer)
    .fetch_one(&mut *tx)
    .await?)
}

/// Insert a `contribution` row referencing its audit, returning its id.
async fn insert_contribution(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    audit_id: Uuid,
) -> Result<Uuid, ServiceError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO contribution (ehr_id, audit_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(ehr_id)
    .bind(audit_id)
    .fetch_one(&mut *tx)
    .await?)
}

/// Insert an `audit` row and its enclosing `contribution`, returning both ids.
async fn write_contribution(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    audit: &AuditInput,
) -> Result<(Uuid, Uuid), ServiceError> {
    let audit_id = insert_audit(tx, audit).await?;
    let contribution_id = insert_contribution(tx, ehr_id, audit_id).await?;
    Ok((contribution_id, audit_id))
}

/// Bulk-insert the decomposed node rows for one version.
async fn insert_nodes(
    tx: &mut PgConnection,
    vo_id: Uuid,
    sys_version: i32,
    ehr_id: Uuid,
    rows: &[NodeRow],
) -> Result<(), ServiceError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO node (vo_id, sys_version, num, num_cap, parent_num, citem_num, ehr_id, \
         rm_type, archetype, name, path, data) ",
    );
    qb.push_values(rows, |mut b, row| {
        b.push_bind(vo_id)
            .push_bind(sys_version)
            .push_bind(row.num)
            .push_bind(row.num_cap)
            .push_bind(row.parent_num)
            .push_bind(row.citem_num)
            .push_bind(ehr_id)
            .push_bind(&row.rm_type)
            .push_bind(&row.archetype)
            .push_bind(&row.name)
            .push_bind(&row.path)
            .push_bind(&row.data);
    });
    qb.build().execute(&mut *tx).await?;
    Ok(())
}

/// One change applied within a CONTRIBUTION (the openEHR change-set unit).
pub(super) enum Change {
    /// Create a new versioned object.
    Create {
        kind: Kind,
        canonical: serde_json::Value,
        template_id: Option<String>,
    },
    /// Commit a new version of an existing object.
    Modify {
        vo_id: Uuid,
        kind: Kind,
        canonical: serde_json::Value,
        expected: Option<i32>,
        template_id: Option<String>,
    },
    /// Logically delete an object (a content-less `deleted` version).
    Delete {
        vo_id: Uuid,
        kind: Kind,
        expected: Option<i32>,
    },
}

/// The core write path shared by single-object writes and CONTRIBUTION commits:
/// apply one [`Change`] under an already-open contribution + version audit.
async fn apply_change(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    contribution_id: Uuid,
    audit_id: Uuid,
    change: Change,
) -> Result<Committed, ServiceError> {
    match change {
        Change::Create {
            kind,
            canonical,
            template_id,
        } => {
            let rows = decompose(canonical)?;
            let vo_id = Uuid::now_v7();
            insert_vo_version(
                tx,
                vo_id,
                kind,
                ehr_id,
                1,
                false,
                contribution_id,
                audit_id,
                template_id.as_deref(),
            )
            .await?;
            insert_nodes(tx, vo_id, 1, ehr_id, &rows).await?;
            Ok(Committed {
                vo_id,
                sys_version: 1,
                contribution_id,
            })
        }
        Change::Modify {
            vo_id,
            kind,
            canonical,
            expected,
            template_id,
        } => {
            let rows = decompose(canonical)?;
            let next = next_version(&mut *tx, ehr_id, vo_id, kind, expected).await?;
            close_current(&mut *tx, vo_id).await?;
            insert_vo_version(
                tx,
                vo_id,
                kind,
                ehr_id,
                next,
                false,
                contribution_id,
                audit_id,
                template_id.as_deref(),
            )
            .await?;
            insert_nodes(tx, vo_id, next, ehr_id, &rows).await?;
            Ok(Committed {
                vo_id,
                sys_version: next,
                contribution_id,
            })
        }
        Change::Delete {
            vo_id,
            kind,
            expected,
        } => {
            let next = next_version(&mut *tx, ehr_id, vo_id, kind, expected).await?;
            close_current(&mut *tx, vo_id).await?;
            insert_vo_version(
                tx,
                vo_id,
                kind,
                ehr_id,
                next,
                true,
                contribution_id,
                audit_id,
                None,
            )
            .await?;
            Ok(Committed {
                vo_id,
                sys_version: next,
                contribution_id,
            })
        }
    }
}

/// Validate an update/delete target (belongs to `ehr_id`, `If-Match` matches)
/// and return the next version number. Locks the current row `FOR UPDATE`.
async fn next_version(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<i32>,
) -> Result<i32, ServiceError> {
    let (owner, current) = current_version(&mut *tx, vo_id, kind).await?;
    if owner != ehr_id {
        return Err(ServiceError::NotFound(format!(
            "{} {vo_id} in EHR {ehr_id}",
            kind.as_str()
        )));
    }
    if let Some(expected) = expected
        && expected != current
    {
        return Err(ServiceError::VersionConflict(format!(
            "expected version {expected}, current is {current}"
        )));
    }
    Ok(current + 1)
}

/// Insert one `vo_version` row (validity `[now, ∞)`).
#[allow(clippy::too_many_arguments)] // one row's columns; a struct would not read clearer
async fn insert_vo_version(
    tx: &mut PgConnection,
    vo_id: Uuid,
    kind: Kind,
    ehr_id: Uuid,
    sys_version: i32,
    deleted: bool,
    contribution_id: Uuid,
    audit_id: Uuid,
    template_id: Option<&str>,
) -> Result<(), ServiceError> {
    sqlx::query(
        "INSERT INTO vo_version \
         (vo_id, kind, ehr_id, sys_version, sys_period, deleted, contribution_id, audit_id, template_id) \
         VALUES ($1, $2, $3, $4, tstzrange(now(), NULL, '[)'), $5, $6, $7, $8)",
    )
    .bind(vo_id)
    .bind(kind.as_str())
    .bind(ehr_id)
    .bind(sys_version)
    .bind(deleted)
    .bind(contribution_id)
    .bind(audit_id)
    .bind(template_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Create the first version of a new versioned object under its own contribution.
pub(super) async fn create(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    kind: Kind,
    canonical: serde_json::Value,
    template_id: Option<&str>,
    audit: &AuditInput,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id) = write_contribution(tx, ehr_id, audit).await?;
    apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        Change::Create {
            kind,
            canonical,
            template_id: template_id.map(str::to_owned),
        },
    )
    .await
}

/// Commit a new version of an existing object under its own contribution.
#[allow(clippy::too_many_arguments)] // the write parameters; a struct would not read clearer
pub(super) async fn update(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    vo_id: Uuid,
    kind: Kind,
    canonical: serde_json::Value,
    expected: Option<i32>,
    template_id: Option<&str>,
    audit: &AuditInput,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id) = write_contribution(tx, ehr_id, audit).await?;
    apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        Change::Modify {
            vo_id,
            kind,
            canonical,
            expected,
            template_id: template_id.map(str::to_owned),
        },
    )
    .await
}

/// Logically delete an object under its own contribution.
pub(super) async fn delete(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    vo_id: Uuid,
    kind: Kind,
    expected: Option<i32>,
    audit: &AuditInput,
) -> Result<Committed, ServiceError> {
    let (contribution_id, audit_id) = write_contribution(tx, ehr_id, audit).await?;
    apply_change(
        tx,
        ehr_id,
        contribution_id,
        audit_id,
        Change::Delete {
            vo_id,
            kind,
            expected,
        },
    )
    .await
}

/// Commit a set of changes atomically under one CONTRIBUTION. `contribution_audit`
/// is the CONTRIBUTION's own audit; each change carries its VERSION `commit_audit`.
pub(super) async fn commit_contribution(
    tx: &mut PgConnection,
    ehr_id: Uuid,
    contribution_audit: &AuditInput,
    changes: Vec<(AuditInput, Change)>,
) -> Result<(Uuid, Vec<Committed>), ServiceError> {
    let contribution_audit_id = insert_audit(tx, contribution_audit).await?;
    let contribution_id = insert_contribution(tx, ehr_id, contribution_audit_id).await?;
    let mut committed = Vec::with_capacity(changes.len());
    for (version_audit, change) in changes {
        let audit_id = insert_audit(tx, &version_audit).await?;
        committed.push(apply_change(tx, ehr_id, contribution_id, audit_id, change).await?);
    }
    Ok((contribution_id, committed))
}

/// The current (`upper_inf`) version number of an object, plus its `ehr_id`.
/// Locks the row `FOR UPDATE` so concurrent updates serialize.
async fn current_version(
    tx: &mut PgConnection,
    vo_id: Uuid,
    kind: Kind,
) -> Result<(Uuid, i32), ServiceError> {
    let row = sqlx::query(
        "SELECT ehr_id, sys_version FROM vo_version \
         WHERE vo_id = $1 AND kind = $2 AND upper_inf(sys_period) FOR UPDATE",
    )
    .bind(vo_id)
    .bind(kind.as_str())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.as_str())))?;
    Ok((row.try_get("ehr_id")?, row.try_get("sys_version")?))
}

async fn close_current(tx: &mut PgConnection, vo_id: Uuid) -> Result<(), ServiceError> {
    sqlx::query(
        "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), now(), '[)') \
         WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Read the current version of an object by id (any kind). `None` if it never
/// existed; a `deleted` current version is returned with `deleted = true` so
/// callers can distinguish 404 (never existed) from 410 (deleted).
pub(super) async fn read_current(
    pool: &PgPool,
    vo_id: Uuid,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(meta) = sqlx::query(
        "SELECT ehr_id, sys_version, deleted, contribution_id FROM vo_version \
         WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let sys_version: i32 = meta.try_get("sys_version")?;
    let canonical = read_nodes(pool, vo_id, sys_version).await?;
    Ok(Some(VersionRead {
        vo_id,
        ehr_id: meta.try_get("ehr_id")?,
        sys_version,
        deleted: meta.try_get("deleted")?,
        contribution_id: meta.try_get("contribution_id")?,
        canonical,
    }))
}

/// Read a specific `sys_version` of an object (for `.../version/{version_uid}`).
pub(super) async fn read_version(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(meta) = sqlx::query(
        "SELECT ehr_id, deleted, contribution_id FROM vo_version \
         WHERE vo_id = $1 AND sys_version = $2",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let canonical = read_nodes(pool, vo_id, sys_version).await?;
    Ok(Some(VersionRead {
        vo_id,
        ehr_id: meta.try_get("ehr_id")?,
        sys_version,
        deleted: meta.try_get("deleted")?,
        contribution_id: meta.try_get("contribution_id")?,
        canonical,
    }))
}

/// Read the version of an object that was current at a given instant
/// (time-travel): the row whose `sys_period` contains `at`. `None` if the
/// object did not exist at that time.
pub(super) async fn version_at(
    pool: &PgPool,
    vo_id: Uuid,
    at: jiff::Timestamp,
) -> Result<Option<VersionRead>, ServiceError> {
    let Some(meta) = sqlx::query(
        "SELECT ehr_id, sys_version, deleted, contribution_id FROM vo_version \
         WHERE vo_id = $1 AND sys_period @> $2::timestamptz",
    )
    .bind(vo_id)
    .bind(at.to_string())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let sys_version: i32 = meta.try_get("sys_version")?;
    let canonical = read_nodes(pool, vo_id, sys_version).await?;
    Ok(Some(VersionRead {
        vo_id,
        ehr_id: meta.try_get("ehr_id")?,
        sys_version,
        deleted: meta.try_get("deleted")?,
        contribution_id: meta.try_get("contribution_id")?,
        canonical,
    }))
}

/// Reassemble the canonical JSON of one stored version from its `node` rows.
async fn read_nodes(
    pool: &PgPool,
    vo_id: Uuid,
    sys_version: i32,
) -> Result<serde_json::Value, ServiceError> {
    let rows = sqlx::query(
        "SELECT num, num_cap, parent_num, citem_num, rm_type, archetype, name, path, data \
         FROM node WHERE vo_id = $1 AND sys_version = $2 ORDER BY num",
    )
    .bind(vo_id)
    .bind(sys_version)
    .fetch_all(pool)
    .await?;

    let mut node_rows = Vec::with_capacity(rows.len());
    for row in rows {
        node_rows.push(NodeRow {
            num: row.try_get("num")?,
            num_cap: row.try_get("num_cap")?,
            parent_num: row.try_get("parent_num")?,
            citem_num: row.try_get("citem_num")?,
            rm_type: row.try_get("rm_type")?,
            archetype: row.try_get("archetype")?,
            name: row.try_get("name")?,
            path: row.try_get("path")?,
            data: row.try_get("data")?,
        });
    }
    Ok(reassemble(&node_rows)?)
}
