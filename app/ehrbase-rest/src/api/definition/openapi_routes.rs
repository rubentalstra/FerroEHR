//! Native `utoipa-axum` routing for the Definition API group (ADL 1.4 / ADL 2
//! templates + stored queries).
//!
//! Operation semantics are the ITS-REST Definition API
//! (`docs/specs/openehr/ITS-REST`); no openEHR spec governs the OAS layout. Each
//! handler forwards to the group dispatcher through [`guarded_dispatch`], so the
//! wire behaviour is identical to the former table-driven `mount` adapter.
//!
//! NOTE (operation ids): a few generated operation ids carry `.` (e.g.
//! `definition_template_adl1.4_list`, `definition_query_store.yaml`) — invalid
//! Rust identifiers, so the handler fn names sanitise `.` to `_` while the op
//! string passed to the dispatcher is the verbatim generated id it matches on.
//!
//! ## Prose-vs-OAS reconciliations (documented real wire, per handler)
//!
//! - **ADL 2 upload** (`definition_template_adl2_upload.yaml`): the OAS declares
//!   `201/400/409`; the REST adapter emits the `409` on a duplicate HRID (the
//!   native SM `upload_artefact` would replace — this seam does not), and
//!   additionally returns `422` when the ADL2/AOM2 artefact is invalid on the
//!   store path (the OAS folds that under `400`). The `at_version` query
//!   parameter is `deprecated: true` and is dropped (spec-permitted); only
//!   `Prefer` is honoured.
//! - **ADL 2 get / version get** (`definition_template_adl2_get.yaml`,
//!   `_version_get.yaml` — the latter `deprecated: true`): the OAS lists a `400`
//!   that is unreachable here (`template_id`/`version` are path segments, always
//!   present); the build serves the `text/plain` ADL2 source and the
//!   `application/json` `OperationalTemplateV2` canonical-JSON projection (the
//!   OAS declares that schema as an opaque `type: object`, so the AOM2 canonical
//!   JSON satisfies it). The response declares no `application/xml` body, so an
//!   `Accept` naming *only* `application/xml` is a `406`.
//! - **ADL 2 example get** (`_example_get`): the stored ADL2 template is compiled
//!   to its operational template and turned into a Web Template (the am24 front
//!   end), which the shared example generator walks into an example COMPOSITION —
//!   served across the four `Accept_LOCATABLE` forms (`200`), with `400`/`404`/
//!   `406` exactly as the ADL 1.4 example endpoint.
//! - **ADL 1.4 upload** (`definition_template_adl1.4_upload.yaml`): the OAS
//!   declares `201/400/409`; our wire additionally returns `422` when the OPT
//!   parses as XML but is structurally invalid (the OAS folds that under `400`).
//! - **ADL 1.4 get / example** (`_get`, `_example_get`): the OAS `400` on `_get`
//!   is unreachable (only a path parameter); the reachable statuses are the
//!   ones documented on each operation below.
//! - **Stored-query list by name** (`definition_query_list.yaml`): the qualified
//!   name is a prefix pattern (empty = wildcard), so an unmatched name yields an
//!   empty `200` list, never `404` — matching the OAS, which declares only
//!   `200`.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The Definition API group as a native `utoipa-axum` router (group-relative
/// paths; nested under the configured base path).
// `definition_template_adl2_version_get` carries `#[deprecated]` (the OAS marks
// the operation `deprecated: true`, reflected into the served OpenAPI); the
// `routes!` macro references it by name, so the deprecation lint is allowed here.
#[allow(deprecated)]
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
        .routes(routes!(definition_query_list_all))
        .routes(routes!(definition_query_list, definition_query_store_yaml))
        .routes(routes!(
            definition_query_version_get,
            definition_query_version_store_yaml
        ))
}

