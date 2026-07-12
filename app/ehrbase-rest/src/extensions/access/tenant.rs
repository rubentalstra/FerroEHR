//! Tenant-resolution middleware.
//!
//! No openEHR spec governs multi-tenancy — it is our own deployment extension
//! (the SM assumes a single logical repository). Flagged here per the repo rule
//! that spec-silent behaviour is called out rather than presented as conformance.
//!
//! Runs *inside* the authentication layer (so the [`Principal`] and its JWT
//! claims are established), resolves the request's tenant from the configured
//! claim — or an optional dev header override — via the platform's
//! [`TenantAdapter::tenant_resolve`], and opens the
//! [`ehrbase_sm::tenant`] task-local scope around the rest of the request. The
//! application's tenant-scoped pool then reads that scope on every acquired
//! connection to set `ehrbase.tenant_id` for RLS.
//!
//! Only installed when `tenancy.enabled` (`crate::router`), so single-tenant
//! deployments pay nothing. A request that carries no tenant key, or an
//! unknown/unresolvable one, runs **unscoped** → the reserved default tenant:
//! cross-tenant access is an engine-level empty set, never a `403`.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use ehrbase_sm::Platform;

use crate::extensions::access::authn::current_principal;
use crate::extensions::access::authz::roles::claim_string;
use crate::state::AppState;

/// Resolve and scope the request's tenant. See the module docs.
pub async fn middleware<S: Platform>(
    State(state): State<AppState<S>>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = &state.config().tenancy;

    // A dev header override (when configured) wins over the JWT claim; otherwise
    // the tenant key is the configured claim on the authenticated principal.
    let key = cfg
        .header
        .as_deref()
        .and_then(|h| req.headers().get(h))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| current_principal().and_then(|p| claim_string(&p.claims, &cfg.claim)));

    // Resolve to a context. A resolution error or an unknown key leaves the
    // request unscoped (default tenant) — an engine-level empty set, not a 403.
    let ctx = match key {
        Some(k) => state.backend().tenant_resolve(&k).await.ok().flatten(),
        None => None,
    };

    match ctx {
        Some(ctx) => ehrbase_sm::tenant::scope(ctx, next.run(req)).await,
        None => next.run(req).await,
    }
}
