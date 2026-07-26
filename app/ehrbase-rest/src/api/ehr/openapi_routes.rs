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

/// Retrieve an EHR by subject id (`GET /ehr`).
///
/// Matches `subject_id`/`subject_namespace` against the EHR's
/// `EHR_STATUS.subject.external_ref.id.value` and `.namespace`.
#[utoipa::path(
    get, path = "/ehr", tag = "EHR",
    params(
        ("subject_id" = String, Query,
         description = "The EHR subject id (matched against \
                        EHR_STATUS.subject.external_ref.id.value). Required."),
        ("subject_namespace" = String, Query,
         description = "The EHR subject id namespace (matched against \
                        EHR_STATUS.subject.external_ref.namespace). Required.")
    ),
    responses(
        (status = 200, description = "The EHR (canonical JSON/XML per `Accept`); \
                                      `ETag` (weak `W/` form) carries \
                                      `EHR.ehr_id.value`.",
         body = serde_json::Value),
        (status = 400, description = "A required subject query parameter is \
                                      missing or malformed.",
         body = serde_json::Value),
        (status = 404, description = "No EHR exists with the supplied subject \
                                      id and namespace.",
         body = serde_json::Value)
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

/// Create a new EHR with an auto-generated id (`POST /ehr`).
///
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the creating CONTRIBUTION and its `EHR_STATUS`
/// version (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    post, path = "/ehr", tag = "EHR",
    params(
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Committal metadata for the EHR_STATUS VERSION the \
                        creation commits, as an attribute-path list — e.g. \
                        `lifecycle_state.code_string=\"532\"`. Merged with the \
                        server defaults."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the creating CONTRIBUTION, \
                        as an attribute-path list; the header MAY repeat — e.g. \
                        `description.value=\"EHR opened at triage\"`, \
                        `committer.name=\"John Doe\",\
                        committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",\
                        committer.external_ref.namespace=\"demographic\",\
                        committer.external_ref.type=\"PERSON\"`, \
                        `system_id=\"example.openehr.systemid\"`. \
                        `change_type` is constrained to `249|creation|` (a \
                        create commits a first version); `time_committed` is \
                        always server-set, and an omitted `system_id` defaults \
                        to the server's configured identifier.")
    ),
    request_body(content = serde_json::Value,
                 description = "Optional EHR_STATUS for the new EHR; when \
                                omitted a default (is_queryable=true, \
                                is_modifiable=true, PARTY_SELF subject) is used."),
    responses(
        (status = 201, description = "Created; `ETag` (weak `W/` form) carries \
                                      the new `ehr_id`, `Last-Modified` the \
                                      creation instant, `Location` the EHR URL. \
                                      Body per `Prefer` (representation or \
                                      identifier; empty for minimal).",
         body = serde_json::Value),
        (status = 400, description = "The request could not be parsed, or a \
                                      committal `change_type` names a legal \
                                      audit_change_type code that contradicts a \
                                      creation.",
         body = serde_json::Value),
        (status = 409, description = "An EHR already exists for the subject \
                                      id/namespace of the supplied EHR_STATUS.",
         body = serde_json::Value),
        (status = 422, description = "The supplied EHR_STATUS is semantically \
                                      invalid, or a committal \
                                      `change_type`/`lifecycle_state` is not a \
                                      member of its openEHR terminology group.",
         body = serde_json::Value)
    )
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
    params(("ehr_id" = String, Path,
            description = "EHR identifier, taken from EHR.ehr_id.value (a UUID).")),
    responses(
        (status = 200, description = "The EHR (canonical JSON/XML per `Accept`); \
                                      `ETag` (weak `W/` form) carries \
                                      `EHR.ehr_id.value`.",
         body = serde_json::Value),
        (status = 404, description = "No EHR exists with `ehr_id`.",
         body = serde_json::Value)
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
///
/// `ehr_id` must be a valid `HIER_OBJECT_ID` (a UUID is strongly recommended).
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the creating CONTRIBUTION and its `EHR_STATUS`
/// version (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}", tag = "EHR",
    params(
        ("ehr_id" = String, Path,
         description = "The client-supplied EHR id (a valid `HIER_OBJECT_ID`; \
                        a UUID is strongly recommended)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-version" = Option<String>, Header,
         description = "Committal metadata for the EHR_STATUS VERSION the \
                        creation commits, as an attribute-path list — e.g. \
                        `lifecycle_state.code_string=\"532\"`. Merged with the \
                        server defaults."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the creating CONTRIBUTION, \
                        as an attribute-path list; the header MAY repeat — e.g. \
                        `description.value=\"EHR opened at triage\"`, \
                        `committer.name=\"John Doe\",\
                        committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",\
                        committer.external_ref.namespace=\"demographic\",\
                        committer.external_ref.type=\"PERSON\"`, \
                        `system_id=\"example.openehr.systemid\"`. \
                        `change_type` is constrained to `249|creation|` (a \
                        create commits a first version); `time_committed` is \
                        always server-set, and an omitted `system_id` defaults \
                        to the server's configured identifier.")
    ),
    request_body(content = serde_json::Value,
                 description = "Optional EHR_STATUS for the new EHR; when \
                                omitted a default (is_queryable=true, \
                                is_modifiable=true, PARTY_SELF subject) is used."),
    responses(
        (status = 201, description = "Created; `ETag` (weak `W/` form) carries \
                                      the `ehr_id`, `Last-Modified` the creation \
                                      instant, `Location` the EHR URL. Body per \
                                      `Prefer` (representation or identifier; \
                                      empty for minimal).",
         body = serde_json::Value),
        (status = 400, description = "`ehr_id` is not a valid `HIER_OBJECT_ID`, \
                                      the request could not be parsed, or a \
                                      committal `change_type` names a legal \
                                      audit_change_type code that contradicts a \
                                      creation.",
         body = serde_json::Value),
        (status = 409, description = "An EHR already exists with this `ehr_id`.",
         body = serde_json::Value),
        (status = 422, description = "The supplied EHR_STATUS is semantically \
                                      invalid, or a committal \
                                      `change_type`/`lifecycle_state` is not a \
                                      member of its openEHR terminology group.",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value \
                        (an OBJECT_VERSION_ID, e.g. \
                        `…::openEHRSys.example.com::2`).")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS at that version; `ETag` \
                                      (weak `W/` form) carries the version uid, \
                                      `Last-Modified` the commit time. Any \
                                      associated item tags MAY be echoed in the \
                                      `openehr-item-tag`/`openehr-version-item-tag` \
                                      response headers.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no EHR_STATUS version \
                                      with `version_uid`.",
         body = serde_json::Value)
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
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the version \
                        extant at that time is returned. Absent means the \
                        latest version. The timezone is optional — \
                        server-local when omitted.")
    ),
    responses(
        (status = 200, description = "The EHR_STATUS; `ETag` (weak `W/` form) \
                                      carries the version uid, `Last-Modified` \
                                      the commit time. Item tags MAY be echoed \
                                      in the `openehr-item-tag`/\
                                      `openehr-version-item-tag` response headers.",
         body = serde_json::Value),
        (status = 400, description = "Malformed `version_at_time`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no EHR_STATUS version \
                                      at the specified time.",
         body = serde_json::Value)
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
///
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the commit
/// (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/ehr_status", tag = "EHR_STATUS",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("If-Match" = String, Header,
         description = "The latest EHR_STATUS version uid (the \
                        `preceding_version_uid`), double-quoted (weak `W/` \
                        form also accepted). Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_EHR_STATUS \
                        (VERSIONED_OBJECT-level); an empty value removes all. \
                        MAY be echoed back."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new EHR_STATUS VERSION; an empty \
                        value removes all. MAY be echoed back.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new EHR_STATUS."),
    responses(
        (status = 200, description = "Updated; body per `Prefer` \
                                      (representation or identifier). `ETag` \
                                      (weak `W/` form) + `Location` carry the \
                                      new version, `Last-Modified` its commit \
                                      time.",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer: return=minimal`); `ETag` \
                                      (weak `W/` form) + `Location` carry the \
                                      new version, `Last-Modified` its commit \
                                      time."),
        (status = 400, description = "Invalid EHR_STATUS, or `If-Match` expected \
                                      but missing/malformed.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.",
         body = serde_json::Value)
    )
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
    params(("ehr_id" = String, Path,
            description = "EHR identifier, taken from EHR.ehr_id.value (a UUID).")),
    responses(
        (status = 200, description = "The VERSIONED_EHR_STATUS container \
                                      (canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value)
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
    params(("ehr_id" = String, Path,
            description = "EHR identifier, taken from EHR.ehr_id.value (a UUID).")),
    responses(
        (status = 200, description = "The REVISION_HISTORY of the \
                                      VERSIONED_EHR_STATUS (canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value)
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
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the VERSION \
                        extant at that time is returned. Absent means the \
                        latest VERSION. The timezone is optional — \
                        server-local when omitted.")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION of the EHR_STATUS \
                                      (canonical JSON/XML); `ETag` (weak `W/` \
                                      form) carries the version uid, \
                                      `Last-Modified` its `commit_audit` \
                                      `time_committed`.",
         body = serde_json::Value),
        (status = 400, description = "Malformed `version_at_time`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no VERSION at the \
                                      specified time.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value \
                        (an OBJECT_VERSION_ID).")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION of the EHR_STATUS \
                                      identified by `version_uid` (canonical \
                                      JSON/XML); `ETag` (weak `W/` form) carries \
                                      the version uid, `Last-Modified` its \
                                      `commit_audit` `time_committed`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no VERSION with \
                                      `version_uid`.",
         body = serde_json::Value)
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

/// Create the first version of a COMPOSITION
/// (`POST /ehr/{ehr_id}/composition`).
///
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the commit
/// (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/composition", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-template-id" = Option<String>, Header,
         description = "The template id; required when the body is a Simplified \
                        Format (which carries no template_id)."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_COMPOSITION \
                        (VERSIONED_OBJECT-level); an empty value removes all. \
                        MAY be echoed back."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new COMPOSITION VERSION; an \
                        empty value removes all. MAY be echoed back.")
    ),
    request_body(

        // COMPOSITION content negotiates canonical JSON/XML + the two Simplified
        // Formats (Resources.md §Simplified Formats; simplified_formats/master05).
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A COMPOSITION in canonical JSON/XML or a Simplified Format \
                       (the `openehr-template-id` header is required for a simplified body)."
    ),
    responses(
        (
            status = 201, description = "Created; `ETag` (weak `W/` form) carries \
                                        the new version uid, `Last-Modified` its \
                                        commit time, `Location` the COMPOSITION \
                                        version URL. Body per `Prefer` \
                                        (representation or identifier; empty for \
                                        minimal). Item tags MAY be echoed in the \
                                        `openehr-item-tag`/\
                                        `openehr-version-item-tag` response headers.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "The COMPOSITION could not be parsed, or a \
                                      required header/parameter is missing or \
                                      invalid.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 422, description = "The COMPOSITION parsed but failed semantic \
                                      validation (e.g. unknown template, or the \
                                      template does not validate the content).",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "Either an OBJECT_VERSION_ID (VERSION.uid.value — a \
                        specific version) or a HIER_OBJECT_ID \
                        (VERSIONED_OBJECT.uid.value — the version container, \
                        resolving with `version_at_time` or to the latest)."),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; used only when \
                        `uid_based_id` is a HIER_OBJECT_ID. Absent means the \
                        latest version. The timezone is optional — \
                        server-local when omitted.")
    ),
    responses(
        (
            status = 200, description = "The COMPOSITION; `ETag` (weak `W/` form) \
                                        carries the version uid, `Last-Modified` \
                                        the commit time. Item tags MAY be echoed \
                                        in the `openehr-item-tag`/\
                                        `openehr-version-item-tag` response headers.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 204, description = "The COMPOSITION was (logically) deleted at \
                                      the requested time."),
        (status = 400, description = "Malformed `version_at_time`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no COMPOSITION version \
                                      at the specified time.",
         body = serde_json::Value)
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
///
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the commit
/// (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "The HIER_OBJECT_ID (VERSIONED_OBJECT.uid.value) of the \
                        COMPOSITION to update; a body COMPOSITION.uid, if \
                        present, must identify the same versioned object."),
        ("If-Match" = String, Header,
         description = "The latest COMPOSITION version uid (the \
                        `preceding_version_uid`), double-quoted (weak `W/` form \
                        also accepted). Required."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-template-id" = Option<String>, Header,
         description = "The template id; required when the body is a Simplified \
                        Format (which carries no template_id)."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_COMPOSITION \
                        (VERSIONED_OBJECT-level); an empty value removes all. \
                        MAY be echoed back."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new COMPOSITION VERSION; an \
                        empty value removes all. MAY be echoed back.")
    ),
    request_body(

        content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A COMPOSITION in canonical JSON/XML or a Simplified Format \
                       (the `openehr-template-id` header is required for a simplified body)."
    ),
    responses(
        (
            status = 200, description = "Updated; body per `Prefer` \
                                        (representation or identifier). `ETag` \
                                        (weak `W/` form) + `Location` carry the \
                                        new version, `Last-Modified` its commit \
                                        time. Item tags MAY be echoed in \
                                        the `openehr-item-tag`/\
                                        `openehr-version-item-tag` response headers.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 204, description = "Updated (`Prefer: return=minimal`); `ETag` \
                                      (weak `W/` form) + `Location` carry the new \
                                      version, `Last-Modified` its commit time."),
        (status = 400, description = "The COMPOSITION could not be parsed, a body \
                                      uid mismatches the path, or `If-Match` is \
                                      missing/malformed.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not match the latest \
                                      version; `ETag` carries the current latest \
                                      version uid.",
         body = serde_json::Value),
        (status = 422, description = "The COMPOSITION parsed but failed semantic \
                                      validation (e.g. unknown template, or the \
                                      template does not validate the content).",
         body = serde_json::Value)
    )
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
///
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the deletion commit
/// (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "The OBJECT_VERSION_ID (VERSION.uid.value) of the latest \
                        version — the `preceding_version_uid` to delete; a bare \
                        HIER_OBJECT_ID is rejected with 400.")
    ),
    responses(
        (status = 204, description = "Logically deleted (a new deleted version \
                                      is committed); `ETag` carries the deleted \
                                      version uid and `Last-Modified` its commit \
                                      time."),
        (status = 400, description = "`uid_based_id` is not an OBJECT_VERSION_ID, \
                                      or the COMPOSITION is already deleted.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value),
        (status = 409, description = "`uid_based_id` is not the latest version \
                                      (stale); `ETag` carries the current latest \
                                      version uid.",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value (a HIER_OBJECT_ID UUID).")
    ),
    responses(
        (status = 200, description = "The VERSIONED_COMPOSITION container \
                                      (canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `versioned_object_uid`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value (a HIER_OBJECT_ID UUID).")
    ),
    responses(
        (status = 200, description = "The REVISION_HISTORY of the \
                                      VERSIONED_COMPOSITION (canonical JSON/XML).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `versioned_object_uid`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value (a HIER_OBJECT_ID UUID)."),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the VERSION \
                        extant at that time is returned. Absent means the \
                        latest VERSION. The timezone is optional — \
                        server-local when omitted.")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION of the COMPOSITION \
                                      (canonical JSON/XML); `ETag` (weak `W/` \
                                      form) carries the version uid, \
                                      `Last-Modified` its `commit_audit` \
                                      `time_committed`.",
         body = serde_json::Value),
        (status = 400, description = "Malformed `version_at_time`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `versioned_object_uid`, \
                                      or no VERSION at the specified time.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value (a HIER_OBJECT_ID UUID)."),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value \
                        (an OBJECT_VERSION_ID).")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION of the COMPOSITION \
                                      identified by `version_uid` (canonical \
                                      JSON/XML); `ETag` (weak `W/` form) carries \
                                      the version uid, `Last-Modified` its \
                                      `commit_audit` `time_committed`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `versioned_object_uid`, \
                                      or no VERSION with `version_uid`.",
         body = serde_json::Value)
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
                        version. The timezone is optional — server-local when \
                        omitted."),
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
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default), `return=representation`, \
                        or `return=identifier`."),
        ("openehr-template-id" = Option<String>, Header,
         description = "The template id; required when an inner \
                        `versions[].data` uses a Simplified Format (which \
                        carries no template_id).")
    ),
    request_body(

        // The envelope is always canonical JSON; a Simplified media type selects
        // the inner `versions[i].data` COMPOSITION form (contribution_create.yaml
        // §Simplified Formats). No canonical-XML CONTRIBUTION wire shape exists.
        content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json")),
        description = "A CONTRIBUTION (canonical JSON envelope; inner versions[].data \
                       may be a Simplified Format with the `openehr-template-id` header)."
    ),
    responses(
        (
            status = 201, description = "Created; `ETag` (weak `W/` form) carries \
                                        the new `contribution_uid`, `Location` \
                                        the CONTRIBUTION URL. Body per `Prefer` \
                                        (representation or identifier; empty for \
                                        minimal).",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "The CONTRIBUTION could not be parsed or is \
                                      invalid (e.g. a version's modification type \
                                      does not match — a MODIFICATION as first \
                                      version).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 409, description = "A CONTRIBUTION with the supplied `uid` \
                                      already exists.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("offset" = Option<i64>, Query,
         description = "OUR EXTENSION — row offset into the newest-first list \
                        (default 0)."),
        ("fetch" = Option<i64>, Query,
         description = "OUR EXTENSION — maximum rows to return (default 20, \
                        capped at 100).")
    ),
    responses(
        (status = 200, description = "The EHR's CONTRIBUTIONs, newest first: \
                                      `{ rows: [{ uid, time_committed, committer, \
                                      change_type }], total }`.",
         body = serde_json::Value),
        (status = 400, description = "Malformed `offset` or `fetch` query \
                                      parameter.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("contribution_uid" = String, Path,
         description = "The CONTRIBUTION uid (a UUID).")
    ),
    responses(
        (
            status = 200, description = "The CONTRIBUTION (canonical JSON \
                                        envelope; when a Simplified Format is \
                                        requested, only each `versions[].data` \
                                        payload uses that form).",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 404, description = "Unknown `ehr_id`, or no CONTRIBUTION with \
                                      `contribution_uid`.",
         body = serde_json::Value)
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
///
/// Lists every `ITEM_TAG` on any target within the EHR, optionally filtered.
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/tags", tag = "ITEM_TAG",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("tag_key" = Option<String>, Query,
         description = "Filter by ITEM_TAG key; omit to match any."),
        ("tag_value" = Option<String>, Query,
         description = "Filter by ITEM_TAG value; omit to match any."),
        ("tag_target_path" = Option<String>, Query,
         description = "Filter by ITEM_TAG target_path; omit to match any.")
    ),
    responses(
        (status = 200, description = "The matching ITEM_TAG list (empty when none \
                                      match).",
         body = serde_json::Value),
        (status = 400, description = "A filter query parameter is malformed.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (tags of a specific COMPOSITION \
                        version) or a HIER_OBJECT_ID (tags of the \
                        VERSIONED_COMPOSITION container).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAG list for the target (empty \
                                      when none).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; `204 No Content`) or \
                        `return=representation` (`200` with the stored \
                        ITEM_TAG list). `return=identifier` behaves as \
                        `minimal` (a tag list has no single identifier)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (tags of a specific COMPOSITION \
                        version) or a HIER_OBJECT_ID (tags of the \
                        VERSIONED_COMPOSITION container).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to associate with the \
                                target; an empty list removes all tags."),
    responses(
        (status = 200, description = "Updated; the stored ITEM_TAG list is \
                                      returned (`Prefer: \
                                      return=representation`).",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer` missing or \
                                      `return=minimal` — the default; \
                                      `204_updated.yaml`)."),
        (status = 400, description = "The ITEM_TAG list is malformed.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (a specific COMPOSITION version) or \
                        a HIER_OBJECT_ID (the VERSIONED_COMPOSITION container)."),
        ("key" = String, Path,
         description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The matching ITEM_TAG(s) were deleted."),
        (status = 404, description = "Unknown `ehr_id`, `uid_based_id`, or no \
                                      ITEM_TAG with `key`.",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (tags of a specific EHR_STATUS \
                        version) or a HIER_OBJECT_ID (tags of the \
                        VERSIONED_EHR_STATUS container).")
    ),
    responses(
        (status = 200, description = "The ITEM_TAG list for the target (empty \
                                      when none).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value)
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("Prefer" = Option<String>, Header,
         description = "`return=minimal` (default; `204 No Content`) or \
                        `return=representation` (`200` with the stored \
                        ITEM_TAG list). `return=identifier` behaves as \
                        `minimal` (a tag list has no single identifier)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (tags of a specific EHR_STATUS \
                        version) or a HIER_OBJECT_ID (tags of the \
                        VERSIONED_EHR_STATUS container).")
    ),
    request_body(content = serde_json::Value,
                 description = "The full ITEM_TAG list to associate with the \
                                target; an empty list removes all tags."),
    responses(
        (status = 200, description = "Updated; the stored ITEM_TAG list is \
                                      returned (`Prefer: \
                                      return=representation`).",
         body = serde_json::Value),
        (status = 204, description = "Updated (`Prefer` missing or \
                                      `return=minimal` — the default; \
                                      `204_updated.yaml`)."),
        (status = 400, description = "The ITEM_TAG list is malformed.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id` or `uid_based_id`.",
         body = serde_json::Value)
    )
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
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value (a UUID)."),
        ("uid_based_id" = String, Path,
         description = "An OBJECT_VERSION_ID (a specific EHR_STATUS version) or a \
                        HIER_OBJECT_ID (the VERSIONED_EHR_STATUS container)."),
        ("key" = String, Path,
         description = "The ITEM_TAG key to delete.")
    ),
    responses(
        (status = 204, description = "The matching ITEM_TAG(s) were deleted."),
        (status = 404, description = "Unknown `ehr_id`, `uid_based_id`, or no \
                                      ITEM_TAG with `key`.",
         body = serde_json::Value)
    )
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
