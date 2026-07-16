//! The tenant registry and its [`TenantAdapter`] impl on [`EhrbaseService`]
//! (G-12-06).
//!
//! **No openEHR spec governs this — our own design/extension.** master13 is
//! informative deployment guidance and prescribes no multi-tenancy mechanism;
//! master07 governs the `EHR_ACCESS` object and authn-at-deployment, not a
//! tenant registry. Quarantined under `crate::extensions`
//! (`docs/design/platform/12-extensions.md`). Gate: tenancy-resolution
//! middleware is active only when a deployment configures it; with it off the
//! `tenant` table is never consulted (byte-identical single-tenant behaviour).
//!
//! **Stage-2-adjacent (multi-tenancy): quarantine only.** This module carries
//! the tenancy surface exactly as it exists; it is NOT extended here (enterprise
//! multi-tenancy is Stage 2).
//!
//! A tenant is one logical openEHR system with its own `system_id`. This module
//! owns the CRUD against the `tenant` table plus the claim/header →
//! [`TenantContext`] resolution the tenant-resolution middleware calls once per
//! request. The `tenant` table is deliberately NOT RLS-scoped (it is the
//! registry every tenant's isolation is defined against), so these queries need
//! no session tenant context — and resolution can run before the request's
//! tenant scope is established (no chicken-and-egg).
//
// The helpers below read the `pub(crate)` `pool` + `tenant_cache` fields of
// `crate::service::EhrbaseService`.

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::extensions::tenant_context::TenantContext;
use crate::service::status::SmError;

use crate::service::{EhrbaseService, ServiceError};

/// The reserved default tenant: the nil uuid, owner of every row
/// created while tenancy is off. Matches `ext.current_tenant_id()`'s fallback
/// (`migrations/ext/0002_tenant_context.sql`) and cannot be deleted.
const DEFAULT_TENANT_ID: Uuid = Uuid::nil();

impl EhrbaseService {
    /// Map a `tenant` row to its JSON record.
    fn tenant_row(row: &sqlx::postgres::PgRow) -> Result<Value, ServiceError> {
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let system_id: String = row.try_get("system_id")?;
        let created_at = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
            .to_jiff();
        Ok(json!({
            "id": id.to_string(),
            "name": name,
            "system_id": system_id,
            "created_at": created_at.to_string(),
        }))
    }

    /// List every tenant (newest first).
    async fn list_tenants(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, name, system_id, created_at FROM tenant ORDER BY created_at DESC, id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::tenant_row).collect()
    }

    /// Fetch one tenant by id.
    async fn get_tenant(&self, id: Uuid) -> Result<Value, ServiceError> {
        let row = sqlx::query("SELECT id, name, system_id, created_at FROM tenant WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref()
            .map(Self::tenant_row)
            .ok_or_else(|| ServiceError::NotFound(format!("tenant {id}")))?
    }

    /// Create a tenant from `{name, system_id}`.
    async fn create_tenant(&self, body: &Value) -> Result<Value, ServiceError> {
        let name = required_str(body, "name")?;
        let system_id = required_str(body, "system_id")?;
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

    /// Update a tenant's `name`/`system_id`.
    async fn update_tenant(&self, id: Uuid, body: &Value) -> Result<Value, ServiceError> {
        let name = required_str(body, "name")?;
        let system_id = required_str(body, "system_id")?;
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
        row.as_ref()
            .map(Self::tenant_row)
            .ok_or_else(|| ServiceError::NotFound(format!("tenant {id}")))?
    }

    /// Delete a tenant — only when it is not the reserved default and owns no
    /// data. The emptiness check scopes a transaction to the
    /// *target* tenant via `SET LOCAL`, so the RLS policy admits the target's
    /// rows regardless of the caller's own tenant context.
    async fn delete_tenant(&self, id: Uuid) -> Result<(), ServiceError> {
        if id == DEFAULT_TENANT_ID {
            return Err(ServiceError::Conflict(
                "the reserved default tenant cannot be deleted".to_owned(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('ehrbase.tenant_id', $1, true)")
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
            return Err(ServiceError::Conflict(format!(
                "tenant {id} is not empty ({owned} owned object(s)); purge its data first"
            )));
        }
        let deleted = sqlx::query("DELETE FROM tenant WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("tenant {id}")));
        }
        tx.commit().await?;
        self.invalidate_tenant_cache();
        Ok(())
    }

    /// Resolve a claim/header value (a tenant name or uuid string) to its
    /// [`TenantContext`], caching the hit in-process.
    async fn resolve_tenant(&self, key: &str) -> Result<Option<TenantContext>, ServiceError> {
        if let Ok(cache) = self.tenant_cache.read()
            && let Some(ctx) = cache.get(key)
        {
            return Ok(Some(ctx.clone()));
        }
        let row = sqlx::query("SELECT id, system_id FROM tenant WHERE name = $1 OR id::text = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let ctx = TenantContext {
            tenant_id: row.try_get("id")?,
            system_id: row.try_get("system_id")?,
        };
        if let Ok(mut cache) = self.tenant_cache.write() {
            cache.insert(key.to_owned(), ctx.clone());
        }
        Ok(Some(ctx))
    }

    /// Drop the whole resolver cache after any tenant CRUD write (a rename /
    /// `system_id` change / delete can invalidate a cached `name→context` entry).
    fn invalidate_tenant_cache(&self) {
        if let Ok(mut cache) = self.tenant_cache.write() {
            cache.clear();
        }
    }
}

/// Read a required non-empty string field from the JSON body, else `400`.
fn required_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, ServiceError> {
    body.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ServiceError::BadRequest(format!("`{field}` is required and non-empty")))
}

/// Map a unique-name violation to `Conflict` (409); other DB errors pass through.
fn map_insert_error(e: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db) = &e
        && db.is_unique_violation()
    {
        return ServiceError::Conflict("a tenant with that name already exists".to_owned());
    }
    ServiceError::Database(e)
}

impl EhrbaseService {
    pub async fn tenant_list(&self) -> Result<Vec<Value>, SmError> {
        Ok(self.list_tenants().await?)
    }

    pub async fn tenant_create(&self, a_tenant: Value) -> Result<Value, SmError> {
        Ok(self.create_tenant(&a_tenant).await?)
    }

    pub async fn tenant_get(&self, a_tenant_id: Uuid) -> Result<Value, SmError> {
        Ok(self.get_tenant(a_tenant_id).await?)
    }

    pub async fn tenant_update(&self, a_tenant_id: Uuid, a_tenant: Value) -> Result<Value, SmError> {
        Ok(self.update_tenant(a_tenant_id, &a_tenant).await?)
    }

    pub async fn tenant_delete(&self, a_tenant_id: Uuid) -> Result<(), SmError> {
        Ok(self.delete_tenant(a_tenant_id).await?)
    }

    pub async fn tenant_resolve(&self, key: &str) -> Result<Option<TenantContext>, SmError> {
        Ok(self.resolve_tenant(key).await?)
    }
}
