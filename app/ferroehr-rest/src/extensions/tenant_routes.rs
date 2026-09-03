// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! HTTP dispatch for the tenant admin extension API group over the
//! `ferroehr::service::TenantAdapter` seam.
//!
//! **No openEHR spec governs this — our own enterprise feature (multi-
//! tenancy).** The tenancy model has zero SM/ITS-REST governance, so this
//! surface is ours: exposed under the server's extension namespace and excluded
//! from the ITS-REST drift check. It is mounted under `/admin/` (the tenant
//! registry is an administrative resource), so the coarse RBAC gate fail-safe
//! classes it as `Admin` (requires the admin role when RBAC is on), matching
//! the physical-delete ADMIN group.
//!
//! NOTE (no SM call, no ABAC/audit): the CRUD dispatches to the
//! `TenantAdapter` extension, not an SM interface. Like the terminology
//! extension it carries no ABAC resource kind (the generic PEP `Skip`s it) and
//! no ATNA audit-table entry — the fallbacks apply automatically.
//!
//! The group is config-gated (`AppConfig::tenancy.enabled`, default `false`):
//! when disabled every route answers `404` without touching the backend, so a
//! single-tenant deployment never exposes tenant administration.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts, guarded_dispatch};
use crate::overview::error::RestError;

use crate::negotiate;
use crate::state::AppState;

/// The tenant admin extension routes as a native `utoipa-axum` router
/// (group-relative paths; nested under `base_path`), mounted under `/admin`
/// (the coarse RBAC gate classes it `Admin`). Served through [`guarded_dispatch`]
/// → [`dispatch`]. No openEHR spec governs multi-tenancy — our own extension.
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    // One `routes!` per PATH (handlers in a single call must share the path;
    // mixing paths panics at router build with "Overlapping method route").
    OpenApiRouter::new()
        .routes(routes!(tenant_list, tenant_create))
        .routes(routes!(tenant_current))
        .routes(routes!(tenant_get, tenant_update, tenant_delete))
}

/// List every tenant (`GET /admin/tenant`).
///
/// Config-gated: `404` when `tenancy.enabled` is off (the route stays mounted
/// but the backend is never consulted).
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    get, path = "/admin/tenant", tag = "tenancy",
    responses(
        (status = 200, description = "The tenant records.", body = serde_json::Value),
        (status = 404, description = "The tenancy extension is disabled (`tenancy.enabled` off). With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_list", parts, dispatch).await
}

/// Create a tenant (`POST /admin/tenant`). Body: `{name, system_id}`.
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    post, path = "/admin/tenant", tag = "tenancy",
    request_body(content = serde_json::Value, description = "The tenant definition `{name, system_id}` (canonical JSON)."),
    responses(
        (status = 201, description = "Created; the stored tenant record is returned.", body = serde_json::Value),
        (status = 400, description = "`name`/`system_id` is missing or empty, or the body is not valid JSON.", body = serde_json::Value),
        (status = 409, description = "A tenant with that name already exists.", body = serde_json::Value),
        (status = 415, description = "The request `Content-Type` is not `application/json`.", body = serde_json::Value),
        (status = 404, description = "The tenancy extension is disabled (`tenancy.enabled` off). With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_create", parts, dispatch).await
}

/// The tenant THIS request's credential resolves to (`GET /admin/tenant/current`).
///
/// Read-only session context for operators: the tenant-resolution middleware
/// has already resolved the caller (claim, or the dev header override) by the
/// time any handler runs, so this answers from that ambient scope — never a
/// viewer-side computation, never a selector. An unscoped request reports the
/// reserved default tenant (`{"default": true, "tenant": null}`); a scoped one
/// carries its registry record (`{"default": false, "tenant": {…}}`). The
/// static `current` segment cannot collide with a tenant id — ids are UUIDs.
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    get, path = "/admin/tenant/current", tag = "tenancy",
    responses(
        (status = 200, description = "The caller's resolved tenant: `{\"default\": bool, \"tenant\": record|null}` — `default: true` (tenant `null`) when the request runs unscoped on the reserved default tenant.", body = serde_json::Value),
        (status = 404, description = "The tenancy extension is disabled (`tenancy.enabled` off). With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_current(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_current", parts, dispatch).await
}

/// Read one tenant by id (`GET /admin/tenant/{tenant_id}`).
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    get, path = "/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    responses(
        (status = 200, description = "The tenant record.", body = serde_json::Value),
        (status = 400, description = "`tenant_id` is not a valid UUID.", body = serde_json::Value),
        (status = 404, description = "No tenant with that id, or the tenancy extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_get", parts, dispatch).await
}