/// List the stored ADL 1.4 operational templates
/// (`GET /definition/template/adl1.4`).
#[utoipa::path(
    get, path = "/definition/template/adl1.4", tag = "ADL1.4",
    params(
        ("template_id" = Option<String>, Query,
         description = "Glob pattern (supports `*`) matched against each stored \
                        `template_id`; omit to match any."),
        ("concept" = Option<String>, Query,
         description = "Glob pattern (supports `*`) matched against each \
                        template's `concept`; omit to match any."),
        ("version" = Option<String>, Query,
         description = "Version filter (e.g. `1.2.*`, or `*` for all versions); \
                        absent returns only the latest version of each match."),
        ("offset" = Option<i64>, Query,
         description = "Row offset into the result set (`0`-based, default `0`)."),
        ("fetch" = Option<i64>, Query,
         description = "Maximum rows to return; absent/`0` returns all matches.")
    ),
    responses(
        (status = 200, description = "The matching template summaries (canonical \
                                      JSON; empty when none match).",
         body = serde_json::Value)
    )
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

/// Upload an ADL 1.4 operational template
/// (`POST /definition/template/adl1.4`).
///
/// The template arrives as canonical OPT XML (`Content-Type: application/xml`).
#[utoipa::path(
    post, path = "/definition/template/adl1.4", tag = "ADL1.4",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the stored OPT XML), or \
                        `return=identifier` (the `{template_id}` JSON object).")
    ),
    request_body(content((serde_json::Value = "application/xml")),
                 description = "The operational template as canonical OPT XML."),
    responses(
        (status = 201, description = "Stored; `Location` addresses the template, \
                                      `ETag` (weak `W/` form) carries the \
                                      `template_id`. Body per `Prefer` \
                                      (representation → OPT XML; identifier → \
                                      `{template_id}`; empty for minimal).",
         body = serde_json::Value),
        (status = 400, description = "The request body is not an XML template \
                                      string.",
         body = serde_json::Value),
        (status = 409, description = "A template with the same `template_id` is \
                                      already stored.",
         body = serde_json::Value),
        (status = 422, description = "OUR WIRE — the OPT XML parsed as XML but is \
                                      structurally invalid (the OAS folds this \
                                      under `400`); an `invalid_template` reject.",
         body = serde_json::Value)
    )
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

