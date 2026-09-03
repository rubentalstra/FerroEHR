// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The tenant registry: CRUD over the `tenant` table + claim/header →
//! [`TenantContext`] resolution, as methods on `FerroEhrService`.
//!
//! **No openEHR spec governs this — our own design/extension.** master13 is
//! informative deployment guidance and prescribes no multi-tenancy mechanism;
//! master07 governs the `EHR_ACCESS` object and authn-at-deployment, not a
//! tenant registry. Gate: the tenancy-resolution middleware is active only when
//! a deployment configures it; with it off the `tenant` table is never
//! consulted (byte-identical single-tenant behaviour).
//!
//! **Stage-2-adjacent (multi-tenancy): quarantine only.** This module carries
//! the tenancy surface exactly as it exists; it is NOT extended here
//! (enterprise multi-tenancy is Stage 2).
//!
//! A tenant is one logical openEHR system with its own `system_id`. The
//! `tenant` table is deliberately NOT RLS-scoped (it is the registry every
//! tenant's isolation is defined against), so these queries need no session
//! tenant context — and resolution can run before the request's tenant scope
//! is established (no chicken-and-egg).
//
// The helpers below read the `pub(crate)` `pool` + `tenant_cache` fields of
// `crate::service::FerroEhrService`.

use sqlx::Row;
use uuid::Uuid;

use crate::extensions::tenant_context::TenantContext;
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};

/// A stored tenant registry record (the tenant admin API's response row).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TenantRecord {
    /// The tenant's surrogate UUID key.
    pub id: Uuid,
    /// The unique tenant name (the claim/header resolution key).
    pub name: String,
    /// The tenant's openEHR `system_id`.
    pub system_id: String,
    /// The registry row's creation instant.
    pub created_at: jiff::Timestamp,
}

/// A client-submitted tenant definition (`{name, system_id}`, create + update).
///
/// Both fields are required at parse; emptiness-after-trim is validated by the
/// service (`400` either way).
#[derive(Debug, serde::Deserialize)]
pub struct TenantDefinition {
    /// The tenant name (required; non-empty after trimming).
    pub name: String,
    /// The tenant's openEHR `system_id` (required; non-empty after trimming).
    pub system_id: String,
}

impl FerroEhrService {
    /// Map a `tenant` row to its typed record.
    fn tenant_row(row: &sqlx::postgres::PgRow) -> Result<TenantRecord, ServiceError> {
        Ok(TenantRecord {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            system_id: row.try_get("system_id")?,
            created_at: row
                .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
                .to_jiff(),
        })
    }

    /// List every tenant (newest first).
    async fn list_tenants(&self) -> Result<Vec<TenantRecord>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, name, system_id, created_at FROM tenant ORDER BY created_at DESC, id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::tenant_row).collect()
    }

    /// Fetch one tenant by id, or `NotFound`.
    async fn get_tenant(&self, id: Uuid) -> Result<TenantRecord, ServiceError> {
        let row = sqlx::query("SELECT id, name, system_id, created_at FROM tenant WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(Self::tenant_row).ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("tenant {id}"),
            )
        })?
    }

    /// Create a tenant from a [`TenantDefinition`] (both fields non-empty).
    async fn create_tenant(&self, body: &TenantDefinition) -> Result<TenantRecord, ServiceError> {
        let name = required_str(&body.name, "name")?;
        let system_id = required_str(&body.system_id, "system_id")?;
        let row = sqlx::query(
            "INSERT INTO tenant (name, system_id) VALUES ($1, $2) \
             RETURNING id, name, system_id, created_at",
        )
        .bind(name)
        .bind(system_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_insert_error)?;
        self.invalidate_tenant_cache();
        Self::tenant_row(&row)
    }

    /// Update a tenant's `name`/`system_id` (both non-empty).
    async fn update_tenant(
        &self,
        id: Uuid,
        body: &TenantDefinition,
    ) -> Result<TenantRecord, ServiceError> {
        let name = required_str(&body.name, "name")?;
        let system_id = required_str(&body.system_id, "system_id")?;
        let row = sqlx::query(
            "UPDATE tenant SET name = $2, system_id = $3 WHERE id = $1 \
             RETURNING id, name, system_id, created_at",
        )
        .bind(id)
        .bind(name)
        .bind(system_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_insert_error)?;
        self.invalidate_tenant_cache();
        row.as_ref().map(Self::tenant_row).ok_or_else(|| {
            ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("tenant {id}"),
            )
        })?
    }

    /// Delete a tenant — only when it is not the reserved default and owns no
    /// data. The emptiness check scopes a transaction to the *target* tenant
    /// via `SET LOCAL`, so the RLS policy admits the target's rows regardless
    /// of the caller's own tenant context.
    async fn delete_tenant(&self, id: Uuid) -> Result<(), ServiceError> {
        // NOTE: the nil uuid is the reserved default tenant — it matches
        // `ext.current_tenant_id()`'s fallback in
        // `migrations/ext/0002_tenant_context.sql`.
        if id == Uuid::nil() {
            return Err(ServiceError::conflict(
                "the reserved default tenant cannot be deleted".to_owned(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('ferroehr.tenant_id', $1, true)")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        // "Empty" = owns no EHRs and no definition artefacts. The explicit
        // `WHERE tenant_id` is redundant under RLS (the SET LOCAL already scopes
        // to the target) but keeps the count correct when the connection bypasses
        // RLS (a superuser role).
        let owned: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM ehr WHERE tenant_id = $1) \
                  + (SELECT count(*) FROM template_store WHERE tenant_id = $1) \
                  + (SELECT count(*) FROM stored_query WHERE tenant_id = $1) \
                  + (SELECT count(*) FROM archetype_store WHERE tenant_id = $1) \
                  + (SELECT count(*) FROM adl2_artefact WHERE tenant_id = $1)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if owned > 0 {
            return Err(ServiceError::conflict(format!(
                "tenant {id} is not empty ({owned} owned object(s)); purge its data first"
            )));
        }
        let deleted = sqlx::query("DELETE FROM tenant WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(ServiceError::sm(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("tenant {id}"),
            ));
        }
        tx.commit().await?;
        self.invalidate_tenant_cache();
        Ok(())
    }

    /// Resolve a claim/header value (a tenant name or uuid string) to its
    /// [`TenantContext`], caching the hit in-process.
    async fn resolve_tenant(&self, key: &str) -> Result<Option<TenantContext>, ServiceError> {
        if let Some(outcome) = self.tenant_cache.get(key).await {
            return Ok(outcome);
        }
        let row = sqlx::query("SELECT id, system_id FROM tenant WHERE name = $1 OR id::text = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        let outcome = match row {
            Some(row) => Some(TenantContext {
                tenant_id: row.try_get("id")?,
                system_id: row.try_get("system_id")?,
            }),
            // The negative outcome is cached too: an unknown key answers from
            // memory for the TTL window instead of one registry read per
            // request carrying a bogus tenant header.
            None => None,
        };
        self.tenant_cache
            .insert(key.to_owned(), outcome.clone())
            .await;
        Ok(outcome)
    }

    /// Drop the whole resolver cache after any tenant CRUD write (a rename /
    /// `system_id` change / delete can invalidate a cached `name→context`
    /// entry). Tenant writes are rare admin operations; the TTL bounds the
    /// convergence window across instances either way.
    fn invalidate_tenant_cache(&self) {
        self.tenant_cache.invalidate_all();
    }
}

