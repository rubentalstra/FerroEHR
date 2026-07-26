//! Native `utoipa-axum` routing for the Query API group (ad-hoc + stored AQL).
//!
//! Operation semantics are the ITS-REST Query API (`docs/specs/openehr/ITS-REST`;
//! AQL 1.1); no openEHR spec governs the OAS layout. Each handler forwards to the
//! group dispatcher through [`guarded_dispatch`], so the wire behaviour is
//! identical to the former table-driven `mount` adapter.
//!
//! ## Prose-vs-OAS reconciliations (documented real wire)
//!
//! - **`ehr_id` scope** (`docs/query/Request.md` §About the `ehr_id`
//!   parameter): every operation — `GET` and `POST` alike — accepts the EHR
//!   scope as the `ehr_id` query parameter OR the `openehr-ehr-id` request
//!   header. Supplying both is only accepted when they name the same EHR; a
//!   conflict is a `400` (the released text is silent on precedence — register
//!   `AMB-59`). A well-formed-but-absent `ehr_id` is a `404`, a malformed UUID
//!   a `400`.
//! - **JSON-only response** (`200_Query.yaml` declares `application/json`; the
//!   `RESULT_SET` has no canonical-XML shape): an exclusively-XML `Accept`
//!   negotiates to `406` on every operation — documented below as our real wire
//!   though the OAS does not enumerate `406`.
//! - **`ETag`** (`200_Query.yaml` + `headers/ETag_RESULT_SET.yaml`): the `200`
//!   carries a weak `W/"…"` `ETag` — a deterministic content digest of the
//!   assembled `RESULT_SET` (the schema carries no `id`), set only on success.
//! - **`408`** (`408_Query.yaml`): a query that overruns the configured
//!   execution budget (`EHRBASE__QUERY__TIMEOUT_MS`) is a `408 Request Timeout`.
//! - **POST body** (`AdhocQueryExecute` / `Query` schemas): the POST forms carry
//!   `offset`/`fetch`/`query_parameters` (and `q` for ad-hoc) in the JSON body;
//!   the `ehr_id` scope still comes from the query string or header.

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

/// Execute an ad-hoc AQL query from the query string (`GET /query/aql`).
#[utoipa::path(
    get, path = "/query/aql", tag = "Query",
    params(
        ("q" = String, Query,
         description = "The AQL query text to execute. Required."),
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) to scope the query to; may instead be \
                        supplied as the `openehr-ehr-id` header. Supplying both \
                        is accepted only when they name the same EHR; a \
                        conflict is a 400."),
        ("offset" = Option<i64>, Query,
         description = "Row offset into the result set (`0`-based, default `0`)."),
        ("fetch" = Option<i64>, Query,
         description = "Maximum rows to fetch (default is implementation-defined)."),
        ("query_parameters" = Option<String>, Query,
         description = "AQL `$name` binds as repeated `query_parameters` entries.")
    ),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "`q` is missing, the AQL fails to \
                                      parse/type-check, or `ehr_id` is a \
                                      malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "A well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
    )
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

/// Execute an ad-hoc AQL query from the request body (`POST /query/aql`).
#[utoipa::path(
    post, path = "/query/aql", tag = "Query",
    params(
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) scope; may instead be supplied as the \
                        `openehr-ehr-id` header (both are accepted only when \
                        they name the same EHR; a conflict is a 400). The body \
                        carries `q`/`offset`/`fetch`/`query_parameters`.")
    ),
    request_body(content((serde_json::Value = "application/json")),
                 description = "An `AdhocQueryExecute`: `q` (required) plus \
                                optional `offset`, `fetch`, and \
                                `query_parameters`."),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "`q` is missing, the AQL fails to \
                                      parse/type-check, the JSON body is invalid, \
                                      or `ehr_id` is a malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "A well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
    )
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

