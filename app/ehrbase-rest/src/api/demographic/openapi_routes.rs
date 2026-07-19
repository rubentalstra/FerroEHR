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
//! ([`super::dispatch::dispatch`]) through [`guarded_dispatch`] — so the wire behaviour
//! is identical to the former table-driven `mount()` adapter (same
//! `EHR_ACCESS` gate, ABAC PEP, and ATNA audit tagging).
//!
//! The `#[utoipa::path]` documentation is spec-exact against the demographic
//! operation YAMLs (`operations/{agent,person,group,organisation,role}_*.yaml`,
//! `versioned_party_*.yaml`, `demographic_contribution_*.yaml`, the `*_tags_*`
//! ops) and their `$ref`d responses/parameters/headers, with the
//! DEVELOPMENT-status gaps filled from the ITS-REST overview prose
//! (`docs/overview/Requests_and_responses.md`): the weak `W/` `ETag` MUST, the
//! `Prefer` `return=minimal|representation|identifier` triad, the
//! committal-header (`openehr-version`/`openehr-audit-details`) MUST-accept
//! rule, and the `If-Match` 400/412 rules.
//! Every versioned response also carries `Last-Modified` from the version's
//! commit time (overview §"`ETag` and Last-Modified": both SHOULD accompany
//! versioned resources). Demographic `PARTY`/`PARTY_RELATIONSHIP`
//! resources are not templated, so a Simplified-Format `Content-Type`/`Accept`
//! is rejected (`415`/`406` — our own design, since no template governs a
//! party); this is not in the YAMLs' `Accept_LOCATABLE` enum but is the real
//! wire ([`super::party`] runs `guard_non_templated` for every op).
//!
//! The own-design `PARTY_RELATIONSHIP` extension is *not* here — it lives in
//! [`super::relationship`] (no ITS-REST operation governs it).

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The standard Demographic API group as a native `utoipa-axum` router.
/// Group-relative paths (nested under the configured `base_path`); every
/// operation runs through [`guarded_dispatch`] with the demographic group
/// [`dispatch`](super::dispatch::dispatch), so the wire behaviour is identical to the
/// former table-driven `mount` adapter.
pub(crate) fn routes() -> OpenApiRouter<AppState> {
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
// (`super::dispatch::dispatch`), so the EHR_ACCESS gate, ABAC PEP, and ATNA audit tagging
// apply uniformly.

// ── AGENT ───────────────────────────────────────────────────────────────────

/// Create an `AGENT` (`POST /demographic/agent`).
#[utoipa::path(
    post, path = "/demographic/agent", tag = "AGENT",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created AGENT), or \
                        `return=identifier` (only the uid)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION (e.g. \
                        `lifecycle_state.code_string`); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS (committer, \
                        description, change_type); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the VERSIONED_PARTY; the \
                        stored set is echoed in the response header."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with this VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The AGENT (RM canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`; stored \
                                      ITEM_TAGs ride the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted AGENT.",
         body = serde_json::Value),
        (status = 404, description = "A referenced resource does not exist.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The AGENT is syntactically valid but \
                                      fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_create", parts, super::dispatch::dispatch).await
}

/// Retrieve an `AGENT` by uid-based id
/// (`GET /demographic/agent/{uid_based_id}`).
#[utoipa::path(
    get, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (a specific `version_uid`) \
                        or a HIER_OBJECT_ID (`versioned_object_uid`) for the \
                        latest / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The AGENT (RM canonical JSON/XML); `ETag` \
                                      carries the version uid (weak `W/` form), \
                                      any ITEM_TAGs ride the item-tag response \
                                      headers.", body = serde_json::Value),
        (status = 204, description = "The AGENT version at the requested time \
                                      is deleted."),
        (status = 404, description = "Unknown AGENT, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_get", parts, super::dispatch::dispatch).await
}