/// Update one tenant's name/`system_id` (`PUT /admin/tenant/{tenant_id}`).
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    put, path = "/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    request_body(content = serde_json::Value, description = "The updated tenant definition `{name, system_id}` (canonical JSON)."),
    responses(
        (status = 200, description = "Updated; the stored tenant record is returned.", body = serde_json::Value),
        (status = 400, description = "`tenant_id` is not a valid UUID, a field is missing/empty, or the body is not valid JSON.", body = serde_json::Value),
        (status = 409, description = "A tenant with that name already exists.", body = serde_json::Value),
        (status = 415, description = "The request `Content-Type` is not `application/json`.", body = serde_json::Value),
        (status = 404, description = "No tenant with that id, or the tenancy extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_update", parts, dispatch).await
}

/// Delete one tenant — only when empty and not the reserved default
/// (`DELETE /admin/tenant/{tenant_id}`).
///
/// OUR OWN EXTENSION — no openEHR spec governs this: multi-tenancy has no SM
/// or ITS-REST governance at all, so the whole group (paths, payloads, status
/// codes) is our own design.
#[utoipa::path(
    delete, path = "/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 400, description = "`tenant_id` is not a valid UUID.", body = serde_json::Value),
        (status = 409, description = "The tenant is the reserved default, or still owns data (purge it first).", body = serde_json::Value),
        (status = 404, description = "No tenant with that id, or the tenancy extension is disabled. With authentication enabled, an unauthenticated request to a disabled group is answered `401` first (the group gate sits behind authentication).", body = serde_json::Value)
    )
)]
pub(crate) async fn tenant_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "tenant_delete", parts, dispatch).await
}

pub(crate) fn dispatch(state: AppState, op: &'static str, parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

/// The `GET /admin/tenant/current` answer: the tenant the caller's credential
/// resolves to. `default: true` (with `tenant` absent) means the request ran
/// unscoped on the reserved default tenant.
#[derive(Debug, serde::Serialize)]
struct CurrentTenant {
    /// Whether the request ran unscoped on the reserved default tenant.
    default: bool,
    /// The resolved registry record; `None` on the default tenant.
    tenant: Option<ferroehr::extensions::tenancy::TenantRecord>,
}

async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    // Config gate: the group is opt-in. When disabled every route answers 404
    // (as if unmounted) without consulting the backend.
    if !state.config().tenancy.enabled {
        return Err(RestError(ApiError::NotFound(
            "tenant admin API is disabled".to_owned(),
        )));
    }

    let h = &parts.headers;

    match op {
        "tenant_list" => {
            let items = state.backend().tenant_list().await?;
            Ok(negotiate::respond(h, StatusCode::OK, &items))
        }
        "tenant_create" => {
            let body: ferroehr::extensions::tenancy::TenantDefinition =
                negotiate::typed_json(h, &parts.body)?;
            let created = state.backend().tenant_create(body).await?;
            Ok(negotiate::respond(h, StatusCode::CREATED, &created))
        }
        "tenant_current" => {
            // The middleware resolved the caller before any handler ran; the
            // ambient scope is therefore the answer, re-read from the registry
            // so the record is current.
            let answer = match ferroehr::extensions::tenant_context::current() {
                Some(ctx) => CurrentTenant {
                    default: false,
                    tenant: Some(state.backend().tenant_get(ctx.tenant_id).await?),
                },
                None => CurrentTenant {
                    default: true,
                    tenant: None,
                },
            };
            Ok(negotiate::respond(h, StatusCode::OK, &answer))
        }
        "tenant_get" => {
            let id = tenant_id(&parts)?;
            let item = state.backend().tenant_get(id).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &item))
        }
        "tenant_update" => {
            let id = tenant_id(&parts)?;
            let body: ferroehr::extensions::tenancy::TenantDefinition =
                negotiate::typed_json(h, &parts.body)?;
            let updated = state.backend().tenant_update(id, body).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &updated))
        }
        "tenant_delete" => {
            let id = tenant_id(&parts)?;
            state.backend().tenant_delete(id).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted tenant operation: {other}"
        )))),
    }
}

/// Parse the `{tenant_id}` path parameter as a UUID → `400` when malformed
/// (a missing param is a routing bug → `500`).
#[expect(
    clippy::map_err_ignore,
    reason = "`uuid::Error` carries only \"this is not a UUID\", which the 400 body \
              already states"
)]
fn tenant_id(parts: &RequestParts) -> Result<Uuid, RestError> {
    let raw = parts.path.get("tenant_id").ok_or_else(|| {
        RestError(ApiError::Internal(
            "missing path parameter `tenant_id`".to_owned(),
        ))
    })?;
    raw.parse::<Uuid>()
        .map_err(|_| RestError(ApiError::BadRequest(format!("invalid tenant id `{raw}`"))))
}
