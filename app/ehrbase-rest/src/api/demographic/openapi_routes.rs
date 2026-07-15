//! Native `utoipa-axum` routing for the **standard Demographic API group**
//! (`x-status: DEVELOPMENT` in the vendored
//! `docs/specs/openehr/ITS-REST/specifications/demographic.openapi.yaml`): the
//! party CRUD (`agent`/`group`/`organisation`/`person`/`role`), the
//! `versioned_party` reads, `contribution` create/get, and the `ITEM_TAG`
//! sub-resources (`demographic_tags_get` + the per-party `*_tags_*`).
//!
//! Our own wire follows the vendored demographic OAS operation ids verbatim.
//! Each `#[utoipa::path]` handler single-sources its route and its `OpenAPI`
//! path, then forwards to the demographic group dispatcher
//! ([`super::dispatch`]) through [`guarded_dispatch`] — so the wire behaviour
//! is identical to the former table-driven `mount()` adapter (same
//! `EHR_ACCESS` gate, ABAC PEP, and ATNA audit tagging).
//!
//! The own-design `PARTY_RELATIONSHIP` extension is *not* here — it lives in
//! [`super::relationship`] (no ITS-REST operation governs it).

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;
use ehrbase_sm::Platform;

/// The standard Demographic API group as a native `utoipa-axum` router.
/// Group-relative paths (nested under the configured `base_path`); every
/// operation runs through [`guarded_dispatch`] with the demographic group
/// [`dispatch`](super::dispatch), so the wire behaviour is identical to the
/// former table-driven `mount` adapter.
pub(crate) fn routes<S: Platform>() -> OpenApiRouter<AppState<S>> {
    OpenApiRouter::new()
        .routes(routes!(agent_create))
        .routes(routes!(agent_get, agent_update, agent_delete))
        .routes(routes!(group_create))
        .routes(routes!(group_get, group_update, group_delete))
        .routes(routes!(organisation_create))
        .routes(routes!(
            organisation_get,
            organisation_update,
            organisation_delete
        ))
        .routes(routes!(person_create))
        .routes(routes!(person_get, person_update, person_delete))
        .routes(routes!(role_create))
        .routes(routes!(role_get, role_update, role_delete))
        .routes(routes!(versioned_party_get))
        .routes(routes!(versioned_party_revision_history))
        .routes(routes!(versioned_party_version_get_at_time))
        .routes(routes!(versioned_party_version_get_by_id))
        .routes(routes!(contribution_create))
        .routes(routes!(contribution_get))
        .routes(routes!(demographic_tags_get))
        .routes(routes!(agent_tags_get, agent_tags_update))
        .routes(routes!(agent_tags_delete))
        .routes(routes!(group_tags_get, group_tags_update))
        .routes(routes!(group_tags_delete))
        .routes(routes!(organisation_tags_get, organisation_tags_update))
        .routes(routes!(organisation_tags_delete))
        .routes(routes!(person_tags_get, person_tags_update))
        .routes(routes!(person_tags_delete))
        .routes(routes!(role_tags_get, role_tags_update))
        .routes(routes!(role_tags_delete))
}

// ── Handlers ──────────────────────────────────────────────────────────────
// Each snapshots the request into `RequestParts` (identical to the generated-
// group `mount` adapter) and runs it through the demographic group dispatcher
// (`super::dispatch`), so the EHR_ACCESS gate, ABAC PEP, and ATNA audit tagging
// apply uniformly.

// ── AGENT ───────────────────────────────────────────────────────────────────

/// Create an `AGENT`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/agent", tag = "AGENT",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn agent_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_create", parts, super::dispatch::<S>).await
}

/// Read an `AGENT` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The AGENT (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn agent_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_get", parts, super::dispatch::<S>).await
}

/// Update an `AGENT` (If-Match required). 200 with the updated resource.
#[utoipa::path(
    put, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn agent_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_update", parts, super::dispatch::<S>).await
}

/// Delete an `AGENT` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn agent_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_delete", parts, super::dispatch::<S>).await
}

// ── GROUP ─────────────────────────────────────────────────────────────────

