//! Native utoipa-axum routing for the EHR API group. No openEHR spec governs an
//! OAS layout; the operation semantics are the ITS-REST EHR API
//! (docs/specs/openehr/ITS-REST). Each handler forwards to the group dispatcher
//! through `guarded_dispatch`, so wire behaviour is identical to the former
//! `mount()` adapter.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use ehrbase_sm::Platform;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The EHR-group routes as a native `utoipa-axum` router: each `#[utoipa::path]`
/// handler single-sources its route and its `OpenAPI` path. Group-relative paths
/// (nested under the configured `base_path`); every operation is served through
/// [`guarded_dispatch`] → [`crate::api::ehr::dispatch::dispatch`], so the wire
/// behaviour is identical to the former table-driven `mount` adapter.
pub(crate) fn routes<S: Platform>() -> OpenApiRouter<AppState<S>> {
    OpenApiRouter::new()
        .routes(routes!(ehr_get_by_subject, ehr_create))
        .routes(routes!(ehr_get_by_id, ehr_create_with_id))
        .routes(routes!(ehr_status_get_by_version_id))
        .routes(routes!(ehr_status_get_at_time, ehr_status_update))
        .routes(routes!(versioned_ehr_status_get))
        .routes(routes!(versioned_ehr_status_revision_history))
        .routes(routes!(versioned_ehr_status_version_get_at_time))
        .routes(routes!(versioned_ehr_status_version_get_by_id))
        .routes(routes!(composition_create))
        .routes(routes!(
            composition_get,
            composition_update,
            composition_delete
        ))
        .routes(routes!(versioned_composition_get))
        .routes(routes!(versioned_composition_revision_history))
        .routes(routes!(versioned_composition_version_get_at_time))
        .routes(routes!(versioned_composition_version_get_by_id))
        .routes(routes!(
            directory_get_at_time,
            directory_update,
            directory_create,
            directory_delete
        ))
        .routes(routes!(directory_get_by_version_id))
        .routes(routes!(contribution_create))
        .routes(routes!(contribution_get))
        .routes(routes!(ehr_tags_get))
        .routes(routes!(composition_tags_get, composition_tags_update))
        .routes(routes!(composition_tags_delete))
        .routes(routes!(ehr_status_tags_get, ehr_status_tags_update))
        .routes(routes!(ehr_status_tags_delete))
}

// ── Handlers (ITS-REST EHR API semantics) ────────────────────────────────────
// Every handler snapshots the request into `RequestParts` (identical to the
// table-driven adapter) and runs it through the shared guarded dispatch onto the
// EHR-group dispatcher, so the EHR_ACCESS gate, ABAC PEP, and ATNA audit tagging
// apply uniformly and the wire behaviour is unchanged.

// ── EHR ───────────────────────────────────────────────────────────────────────