/// Trim a submitted field and require it non-empty, else `400`.
fn required_str<'a>(raw: &'a str, field: &str) -> Result<&'a str, ServiceError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ServiceError::precondition(format!(
            "`{field}` is required and non-empty"
        )));
    }
    Ok(trimmed)
}

/// Map a unique-name violation to `Conflict` (409); other DB errors pass through.
fn map_insert_error(e: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db) = &e
        && db.is_unique_violation()
    {
        return ServiceError::conflict("a tenant with that name already exists".to_owned());
    }
    ServiceError::Database(e)
}

impl FerroEhrService {
    /// List every tenant (newest first) as PHI-free typed records.
    ///
    /// # Errors
    /// [`SmError`] wrapping a database failure.
    pub async fn tenant_list(&self) -> Result<Vec<TenantRecord>, SmError> {
        Ok(self.list_tenants().await?)
    }

    /// Create a tenant from a [`TenantDefinition`].
    ///
    /// # Errors
    /// `BadRequest` when `name`/`system_id` is empty after trimming; `Conflict`
    /// when the name is already taken; otherwise a database failure.
    pub async fn tenant_create(&self, a_tenant: TenantDefinition) -> Result<TenantRecord, SmError> {
        Ok(self.create_tenant(&a_tenant).await?)
    }

    /// Fetch one tenant by id.
    ///
    /// # Errors
    /// `NotFound` when the id is unknown; otherwise a database failure.
    pub async fn tenant_get(&self, a_tenant_id: Uuid) -> Result<TenantRecord, SmError> {
        Ok(self.get_tenant(a_tenant_id).await?)
    }

    /// Replace a tenant's `name`/`system_id`.
    ///
    /// # Errors
    /// `BadRequest` when a field is empty after trimming; `Conflict` on a
    /// duplicate name; `NotFound` when the id is unknown; otherwise a database
    /// failure.
    pub async fn tenant_update(
        &self,
        a_tenant_id: Uuid,
        a_tenant: TenantDefinition,
    ) -> Result<TenantRecord, SmError> {
        Ok(self.update_tenant(a_tenant_id, &a_tenant).await?)
    }

    /// Delete a tenant that is empty and not the reserved default.
    ///
    /// # Errors
    /// `Conflict` when the tenant is the reserved default or still owns data;
    /// `NotFound` when the id is unknown; otherwise a database failure.
    pub async fn tenant_delete(&self, a_tenant_id: Uuid) -> Result<(), SmError> {
        Ok(self.delete_tenant(a_tenant_id).await?)
    }

    /// Resolve a claim/header value (a tenant name or uuid string) to its
    /// [`TenantContext`]; `None` when no tenant matches.
    ///
    /// # Errors
    /// [`SmError`] wrapping a database failure.
    pub async fn tenant_resolve(&self, key: &str) -> Result<Option<TenantContext>, SmError> {
        Ok(self.resolve_tenant(key).await?)
    }
}