/// Update an `AGENT` (`PUT /demographic/agent/{uid_based_id}`).
#[utoipa::path(
    put, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the AGENT \
                        to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation`, or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the new VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new AGENT (RM canonical JSON or XML); any \
                                `uid` must match the path id."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation` \
                                      or `return=identifier`); `ETag`/`Location` \
                                      carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown AGENT.", body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The AGENT fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_update", parts, super::dispatch::dispatch).await
}

/// Delete an `AGENT` (`DELETE /demographic/agent/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/demographic/agent/{uid_based_id}", tag = "AGENT",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the AGENT is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown AGENT.", body = serde_json::Value),
        (status = 409, description = "The supplied `uid_based_id` is not the \
                                      latest version; `ETag` carries the current \
                                      latest version uid.", body = serde_json::Value)
    )
)]
pub(crate) async fn agent_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_delete", parts, super::dispatch::dispatch).await
}

// ── GROUP ─────────────────────────────────────────────────────────────────

/// Create a `GROUP` (`POST /demographic/group`).
#[utoipa::path(
    post, path = "/demographic/group", tag = "GROUP",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created GROUP), or \
                        `return=identifier` (only the uid)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION (e.g. \
                        `lifecycle_state.code_string`); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS (committer, \
                        description, change_type); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the VERSIONED_PARTY; the \
                        stored set is echoed in the response header."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with this VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The GROUP (RM canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`; stored \
                                      ITEM_TAGs ride the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted GROUP.",
         body = serde_json::Value),
        (status = 404, description = "A referenced resource does not exist.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The GROUP is syntactically valid but \
                                      fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_create", parts, super::dispatch::dispatch).await
}

/// Retrieve a `GROUP` by uid-based id
/// (`GET /demographic/group/{uid_based_id}`).
#[utoipa::path(
    get, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (a specific `version_uid`) \
                        or a HIER_OBJECT_ID (`versioned_object_uid`) for the \
                        latest / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The GROUP (RM canonical JSON/XML); `ETag` \
                                      carries the version uid (weak `W/` form), \
                                      any ITEM_TAGs ride the item-tag response \
                                      headers.", body = serde_json::Value),
        (status = 204, description = "The GROUP version at the requested time \
                                      is deleted."),
        (status = 404, description = "Unknown GROUP, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_get", parts, super::dispatch::dispatch).await
}

/// Update a `GROUP` (`PUT /demographic/group/{uid_based_id}`).
#[utoipa::path(
    put, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the GROUP \
                        to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation`, or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the new VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new GROUP (RM canonical JSON or XML); any \
                                `uid` must match the path id."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation` \
                                      or `return=identifier`); `ETag`/`Location` \
                                      carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown GROUP.", body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The GROUP fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_update", parts, super::dispatch::dispatch).await
}

/// Delete a `GROUP` (`DELETE /demographic/group/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/demographic/group/{uid_based_id}", tag = "GROUP",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the GROUP is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown GROUP.", body = serde_json::Value),
        (status = 409, description = "The supplied `uid_based_id` is not the \
                                      latest version; `ETag` carries the current \
                                      latest version uid.", body = serde_json::Value)
    )
)]
pub(crate) async fn group_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_delete", parts, super::dispatch::dispatch).await
}

// ── ORGANISATION ────────────────────────────────────────────────────────────

