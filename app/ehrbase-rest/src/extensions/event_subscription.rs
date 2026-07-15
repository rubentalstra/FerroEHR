//! HTTP dispatch for the event-subscription admin extension API group over the
//! [`EventSubscriptionAdapter`](ehrbase_sm::EventSubscriptionAdapter) seam.
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

use axum::response::{IntoResponse, Response};
use http::StatusCode;
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts};
use crate::overview::error::RestError;
use ehrbase_sm::Platform;

use crate::negotiate;
use crate::state::AppState;

/// The event-subscription extension routes — our own extension (no
/// ITS-REST contract), mounted alongside the generated `ROUTES`. Group-relative
/// paths (nested under the configured `base_path`).
pub(crate) const EVENT_SUBSCRIPTION_ROUTES: &[(&str, &str, &str)] = &[
    // List every subscription.
    (
        "GET",
        "/admin/event_subscription",
        "event_subscription_list",
    ),
    // Create a subscription (body: {name, kind?, change_type?, template_id?,
    // archetype?, enabled?}); 201 with the stored record.
    (
        "POST",
        "/admin/event_subscription",
        "event_subscription_create",
    ),
    // Read one subscription by id.
    (
        "GET",
        "/admin/event_subscription/{subscription_id}",
        "event_subscription_get",
    ),
    // Replace one subscription's predicates + enabled.
    (
        "PUT",
        "/admin/event_subscription/{subscription_id}",
        "event_subscription_update",
    ),
    // Delete one subscription.
    (
        "DELETE",
        "/admin/event_subscription/{subscription_id}",
        "event_subscription_delete",
    ),
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
