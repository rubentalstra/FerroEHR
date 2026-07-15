//! HTTP dispatch for the tenant admin extension API group over the
//! [`TenantAdapter`](ehrbase_sm::TenantAdapter) seam.
//!
//! **No openEHR spec governs this — our own enterprise feature (E2, multi-
//! tenancy).** The tenancy model has zero SM/ITS-REST governance, so this
//! surface is ours: exposed under the server's extension namespace and excluded
//! from the ITS-REST drift check (design record: `docs/enterprise/product-roadmap.md`
//! §2.3 and the classification register `docs/design/its-rest/extensions.md`).
//! It is mounted under `/admin/` (the tenant registry is an administrative
//! resource), so the coarse RBAC gate fail-safe classes it as `Admin` (requires
//! the admin role when RBAC is on), matching the physical-delete ADMIN group.
//!
//! PORT NOTE (no SM call, no ABAC/audit): the CRUD dispatches to the
//! [`TenantAdapter`] extension, not an SM interface. Like the terminology
//! extension it carries no ABAC resource kind (the generic PEP `Skip`s it) and
//! no ATNA audit-table entry — the fallbacks apply automatically.
//!
//! The group is config-gated (`AppConfig::tenancy.enabled`, default `false`):
//! when disabled every route answers `404` without touching the backend, so a
//! single-tenant deployment never exposes tenant administration.

use axum::response::{IntoResponse, Response};
use http::StatusCode;
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts};
use crate::overview::error::RestError;
use ehrbase_sm::Platform;

use crate::negotiate;
use crate::state::AppState;

/// The tenant admin extension routes — our own extension (no ITS-REST
/// contract), mounted alongside the generated `ROUTES`. Group-relative paths
/// (nested under the configured `base_path`).
pub(crate) const TENANT_ROUTES: &[(&str, &str, &str)] = &[
    // List every tenant.
    ("GET", "/admin/tenant", "tenant_list"),
    // Create a tenant (body: {name, system_id}); 201 with the stored record.
    ("POST", "/admin/tenant", "tenant_create"),
    // Read one tenant by id.
    ("GET", "/admin/tenant/{tenant_id}", "tenant_get"),
    // Update one tenant's name/system_id.
    ("PUT", "/admin/tenant/{tenant_id}", "tenant_update"),
    // Delete one tenant (only when empty and not the reserved default).
    ("DELETE", "/admin/tenant/{tenant_id}", "tenant_delete"),
];

pub(crate) fn dispatch<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

async fn run<S: Platform>(
    state: AppState<S>,
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
            let body = negotiate::json_value(h, &parts.body)?;
            let created = state.backend().tenant_create(body).await?;
            Ok(negotiate::respond(h, StatusCode::CREATED, &created))
        }
        "tenant_get" => {
            let id = tenant_id(&parts)?;
            let item = state.backend().tenant_get(id).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &item))
        }
        "tenant_update" => {
            let id = tenant_id(&parts)?;
            let body = negotiate::json_value(h, &parts.body)?;
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
fn tenant_id(parts: &RequestParts) -> Result<Uuid, RestError> {
    let raw = parts.path.get("tenant_id").ok_or_else(|| {
        RestError(ApiError::Internal(
            "missing path parameter `tenant_id`".to_owned(),
        ))
    })?;
    raw.parse::<Uuid>()
        .map_err(|_| RestError(ApiError::BadRequest(format!("invalid tenant id `{raw}`"))))
}