/// Create an `ORGANISATION` (`POST /demographic/organisation`).
#[utoipa::path(
    post, path = "/demographic/organisation", tag = "ORGANISATION",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created ORGANISATION), or \
                        `return=identifier` (only the uid)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION (e.g. \
                        `lifecycle_state.code_string`); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS (committer, \
                        description, change_type); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the VERSIONED_PARTY; the \
                        stored set is echoed in the response header."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with this VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The ORGANISATION (RM canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`; stored \
                                      ITEM_TAGs ride the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted ORGANISATION.",
         body = serde_json::Value),
        (status = 404, description = "A referenced resource does not exist.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The ORGANISATION is syntactically valid \
                                      but fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_create",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve an `ORGANISATION` by uid-based id
/// (`GET /demographic/organisation/{uid_based_id}`).
#[utoipa::path(
    get, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (a specific `version_uid`) \
                        or a HIER_OBJECT_ID (`versioned_object_uid`) for the \
                        latest / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The ORGANISATION (RM canonical JSON/XML); \
                                      `ETag` carries the version uid (weak `W/` \
                                      form), any ITEM_TAGs ride the item-tag \
                                      response headers.", body = serde_json::Value),
        (status = 204, description = "The ORGANISATION version at the requested \
                                      time is deleted."),
        (status = 404, description = "Unknown ORGANISATION, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "organisation_get", parts, super::dispatch::dispatch).await
}

/// Update an `ORGANISATION`
/// (`PUT /demographic/organisation/{uid_based_id}`).
#[utoipa::path(
    put, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the \
                        ORGANISATION to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation`, or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the new VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new ORGANISATION (RM canonical JSON or XML); \
                                any `uid` must match the path id."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation` \
                                      or `return=identifier`); `ETag`/`Location` \
                                      carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown ORGANISATION.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The ORGANISATION fails RM/semantic \
                                      validation.", body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_update",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Delete an `ORGANISATION`
/// (`DELETE /demographic/organisation/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/demographic/organisation/{uid_based_id}", tag = "ORGANISATION",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the ORGANISATION is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown ORGANISATION.",
         body = serde_json::Value),
        (status = 409, description = "The supplied `uid_based_id` is not the \
                                      latest version; `ETag` carries the current \
                                      latest version uid.", body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

// ── PERSON ────────────────────────────────────────────────────────────────

/// Create a `PERSON` (`POST /demographic/person`).
#[utoipa::path(
    post, path = "/demographic/person", tag = "PERSON",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created PERSON), or \
                        `return=identifier` (only the uid)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION (e.g. \
                        `lifecycle_state.code_string`); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS (committer, \
                        description, change_type); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the VERSIONED_PARTY; the \
                        stored set is echoed in the response header."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with this VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The PERSON (RM canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`; stored \
                                      ITEM_TAGs ride the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted PERSON.",
         body = serde_json::Value),
        (status = 404, description = "A referenced resource does not exist.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The PERSON is syntactically valid but \
                                      fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_create", parts, super::dispatch::dispatch).await
}

/// Retrieve a `PERSON` by uid-based id
/// (`GET /demographic/person/{uid_based_id}`).
#[utoipa::path(
    get, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (a specific `version_uid`) \
                        or a HIER_OBJECT_ID (`versioned_object_uid`) for the \
                        latest / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The PERSON (RM canonical JSON/XML); `ETag` \
                                      carries the version uid (weak `W/` form), \
                                      any ITEM_TAGs ride the item-tag response \
                                      headers.", body = serde_json::Value),
        (status = 204, description = "The PERSON version at the requested time \
                                      is deleted."),
        (status = 404, description = "Unknown PERSON, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_get", parts, super::dispatch::dispatch).await
}

/// Update a `PERSON` (`PUT /demographic/person/{uid_based_id}`).
#[utoipa::path(
    put, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the PERSON \
                        to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation`, or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the new VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new PERSON (RM canonical JSON or XML); any \
                                `uid` must match the path id."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation` \
                                      or `return=identifier`); `ETag`/`Location` \
                                      carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown PERSON.", body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The PERSON fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_update", parts, super::dispatch::dispatch).await
}

/// Delete a `PERSON` (`DELETE /demographic/person/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/demographic/person/{uid_based_id}", tag = "PERSON",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the PERSON is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown PERSON.", body = serde_json::Value),
        (status = 409, description = "The supplied `uid_based_id` is not the \
                                      latest version; `ETag` carries the current \
                                      latest version uid.", body = serde_json::Value)
    )
)]
pub(crate) async fn person_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_delete", parts, super::dispatch::dispatch).await
}

// ── ROLE ────────────────────────────────────────────────────────────────────

