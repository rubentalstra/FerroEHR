//! Native `utoipa-axum` routing for the Query API group (ad-hoc + stored AQL).
//!
//! Operation semantics are the ITS-REST Query API (`docs/specs/openehr/ITS-REST`;
//! AQL 1.1); no openEHR spec governs the OAS layout. Each handler forwards to the
//! group dispatcher through [`guarded_dispatch`], so the wire behaviour is
//! identical to the former table-driven `mount` adapter.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The Query API group as a native `utoipa-axum` router (group-relative paths).
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(
            query_execute_adhoc_query,
            query_execute_adhoc_query_body
        ))
        .routes(routes!(
            query_execute_stored_query,
            query_execute_stored_query_body
        ))
        .routes(routes!(
            query_execute_stored_query_version,
            query_execute_stored_query_version_body
        ))
}

/// Execute an ad-hoc AQL query supplied in the `q` query parameter.
#[utoipa::path(
    get, path = "/query/aql", tag = "Query",
    responses((status = 200, description = "The RESULT_SET.", body = serde_json::Value))
)]
pub(crate) async fn query_execute_adhoc_query(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_adhoc_query",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Execute an ad-hoc AQL query supplied in the request body.
#[utoipa::path(
    post, path = "/query/aql", tag = "Query",
    responses((status = 200, description = "The RESULT_SET.", body = serde_json::Value))
)]
pub(crate) async fn query_execute_adhoc_query_body(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_adhoc_query_body",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Execute a named stored query (parameters in the query string).
#[utoipa::path(
    get, path = "/query/{qualified_query_name}", tag = "Query",
    params(("qualified_query_name" = String, Path, description = "The qualified stored-query name.")),
    responses(
        (status = 200, description = "The RESULT_SET.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query.", body = serde_json::Value)
    )
)]
pub(crate) async fn query_execute_stored_query(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_stored_query",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Execute a named stored query (parameters in the request body).
#[utoipa::path(
    post, path = "/query/{qualified_query_name}", tag = "Query",
    params(("qualified_query_name" = String, Path, description = "The qualified stored-query name.")),
    responses(
        (status = 200, description = "The RESULT_SET.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query.", body = serde_json::Value)
    )
)]
pub(crate) async fn query_execute_stored_query_body(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_stored_query_body",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Execute a named stored query at a specific version (query-string params).
#[utoipa::path(
    get, path = "/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path, description = "The qualified stored-query name."),
        ("version" = String, Path, description = "The stored-query version.")
    ),
    responses(
        (status = 200, description = "The RESULT_SET.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query version.", body = serde_json::Value)
    )
)]
pub(crate) async fn query_execute_stored_query_version(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_stored_query_version",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Execute a named stored query at a specific version (body params).
#[utoipa::path(
    post, path = "/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path, description = "The qualified stored-query name."),
        ("version" = String, Path, description = "The stored-query version.")
    ),
    responses(
        (status = 200, description = "The RESULT_SET.", body = serde_json::Value),
        (status = 404, description = "Unknown stored query version.", body = serde_json::Value)
    )
)]
pub(crate) async fn query_execute_stored_query_version_body(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "query_execute_stored_query_version_body",
        parts,
        super::dispatch::dispatch,
    )
    .await
}