/// Retrieve one ADL 1.4 template by id
/// (`GET /definition/template/adl1.4/{template_id}`).
#[utoipa::path(
    get, path = "/definition/template/adl1.4/{template_id}", tag = "ADL1.4",
    params(
        ("template_id" = String, Path,
         description = "The `template_id`; a partial id resolves to the latest \
                        major version of that template."),
        ("Accept" = Option<String>, Header,
         description = "`application/xml` (the canonical OPT; also the default \
                        for absent/`*/*`), or `application/openehr.wt+json` / \
                        `application/json` (the Web Template document — the only \
                        JSON projection of an OPT).")
    ),
    responses(
        (
            status = 200, description = "The template; `ETag` (weak `W/` form) \
                                        carries the `template_id`. Body per \
                                        `Accept`: OPT XML, or the Web Template.",
            content((serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt+json"), (serde_json::Value = "application/json"))
        ),
        (status = 404, description = "No template with `template_id`.",
         body = serde_json::Value),
        (status = 406, description = "`Accept` is outside `application/xml`, \
                                      `application/openehr.wt+json`, and \
                                      `application/json`.",
         body = serde_json::Value)
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

/// Generate an example COMPOSITION for an ADL 1.4 template
/// (`GET /definition/template/adl1.4/{template_id}/example`).
#[utoipa::path(
    get, path = "/definition/template/adl1.4/{template_id}/example", tag = "ADL1.4",
    params(
        ("template_id" = String, Path,
         description = "The `template_id`; a partial id resolves to the latest \
                        major version."),
        ("type" = Option<String>, Query,
         description = "`input` (default; ready to commit) or `output` (as it \
                        would appear when retrieved)."),
        ("detail_level" = Option<String>, Query,
         description = "Example detail: `required` (default), `medium`, or \
                        `complete`."),
        ("Accept" = Option<String>, Header,
         description = "One of `application/json` (default), `application/xml`, \
                        `application/openehr.wt.flat+json`, or \
                        `application/openehr.wt.structured+json`.")
    ),
    responses(
        (
            status = 200, description = "The generated example COMPOSITION, \
                                        serialized per `Accept` (canonical \
                                        JSON/XML or a Simplified Format).",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "`type` or `detail_level` is outside its \
                                      enumerated set.",
         body = serde_json::Value),
        (status = 404, description = "No template with `template_id`.",
         body = serde_json::Value),
        (status = 406, description = "`Accept` is outside the four supported \
                                      example representations.",
         body = serde_json::Value)
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

/// List the stored ADL 2 templates (`GET /definition/template/adl2`).
#[utoipa::path(
    get, path = "/definition/template/adl2", tag = "ADL2",
    params(
        ("template_id" = Option<String>, Query,
         description = "Glob pattern (supports `*`) matched against each stored \
                        `template_id`; omit to match any."),
        ("concept" = Option<String>, Query,
         description = "Glob pattern (supports `*`) matched against each \
                        template's `concept`; omit to match any."),
        ("version" = Option<String>, Query,
         description = "Version filter (e.g. `1.2.*`, or `*` for all versions); \
                        absent returns only the latest version of each match."),
        ("offset" = Option<i64>, Query,
         description = "Row offset into the result set (`0`-based, default `0`)."),
        ("fetch" = Option<i64>, Query,
         description = "Maximum rows to return; absent/`0` returns all matches.")
    ),
    responses(
        (status = 200, description = "The matching `TemplateMetadata` rows \
                                      (`template_id`, `concept`, `archetype_id`, \
                                      `created_timestamp`; empty when none match).",
         body = serde_json::Value)
    )
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

/// Upload an ADL 2 operational template (`POST /definition/template/adl2`).
///
/// The template arrives as `text/plain` ADL2 source. The deprecated `at_version`
/// query parameter is dropped (spec-permitted).
#[utoipa::path(
    post, path = "/definition/template/adl2", tag = "ADL2",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the stored ADL2 source, \
                        `text/plain`), or `return=identifier` (the \
                        `{template_id}` JSON object).")
    ),
    request_body(content((String = "text/plain")),
                 description = "The ADL2 operational-template source."),
    responses(
        (status = 201, description = "Stored; `Location` addresses the template \
                                      and `ETag` (weak `W/` form) carries its \
                                      `ARCHETYPE_HRID`. Body per `Prefer` \
                                      (representation → ADL2 source; identifier \
                                      → `{template_id}`; empty for minimal).",
         body = serde_json::Value),
        (status = 400, description = "The request body is not valid UTF-8 text.",
         body = serde_json::Value),
        (status = 409, description = "An ADL2 template with the same HRID is \
                                      already stored.",
         body = serde_json::Value),
        (status = 422, description = "OUR WIRE — the ADL2 source is invalid: \
                                      unparseable (S-codes) or failing an AOM2 \
                                      validation phase (V-codes); the `Error` \
                                      body's `validationErrors` carry the rule \
                                      codes (the OAS folds this under `400`).",
         body = serde_json::Value)
    )
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

/// Retrieve one ADL 2 template by id
/// (`GET /definition/template/adl2/{template_id}`).
///
/// Served as `text/plain` ADL2 source or the `application/json`
/// `OperationalTemplateV2` canonical-JSON projection, per `Accept`.
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}", tag = "ADL2",
    params(
        ("template_id" = String, Path,
         description = "The `template_id`; a partial id resolves to the latest \
                        matching version."),
        ("Accept" = Option<String>, Header,
         description = "`text/plain` (the ADL2 source; also the default for \
                        absent/`*/*`/`text/*`) or `application/json` (the \
                        `OperationalTemplateV2` canonical JSON). \
                        `application/xml` has no declared response body, so an \
                        `Accept` naming only it is a `406`.")
    ),
    responses(
        (status = 200, description = "The operational template — `text/plain` \
                                      ADL2 source or `application/json` \
                                      `OperationalTemplateV2`; `ETag` (weak `W/` \
                                      form) carries the RESOLVED \
                                      `ARCHETYPE_HRID` of the served artefact.",
         content((String = "text/plain"), (serde_json::Value = "application/json"))),
        (status = 404, description = "No template with `template_id`.",
         body = serde_json::Value),
        (status = 406, description = "`Accept` names only `application/xml`, \
                                      which has no declared response body.",
         body = serde_json::Value)
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

/// Generate an example COMPOSITION for an ADL 2 template
/// (`GET /definition/template/adl2/{template_id}/example`).
///
/// The stored ADL2 template is compiled to its operational template and turned
/// into a Web Template (the am24 front end), which the shared example generator
/// walks into a canonical example COMPOSITION — the same generator the ADL 1.4
/// example endpoint uses.
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}/example", tag = "ADL2",
    params(
        ("template_id" = String, Path,
         description = "The `template_id`; a partial id resolves to the latest \
                        matching version."),
        ("type" = Option<String>, Query,
         description = "`input` (default; ready to commit) or `output` (as it \
                        would appear when retrieved)."),
        ("detail_level" = Option<String>, Query,
         description = "Example detail: `required` (default), `medium`, or \
                        `complete`."),
        ("Accept" = Option<String>, Header,
         description = "One of `application/json` (default), `application/xml`, \
                        `application/openehr.wt.flat+json`, or \
                        `application/openehr.wt.structured+json`.")
    ),
    responses(
        (
            status = 200, description = "The generated example COMPOSITION, \
                                        serialized per `Accept` (canonical \
                                        JSON/XML or a Simplified Format).",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "`type` or `detail_level` is outside its \
                                      enumerated set.",
         body = serde_json::Value),
        (status = 404, description = "No template with `template_id`.",
         body = serde_json::Value),
        (status = 406, description = "`Accept` is outside the four supported \
                                      example representations.",
         body = serde_json::Value)
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

/// Retrieve one ADL 2 template at a specific version
/// (`GET /definition/template/adl2/{template_id}/{version}`).
///
/// DEPRECATED in the OAS but served: resolves `template_id` + the SEMVER
/// `version` (exact or `{major}[.{minor}]` prefix → highest match) and returns
/// the same representations as `_get` (`text/plain` source / `application/json`
/// `OperationalTemplateV2`).
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}/{version}", tag = "ADL2",
    params(
        ("template_id" = String, Path,
         description = "The `template_id`; a partial id resolves within the \
                        matching version."),
        ("version" = String, Path,
         description = "A SEMVER version (exact, or a `{major}`/`{major}.{minor}` \
                        prefix resolving to the highest match)."),
        ("Accept" = Option<String>, Header,
         description = "`text/plain` (the ADL2 source) or `application/json` \
                        (the `OperationalTemplateV2` canonical JSON); \
                        `application/xml` only → `406`.")
    ),
    responses(
        (status = 200, description = "The operational template at `version` — \
                                      `text/plain` ADL2 source or \
                                      `application/json` `OperationalTemplateV2`; \
                                      `ETag` (weak `W/` form) carries the \
                                      RESOLVED `ARCHETYPE_HRID`, not the \
                                      addressed prefix.",
         content((String = "text/plain"), (serde_json::Value = "application/json"))),
        (status = 404, description = "No template with `template_id` at `version`.",
         body = serde_json::Value),
        (status = 406, description = "`Accept` names only `application/xml`, \
                                      which has no declared response body.",
         body = serde_json::Value)
    )
)]
#[deprecated = "ITS-REST marks this operation deprecated \
                (definition_template_adl2_version_get.yaml); served for compatibility"]
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