/// Create a `ROLE` (`POST /demographic/role`).
#[utoipa::path(
    post, path = "/demographic/role", tag = "ROLE",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created ROLE), or \
                        `return=identifier` (only the uid)."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION (e.g. \
                        `lifecycle_state.code_string`); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS (committer, \
                        description, change_type); accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the VERSIONED_PARTY; the \
                        stored set is echoed in the response header."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with this VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The ROLE (RM canonical JSON or XML)."),
    responses(
        (status = 201, description = "Created; `ETag` carries the new version \
                                      uid (weak `W/` form), `Location` the \
                                      resource URL. Body per `Prefer`; stored \
                                      ITEM_TAGs ride the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a precondition \
                                      violation on the submitted ROLE.",
         body = serde_json::Value),
        (status = 404, description = "A referenced resource does not exist.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The ROLE is syntactically valid but \
                                      fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_create(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_create", parts, super::dispatch::dispatch).await
}

/// Retrieve a `ROLE` by uid-based id
/// (`GET /demographic/role/{uid_based_id}`).
#[utoipa::path(
    get, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (a specific `version_uid`) \
                        or a HIER_OBJECT_ID (`versioned_object_uid`) for the \
                        latest / at-time version."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; when the id is a \
                        `versioned_object_uid`, selects the version extant at \
                        that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The ROLE (RM canonical JSON/XML); `ETag` \
                                      carries the version uid (weak `W/` form), \
                                      any ITEM_TAGs ride the item-tag response \
                                      headers.", body = serde_json::Value),
        (status = 204, description = "The ROLE version at the requested time \
                                      is deleted."),
        (status = 404, description = "Unknown ROLE, or no version at the \
                                      requested `version_at_time`.",
         body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_get", parts, super::dispatch::dispatch).await
}

/// Update a `ROLE` (`PUT /demographic/role/{uid_based_id}`).
#[utoipa::path(
    put, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID `versioned_object_uid` of the ROLE \
                        to update."),
        ("If-Match" = String, Header,
         description = "The latest `version_uid` (the `preceding_version_uid`), \
                        double-quoted (weak `W/` form also accepted). \
                        Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation`, or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the new VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "ITEM_TAGs to associate with the new VERSION; the stored \
                        set is echoed in the response header.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new ROLE (RM canonical JSON or XML); any \
                                `uid` must match the path id."),
    responses(
        (status = 200, description = "Updated (`Prefer: return=representation` \
                                      or `return=identifier`); `ETag`/`Location` \
                                      carry the new version.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); \
                                      `ETag`/`Location` carry the new version."),
        (status = 400, description = "Malformed request, or missing `If-Match`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown ROLE.", body = serde_json::Value),
        (status = 406, description = "A Simplified Format was requested via \
                                      `Accept` (parties are not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.", body = serde_json::Value),
        (status = 415, description = "A Simplified Format `Content-Type` was \
                                      sent (parties are not templated).",
         body = serde_json::Value),
        (status = 422, description = "The ROLE fails RM/semantic validation.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_update", parts, super::dispatch::dispatch).await
}

/// Delete a `ROLE` (`DELETE /demographic/role/{uid_based_id}`).
#[utoipa::path(
    delete, path = "/demographic/role/{uid_based_id}", tag = "ROLE",
    params(
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID `version_uid` of the latest \
                        version (the `preceding_version_uid`) to delete."),
        ("openehr-version" = Option<String>, Header,
         description = "Optional committal metadata for the delete VERSION; \
                        accepted per the committal-header MUST-accept rule."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Optional committal AUDIT_DETAILS; accepted per the \
                        committal-header MUST-accept rule.")
    ),
    responses(
        (status = 204, description = "Logically deleted; `ETag` carries the \
                                      deleted version uid."),
        (status = 400, description = "Malformed request, or the ROLE is \
                                      already deleted.", body = serde_json::Value),
        (status = 404, description = "Unknown ROLE.", body = serde_json::Value),
        (status = 409, description = "The supplied `uid_based_id` is not the \
                                      latest version; `ETag` carries the current \
                                      latest version uid.", body = serde_json::Value)
    )
)]
pub(crate) async fn role_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_delete", parts, super::dispatch::dispatch).await
}

