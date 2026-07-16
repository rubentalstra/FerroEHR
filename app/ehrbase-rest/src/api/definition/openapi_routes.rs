//! Native `utoipa-axum` routing for the Definition API group (ADL 1.4 / ADL 2
//! templates + stored queries).
//!
//! Operation semantics are the ITS-REST Definition API
//! (`docs/specs/openehr/ITS-REST`); no openEHR spec governs the OAS layout. Each
//! handler forwards to the group dispatcher through [`guarded_dispatch`], so the
//! wire behaviour is identical to the former table-driven `mount` adapter.
//!
//! PORT NOTE (operation ids): a few generated operation ids carry `.` (e.g.
//! `definition_template_adl1.4_list`, `definition_query_store.yaml`) — invalid
//! Rust identifiers, so the handler fn names sanitise `.` to `_` while the op
//! string passed to the dispatcher is the verbatim generated id it matches on.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The Definition API group as a native `utoipa-axum` router (group-relative
/// paths; nested under the configured base path).
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(
            definition_template_adl1_4_list,
            definition_template_adl1_4_upload
        ))
        .routes(routes!(definition_template_adl1_4_get))
        .routes(routes!(definition_template_adl1_4_example_get))
        .routes(routes!(
            definition_template_adl2_list,
            definition_template_adl2_upload
        ))
        .routes(routes!(definition_template_adl2_get))
        .routes(routes!(definition_template_adl2_example_get))
        .routes(routes!(definition_template_adl2_version_get))
        .routes(routes!(definition_query_list, definition_query_store_yaml))
        .routes(routes!(
            definition_query_version_get,
            definition_query_version_store_yaml
        ))
}

/// List every stored ADL 1.4 operational template.
#[utoipa::path(
    get, path = "/definition/template/adl1.4", tag = "ADL1.4",
    responses((status = 200, description = "The template ids.", body = serde_json::Value))
)]
pub(crate) async fn definition_template_adl1_4_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl1.4_list",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Upload an ADL 1.4 operational template (OPT XML).
#[utoipa::path(
    post, path = "/definition/template/adl1.4", tag = "ADL1.4",
    responses((status = 201, description = "Template stored.", body = serde_json::Value))
)]
pub(crate) async fn definition_template_adl1_4_upload(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl1.4_upload",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve one ADL 1.4 template by id.
#[utoipa::path(
    get, path = "/definition/template/adl1.4/{template_id}", tag = "ADL1.4",
    params(("template_id" = String, Path, description = "The template id.")),
    responses(
        (status = 200, description = "The template.", body = serde_json::Value),
        (status = 404, description = "Unknown template.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_template_adl1_4_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl1.4_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// An example COMPOSITION for an ADL 1.4 template.
#[utoipa::path(
    get, path = "/definition/template/adl1.4/{template_id}/example", tag = "ADL1.4",
    params(("template_id" = String, Path, description = "The template id.")),
    responses(
        (status = 200, description = "An example COMPOSITION.", body = serde_json::Value),
        (status = 404, description = "Unknown template.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_template_adl1_4_example_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl1.4_example_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// List every stored ADL 2 template.
#[utoipa::path(
    get, path = "/definition/template/adl2", tag = "ADL2",
    responses((status = 200, description = "The template ids.", body = serde_json::Value))
)]
pub(crate) async fn definition_template_adl2_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl2_list",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Upload an ADL 2 template.
#[utoipa::path(
    post, path = "/definition/template/adl2", tag = "ADL2",
    responses((status = 201, description = "Template stored.", body = serde_json::Value))
)]
pub(crate) async fn definition_template_adl2_upload(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl2_upload",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve one ADL 2 template by id.
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}", tag = "ADL2",
    params(("template_id" = String, Path, description = "The template id.")),
    responses(
        (status = 200, description = "The template.", body = serde_json::Value),
        (status = 404, description = "Unknown template.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_template_adl2_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl2_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// An example COMPOSITION for an ADL 2 template.
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}/example", tag = "ADL2",
    params(("template_id" = String, Path, description = "The template id.")),
    responses(
        (status = 200, description = "An example COMPOSITION.", body = serde_json::Value),
        (status = 404, description = "Unknown template.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_template_adl2_example_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl2_example_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve one ADL 2 template at a specific version.
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}/{version}", tag = "ADL2",
    params(
        ("template_id" = String, Path, description = "The template id."),
        ("version" = String, Path, description = "The template version.")
    ),
    responses(
        (status = 200, description = "The template version.", body = serde_json::Value),
        (status = 404, description = "Unknown template version.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_template_adl2_version_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_template_adl2_version_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// List the stored versions of a named stored query.
#[utoipa::path(
    get, path = "/definition/query/{qualified_query_name}", tag = "Query",
    params(("qualified_query_name" = String, Path, description = "The qualified stored-query name.")),
    responses(
        (status = 200, description = "The stored query versions.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_query_list(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_query_list",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Store a named AQL query (auto-versioned).
#[utoipa::path(
    put, path = "/definition/query/{qualified_query_name}", tag = "Query",
    params(("qualified_query_name" = String, Path, description = "The qualified stored-query name.")),
    responses((status = 200, description = "Stored.", body = serde_json::Value))
)]
pub(crate) async fn definition_query_store_yaml(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_query_store.yaml",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a named stored query at a specific version.
#[utoipa::path(
    get, path = "/definition/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path, description = "The qualified stored-query name."),
        ("version" = String, Path, description = "The stored-query version.")
    ),
    responses(
        (status = 200, description = "The stored query.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query version.", body = serde_json::Value)
    )
)]
pub(crate) async fn definition_query_version_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_query_version_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Store a named AQL query at a specific version.
#[utoipa::path(
    put, path = "/definition/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path, description = "The qualified stored-query name."),
        ("version" = String, Path, description = "The stored-query version.")
    ),
    responses((status = 200, description = "Stored.", body = serde_json::Value))
)]
pub(crate) async fn definition_query_version_store_yaml(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_query_version_store.yaml",
        parts,
        super::dispatch::dispatch,
    )
    .await
}