/// List every stored query (`GET /definition/query`).
///
/// OUR OWN EXTENSION — no openEHR spec governs the bare form; the vendored OAS
/// defines only the `{qualified_query_name}` route. This is the empty-prefix
/// case of that route's documented prefix semantics ("List stored queries").
#[utoipa::path(
    get, path = "/definition/query", tag = "Query",
    responses(
        (status = 200, description = "Every stored query, all versions (canonical \
                                      JSON; empty when none are stored).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn definition_query_list_all(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "definition_query_list_all",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// List the stored versions of a named stored query
/// (`GET /definition/query/{qualified_query_name}`).
///
/// The name is a prefix pattern (`[{namespace}::]{query-name}`), so an unmatched
/// name yields an empty `200` list, never `404` (see the module doc).
#[utoipa::path(
    get, path = "/definition/query/{qualified_query_name}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name as a prefix pattern \
                        (`[{namespace}::]{query-name}`); it matches every query \
                        whose name starts with it (case-insensitive).")
    ),
    responses(
        (status = 200, description = "The matching stored queries, all versions \
                                      (canonical JSON; empty when none match).",
         body = serde_json::Value)
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

/// Store a named AQL query, server-assigned SEMVER
/// (`PUT /definition/query/{qualified_query_name}`).
///
/// The AQL text is the `text/plain` request body.
#[utoipa::path(
    put, path = "/definition/query/{qualified_query_name}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`)."),
        ("query_type" = Option<String>, Query,
         description = "The query formalism (default `AQL`); a non-AQL formalism \
                        is an honest unsupported-formalism `400`.")
    ),
    request_body(content((String = "text/plain")),
                 description = "The AQL query text."),
    responses(
        (status = 200, description = "Stored; `Location` addresses the stored \
                                      query at its auto-assigned version.",
         body = serde_json::Value),
        (status = 400, description = "The AQL fails to parse, `query_type` is \
                                      an unsupported non-AQL formalism, or the \
                                      query-name is the reserved `aql` \
                                      (case-insensitive).",
         body = serde_json::Value)
    )
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