// ── VERSIONED_PARTY ──────────────────────────────────────────────────────────

/// Retrieve the `VERSIONED_PARTY` container
/// (`GET /demographic/versioned_party/{versioned_object_uid}`).
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}", tag = "VERSIONED_PARTY",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The VERSIONED_PARTY (RM canonical \
                                      JSON/XML).", body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve the party's `REVISION_HISTORY`
/// (`GET /demographic/versioned_party/{versioned_object_uid}/revision_history`).
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/revision_history", tag = "VERSIONED_PARTY",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The REVISION_HISTORY (RM canonical \
                                      JSON/XML).", body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_revision_history(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_revision_history",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve the party VERSION at a point in time
/// (`GET /demographic/versioned_party/{versioned_object_uid}/version`).
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/version", tag = "VERSIONED_PARTY",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`)."),
        ("version_at_time" = Option<String>, Query,
         description = "Extended ISO 8601 instant; selects the VERSION extant \
                        at that time (latest when omitted).")
    ),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON/XML); \
                                      `ETag` carries the version uid (weak `W/` \
                                      form).", body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY, or no version at \
                                      the requested `version_at_time`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_version_get_at_time(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_version_get_at_time",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a specific party VERSION by version uid
/// (`GET /demographic/versioned_party/{versioned_object_uid}/version/{version_uid}`).
#[utoipa::path(
    get, path = "/demographic/versioned_party/{versioned_object_uid}/version/{version_uid}", tag = "VERSIONED_PARTY",
    params(
        ("versioned_object_uid" = String, Path,
         description = "The VERSIONED_PARTY uid (a HIER_OBJECT_ID / \
                        `versioned_object_uid`)."),
        ("version_uid" = String, Path,
         description = "The VERSION identifier (OBJECT_VERSION_ID / \
                        `version_uid`).")
    ),
    responses(
        (status = 200, description = "The VERSION (RM canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown VERSIONED_PARTY or version.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn versioned_party_version_get_by_id(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "versioned_party_version_get_by_id",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

// ── CONTRIBUTION ─────────────────────────────────────────────────────────────

/// Create a demographic `CONTRIBUTION` (`POST /demographic/contribution`).
#[utoipa::path(
    post, path = "/demographic/contribution", tag = "CONTRIBUTION",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body), \
                        `return=representation` (the created CONTRIBUTION), or \
                        `return=identifier` (only the uid).")
    ),
    request_body(content = serde_json::Value,
                 description = "The CONTRIBUTION (canonical JSON/XML envelope); \
                                the `audit` and each `versions[i].commit_audit` \
                                are UPDATE_AUDIT objects; an optional `uid` is \
                                honoured when not already in use."),
    responses(
        (status = 201, description = "Created; `ETag` carries the \
                                      `contribution_uid` (weak `W/` form), \
                                      `Location` the resource URL. Body per \
                                      `Prefer`.", body = serde_json::Value),
        (status = 400, description = "Malformed request, or a modification type \
                                      that does not match the operation (e.g. a \
                                      first-version MODIFICATION).",
         body = serde_json::Value),
        (status = 409, description = "A CONTRIBUTION with the same `uid` already \
                                      exists.", body = serde_json::Value)
    )
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
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a demographic `CONTRIBUTION` by uid
/// (`GET /demographic/contribution/{contribution_uid}`).
#[utoipa::path(
    get, path = "/demographic/contribution/{contribution_uid}", tag = "CONTRIBUTION",
    params(
        ("contribution_uid" = String, Path,
         description = "The CONTRIBUTION uid.")
    ),
    responses(
        (status = 200, description = "The CONTRIBUTION (canonical JSON/XML \
                                      envelope).", body = serde_json::Value),
        (status = 404, description = "No CONTRIBUTION with that \
                                      `contribution_uid`.", body = serde_json::Value)
    )
)]
pub(crate) async fn contribution_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "contribution_get", parts, super::dispatch::dispatch).await
}

