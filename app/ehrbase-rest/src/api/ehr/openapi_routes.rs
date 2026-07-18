//! Native utoipa-axum routing for the EHR API group. No openEHR spec governs an
//! OAS layout; the operation semantics are the ITS-REST EHR API
//! (docs/specs/openehr/ITS-REST). Each handler forwards to the group dispatcher
//! through `guarded_dispatch`, so wire behaviour is identical to the former
//! `mount()` adapter.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The EHR-group routes as a native `utoipa-axum` router: each `#[utoipa::path]`
/// handler single-sources its route and its `OpenAPI` path. Group-relative paths
/// (nested under the configured `base_path`); every operation is served through
/// [`guarded_dispatch`] → [`crate::api::ehr::dispatch::dispatch`], so the wire
/// behaviour is identical to the former table-driven `mount` adapter.
pub(crate) fn routes() -> OpenApiRouter<AppState> {
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
        // POST + GET share `/ehr/{ehr_id}/contribution`, so they are one route.
        .routes(routes!(contribution_create, contribution_list))
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
    get, path = "/ehr", tag = "EHR",
    responses(
        (status = 200, description = "The EHR.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_get_by_subject(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_get_by_subject",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Create a new EHR (`POST /ehr`).
#[utoipa::path(
    post, path = "/ehr", tag = "EHR",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn ehr_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_create",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve an EHR by id (`GET /ehr/{ehr_id}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}", tag = "EHR",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_get_by_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Create an EHR with a client-supplied id (`PUT /ehr/{ehr_id}`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}", tag = "EHR",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 200, description = "Created/updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_create_with_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_create_with_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── EHR_STATUS ────────────────────────────────────────────────────────────────

/// Retrieve an `EHR_STATUS` at a version id
/// (`GET /ehr/{ehr_id}/ehr_status/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status/{version_uid}", tag = "EHR_STATUS",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_get_by_version_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_get_by_version_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve the `EHR_STATUS` at a point in time
/// (`GET /ehr/{ehr_id}/ehr_status`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status", tag = "EHR_STATUS",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Update the `EHR_STATUS` (`PUT /ehr/{ehr_id}/ehr_status`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/ehr_status", tag = "EHR_STATUS",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_status_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_update",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── VERSIONED_EHR_STATUS ──────────────────────────────────────────────────────

/// Retrieve the `VERSIONED_EHR_STATUS` container
/// (`GET /ehr/{ehr_id}/versioned_ehr_status`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status", tag = "EHR_STATUS",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The VERSIONED_EHR_STATUS.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve the `EHR_STATUS` revision history
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/revision_history", tag = "EHR_STATUS",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The revision history.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_revision_history(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_revision_history",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve an `EHR_STATUS` version at a point in time
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/version`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/version", tag = "EHR_STATUS",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The EHR_STATUS version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_version_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_version_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve an `EHR_STATUS` version by id
/// (`GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}", tag = "EHR_STATUS",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid.")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_ehr_status_version_get_by_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_ehr_status_version_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── COMPOSITION ───────────────────────────────────────────────────────────────

/// Create a COMPOSITION (`POST /ehr/{ehr_id}/composition`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/composition", tag = "COMPOSITION",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    request_body(

        // COMPOSITION content negotiates canonical JSON/XML + the two Simplified
        // Formats (Resources.md §Simplified Formats; simplified_formats/master05).
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A COMPOSITION in canonical JSON/XML or a Simplified Format \
                       (the `openehr-template-id` header is required for a simplified body)."
    ),
    responses((
        status = 201, description = "Created.",
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
    ))
)]
pub(crate) async fn composition_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_create",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve a COMPOSITION
/// (`GET /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses(
        (
            status = 200, description = "The COMPOSITION.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn composition_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Update a COMPOSITION
/// (`PUT /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    request_body(

        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A COMPOSITION in canonical JSON/XML or a Simplified Format \
                       (the `openehr-template-id` header is required for a simplified body)."
    ),
    responses((
        status = 200, description = "Updated.",
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
    ))
)]
pub(crate) async fn composition_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_update",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Delete a COMPOSITION
/// (`DELETE /ehr/{ehr_id}/composition/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn composition_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_delete",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── VERSIONED_COMPOSITION ─────────────────────────────────────────────────────