/// Execute a named stored query, latest version, from the query string
/// (`GET /query/{qualified_query_name}`).
#[utoipa::path(
    get, path = "/query/{qualified_query_name}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`); executed at its latest \
                        version."),
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) scope; may instead be supplied as the \
                        `openehr-ehr-id` header. Supplying both is accepted \
                        only when they name the same EHR; a conflict is a \
                        400."),
        ("offset" = Option<i64>, Query,
         description = "Row offset into the result set (`0`-based, default `0`)."),
        ("fetch" = Option<i64>, Query,
         description = "Maximum rows to fetch (default is implementation-defined)."),
        ("query_parameters" = Option<String>, Query,
         description = "AQL `$name` binds as repeated `query_parameters` entries.")
    ),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "The stored AQL fails to type-check, or \
                                      `ehr_id` is a malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "No stored query with that name, or a \
                                      well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
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

/// Execute a named stored query, latest version, from the request body
/// (`POST /query/{qualified_query_name}`).
#[utoipa::path(
    post, path = "/query/{qualified_query_name}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`); executed at its latest \
                        version."),
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) scope; may instead be supplied as the \
                        `openehr-ehr-id` header. Supplying both is accepted \
                        only when they name the same EHR; a conflict is a \
                        400.")
    ),
    request_body(content((serde_json::Value = "application/json")),
                 description = "A `Query`: optional `offset`, `fetch`, and \
                                `query_parameters` (no `q` — the AQL is stored)."),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "The stored AQL fails to type-check, the \
                                      JSON body is invalid, or `ehr_id` is a \
                                      malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "No stored query with that name, or a \
                                      well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
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

/// Execute a named stored query at a specific version, from the query string
/// (`GET /query/{qualified_query_name}/{version}`).
#[utoipa::path(
    get, path = "/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`)."),
        ("version" = String, Path,
         description = "A SEMVER version (exact, or a `{major}`/`{major}.{minor}` \
                        prefix resolving to the highest matching version)."),
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) scope; may instead be supplied as the \
                        `openehr-ehr-id` header. Supplying both is accepted \
                        only when they name the same EHR; a conflict is a \
                        400."),
        ("offset" = Option<i64>, Query,
         description = "Row offset into the result set (`0`-based, default `0`)."),
        ("fetch" = Option<i64>, Query,
         description = "Maximum rows to fetch (default is implementation-defined)."),
        ("query_parameters" = Option<String>, Query,
         description = "AQL `$name` binds as repeated `query_parameters` entries.")
    ),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "The stored AQL fails to type-check, or \
                                      `ehr_id` is a malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "No stored query at that name and version, \
                                      or a well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
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

/// Execute a named stored query at a specific version, from the request body
/// (`POST /query/{qualified_query_name}/{version}`).
#[utoipa::path(
    post, path = "/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`)."),
        ("version" = String, Path,
         description = "A SEMVER version (exact, or a `{major}`/`{major}.{minor}` \
                        prefix resolving to the highest matching version)."),
        ("openehr-ehr-id" = Option<String>, Header,
         description = "Alternative form of the `ehr_id` EHR scope (ITS-REST \
                        `docs/query/Request.md` §About the `ehr_id` parameter). \
                        Accepted alongside the `ehr_id` query parameter only \
                        when both name the same EHR; a conflict is a 400."),
        ("ehr_id" = Option<String>, Query,
         description = "Optional EHR (UUID) scope; may instead be supplied as the \
                        `openehr-ehr-id` header. Supplying both is accepted \
                        only when they name the same EHR; a conflict is a \
                        400.")
    ),
    request_body(content((serde_json::Value = "application/json")),
                 description = "A `Query`: optional `offset`, `fetch`, and \
                                `query_parameters` (no `q` — the AQL is stored)."),
    responses(
        (status = 200, description = "The RESULT_SET (canonical JSON); `ETag` \
                                      (weak `W/` form) identifies the result set.",
         body = serde_json::Value),
        (status = 400, description = "The stored AQL fails to type-check, the \
                                      JSON body is invalid, or `ehr_id` is a \
                                      malformed UUID.",
         body = serde_json::Value),
        (status = 404, description = "No stored query at that name and version, \
                                      or a well-formed `ehr_id` scope names no \
                                      existing EHR.",
         body = serde_json::Value),
        (status = 406, description = "OUR WIRE — an exclusively-XML `Accept` (the \
                                      RESULT_SET is JSON only).",
         body = serde_json::Value),
        (status = 408, description = "The query overran the configured execution \
                                      budget.",
         body = serde_json::Value)
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