// ── ITEM_TAG sub-resources ───────────────────────────────────────────────────

/// List `ITEM_TAG`s across the demographic surface (`GET /demographic/tags`).
#[utoipa::path(
    get, path = "/demographic/tags", tag = "ITEM_TAG",
    params(
        ("tag_key" = Option<String>, Query,
         description = "Filter by ITEM_TAG `key` (exact match)."),
        ("tag_value" = Option<String>, Query,
         description = "Filter by ITEM_TAG `value` (exact match)."),
        ("tag_target_path" = Option<String>, Query,
         description = "Filter by ITEM_TAG `target_path` (exact match).")
    ),
    responses(
        (status = 200, description = "The matching ITEM_TAGs (an empty array \
                                      when none match).", body = serde_json::Value),
        (status = 400, description = "A filter query parameter is invalid.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn demographic_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "demographic_tags_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve an `AGENT`'s `ITEM_TAG`s
/// (`GET /demographic/agent/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/demographic/agent/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target AGENT VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAGs on the target (an empty \
                                      array when none exist).",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_get", parts, super::dispatch::dispatch).await
}

/// Replace an `AGENT`'s `ITEM_TAG`s
/// (`PUT /demographic/agent/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/demographic/agent/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target AGENT VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation` (the stored ITEM_TAG list).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to store; an empty array \
                                removes all tags on the target."),
    responses(
        (status = 200, description = "Stored (`Prefer: return=representation`); \
                                      body is the stored ITEM_TAG list.",
         body = serde_json::Value),
        (status = 204, description = "Stored (`Prefer: return=minimal`)."),
        (status = 400, description = "The ITEM_TAG list is invalid.",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_update", parts, super::dispatch::dispatch).await
}

/// Delete one `ITEM_TAG` from an `AGENT` by key
/// (`DELETE /demographic/agent/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/demographic/agent/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target AGENT VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("key" = String, Path, description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The ITEM_TAG was deleted."),
        (status = 404, description = "The `uid_based_id` does not exist, or no \
                                      ITEM_TAG has that `key`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn agent_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "agent_tags_delete", parts, super::dispatch::dispatch).await
}

/// Retrieve a `GROUP`'s `ITEM_TAG`s
/// (`GET /demographic/group/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/demographic/group/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target GROUP VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAGs on the target (an empty \
                                      array when none exist).",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_get", parts, super::dispatch::dispatch).await
}

/// Replace a `GROUP`'s `ITEM_TAG`s
/// (`PUT /demographic/group/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/demographic/group/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target GROUP VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation` (the stored ITEM_TAG list).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to store; an empty array \
                                removes all tags on the target."),
    responses(
        (status = 200, description = "Stored (`Prefer: return=representation`); \
                                      body is the stored ITEM_TAG list.",
         body = serde_json::Value),
        (status = 204, description = "Stored (`Prefer: return=minimal`)."),
        (status = 400, description = "The ITEM_TAG list is invalid.",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_update", parts, super::dispatch::dispatch).await
}

/// Delete one `ITEM_TAG` from a `GROUP` by key
/// (`DELETE /demographic/group/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/demographic/group/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target GROUP VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("key" = String, Path, description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The ITEM_TAG was deleted."),
        (status = 404, description = "The `uid_based_id` does not exist, or no \
                                      ITEM_TAG has that `key`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn group_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "group_tags_delete", parts, super::dispatch::dispatch).await
}

/// Retrieve an `ORGANISATION`'s `ITEM_TAG`s
/// (`GET /demographic/organisation/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/demographic/organisation/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ORGANISATION VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAGs on the target (an empty \
                                      array when none exist).",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_tags_get",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Replace an `ORGANISATION`'s `ITEM_TAG`s
