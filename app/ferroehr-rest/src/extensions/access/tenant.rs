// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Tenant-resolution middleware.
//!
//! No openEHR spec governs multi-tenancy — it is our own deployment extension
//! (the SM assumes a single logical repository). Flagged here per the repo rule
//! that spec-silent behaviour is called out rather than presented as conformance.
//!
//! Runs *inside* the authentication layer (so the [`crate::extensions::access::authn::Principal`] and its JWT
//! claims are established), resolves the request's tenant from the configured
//! claim — or an optional dev header override — via the platform's
//! `ferroehr::service::admin::tenant::TenantAdapter::tenant_resolve`, and opens the
//! `ferroehr::service::admin::tenant` task-local scope around the rest of the request. The
//! application's tenant-scoped pool then reads that scope on every acquired
//! connection to set `ferroehr.tenant_id` for RLS.
//!
//! Only installed when `tenancy.enabled` (`crate::router::router`), so
//! single-tenant deployments pay nothing.
//!
//! A request carrying NO tenant key runs **unscoped** → the reserved default
//! tenant. A key that names no registered tenant is refused `403` under the
//! default `tenancy.unknown_tenant = "refuse"`, because the fall-through would
//! hand the caller that same default tenant, and it owns every row written
//! while tenancy was off. `"default_tenant"` restores the fall-through for a
//! deployment that prefers a cross-tenant access to look like an empty result
//! set rather than a `403` confirming another tenant exists; that argument
//! holds only while the default tenant is empty, which boot checks.
//!
//! A resolution ERROR is not an unknown tenant: an unreachable registry
//! answers `503`, never the default tenant.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use ferroehr::config::server::UnknownTenant;
use openehr_its::rest::runtime::ApiError;

use crate::extensions::access::authn::current_principal;
use crate::extensions::access::authz::roles::claim_string;
use crate::state::AppState;

/// Resolve and scope the request's tenant. See the module docs.
pub async fn middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
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

    // Resolve to a context. An UNKNOWN key is refused under the default
    // `unknown_tenant = "refuse"` and falls through to the default tenant only
    // when the deployment asks for it (module docs). A resolution ERROR
    // (registry unreachable ≠ unknown tenant) never falls through either — it
    // answers 503 like any other dependency failure.
    let ctx = match key {
        Some(k) => match state.backend().tenant_resolve(&k).await {
            Ok(Some(ctx)) => Some(ctx),
            Ok(None) if cfg.unknown_tenant == UnknownTenant::Refuse => {
                return crate::overview::error::RestError::from(ApiError::Forbidden(
                    "the request's tenant is not registered".to_owned(),
                ))
                .into_response();
            }
            Ok(None) => None,
            Err(e) => {
                return crate::overview::error::RestError::from(e).into_response();
            }
        },
        None => None,
    };

    match ctx {
        Some(ctx) => {
            let mut resp =
                ferroehr::extensions::tenant_context::scope(ctx.clone(), next.run(req)).await;
            // Republish the resolved tenant onto the response (the task-local
            // scope has already exited) so the outermost ATNA audit layer can
            // stamp the record's tenant — the same pattern the auth layer uses
            // for `Principal`.
            resp.extensions_mut().insert(ctx);
            resp
        }
        None => next.run(req).await,
    }
}
