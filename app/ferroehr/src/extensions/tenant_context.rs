// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Multi-tenancy request context.
//!
//! **No openEHR spec governs this — our own design/extension.** A tenant is one
//! logical openEHR system with its own `system_id`. This module carries the
//! *resolved* tenant of the in-flight request through a tokio task-local,
//! mirroring exactly how `ferroehr-rest` carries the authenticated principal:
//! the tenant-resolution middleware opens a [`scope`] around the request, and
//! two consumers read it ambiently — never through a trait signature:
//!
//!   * the application's connection pool issues `SET ferroehr.tenant_id` from it
//!     on every acquired connection (so `PostgreSQL` RLS scopes reads AND
//!     writes), and
//!   * the service reads the tenant's `system_id` for per-tenant version
//!     identity / audits / `EHR.system_id`.
//!
//! Tenancy is OFF by default: the middleware is never installed, the task-local
//! is never set, [`current`] returns `None`, and behaviour is byte-identical to
//! pre-tenancy.

use std::future::Future;

use uuid::Uuid;

/// The resolved tenant of the current request: its id (the RLS scope key) and
/// its own logical-system id.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// The tenant's uuid — the value stamped into `ferroehr.tenant_id` and matched
    /// by the RLS `tenant_isolation` policy.
    pub tenant_id: Uuid,
    /// The tenant's own logical openEHR `system_id`.
    pub system_id: String,
}

tokio::task_local! {
    static CURRENT_TENANT: TenantContext;
}

/// Run `f` with `ctx` as the current request's tenant. Downstream `.await`s —
/// service calls, pool acquisitions — observe it via [`current`].
pub async fn scope<F>(ctx: TenantContext, f: F) -> F::Output
where
    F: Future,
{
    CURRENT_TENANT.scope(ctx, f).await
}

/// The current request's tenant, or `None` when tenancy is off / the task-local
/// is not in scope (background workers, boot-time paths, single-tenant mode).
#[must_use]
pub fn current() -> Option<TenantContext> {
    CURRENT_TENANT.try_with(Clone::clone).ok()
}