/// Retrieve an EHR by subject (`GET /ehr`).
#[utoipa::path(
    get, path = "/ehr", tag = "ehr",
    responses(
        (status = 200, description = "The EHR.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_get_by_subject<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_get_by_subject",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Create a new EHR (`POST /ehr`).
#[utoipa::path(
    post, path = "/ehr", tag = "ehr",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn ehr_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_create",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve an EHR by id (`GET /ehr/{ehr_id}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_get_by_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Create an EHR with a client-supplied id (`PUT /ehr/{ehr_id}`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 200, description = "Created/updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_create_with_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_create_with_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── EHR_STATUS ────────────────────────────────────────────────────────────────

/// Retrieve an `EHR_STATUS` at a version id
/// (`GET /ehr/{ehr_id}/ehr_status/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status/{version_uid}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_get_by_version_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_get_by_version_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve the `EHR_STATUS` at a point in time
/// (`GET /ehr/{ehr_id}/ehr_status`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Update the `EHR_STATUS` (`PUT /ehr/{ehr_id}/ehr_status`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/ehr_status", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_status_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_update",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── VERSIONED_EHR_STATUS ──────────────────────────────────────────────────────

/// Retrieve the `VERSIONED_EHR_STATUS` container
/// (`GET /ehr/{ehr_id}/versioned_ehr_status`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The VERSIONED_EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve the `EHR_STATUS` revision history
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/revision_history", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The revision history.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_revision_history<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_revision_history",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve an `EHR_STATUS` version at a point in time
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/version`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/version", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR_STATUS version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_version_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_version_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve an `EHR_STATUS` version by id
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_version_get_by_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_version_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── COMPOSITION ───────────────────────────────────────────────────────────────

/// Create a COMPOSITION (`POST /ehr/{ehr_id}/composition`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/composition", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn composition_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_create",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve a COMPOSITION
/// (`GET /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses(
        (status = 200, description = "The COMPOSITION.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn composition_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Update a COMPOSITION
/// (`PUT /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn composition_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_update",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Delete a COMPOSITION
/// (`DELETE /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn composition_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_delete",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── VERSIONED_COMPOSITION ─────────────────────────────────────────────────────

/// Retrieve the `VERSIONED_COMPOSITION` container
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The VERSIONED_COMPOSITION.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve the COMPOSITION revision history
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history",
    tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The revision history.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_revision_history<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_revision_history",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve a COMPOSITION version at a point in time
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version",
    tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The COMPOSITION version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_version_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_version_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve a COMPOSITION version by id
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
    tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The COMPOSITION version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_version_get_by_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_version_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── DIRECTORY ─────────────────────────────────────────────────────────────────

/// Retrieve the directory (FOLDER) at a point in time
/// (`GET /ehr/{ehr_id}/directory`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/directory", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The DIRECTORY.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn directory_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Update the directory (FOLDER) (`PUT /ehr/{ehr_id}/directory`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/directory", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn directory_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_update",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Create the directory (FOLDER) (`POST /ehr/{ehr_id}/directory`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/directory", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn directory_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_create",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Delete the directory (FOLDER) (`DELETE /ehr/{ehr_id}/directory`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/directory", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn directory_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_delete",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve the directory (FOLDER) by version id
/// (`GET /ehr/{ehr_id}/directory/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/directory/{version_uid}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The DIRECTORY.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn directory_get_by_version_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_get_by_version_id",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── CONTRIBUTION ──────────────────────────────────────────────────────────────

/// Create a CONTRIBUTION (`POST /ehr/{ehr_id}/contribution`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/contribution", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn contribution_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "contribution_create",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve a CONTRIBUTION
/// (`GET /ehr/{ehr_id}/contribution/{contribution_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/contribution/{contribution_uid}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("contribution_uid" = String, Path, description = "The contribution uid.")
    ),
    responses(
        (status = 200, description = "The CONTRIBUTION.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn contribution_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "contribution_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

// ── Item tags ─────────────────────────────────────────────────────────────────

/// Retrieve the EHR-level item tags (`GET /ehr/{ehr_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/tags", tag = "ehr",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve a COMPOSITION's item tags
/// (`GET /ehr/{ehr_id}/composition/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn composition_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Replace a COMPOSITION's item tags
/// (`PUT /ehr/{ehr_id}/composition/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn composition_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_update",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Delete a COMPOSITION item tag by key
/// (`DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid."),
        ("key" = String, Path, description = "The tag key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn composition_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_delete",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Retrieve an `EHR_STATUS`'s item tags
/// (`GET /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid.")
    ),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Replace an `EHR_STATUS`'s item tags
/// (`PUT /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid.")
    ),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_status_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_update",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}

/// Delete an `EHR_STATUS` item tag by key
/// (`DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}", tag = "ehr",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid."),
        ("key" = String, Path, description = "The tag key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn ehr_status_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_delete",
        parts,
        crate::api::ehr::dispatch::dispatch::<S>,
    )
    .await
}
