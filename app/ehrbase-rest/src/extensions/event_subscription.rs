//! HTTP dispatch for the event-subscription admin extension API group over the
//! [`EventSubscriptionAdapter`](ehrbase::service::EventSubscriptionAdapter) seam.
//!
//! **No openEHR spec governs this — our own enterprise feature (E1, eventing).**
//! Event/subscription semantics have no SM or ITS-REST governance, so this
//! surface is ours: exposed under the server's extension namespace and excluded
//! from the ITS-REST drift check (design record: `docs/enterprise/product-roadmap.md`
//! §2.2 and the classification register `docs/design/its-rest/extensions.md`).
//! It is mounted under `/admin/` (a subscription store is an administrative
//! resource), so the coarse RBAC gate fail-safe classes it as `Admin` (requires
//! the admin role when RBAC is on), matching the physical-delete ADMIN group.
//!
//! PORT NOTE (no SM call, no ABAC/audit): the CRUD dispatches to the
//! [`EventSubscriptionAdapter`] extension, not an SM interface. Like the
//! terminology extension it carries no ABAC resource kind (the generic PEP
//! `Skip`s it) and no ATNA audit-table entry (subscriptions are configuration,
//! not PHI access) — the fallbacks apply automatically.
//!
//! The group is config-gated (`AppConfig::events_admin_api`, default
//! `false`): when disabled every route answers `404` without touching the
//! backend.

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

/// The event-subscription extension routes as a native `utoipa-axum` router
/// (group-relative paths; nested under `base_path`), mounted under `/admin`
/// (the coarse RBAC gate classes it `Admin`). Served through [`guarded_dispatch`]
/// → [`dispatch`]. No openEHR spec governs eventing — our own extension.
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    // One `routes!` per PATH (handlers in a single call must share the path;
    // mixing paths panics at router build with "Overlapping method route").
    OpenApiRouter::new()
        .routes(routes!(event_subscription_list, event_subscription_create))
        .routes(routes!(
            event_subscription_get,
            event_subscription_update,
            event_subscription_delete
        ))
}

/// List every event subscription.
#[utoipa::path(
    get, path = "/admin/event_subscription", tag = "event-subscription",
    responses((status = 200, description = "The subscription records.", body = serde_json::Value))
)]
pub(crate) async fn event_subscription_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "event_subscription_list", parts, dispatch).await
}

/// Create a subscription. Body: `{name, kind?, change_type?, template_id?,
/// archetype?, enabled?}`.
#[utoipa::path(
    post, path = "/admin/event_subscription", tag = "event-subscription",
    request_body(content = serde_json::Value, description = "The subscription definition."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn event_subscription_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "event_subscription_create", parts, dispatch).await
}

/// Read one subscription by id. 404 when absent.
#[utoipa::path(
    get, path = "/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    responses(
        (status = 200, description = "The subscription record.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn event_subscription_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "event_subscription_get", parts, dispatch).await
}

/// Replace one subscription's predicates + enabled flag.
#[utoipa::path(
    put, path = "/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    request_body(content = serde_json::Value, description = "The updated subscription definition."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn event_subscription_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "event_subscription_update", parts, dispatch).await
}

/// Delete one subscription.
#[utoipa::path(
    delete, path = "/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn event_subscription_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "event_subscription_delete", parts, dispatch).await
}

pub(crate) fn dispatch(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    // Config gate: the group is opt-in. When disabled every route answers 404
    // (as if unmounted) without consulting the backend.
    if !state.config().events_admin_api {
        return Err(RestError(ApiError::NotFound(
            "event subscription API is disabled".to_owned(),
        )));
    }

    let h = &parts.headers;

    match op {
        "event_subscription_list" => {
            let items = state.backend().event_subscription_list().await?;
            Ok(negotiate::respond(h, StatusCode::OK, &items))
        }
        "event_subscription_create" => {
            let body = negotiate::json_value(h, &parts.body)?;
            let created = state.backend().event_subscription_create(body).await?;
            Ok(negotiate::respond(h, StatusCode::CREATED, &created))
        }
        "event_subscription_get" => {
            let id = subscription_id(&parts)?;
            let item = state.backend().event_subscription_get(id).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &item))
        }
        "event_subscription_update" => {
            let id = subscription_id(&parts)?;
            let body = negotiate::json_value(h, &parts.body)?;
            let updated = state.backend().event_subscription_update(id, body).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &updated))
        }
        "event_subscription_delete" => {
            let id = subscription_id(&parts)?;
            state.backend().event_subscription_delete(id).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted event subscription operation: {other}"
        )))),
    }
}

/// Parse the `{subscription_id}` path parameter as a UUID → `400` when malformed
/// (a missing param is a routing bug → `500`).
fn subscription_id(parts: &RequestParts) -> Result<Uuid, RestError> {
    let raw = parts.path.get("subscription_id").ok_or_else(|| {
        RestError(ApiError::Internal(
            "missing path parameter `subscription_id`".to_owned(),
        ))
    })?;
    raw.parse::<Uuid>().map_err(|_| {
        RestError(ApiError::BadRequest(format!(
            "invalid event subscription id `{raw}`"
        )))
    })
}