/// Retrieve the `VERSIONED_COMPOSITION` container
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The VERSIONED_COMPOSITION.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve the COMPOSITION revision history
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history",
    tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The revision history.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_revision_history(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_revision_history",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve a COMPOSITION version at a point in time
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version",
    tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("versioned_object_uid" = String, Path, description = "The versioned object uid.")
    ),
    responses(
        (status = 200, description = "The COMPOSITION version.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_composition_version_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_version_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve a COMPOSITION version by id
/// (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}`).
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
    tag = "COMPOSITION",
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
pub(crate) async fn versioned_composition_version_get_by_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_composition_version_get_by_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── DIRECTORY ─────────────────────────────────────────────────────────────────

/// Retrieve the directory (FOLDER) at a point in time
/// (`GET /ehr/{ehr_id}/directory`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/directory", tag = "DIRECTORY",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; absent means the latest \
                        version."),
        ("path" = Option<String>, Query,
         description = "Slash-separated FOLDER names addressing a sub-folder; \
                        only that subtree is returned.")
    ),
    responses(
        (status = 200, description = "The directory FOLDER (or the addressed \
                                      sub-folder).", body = serde_json::Value),
        (status = 204, description = "The directory was deleted at the \
                                      specified time."),
        (status = 400, description = "Malformed `version_at_time`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown EHR, no version at that time, \
                                      or the path does not resolve.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn directory_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_get_at_time",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Update the directory (FOLDER) (`PUT /ehr/{ehr_id}/directory`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/directory", tag = "DIRECTORY",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("If-Match" = String, Header,
         description = "The latest directory version uid, double-quoted \
                        (weak `W/` form also accepted). Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new directory FOLDER."),
    responses(
        (status = 200, description = "Updated; body per `Prefer` \
                                      (representation or identifier).",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag` carries the new version uid."),
        (status = 400, description = "Invalid FOLDER or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown EHR or no directory.",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current \
                                      latest version uid.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn directory_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_update",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Create the directory (FOLDER) (`POST /ehr/{ehr_id}/directory`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/directory", tag = "DIRECTORY",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`.")
    ),
    request_body(content = serde_json::Value,
                 description = "The directory FOLDER."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new \
                                      version uid (weak form), `Location` \
                                      the version URL. Body per `Prefer`.",
         body = serde_json::Value),
        (status = 400, description = "Invalid FOLDER.",
         body = serde_json::Value),
        (status = 404, description = "Unknown EHR.", body = serde_json::Value),
        (status = 409, description = "A directory already exists for this \
                                      EHR.", body = serde_json::Value)
    )
)]
pub(crate) async fn directory_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_create",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Delete the directory (FOLDER) (`DELETE /ehr/{ehr_id}/directory`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/directory", tag = "DIRECTORY",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("If-Match" = String, Header,
         description = "The latest directory version uid, double-quoted \
                        (weak `W/` form also accepted). Required.")
    ),
    responses(
        (status = 204, description = "Logically deleted (a new deleted \
                                      version is committed)."),
        (status = 400, description = "Missing or malformed `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown EHR or no directory.",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current \
                                      latest version uid.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn directory_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_delete",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve the directory (FOLDER) by version id
/// (`GET /ehr/{ehr_id}/directory/{version_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/directory/{version_uid}", tag = "DIRECTORY",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("version_uid" = String, Path, description = "The version uid."),
        ("path" = Option<String>, Query,
         description = "Slash-separated FOLDER names addressing a sub-folder; \
                        only that subtree is returned.")
    ),
    responses(
        (status = 200, description = "The DIRECTORY.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn directory_get_by_version_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "directory_get_by_version_id",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── CONTRIBUTION ──────────────────────────────────────────────────────────────

/// Create a CONTRIBUTION (`POST /ehr/{ehr_id}/contribution`).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/contribution", tag = "CONTRIBUTION",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    request_body(

        // The envelope is always canonical JSON; a Simplified media type selects
        // the inner `versions[i].data` COMPOSITION form (contribution_create.yaml
        // §Simplified Formats). No canonical-XML CONTRIBUTION wire shape exists.
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A CONTRIBUTION (canonical JSON envelope; inner versions[].data \
                       may be a Simplified Format with the `openehr-template-id` header)."
    ),
    responses((
        status = 201, description = "Created.",
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
    ))
)]
pub(crate) async fn contribution_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "contribution_create",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// List an EHR's CONTRIBUTIONs, newest-first, paged
/// (`GET /ehr/{ehr_id}/contribution`).
///
/// OUR OWN EXTENSION — no openEHR spec governs it (the ITS-REST contract defines
/// only the by-uid CONTRIBUTION GET). Returns
/// `{ "rows": [ { uid, time_committed, committer, change_type } ], "total" }`;
/// `offset` defaults to 0, `fetch` to 20 (capped at 100).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/contribution", tag = "CONTRIBUTION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("offset" = Option<i64>, Query, description = "Row offset (default 0)."),
        ("fetch" = Option<i64>, Query, description = "Max rows (default 20, capped at 100).")
    ),
    responses(
        (status = 200, description = "The EHR's CONTRIBUTIONs (newest first).", body = serde_json::Value),
        (status = 404, description = "Unknown EHR.", body = serde_json::Value)
    )
)]
pub(crate) async fn contribution_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "contribution_list",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve a CONTRIBUTION
/// (`GET /ehr/{ehr_id}/contribution/{contribution_uid}`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/contribution/{contribution_uid}", tag = "CONTRIBUTION",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("contribution_uid" = String, Path, description = "The contribution uid.")
    ),
    responses(
        (
            status = 200, description = "The CONTRIBUTION.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn contribution_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "contribution_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

// ── Item tags ─────────────────────────────────────────────────────────────────

/// Retrieve the EHR-level item tags (`GET /ehr/{ehr_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/tags", tag = "ITEM_TAG",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve a COMPOSITION's item tags
/// (`GET /ehr/{ehr_id}/composition/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn composition_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Replace a COMPOSITION's item tags
/// (`PUT /ehr/{ehr_id}/composition/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid.")
    ),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn composition_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_update",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Delete a COMPOSITION item tag by key
/// (`DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The composition uid."),
        ("key" = String, Path, description = "The tag key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn composition_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "composition_tags_delete",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Retrieve an `EHR_STATUS`'s item tags
/// (`GET /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid.")
    ),
    responses(
        (status = 200, description = "The item tags.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn ehr_status_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_get",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Replace an `EHR_STATUS`'s item tags
/// (`PUT /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid.")
    ),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn ehr_status_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_update",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}

/// Delete an `EHR_STATUS` item tag by key
/// (`DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path, description = "The EHR id."),
        ("uid_based_id" = String, Path, description = "The EHR_STATUS uid."),
        ("key" = String, Path, description = "The tag key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn ehr_status_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "ehr_status_tags_delete",
        parts,
        crate::api::ehr::dispatch::dispatch,
    )
    .await
}