/// Create a `GROUP`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/group", tag = "GROUP",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn group_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_create", parts, super::dispatch::<S>).await
}

/// Read a `GROUP` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The GROUP (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn group_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_get", parts, super::dispatch::<S>).await
}

/// Update a `GROUP` (If-Match required). 200 with the updated resource.
#[utoipa::path(
    put, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn group_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_update", parts, super::dispatch::<S>).await
}

/// Delete a `GROUP` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn group_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_delete", parts, super::dispatch::<S>).await
}

// ── ORGANISATION ────────────────────────────────────────────────────────────

/// Create an `ORGANISATION`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/organisation", tag = "ORGANISATION",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn organisation_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_create", parts, super::dispatch::<S>).await
}

/// Read an `ORGANISATION` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ORGANISATION (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_get", parts, super::dispatch::<S>).await
}

/// Update an `ORGANISATION` (If-Match required). 200 with the updated resource.
#[utoipa::path(
    put, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn organisation_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_update", parts, super::dispatch::<S>).await
}

/// Delete an `ORGANISATION` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn organisation_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_delete", parts, super::dispatch::<S>).await
}

// ── PERSON ────────────────────────────────────────────────────────────────

/// Create a `PERSON`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/person", tag = "PERSON",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn person_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_create", parts, super::dispatch::<S>).await
}

/// Read a `PERSON` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The PERSON (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn person_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_get", parts, super::dispatch::<S>).await
}

/// Update a `PERSON` (If-Match required). 200 with the updated resource.
#[utoipa::path(
    put, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn person_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_update", parts, super::dispatch::<S>).await
}

/// Delete a `PERSON` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn person_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_delete", parts, super::dispatch::<S>).await
}

// ── ROLE ────────────────────────────────────────────────────────────────────

/// Create a `ROLE`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/role", tag = "ROLE",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn role_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_create", parts, super::dispatch::<S>).await
}

/// Read a `ROLE` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ROLE (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn role_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_get", parts, super::dispatch::<S>).await
}

/// Update a `ROLE` (If-Match required). 200 with the updated resource.
#[utoipa::path(
    put, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn role_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_update", parts, super::dispatch::<S>).await
}

/// Delete a `ROLE` (If-Match required).
#[utoipa::path(
    delete, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn role_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_delete", parts, super::dispatch::<S>).await
}

// ── VERSIONED_PARTY ──────────────────────────────────────────────────────────

/// Read the `VERSIONED_PARTY` container. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}", tag = "VERSIONED_PARTY",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses(
        (status = 200, description = "The VERSIONED_PARTY (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "versioned_party_get", parts, super::dispatch::<S>).await
}

/// The party's `REVISION_HISTORY`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/revision_history", tag = "VERSIONED_PARTY",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses(
        (status = 200, description = "The REVISION_HISTORY (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_revision_history<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_revision_history",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// The party VERSION at a point in time (`?version_at_time=`). 404 when absent.
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/version", tag = "VERSIONED_PARTY",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_version_get_at_time<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_version_get_at_time",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// A specific party VERSION by version uid. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/version/{version_uid}", tag = "VERSIONED_PARTY",
    params(
        ("versioned_object_uid" = String, Path, description = "The versioned-object uid."),
        ("version_uid" = String, Path, description = "The OBJECT_VERSION_ID.")
    ),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_version_get_by_id<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_version_get_by_id",
        parts,
        super::dispatch::<S>,
    )
    .await
}

// ── CONTRIBUTION ─────────────────────────────────────────────────────────────

/// Create a demographic `CONTRIBUTION`. 201 with the created resource.
#[utoipa::path(
    post, path = "/demographic/contribution", tag = "CONTRIBUTION",
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
pub(crate) async fn contribution_create<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "contribution_create", parts, super::dispatch::<S>).await
}