/// Retrieve a named stored query at a specific version
/// (`GET /definition/query/{qualified_query_name}/{version}`).
#[utoipa::path(
    get, path = "/definition/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`)."),
        ("version" = String, Path,
         description = "A SEMVER version (exact, or a `{major}`/`{major}.{minor}` \
                        prefix resolving to the highest matching version).")
    ),
    responses(
        (status = 200, description = "The stored query and its metadata \
                                      (canonical JSON).",
         body = serde_json::Value),
        (status = 404, description = "No stored query matches that name and \
                                      version.",
         body = serde_json::Value)
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

/// Store a named AQL query at a specific version
/// (`PUT /definition/query/{qualified_query_name}/{version}`).
///
/// The AQL text is the `text/plain` request body; the version is stored verbatim.
#[utoipa::path(
    put, path = "/definition/query/{qualified_query_name}/{version}", tag = "Query",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified stored-query name \
                        (`[{namespace}::]{query-name}`)."),
        ("version" = String, Path,
         description = "The exact SEMVER version to store the query under."),
        ("query_type" = Option<String>, Query,
         description = "The query formalism (default `AQL`); a non-AQL formalism \
                        is an honest unsupported-formalism `400`.")
    ),
    request_body(content((String = "text/plain")),
                 description = "The AQL query text."),
    responses(
        (status = 200, description = "Stored; `Location` addresses the stored \
                                      query at this version.",
         body = serde_json::Value),
        (status = 400, description = "The AQL fails to parse, `query_type` is \
                                      an unsupported non-AQL formalism, or the \
                                      query-name is the reserved `aql` \
                                      (case-insensitive).",
         body = serde_json::Value),
        (status = 409, description = "A stored query already exists at this \
                                      `(name, version)`.",
         body = serde_json::Value)
    )
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