/// (`PUT /demographic/organisation/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/demographic/organisation/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ORGANISATION VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation` (the stored ITEM_TAG list).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to store; an empty array \
                                removes all tags on the target."),
    responses(
        (status = 200, description = "Stored (`Prefer: return=representation`); \
                                      body is the stored ITEM_TAG list.",
         body = serde_json::Value),
        (status = 204, description = "Stored (`Prefer: return=minimal`)."),
        (status = 400, description = "The ITEM_TAG list is invalid.",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_tags_update",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Delete one `ITEM_TAG` from an `ORGANISATION` by key
/// (`DELETE /demographic/organisation/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/demographic/organisation/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ORGANISATION VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("key" = String, Path, description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The ITEM_TAG was deleted."),
        (status = 404, description = "The `uid_based_id` does not exist, or no \
                                      ITEM_TAG has that `key`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn organisation_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "organisation_tags_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a `PERSON`'s `ITEM_TAG`s
/// (`GET /demographic/person/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/demographic/person/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target PERSON VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAGs on the target (an empty \
                                      array when none exist).",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "person_tags_get", parts, super::dispatch::dispatch).await
}

/// Replace a `PERSON`'s `ITEM_TAG`s
/// (`PUT /demographic/person/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/demographic/person/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target PERSON VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation` (the stored ITEM_TAG list).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to store; an empty array \
                                removes all tags on the target."),
    responses(
        (status = 200, description = "Stored (`Prefer: return=representation`); \
                                      body is the stored ITEM_TAG list.",
         body = serde_json::Value),
        (status = 204, description = "Stored (`Prefer: return=minimal`)."),
        (status = 400, description = "The ITEM_TAG list is invalid.",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "person_tags_update",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Delete one `ITEM_TAG` from a `PERSON` by key
/// (`DELETE /demographic/person/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/demographic/person/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target PERSON VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("key" = String, Path, description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The ITEM_TAG was deleted."),
        (status = 404, description = "The `uid_based_id` does not exist, or no \
                                      ITEM_TAG has that `key`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn person_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "person_tags_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Retrieve a `ROLE`'s `ITEM_TAG`s
/// (`GET /demographic/role/{uid_based_id}/tags`).
#[utoipa::path(
    get, path = "/demographic/role/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ROLE VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAGs on the target (an empty \
                                      array when none exist).",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_tags_get(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_get", parts, super::dispatch::dispatch).await
}

/// Replace a `ROLE`'s `ITEM_TAG`s
/// (`PUT /demographic/role/{uid_based_id}/tags`).
#[utoipa::path(
    put, path = "/demographic/role/{uid_based_id}/tags", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ROLE VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; empty body) or \
                        `return=representation` (the stored ITEM_TAG list).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to store; an empty array \
                                removes all tags on the target."),
    responses(
        (status = 200, description = "Stored (`Prefer: return=representation`); \
                                      body is the stored ITEM_TAG list.",
         body = serde_json::Value),
        (status = 204, description = "Stored (`Prefer: return=minimal`)."),
        (status = 400, description = "The ITEM_TAG list is invalid.",
         body = serde_json::Value),
        (status = 404, description = "The `uid_based_id` does not exist.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_tags_update(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_update", parts, super::dispatch::dispatch).await
}

/// Delete one `ITEM_TAG` from a `ROLE` by key
/// (`DELETE /demographic/role/{uid_based_id}/tags/{key}`).
#[utoipa::path(
    delete, path = "/demographic/role/{uid_based_id}/tags/{key}", tag = "ITEM_TAG",
    params(
        ("uid_based_id" = String, Path,
         description = "The target ROLE VERSION (`version_uid`) or \
                        VERSIONED_PARTY (`versioned_object_uid`)."),
        ("key" = String, Path, description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The ITEM_TAG was deleted."),
        (status = 404, description = "The `uid_based_id` does not exist, or no \
                                      ITEM_TAG has that `key`.",
         body = serde_json::Value)
    )
)]
pub(crate) async fn role_tags_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "role_tags_delete", parts, super::dispatch::dispatch).await
}