/// Read a demographic `CONTRIBUTION` by uid. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/contribution/{contribution_uid}", tag = "CONTRIBUTION",
    params(("contribution_uid" = String, Path, description = "The CONTRIBUTION uid.")),
    responses(
        (status = 200, description = "The CONTRIBUTION (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn contribution_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "contribution_get", parts, super::dispatch::<S>).await
}

// ── ITEM_TAG sub-resources ───────────────────────────────────────────────────

/// List every `ITEM_TAG` known to the demographic surface. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/tags", tag = "ITEM_TAG",
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn demographic_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "demographic_tags_get", parts, super::dispatch::<S>).await
}

/// Read an `AGENT`'s `ITEM_TAGs`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/agent/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn agent_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_get", parts, super::dispatch::<S>).await
}

/// Upsert an `AGENT`'s `ITEM_TAGs`. 200 with the stored tags.
#[utoipa::path(
    put, path = "/demographic/agent/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn agent_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_update", parts, super::dispatch::<S>).await
}

/// Delete one `ITEM_TAG` from an `AGENT` by key.
#[utoipa::path(
    delete, path = "/demographic/agent/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path, description = "The party uid-based id."),
        ("key" = String, Path, description = "The ITEM_TAG key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn agent_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_delete", parts, super::dispatch::<S>).await
}

/// Read a `GROUP`'s `ITEM_TAGs`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/group/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn group_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_get", parts, super::dispatch::<S>).await
}

/// Upsert a `GROUP`'s `ITEM_TAGs`. 200 with the stored tags.
#[utoipa::path(
    put, path = "/demographic/group/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn group_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_update", parts, super::dispatch::<S>).await
}

/// Delete one `ITEM_TAG` from a `GROUP` by key.
#[utoipa::path(
    delete, path = "/demographic/group/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path, description = "The party uid-based id."),
        ("key" = String, Path, description = "The ITEM_TAG key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn group_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_delete", parts, super::dispatch::<S>).await
}

/// Read an `ORGANISATION`'s `ITEM_TAGs`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/organisation/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_tags_get", parts, super::dispatch::<S>).await
}

/// Upsert an `ORGANISATION`'s `ITEM_TAGs`. 200 with the stored tags.
#[utoipa::path(
    put, path = "/demographic/organisation/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn organisation_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_tags_update",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// Delete one `ITEM_TAG` from an `ORGANISATION` by key.
#[utoipa::path(
    delete, path = "/demographic/organisation/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path, description = "The party uid-based id."),
        ("key" = String, Path, description = "The ITEM_TAG key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn organisation_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_tags_delete",
        parts,
        super::dispatch::<S>,
    )
    .await
}

/// Read a `PERSON`'s `ITEM_TAGs`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/person/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn person_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_tags_get", parts, super::dispatch::<S>).await
}

/// Upsert a `PERSON`'s `ITEM_TAGs`. 200 with the stored tags.
#[utoipa::path(
    put, path = "/demographic/person/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn person_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_tags_update", parts, super::dispatch::<S>).await
}

/// Delete one `ITEM_TAG` from a `PERSON` by key.
#[utoipa::path(
    delete, path = "/demographic/person/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path, description = "The party uid-based id."),
        ("key" = String, Path, description = "The ITEM_TAG key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn person_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_tags_delete", parts, super::dispatch::<S>).await
}

/// Read a `ROLE`'s `ITEM_TAGs`. 404 when absent.
#[utoipa::path(
    get, path = "/demographic/role/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses(
        (status = 200, description = "The ITEM_TAGs.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
pub(crate) async fn role_tags_get<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_get", parts, super::dispatch::<S>).await
}

/// Upsert a `ROLE`'s `ITEM_TAGs`. 200 with the stored tags.
#[utoipa::path(
    put, path = "/demographic/role/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(("uid_based_id" = String, Path, description = "The party uid-based id.")),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
pub(crate) async fn role_tags_update<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_update", parts, super::dispatch::<S>).await
}

/// Delete one `ITEM_TAG` from a `ROLE` by key.
#[utoipa::path(
    delete, path = "/demographic/role/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path, description = "The party uid-based id."),
        ("key" = String, Path, description = "The ITEM_TAG key.")
    ),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn role_tags_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_delete", parts, super::dispatch::<S>).await
}
