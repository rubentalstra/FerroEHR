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
                        EHR_STATUS.subject.external_ref.id.value). Required.",
         example = "ins01"),
        ("subject_namespace" = String, Query,
         description = "The EHR subject id namespace (matched against \
                        EHR_STATUS.subject.external_ref.namespace). Required.",
         example = "demographic")
    ),
    responses(
        (status = 200, description = "The EHR (canonical JSON/XML per `Accept`). \
                                      `ETag` (weak `W/` form) carries \
                                      `EHR.ehr_id.value`. No `Location` and no \
                                      `Last-Modified`: `Requests_and_responses.md` \
                                      §Location forbids `Location` on a `GET` \
                                      (\"It MUST NOT be used to indicate an \
                                      alternate representation of an existing \
                                      resource\"), and the RM `EHR` root is not a \
                                      VERSION, so the §\"ETag and Last-Modified\" \
                                      source `VERSION.commit_audit.time_committed` \
                                      does not exist for it.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<ehr_id>\"` \
                             (`Requests_and_responses.md` §\"ETag and \
                             Last-Modified\": the value \"is usually taken from \
                             e.g. … EHR.ehr_id.value\" and the `W/` weakness \
                             indicator is required since Release 1.1.0)."),
         ),
         example = json!({
             "_type": "EHR",
             "system_id": { "_type": "HIER_OBJECT_ID", "value": "openEHRSys.example.com" },
             "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" },
             "ehr_status": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "VERSIONED_EHR_STATUS",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
             },
             "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
             "ehr_access": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "VERSIONED_EHR_ACCESS",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "1f2a3b4c-5d6e-4f70-8192-a3b4c5d6e7f8" }
             }
         })),
        (status = 400, description = "A required subject query parameter \
                                      (`subject_id`, `subject_namespace`) is \
                                      missing, or a supplied one is malformed \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "No EHR exists with the supplied subject \
                                      id and namespace.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR resource has only the canonical \
                                      `application/json` / `application/xml` \
                                      representations (`Resources.md` §\"XML \
                                      Format\"/§\"JSON Format\": \"If the service \
                                      cannot fulfill this aspect of the request, \
                                      it MUST respond with HTTP status code `406 \
                                      Not Acceptable`\"; the Simplified Formats \
                                      are not defined for a non-templated \
                                      resource).",
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
         description = "Response-verbosity preference \
                        (`Requests_and_responses.md` §\"Representation details \
                        negotiation\"). Exactly one of the three tokens: \
                        `return=minimal` — empty body; `return=identifier` — the \
                        body is only `{ \"uid\": \"<ehr_id>\" }`; \
                        `return=representation` — the full RM `EHR`. An absent \
                        header means `return=minimal` (\"If no `Prefer` header is \
                        provided, the default behavior is assumed to be \
                        `return=minimal`\"); the token actually applied is echoed \
                        in the `Preference-Applied` response header.",
         example = "return=representation"),
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
                                is_modifiable=true, PARTY_SELF subject) is used.",
                 example = json!({
                     "_type": "EHR_STATUS",
                     "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                     "archetype_details": {
                         "_type": "ARCHETYPED",
                         "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                         "rm_version": "1.2.0"
                     },
                     "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                     "subject": {
                         "_type": "PARTY_SELF",
                         "external_ref": {
                             "_type": "PARTY_REF",
                             "namespace": "demographic",
                             "type": "PERSON",
                             "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                         }
                     },
                     "is_queryable": true,
                     "is_modifiable": true
                 })),
    responses(
        (status = 201, description = "Created. `ETag` (weak `W/` form) carries \
                                      the new `ehr_id`, `Last-Modified` the \
                                      creation instant, `Location` the EHR URL, \
                                      and `Preference-Applied` the `Prefer` token \
                                      actually honoured. The body is \
                                      `Prefer`-conditional \
                                      (`Requests_and_responses.md` §\"Prefer \
                                      minimal, identifier or full representation \
                                      response\"): the full RM `EHR` for \
                                      `return=representation` (the \
                                      `representation` example), the single-`uid` \
                                      object for `return=identifier` (the \
                                      `identifier` example), and no body at all \
                                      for the default `return=minimal`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<ehr_id>\"` of the created \
                             EHR (§\"ETag and Last-Modified\")."),
             ("Location" = String,
              description = "The URL of the newly created EHR, \
                             `<base_path>/ehr/<ehr_id>` (§Location: used \"in `201 \
                             Created` responses when a new resource is \
                             successfully created\")."),
             ("Last-Modified" = String,
              description = "The creating CONTRIBUTION's commit instant as an \
                             HTTP-date (§\"ETag and Last-Modified\")."),
             ("Preference-Applied" = String,
              description = "`return=minimal` | `return=identifier` | \
                             `return=representation` — the preference the service \
                             honoured (§\"Representation details negotiation\")."),
         ),
         examples(
             ("representation" = (summary = "Prefer: return=representation — the full RM EHR",
              value = json!({
                  "_type": "EHR",
                  "system_id": { "_type": "HIER_OBJECT_ID", "value": "openEHRSys.example.com" },
                  "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" },
                  "ehr_status": {
                      "_type": "OBJECT_REF",
                      "namespace": "local",
                      "type": "VERSIONED_EHR_STATUS",
                      "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
                  },
                  "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                  "ehr_access": {
                      "_type": "OBJECT_REF",
                      "namespace": "local",
                      "type": "VERSIONED_EHR_ACCESS",
                      "id": { "_type": "HIER_OBJECT_ID", "value": "1f2a3b4c-5d6e-4f70-8192-a3b4c5d6e7f8" }
                  }
              }))),
             ("identifier" = (summary = "Prefer: return=identifier — only the new ehr_id",
              value = json!({ "uid": "7d44b88c-4199-4bad-97dc-d78268e01398" })))
         )),
        (status = 400, description = "The request could not be parsed, or a \
                                      committal `change_type` names a legal \
                                      audit_change_type code that contradicts a \
                                      creation (§\"HTTP status codes\", the `400` \
                                      row: \"malformed request syntax, \
                                      syntactically invalid content\").",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR/EHR_STATUS resources have only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": an unfulfillable `Accept` MUST be \
                                      `406`).",
         body = serde_json::Value),
        (status = 409, description = "An EHR already exists for the subject \
                                      id/namespace of the supplied EHR_STATUS \
                                      (§\"HTTP status codes\", the `409` row: the \
                                      request \"might generate a duplicate or a \
                                      conflict\").",
         body = serde_json::Value),
        (status = 415, description = "The request `Content-Type` is not a format \
                                      this resource can process — notably a \
                                      Simplified Format, which is defined only for \
                                      templated COMPOSITION content \
                                      (`Resources.md` §\"Simplified Formats\": \"If \
                                      the service cannot process the request \
                                      payload as the simplified format is not \
                                      supported, it MUST respond with HTTP status \
                                      code `415 Unsupported Media Type`\"; \
                                      §\"XML Format\"/§\"JSON Format\" carry the \
                                      same MUST for the canonical types).",
         body = serde_json::Value),
        (status = 422, description = "The request was well-formed but cannot be \
                                      followed: the supplied EHR_STATUS is \
                                      semantically invalid, or a committal \
                                      `change_type`/`lifecycle_state` is not a \
                                      member of its openEHR terminology group \
                                      (§\"HTTP status codes\", the `422` row: \
                                      \"The request was well-formed but was unable \
                                      to be followed due to semantic errors\").",
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
            description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                           (`Resources.md` §\"Identifier types\": \"the EHR \
                           identifier is `7d44b88c-4199-4bad-97dc-d78268e01398`, \
                           taken from EHR.ehr_id.value\"; the SM types the \
                           argument `UUID` — `master02-overview.adoc` §\"Functional \
                           Style\", `interface I_EHR_SERVICE { Boolean \
                           has_ehr(UUID an_ehr_id); … }`).",
            example = "7d44b88c-4199-4bad-97dc-d78268e01398")),
    responses(
        (status = 200, description = "The EHR (canonical JSON/XML per `Accept`). \
                                      `ETag` (weak `W/` form) carries \
                                      `EHR.ehr_id.value`. No `Location` and no \
                                      `Last-Modified`: `Requests_and_responses.md` \
                                      §Location forbids `Location` on a `GET`, and \
                                      the RM `EHR` root is not a VERSION, so it has \
                                      no `commit_audit.time_committed` to report.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<ehr_id>\"` \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "EHR",
             "system_id": { "_type": "HIER_OBJECT_ID", "value": "openEHRSys.example.com" },
             "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" },
             "ehr_status": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "VERSIONED_EHR_STATUS",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
             },
             "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
             "ehr_access": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "VERSIONED_EHR_ACCESS",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "1f2a3b4c-5d6e-4f70-8192-a3b4c5d6e7f8" }
             }
         })),
        (status = 400, description = "`ehr_id` is malformed — it is not a UUID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\"). \
                                      A syntactically valid but unknown id is \
                                      `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "No EHR exists with `ehr_id`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR resource has only the canonical \
                                      `application/json` / `application/xml` \
                                      representations (`Resources.md` §\"XML \
                                      Format\"/§\"JSON Format\": an unfulfillable \
                                      `Accept` MUST be `406`).",
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
/// `ehr_id` is a UUID: the SM types the argument `UUID`
/// (`SM/docs/openehr_platform/master02-overview.adoc` §"Functional Style":
/// `UUID create_ehr_with_id(UUID an_ehr_id)`), and ITS-REST identifies an EHR
/// by `EHR.ehr_id.value` in UUID form (`Resources.md` §"Identifier types").
/// Every UUID is a valid `HIER_OBJECT_ID` root, so no released sentence is
/// narrowed by accepting only UUIDs.
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the creating CONTRIBUTION and its `EHR_STATUS`
/// version (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}", tag = "EHR",
    params(
        ("ehr_id" = String, Path,
         description = "The client-supplied EHR id — a UUID, which becomes \
                        `EHR.ehr_id.value` (`Resources.md` §\"Identifier types\"; \
                        the SM argument is typed `UUID` — \
                        `master02-overview.adoc` §\"Functional Style\", `UUID \
                        create_ehr_with_id(UUID an_ehr_id)`). Every UUID is a \
                        valid `HIER_OBJECT_ID` root, so this accepts every id the \
                        released text requires.",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`Requests_and_responses.md` §\"Representation details \
                        negotiation\"). Exactly one of the three tokens: \
                        `return=minimal` — empty body; `return=identifier` — the \
                        body is only `{ \"uid\": \"<ehr_id>\" }`; \
                        `return=representation` — the full RM `EHR`. An absent \
                        header means `return=minimal`; the token actually applied \
                        is echoed in the `Preference-Applied` response header.",
         example = "return=representation"),
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
                                is_modifiable=true, PARTY_SELF subject) is used.",
                 example = json!({
                     "_type": "EHR_STATUS",
                     "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                     "archetype_details": {
                         "_type": "ARCHETYPED",
                         "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                         "rm_version": "1.2.0"
                     },
                     "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                     "subject": {
                         "_type": "PARTY_SELF",
                         "external_ref": {
                             "_type": "PARTY_REF",
                             "namespace": "demographic",
                             "type": "PERSON",
                             "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                         }
                     },
                     "is_queryable": true,
                     "is_modifiable": true
                 })),
    responses(
        (status = 201, description = "Created. `ETag` (weak `W/` form) carries \
                                      the `ehr_id`, `Last-Modified` the creation \
                                      instant, `Location` the EHR URL, and \
                                      `Preference-Applied` the `Prefer` token \
                                      actually honoured. The body is \
                                      `Prefer`-conditional \
                                      (`Requests_and_responses.md` §\"Prefer \
                                      minimal, identifier or full representation \
                                      response\"): the full RM `EHR` for \
                                      `return=representation` (the \
                                      `representation` example), the single-`uid` \
                                      object for `return=identifier` (the \
                                      `identifier` example), and no body at all \
                                      for the default `return=minimal`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<ehr_id>\"` of the created \
                             EHR (§\"ETag and Last-Modified\")."),
             ("Location" = String,
              description = "The URL of the newly created EHR, \
                             `<base_path>/ehr/<ehr_id>` (§Location: used \"in `201 \
                             Created` responses when a new resource is \
                             successfully created\")."),
             ("Last-Modified" = String,
              description = "The creating CONTRIBUTION's commit instant as an \
                             HTTP-date (§\"ETag and Last-Modified\")."),
             ("Preference-Applied" = String,
              description = "`return=minimal` | `return=identifier` | \
                             `return=representation` — the preference the service \
                             honoured (§\"Representation details negotiation\")."),
         ),
         examples(
             ("representation" = (summary = "Prefer: return=representation — the full RM EHR",
              value = json!({
                  "_type": "EHR",
                  "system_id": { "_type": "HIER_OBJECT_ID", "value": "openEHRSys.example.com" },
                  "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" },
                  "ehr_status": {
                      "_type": "OBJECT_REF",
                      "namespace": "local",
                      "type": "VERSIONED_EHR_STATUS",
                      "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
                  },
                  "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                  "ehr_access": {
                      "_type": "OBJECT_REF",
                      "namespace": "local",
                      "type": "VERSIONED_EHR_ACCESS",
                      "id": { "_type": "HIER_OBJECT_ID", "value": "1f2a3b4c-5d6e-4f70-8192-a3b4c5d6e7f8" }
                  }
              }))),
             ("identifier" = (summary = "Prefer: return=identifier — only the ehr_id",
              value = json!({ "uid": "7d44b88c-4199-4bad-97dc-d78268e01398" })))
         )),
        (status = 400, description = "`ehr_id` is not a UUID, the request could \
                                      not be parsed, or a committal `change_type` \
                                      names a legal audit_change_type code that \
                                      contradicts a creation (§\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR/EHR_STATUS resources have only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": an unfulfillable `Accept` MUST be \
                                      `406`).",
         body = serde_json::Value),
        (status = 409, description = "An EHR already exists with this `ehr_id` \
                                      (§\"HTTP status codes\", the `409` row: the \
                                      request \"might generate a duplicate or a \
                                      conflict\"). Also returned when the supplied \
                                      EHR_STATUS names a subject that already owns \
                                      an EHR.",
         body = serde_json::Value),
        (status = 415, description = "The request `Content-Type` is not a format \
                                      this resource can process — notably a \
                                      Simplified Format, which is defined only for \
                                      templated COMPOSITION content \
                                      (`Resources.md` §\"Simplified Formats\": \"If \
                                      the service cannot process the request \
                                      payload as the simplified format is not \
                                      supported, it MUST respond with HTTP status \
                                      code `415 Unsupported Media Type`\"; \
                                      §\"XML Format\"/§\"JSON Format\" carry the \
                                      same MUST for the canonical types).",
         body = serde_json::Value),
        (status = 422, description = "The request was well-formed but cannot be \
                                      followed: the supplied EHR_STATUS is \
                                      semantically invalid, or a committal \
                                      `change_type`/`lifecycle_state` is not a \
                                      member of its openEHR terminology group \
                                      (§\"HTTP status codes\", the `422` row: \
                                      \"The request was well-formed but was unable \
                                      to be followed due to semantic errors\").",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value — an \
                        OBJECT_VERSION_ID \
                        `{object_id}::{creating_system_id}::{version_tree_id}` \
                        (`Resources.md` §\"Identifier types\"). The addressed \
                        uid must name the served version's full three-part \
                        identity; a fabricated `creating_system_id` names no \
                        VERSION here and is `404`.",
         example = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2")
    ),
    responses(
        (status = 200, description = "The bare EHR_STATUS at that version \
                                      (canonical JSON/XML per `Accept`; the \
                                      VERSION envelope is served by the \
                                      `versioned_ehr_status` operations). No \
                                      `Location`: `Requests_and_responses.md` \
                                      §Location forbids it on a `GET` (\"It MUST \
                                      NOT be used to indicate an alternate \
                                      representation of an existing resource\"). \
                                      The `openehr-item-tag`/\
                                      `openehr-version-item-tag` response headers \
                                      are a MAY on reads (§\"openehr-item-tag and \
                                      openehr-version-item-tag\", \"Usage in \
                                      Responses\"): this server emits them only \
                                      on the `PUT` write wrapper, never on a \
                                      `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\"; the `W/` \
                             weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "That version's commit instant as an HTTP-date — \
                             \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\"). The bare EHR_STATUS \
                             body carries no commit audit, so the instant comes \
                             from the version metadata."),
         ),
         example = json!({
             "_type": "EHR_STATUS",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
             "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
             "archetype_details": {
                 "_type": "ARCHETYPED",
                 "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                 "rm_version": "1.2.0"
             },
             "name": { "_type": "DV_TEXT", "value": "EHR Status" },
             "subject": {
                 "_type": "PARTY_SELF",
                 "external_ref": {
                     "_type": "PARTY_REF",
                     "namespace": "demographic",
                     "type": "PERSON",
                     "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                 }
             },
             "is_queryable": true,
             "is_modifiable": true
         })),
        (status = 400, description = "`ehr_id` is not a UUID, or `version_uid` is \
                                      not a well-formed OBJECT_VERSION_ID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\"). \
                                      A syntactically valid but unknown id is \
                                      `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no EHR_STATUS version \
                                      with `version_uid`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR_STATUS resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      defined for templated COMPOSITION content \
                                      only, and EHR_STATUS is not templated).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the version \
                        extant at that time is returned. Absent means the \
                        latest version. The timezone is optional — server-local \
                        when omitted (`Resources.md` §\"Datetime format\": \
                        query parameters \"MUST always use the _extended_ ISO \
                        8601 format\" and \"Timezone SHOULD be only supplied \
                        when needed, otherwise the local timezone is \
                        assumed\").",
         example = "2026-07-26T09:12:44.512Z")
    ),
    responses(
        (status = 200, description = "The bare EHR_STATUS extant at that time \
                                      (canonical JSON/XML per `Accept`). No \
                                      `Location`: `Requests_and_responses.md` \
                                      §Location forbids it on a `GET`. The \
                                      `openehr-item-tag`/\
                                      `openehr-version-item-tag` response headers \
                                      are a MAY on reads (§\"openehr-item-tag and \
                                      openehr-version-item-tag\", \"Usage in \
                                      Responses\"): this server emits them only \
                                      on the `PUT` write wrapper, never on a \
                                      `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` of the \
                             version served (§\"ETag and Last-Modified\"; the \
                             `W/` weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "That version's commit instant as an HTTP-date — \
                             \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\"). The bare EHR_STATUS \
                             body carries no commit audit, so the instant comes \
                             from the version metadata."),
         ),
         example = json!({
             "_type": "EHR_STATUS",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
             "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
             "archetype_details": {
                 "_type": "ARCHETYPED",
                 "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                 "rm_version": "1.2.0"
             },
             "name": { "_type": "DV_TEXT", "value": "EHR Status" },
             "subject": {
                 "_type": "PARTY_SELF",
                 "external_ref": {
                     "_type": "PARTY_REF",
                     "namespace": "demographic",
                     "type": "PERSON",
                     "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                 }
             },
             "is_queryable": true,
             "is_modifiable": true
         })),
        (status = 400, description = "`ehr_id` is not a UUID, or \
                                      `version_at_time` is not an extended ISO \
                                      8601 datetime (`Requests_and_responses.md` \
                                      §\"HTTP status codes\", the `400` row: \
                                      \"malformed request syntax, syntactically \
                                      invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no EHR_STATUS version \
                                      at the specified time.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR_STATUS resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      defined for templated COMPOSITION content \
                                      only, and EHR_STATUS is not templated).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("If-Match" = String, Header,
         description = "The latest EHR_STATUS version uid (which becomes the new \
                        version's `preceding_version_uid`), double-quoted; the \
                        weak `W/\"…\"` form the server emits is also accepted. \
                        Required — `Requests_and_responses.md` §\"If-Match and \
                        accidental overwrites\": \"When the service expects \
                        `If-Match` for an operation, but the client does not \
                        provide it, the service SHOULD respond with `400 Bad \
                        Request`\"; a non-matching value MUST be `412`.",
         example = "W/\"8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1\""),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`Requests_and_responses.md` §\"Representation details \
                        negotiation\"). Exactly one of the three tokens: \
                        `return=minimal` — no body, `204 No Content`; \
                        `return=identifier` — the body is only \
                        `{ \"uid\": \"<new version uid>\" }` at `200 OK`, never \
                        `204` (§\"Prefer only identifier\": \"a variant of \
                        preference that implies minimal response semantics, but \
                        with a non-empty response body\"); \
                        `return=representation` — the full RM `EHR_STATUS` at \
                        `200 OK`. An absent header means `return=minimal` (\"If \
                        no `Prefer` header is provided, the default behavior is \
                        assumed to be `return=minimal`\"); the token actually \
                        applied is echoed in the `Preference-Applied` response \
                        header.",
         example = "return=representation"),
        ("openehr-version" = Option<String>, Header,
         description = "Committal metadata for the new EHR_STATUS VERSION, as an \
                        attribute-path list — e.g. \
                        `lifecycle_state.code_string=\"532\"`. Merged with the \
                        server defaults (`Requests_and_responses.md` \
                        §\"openehr-version and openehr-audit-details\": whatever \
                        is provided \"MUST be merged with the default VERSION and \
                        VERSION.audit_details attributes on commit runtime\")."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the CONTRIBUTION this update \
                        commits, as an attribute-path list; the header MAY repeat \
                        — e.g. `description.value=\"Status corrected at \
                        triage\"`, `committer.name=\"John Doe\",\
                        committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",\
                        committer.external_ref.namespace=\"demographic\",\
                        committer.external_ref.type=\"PERSON\"`, \
                        `system_id=\"example.openehr.systemid\"`. \
                        `change_type` defaults to `251|modification|` and is \
                        client-overridable to any audit_change_type code \
                        consistent with an update (e.g. `250|amendment|`); \
                        `time_committed` \"is always set by the server\", and an \
                        omitted `system_id` defaults to the server's configured \
                        identifier (\"when `system_id` is not provided by the \
                        client, the server MUST set it to its own configured \
                        system identifier\")."),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_EHR_STATUS \
                        (VERSIONED_OBJECT-level target); an empty value removes \
                        all (`Requests_and_responses.md` §\"openehr-item-tag and \
                        openehr-version-item-tag\", \"Usage in Requests\"). MAY \
                        be echoed back in the response header of the same name."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new EHR_STATUS VERSION; an empty \
                        value removes all (same section, \"Usage in Requests\"). \
                        MAY be echoed back in the response header of the same \
                        name.")
    ),
    request_body(content = serde_json::Value,
                 description = "The new EHR_STATUS, canonical JSON or XML per \
                                `Content-Type`. The Simplified Formats do not \
                                apply — EHR_STATUS is not templated.",
                 example = json!({
                     "_type": "EHR_STATUS",
                     "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                     "archetype_details": {
                         "_type": "ARCHETYPED",
                         "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                         "rm_version": "1.2.0"
                     },
                     "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                     "subject": {
                         "_type": "PARTY_SELF",
                         "external_ref": {
                             "_type": "PARTY_REF",
                             "namespace": "demographic",
                             "type": "PERSON",
                             "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                         }
                     },
                     "is_queryable": true,
                     "is_modifiable": false
                 })),
    responses(
        (status = 200, description = "Updated, with a body: the full RM \
                                      `EHR_STATUS` for \
                                      `Prefer: return=representation` (the \
                                      `representation` example) or the \
                                      single-`uid` object for \
                                      `return=identifier` (the `identifier` \
                                      example) — `Requests_and_responses.md` \
                                      §\"Prefer minimal, identifier or full \
                                      representation response\". `ETag`, \
                                      `Last-Modified` and `Location` describe the \
                                      newly committed version, and \
                                      `Preference-Applied` declares the token \
                                      honoured.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<new version uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\" and \"changes \
                             as soon as the resource changes\")."),
             ("Location" = String,
              description = "The URL of the newly committed version, \
                             `<base_path>/ehr/<ehr_id>/ehr_status/<version_uid>` \
                             (§\"Prefer minimal, identifier or full \
                             representation response\": the response \"SHOULD \
                             include a `Location` header pointing to the newly \
                             created or updated resource\")."),
             ("Last-Modified" = String,
              description = "The commit instant of the new version as an \
                             HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
             ("Preference-Applied" = String,
              description = "`return=minimal` | `return=identifier` | \
                             `return=representation` — the preference the service \
                             honoured (§\"Representation details \
                             negotiation\")."),
             ("openehr-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the \
                             VERSIONED_EHR_STATUS, present only when the request \
                             carried the header (§\"openehr-item-tag and \
                             openehr-version-item-tag\", \"Usage in Responses\": \
                             servers \"MAY include\" it \"to confirm the actual \
                             list of ITEM_TAGs stored on the server side\")."),
             ("openehr-version-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the new \
                             EHR_STATUS VERSION, present only when the request \
                             carried the header (same section, \"Usage in \
                             Responses\")."),
         ),
         examples(
             ("representation" = (summary = "Prefer: return=representation — the full RM EHR_STATUS",
              value = json!({
                  "_type": "EHR_STATUS",
                  "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
                  "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                  "archetype_details": {
                      "_type": "ARCHETYPED",
                      "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                      "rm_version": "1.2.0"
                  },
                  "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                  "subject": {
                      "_type": "PARTY_SELF",
                      "external_ref": {
                          "_type": "PARTY_REF",
                          "namespace": "demographic",
                          "type": "PERSON",
                          "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                      }
                  },
                  "is_queryable": true,
                  "is_modifiable": false
              }))),
             ("identifier" = (summary = "Prefer: return=identifier — only the new version uid",
              value = json!({ "uid": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" })))
         )),
        (status = 204, description = "Updated with no body — the default \
                                      `Prefer: return=minimal` \
                                      (`Requests_and_responses.md` §\"Prefer \
                                      minimal, identifier or full representation \
                                      response\": \"If no response body is \
                                      returned, the service SHOULD use `204 No \
                                      Content`\"). The version headers are \
                                      carried exactly as on the `200`.",
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<new version uid>\"` \
                             (§\"ETag and Last-Modified\")."),
             ("Location" = String,
              description = "The URL of the newly committed version, \
                             `<base_path>/ehr/<ehr_id>/ehr_status/<version_uid>` \
                             (§\"Prefer minimal, identifier or full \
                             representation response\")."),
             ("Last-Modified" = String,
              description = "The commit instant of the new version as an \
                             HTTP-date (§\"ETag and Last-Modified\")."),
             ("Preference-Applied" = String,
              description = "`return=minimal` — the preference the service \
                             honoured (§\"Representation details \
                             negotiation\")."),
             ("openehr-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the \
                             VERSIONED_EHR_STATUS, present only when the request \
                             carried the header (§\"openehr-item-tag and \
                             openehr-version-item-tag\", \"Usage in \
                             Responses\")."),
             ("openehr-version-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the new \
                             EHR_STATUS VERSION, present only when the request \
                             carried the header (same section, \"Usage in \
                             Responses\")."),
         )),
        (status = 400, description = "`ehr_id` is not a UUID, the EHR_STATUS \
                                      payload could not be parsed, or `If-Match` \
                                      is missing/empty/not a well-formed \
                                      OBJECT_VERSION_ID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\"; \
                                      §\"If-Match and accidental overwrites\" for \
                                      the missing-`If-Match` case).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the EHR_STATUS resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      defined for templated COMPOSITION content \
                                      only, and EHR_STATUS is not templated).",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not name the latest \
                                      EHR_STATUS version, so the update was not \
                                      performed (`Requests_and_responses.md` \
                                      §\"If-Match and accidental overwrites\": \
                                      the service \"MUST NOT perform the \
                                      requested method\" and \"MUST respond with \
                                      HTTP status code `412 Precondition \
                                      Failed`\").",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<current latest version \
                             uid>\"` — the same section's SHOULD: the service \
                             \"SHOULD return also latest `version_uid` in the \
                             `ETag` response headers\"."),
             ("Last-Modified" = String,
              description = "The commit instant of that current latest version \
                             as an HTTP-date, carried alongside the `ETag` \
                             (§\"ETag and Last-Modified\")."),
         )),
        (status = 415, description = "The request `Content-Type` is not a format \
                                      this resource can process — notably a \
                                      Simplified Format, which is defined only \
                                      for templated COMPOSITION content \
                                      (`Resources.md` §\"JSON Format\": \"If the \
                                      service cannot process the request payload \
                                      as JSON format, it MUST respond with HTTP \
                                      status code `415 Unsupported Media Type`\"; \
                                      §\"Simplified Formats\" carries the same \
                                      MUST for the simplified types).",
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
            description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                           (`Resources.md` §\"Identifier types\").",
            example = "7d44b88c-4199-4bad-97dc-d78268e01398")),
    responses(
        (status = 200, description = "The VERSIONED_EHR_STATUS container \
                                      (canonical JSON/XML per `Accept`): the \
                                      versioned-object identity and its \
                                      `owner_id`/`time_created`, not the version \
                                      content. No `Location`: \
                                      `Requests_and_responses.md` §Location \
                                      forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag \
                             `W/\"<versioned_object_uid>\"` (§\"ETag and \
                             Last-Modified\": the value \"is usually taken from \
                             e.g. VERSIONED_OBJECT.uid.value\"; the `W/` weakness \
                             indicator is required since Release 1.1.0)."),
             ("Last-Modified" = String,
              description = "The commit instant of the container's most recent \
                             version as an HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\": both headers \"SHOULD \
                             be included in responses for VERSION, \
                             VERSIONED_OBJECT, or other resources that have \
                             versioning or unique state identifiers\")."),
         ),
         example = json!({
             "_type": "VERSIONED_EHR_STATUS",
             "uid": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" },
             "owner_id": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "EHR",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" }
             },
             "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" }
         })),
        (status = 400, description = "`ehr_id` is not a UUID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the VERSIONED_EHR_STATUS container has only \
                                      the canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      not defined for this resource).",
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
            description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                           (`Resources.md` §\"Identifier types\").",
            example = "7d44b88c-4199-4bad-97dc-d78268e01398")),
    responses(
        (status = 200, description = "The REVISION_HISTORY of the \
                                      VERSIONED_EHR_STATUS (canonical JSON/XML \
                                      per `Accept`): one REVISION_HISTORY_ITEM \
                                      per committed version, most recent LAST \
                                      (RM common \
                                      `org.openehr.rm.common.revision_history.adoc`, \
                                      `REVISION_HISTORY.items`: \"The items in \
                                      this history in most-recent-last \
                                      order\"). No `Location`: \
                                      `Requests_and_responses.md` §Location \
                                      forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag \
                             `W/\"<versioned_object_uid>\"` of the container the \
                             history belongs to (§\"ETag and Last-Modified\": the \
                             value \"is usually taken from e.g. \
                             VERSIONED_OBJECT.uid.value\")."),
             ("Last-Modified" = String,
              description = "The commit instant of the most recent revision as \
                             an HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "REVISION_HISTORY",
             "items": [
                 {
                     "_type": "REVISION_HISTORY_ITEM",
                     "version_id": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1" },
                     "audits": [ {
                         "_type": "AUDIT_DETAILS",
                         "system_id": "openEHRSys.example.com",
                         "committer": { "_type": "PARTY_IDENTIFIED", "name": "John Doe" },
                         "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                         "change_type": {
                             "_type": "DV_CODED_TEXT",
                             "value": "creation",
                             "defining_code": {
                                 "_type": "CODE_PHRASE",
                                 "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                                 "code_string": "249"
                             }
                         }
                     } ]
                 },
                 {
                     "_type": "REVISION_HISTORY_ITEM",
                     "version_id": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
                     "audits": [ {
                         "_type": "AUDIT_DETAILS",
                         "system_id": "openEHRSys.example.com",
                         "committer": { "_type": "PARTY_IDENTIFIED", "name": "John Doe" },
                         "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                         "change_type": {
                             "_type": "DV_CODED_TEXT",
                             "value": "modification",
                             "defining_code": {
                                 "_type": "CODE_PHRASE",
                                 "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                                 "code_string": "251"
                             }
                         }
                     } ]
                 }
             ]
         })),
        (status = 400, description = "`ehr_id` is not a UUID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the REVISION_HISTORY resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      not defined for this resource).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the VERSION \
                        extant at that time is returned. Absent means the \
                        latest VERSION. The timezone is optional — server-local \
                        when omitted (`Resources.md` §\"Datetime format\": query \
                        parameters \"MUST always use the _extended_ ISO 8601 \
                        format\" and \"Timezone SHOULD be only supplied when \
                        needed, otherwise the local timezone is assumed\").",
         example = "2026-07-26T09:12:44.512Z")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION envelope of the \
                                      EHR_STATUS extant at that time (canonical \
                                      JSON/XML per `Accept`): the version \
                                      identity, its CONTRIBUTION reference, the \
                                      commit audit, the lifecycle state, and the \
                                      EHR_STATUS itself under `data`. No \
                                      `Location`: `Requests_and_responses.md` \
                                      §Location forbids it on a `GET` (\"It MUST \
                                      NOT be used to indicate an alternate \
                                      representation of an existing \
                                      resource\").",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\"; the `W/` \
                             weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "The version's own \
                             `commit_audit.time_committed` as an HTTP-date — \
                             \"For openEHR resources, this value should be \
                             derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "ORIGINAL_VERSION",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
             "preceding_version_uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1" },
             "contribution": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "CONTRIBUTION",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "0826851c-c4c2-4d61-92b9-410fb8275ff0" }
             },
             "commit_audit": {
                 "_type": "AUDIT_DETAILS",
                 "system_id": "openEHRSys.example.com",
                 "committer": { "_type": "PARTY_IDENTIFIED", "name": "John Doe" },
                 "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                 "change_type": {
                     "_type": "DV_CODED_TEXT",
                     "value": "modification",
                     "defining_code": {
                         "_type": "CODE_PHRASE",
                         "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                         "code_string": "251"
                     }
                 }
             },
             "lifecycle_state": {
                 "_type": "DV_CODED_TEXT",
                 "value": "complete",
                 "defining_code": {
                     "_type": "CODE_PHRASE",
                     "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                     "code_string": "532"
                 }
             },
             "data": {
                 "_type": "EHR_STATUS",
                 "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
                 "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                 "archetype_details": {
                     "_type": "ARCHETYPED",
                     "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                     "rm_version": "1.2.0"
                 },
                 "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                 "subject": {
                     "_type": "PARTY_SELF",
                     "external_ref": {
                         "_type": "PARTY_REF",
                         "namespace": "demographic",
                         "type": "PERSON",
                         "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                     }
                 },
                 "is_queryable": true,
                 "is_modifiable": true
             }
         })),
        (status = 400, description = "`ehr_id` is not a UUID, or \
                                      `version_at_time` is not an extended ISO \
                                      8601 datetime (`Requests_and_responses.md` \
                                      §\"HTTP status codes\", the `400` row: \
                                      \"malformed request syntax, syntactically \
                                      invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no VERSION at the \
                                      specified time.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the ORIGINAL_VERSION resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      not defined for this resource).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value — an \
                        OBJECT_VERSION_ID \
                        `{object_id}::{creating_system_id}::{version_tree_id}` \
                        (`Resources.md` §\"Identifier types\"). The addressed \
                        uid must name the served version's full three-part \
                        identity; a fabricated `creating_system_id` names no \
                        VERSION here and is `404`.",
         example = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION envelope of the \
                                      EHR_STATUS identified by `version_uid` \
                                      (canonical JSON/XML per `Accept`): the \
                                      version identity, its CONTRIBUTION \
                                      reference, the commit audit, the lifecycle \
                                      state, and the EHR_STATUS itself under \
                                      `data`. No `Location`: \
                                      `Requests_and_responses.md` §Location \
                                      forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\"; the `W/` \
                             weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "The version's own \
                             `commit_audit.time_committed` as an HTTP-date — \
                             \"For openEHR resources, this value should be \
                             derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "ORIGINAL_VERSION",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
             "preceding_version_uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1" },
             "contribution": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "CONTRIBUTION",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "0826851c-c4c2-4d61-92b9-410fb8275ff0" }
             },
             "commit_audit": {
                 "_type": "AUDIT_DETAILS",
                 "system_id": "openEHRSys.example.com",
                 "committer": { "_type": "PARTY_IDENTIFIED", "name": "John Doe" },
                 "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                 "change_type": {
                     "_type": "DV_CODED_TEXT",
                     "value": "modification",
                     "defining_code": {
                         "_type": "CODE_PHRASE",
                         "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                         "code_string": "251"
                     }
                 }
             },
             "lifecycle_state": {
                 "_type": "DV_CODED_TEXT",
                 "value": "complete",
                 "defining_code": {
                     "_type": "CODE_PHRASE",
                     "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                     "code_string": "532"
                 }
             },
             "data": {
                 "_type": "EHR_STATUS",
                 "uid": { "_type": "OBJECT_VERSION_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2" },
                 "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                 "archetype_details": {
                     "_type": "ARCHETYPED",
                     "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                     "rm_version": "1.2.0"
                 },
                 "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                 "subject": {
                     "_type": "PARTY_SELF",
                     "external_ref": {
                         "_type": "PARTY_REF",
                         "namespace": "demographic",
                         "type": "PERSON",
                         "id": { "_type": "GENERIC_ID", "value": "ins01", "scheme": "demographic" }
                     }
                 },
                 "is_queryable": true,
                 "is_modifiable": true
             }
         })),
        (status = 400, description = "`ehr_id` is not a UUID, or `version_uid` is \
                                      not a well-formed OBJECT_VERSION_ID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\"). \
                                      A syntactically valid but unknown id is \
                                      `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no VERSION with \
                                      `version_uid`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the ORIGINAL_VERSION resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      not defined for this resource).",
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
/// accepted and merged into the creating CONTRIBUTION and its COMPOSITION
/// version (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    post, path = "/ehr/{ehr_id}/composition", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`Requests_and_responses.md` §\"Representation details \
                        negotiation\"). Exactly one of the three tokens: \
                        `return=minimal` — empty body; `return=identifier` — the \
                        body is only `{ \"uid\": \"<new version uid>\" }`; \
                        `return=representation` — the committed COMPOSITION. An \
                        absent header means `return=minimal` (\"If no `Prefer` \
                        header is provided, the default behavior is assumed to be \
                        `return=minimal`\"); the token actually applied is echoed \
                        in the `Preference-Applied` response header. A \
                        Simplified-Format `Accept` always answers with the \
                        committed COMPOSITION in that form — the `Accept` decides \
                        the body there, not `Prefer` — so the applied preference \
                        is then `return=representation` whatever was asked for.",
         example = "return=representation"),
        ("openehr-version" = Option<String>, Header,
         description = "Committal metadata for the COMPOSITION VERSION this \
                        creation commits, as an attribute-path list — e.g. \
                        `lifecycle_state.code_string=\"553\"`, which commits the \
                        version as `553|incomplete|` and relaxes the template's \
                        lower-cardinality limits (RM common master06 §\"Version \
                        Lifecycle\"). The default is `532|complete|`. Merged with \
                        the server defaults (`Requests_and_responses.md` \
                        §\"openehr-version and openehr-audit-details\": whatever \
                        is provided \"MUST be merged with the default VERSION and \
                        VERSION.audit_details attributes on commit runtime\")."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the CONTRIBUTION this \
                        creation commits, as an attribute-path list; the header \
                        MAY repeat — e.g. `description.value=\"Encounter \
                        recorded at triage\"`, `committer.name=\"John Doe\",\
                        committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",\
                        committer.external_ref.namespace=\"demographic\",\
                        committer.external_ref.type=\"PERSON\"`, \
                        `system_id=\"example.openehr.systemid\"`. \
                        `change_type` is constrained to `249|creation|` (a \
                        create commits a first version — RM common master06 \
                        §Contributions); `time_committed` \"is always set by the \
                        server\", and an omitted `system_id` defaults to the \
                        server's configured identifier (\"when `system_id` is not \
                        provided by the client, the server MUST set it to its own \
                        configured system identifier\")."),
        ("openehr-template-id" = Option<String>, Header,
         description = "The operational-template id the committed content is \
                        validated against. `Requests_and_responses.md` \
                        §openehr-template-id: the header \"MUST be used whenever \
                        committing COMPOSITION (via `PUT` or `POST` methods) \
                        using a Simplified Format which does not support \
                        TEMPLATE_ID value under an equivalent \
                        `LOCATABLE.archetype_details.template_id` attribute of \
                        contained data\". A canonical JSON/XML body carries its \
                        own `archetype_details.template_id`, so the header is not \
                        needed there; a `application/openehr.wt.flat+json` / \
                        `…structured+json` commit without it cannot be resolved \
                        to a template and is `422` (`Requests_and_responses.md` \
                        §\"HTTP status codes\", the `422` row — the released text \
                        assigns no status to the missing header, so the code \
                        choice is ours).",
         example = "problem_list.v1"),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_COMPOSITION \
                        (VERSIONED_OBJECT-level target); an empty value removes \
                        all (`Requests_and_responses.md` §\"openehr-item-tag and \
                        openehr-version-item-tag\", \"Usage in Requests\"). MAY \
                        be echoed back in the response header of the same name."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new COMPOSITION VERSION; an empty \
                        value removes all (same section, \"Usage in Requests\"). \
                        MAY be echoed back in the response header of the same \
                        name.")
    ),
    request_body(
        // COMPOSITION content negotiates canonical JSON/XML + the two Simplified
        // Formats (`Resources.md` §"Data representation": a service "MUST
        // support at least one of the openEHR XML or JSON canonical formats" and
        // the Simplified Formats "SHOULD be supported"; §"Simplified Formats"
        // fixes the two media types below).
        content(
            (serde_json::Value = "application/json", example = json!({
                "_type": "COMPOSITION",
                "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                "archetype_details": {
                    "_type": "ARCHETYPED",
                    "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                    "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                    "rm_version": "1.2.0"
                },
                "name": { "_type": "DV_TEXT", "value": "Encounter" },
                "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                "category": {
                    "_type": "DV_CODED_TEXT", "value": "event",
                    "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                },
                "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                "context": {
                    "_type": "EVENT_CONTEXT",
                    "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                    "setting": {
                        "_type": "DV_CODED_TEXT", "value": "other care",
                        "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                    }
                },
                "content": [ {
                    "_type": "EVALUATION",
                    "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                    "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                    "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                    "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                    "subject": { "_type": "PARTY_SELF" },
                    "data": {
                        "_type": "ITEM_TREE",
                        "archetype_node_id": "at0001",
                        "name": { "_type": "DV_TEXT", "value": "Tree" },
                        "items": [ {
                            "_type": "ELEMENT",
                            "archetype_node_id": "at0002",
                            "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                            "value": { "_type": "DV_TEXT", "value": "Asthma" }
                        } ]
                    }
                } ]
            })),
            (serde_json::Value = "application/xml"),
            (serde_json::Value = "application/openehr.wt.flat+json"),
            (serde_json::Value = "application/openehr.wt.structured+json")
        ),
        description = "The COMPOSITION, in canonical JSON/XML or one of the two \
                       Simplified Formats per `Content-Type` (`Resources.md` \
                       §\"Simplified Formats\": \
                       `application/openehr.wt.flat+json` for Simplified Flat \
                       JSON, `application/openehr.wt.structured+json` for \
                       Simplified Structured JSON). The example is canonical \
                       JSON; a simplified body additionally requires the \
                       `openehr-template-id` request header."
    ),
    responses(
        (
            status = 201, description = "Created. `ETag` (weak `W/` form) carries \
                                        the new version uid, `Last-Modified` its \
                                        commit instant, `Location` the COMPOSITION \
                                        version URL, and `Preference-Applied` the \
                                        `Prefer` token actually honoured. The body \
                                        is `Prefer`-conditional \
                                        (`Requests_and_responses.md` §\"Prefer \
                                        minimal, identifier or full representation \
                                        response\"): the committed COMPOSITION for \
                                        `return=representation` (the \
                                        `representation` example), the \
                                        single-`uid` object for \
                                        `return=identifier` (the `identifier` \
                                        example), and no body at all for the \
                                        default `return=minimal`. The response \
                                        representation is negotiated across the \
                                        same four media types as the request.",
            headers(
                ("ETag" = String,
                 description = "The weak entity tag `W/\"<new version uid>\"` \
                                (§\"ETag and Last-Modified\": the value \"is \
                                usually taken from e.g. … VERSION.uid.value\"; the \
                                `W/` weakness indicator is required since Release \
                                1.1.0)."),
                ("Location" = String,
                 description = "The URL of the newly created COMPOSITION version, \
                                `<base_path>/ehr/<ehr_id>/composition/<version_uid>` \
                                (§Location: used \"in `201 Created` responses when \
                                a new resource is successfully created\")."),
                ("Last-Modified" = String,
                 description = "The creating CONTRIBUTION's commit instant as an \
                                HTTP-date — \"derived from \
                                VERSION.commit_audit.time_committed.value\" \
                                (§\"ETag and Last-Modified\")."),
                ("Preference-Applied" = String,
                 description = "`return=minimal` | `return=identifier` | \
                                `return=representation` — the preference the \
                                service honoured (§\"Representation details \
                                negotiation\")."),
                ("openehr-item-tag" = String,
                 description = "Echo of the ITEM_TAG list now stored on the \
                                VERSIONED_COMPOSITION, present only when the \
                                request carried the header (§\"openehr-item-tag \
                                and openehr-version-item-tag\", \"Usage in \
                                Responses\": servers \"MAY include\" it \"to \
                                confirm the actual list of ITEM_TAGs stored on the \
                                server side\")."),
                ("openehr-version-item-tag" = String,
                 description = "Echo of the ITEM_TAG list now stored on the new \
                                COMPOSITION VERSION, present only when the request \
                                carried the header (same section, \"Usage in \
                                Responses\")."),
            ),
            content(
                (serde_json::Value = "application/json", examples(
                    ("representation" = (summary = "Prefer: return=representation — the committed COMPOSITION",
                     value = json!({
                        "_type": "COMPOSITION",
                        "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" },
                        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                        "archetype_details": {
                            "_type": "ARCHETYPED",
                            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                            "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                            "rm_version": "1.2.0"
                        },
                        "name": { "_type": "DV_TEXT", "value": "Encounter" },
                        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                        "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                        "category": {
                            "_type": "DV_CODED_TEXT", "value": "event",
                            "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                        },
                        "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                        "context": {
                            "_type": "EVENT_CONTEXT",
                            "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                            "setting": {
                                "_type": "DV_CODED_TEXT", "value": "other care",
                                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                            }
                        },
                        "content": [ {
                            "_type": "EVALUATION",
                            "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                            "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                            "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                            "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                            "subject": { "_type": "PARTY_SELF" },
                            "data": {
                                "_type": "ITEM_TREE",
                                "archetype_node_id": "at0001",
                                "name": { "_type": "DV_TEXT", "value": "Tree" },
                                "items": [ {
                                    "_type": "ELEMENT",
                                    "archetype_node_id": "at0002",
                                    "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                                    "value": { "_type": "DV_TEXT", "value": "Asthma" }
                                } ]
                            }
                        } ]
                     }))),
                    ("identifier" = (summary = "Prefer: return=identifier — only the new version uid",
                     value = json!({ "uid": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" })))
                )),
                (serde_json::Value = "application/xml"),
                (serde_json::Value = "application/openehr.wt.flat+json"),
                (serde_json::Value = "application/openehr.wt.structured+json")
            )
        ),
        (status = 400, description = "`ehr_id` is not a UUID, the COMPOSITION \
                                      payload could not be parsed, or a committal \
                                      `change_type` names a legal \
                                      audit_change_type code that contradicts a \
                                      creation (only `249|creation|` is compatible \
                                      — RM common master06 §Contributions) \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: a \
                                      COMPOSITION is served as canonical \
                                      `application/json` / `application/xml` or as \
                                      `application/openehr.wt.flat+json` / \
                                      `application/openehr.wt.structured+json` \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\"/§\"Simplified Formats\": \"If the \
                                      service cannot fulfill this aspect of the \
                                      request, it MUST respond with HTTP status \
                                      code `406 Not Acceptable`\").",
         body = serde_json::Value),
        (status = 409, description = "The creation conflicts with the EHR's \
                                      current state (`Requests_and_responses.md` \
                                      §\"HTTP status codes\", the `409` row: the \
                                      request \"might generate a duplicate or a \
                                      conflict\"). Two triggers, both of which are \
                                      OUR OWN DESIGN — the released text assigns \
                                      neither a status code: (1) the EHR already \
                                      holds a live persistent COMPOSITION for the \
                                      same template (no released text defines a \
                                      uniqueness rule for persistent \
                                      COMPOSITIONs — RM ehr master05 §\"Persistent \
                                      Compositions\" and the COMPOSITION \
                                      invariants are silent; the conformance \
                                      catalogue carries it as an ambiguity-register \
                                      entry, reported and excluded from profile \
                                      computation); (2) the EHR is not modifiable \
                                      (`EHR_STATUS.is_modifiable = false`, RM ehr \
                                      master04 §\"EHR Active Status\": the flag \
                                      \"is used to indicate whether the contents \
                                      of an EHR are modifiable\" — the refusal is \
                                      spec-required, but the status choice is \
                                      ours).",
         body = serde_json::Value),
        (status = 415, description = "The request `Content-Type` is not one this \
                                      resource can process — a media type outside \
                                      canonical JSON/XML and the two Simplified \
                                      Formats, or a deprecated \
                                      `…schema+json` variant (`Resources.md` \
                                      §\"Simplified Formats\": \"If the service \
                                      cannot process the request payload as the \
                                      simplified format is not supported, it MUST \
                                      respond with HTTP status code `415 \
                                      Unsupported Media Type`\"; §\"XML \
                                      Format\"/§\"JSON Format\" carry the same \
                                      MUST for the canonical types).",
         body = serde_json::Value),
        (status = 422, description = "The request was well-formed and converted to \
                                      a COMPOSITION, but cannot be followed: the \
                                      named operational template is unknown or \
                                      does not validate the content, an RM \
                                      class-invariant or terminology-binding check \
                                      failed, a committal \
                                      `change_type`/`lifecycle_state` is not a \
                                      member of its openEHR terminology group, or \
                                      a Simplified-Format body arrived without the \
                                      `openehr-template-id` header \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `422` row: \"The request was \
                                      well-formed but was unable to be followed \
                                      due to semantic errors\"; ITS-REST \
                                      `specifications/responses/422.yaml`: \"the \
                                      underlying template is not known or is not \
                                      validating the supplied resource\").",
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
///
/// The one route that accepts BOTH identifier forms (`Resources.md`
/// §"Multiple identifiers for the same resource"): an `OBJECT_VERSION_ID`
/// serves that exact version, a `HIER_OBJECT_ID` container id serves the latest
/// version or the one extant at `version_at_time`. The served body is the
/// BARE COMPOSITION; the `ORIGINAL_VERSION` envelope is served by the
/// `versioned_composition` operations below.
#[utoipa::path(
    get, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("uid_based_id" = String, Path,
         description = "The COMPOSITION address, in either of the two forms \
                        `Resources.md` §\"Multiple identifiers for the same \
                        resource\" defines for this resource. As an \
                        **explicit version reference** it is an \
                        OBJECT_VERSION_ID taken from VERSION.uid.value \
                        (`{object_id}::{creating_system_id}::{version_tree_id}`) \
                        and serves THAT version — including a superseded, \
                        non-current one; the addressed uid must name the served \
                        version's full three-part identity, so a fabricated \
                        `creating_system_id` names no VERSION here and is `404`. \
                        As an **implicit latest version reference** it is a \
                        HIER_OBJECT_ID taken from VERSIONED_OBJECT.uid.value \
                        (the version container) and serves the version extant at \
                        `version_at_time`, or the latest when that parameter is \
                        absent. `version_at_time` applies only to the container \
                        form; combined with an explicit version reference the \
                        version addressing wins and the parameter is ignored (the \
                        released text defines no combination).",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2"),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the version \
                        extant at that time is returned. Applies to the \
                        container (HIER_OBJECT_ID) form of `uid_based_id`; absent \
                        means the latest version. The timezone is optional — \
                        server-local when omitted (`Resources.md` §\"Datetime \
                        format\": query parameters \"MUST always use the \
                        _extended_ ISO 8601 format\" and \"Timezone SHOULD be \
                        only supplied when needed, otherwise the local timezone \
                        is assumed\").",
         example = "2026-07-26T09:12:44.512Z"),
        ("expand_multimedia" = Option<bool>, Query,
         description = "OUR OWN EXTENSION — no openEHR spec governs this \
                        parameter. `true` transparently re-inlines DV_MULTIMEDIA \
                        content this deployment externalized to object storage, \
                        verifying its integrity, so the served COMPOSITION is \
                        byte-identical to the committed one. A no-op when \
                        externalization is off or the body holds no external \
                        media.",
         example = json!(true))
    ),
    responses(
        (
            status = 200, description = "The COMPOSITION, in the representation \
                                        negotiated from `Accept` (canonical \
                                        JSON/XML or one of the two Simplified \
                                        Formats — `Resources.md` §\"Simplified \
                                        Formats\"). The version-identity headers \
                                        are representation-independent: \"the \
                                        `ETag` value is independent of its \
                                        resource serialization format \
                                        (JSON/XML)\" (§\"ETag and \
                                        Last-Modified\"). No `Location`: \
                                        `Requests_and_responses.md` §Location \
                                        forbids it on a `GET` (\"It MUST NOT be \
                                        used to indicate an alternate \
                                        representation of an existing \
                                        resource\"). The `openehr-item-tag`/\
                                        `openehr-version-item-tag` response \
                                        headers are a MAY on reads (§\"openehr-\
                                        item-tag and openehr-version-item-tag\", \
                                        \"Usage in Responses\"): this server \
                                        emits them only on the `POST`/`PUT` write \
                                        wrappers, never on a `GET`.",
            headers(
                ("ETag" = String,
                 description = "The weak entity tag `W/\"<version uid>\"` of the \
                                version actually served — the addressed one for \
                                an explicit version reference, the resolved \
                                latest/extant-at one for the container form \
                                (§\"ETag and Last-Modified\"; the `W/` weakness \
                                indicator is required since Release 1.1.0)."),
                ("Last-Modified" = String,
                 description = "That version's commit instant as an HTTP-date — \
                                \"derived from \
                                VERSION.commit_audit.time_committed.value\" \
                                (§\"ETag and Last-Modified\"). The bare \
                                COMPOSITION body carries no commit audit, so the \
                                instant comes from the version metadata."),
            ),
            content(
                (serde_json::Value = "application/json", example = json!({
                    "_type": "COMPOSITION",
                    "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
                    "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                    "archetype_details": {
                        "_type": "ARCHETYPED",
                        "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                        "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                        "rm_version": "1.2.0"
                    },
                    "name": { "_type": "DV_TEXT", "value": "Encounter" },
                    "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                    "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                    "category": {
                        "_type": "DV_CODED_TEXT", "value": "event",
                        "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                    },
                    "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                    "context": {
                        "_type": "EVENT_CONTEXT",
                        "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                        "setting": {
                            "_type": "DV_CODED_TEXT", "value": "other care",
                            "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                        }
                    },
                    "content": [ {
                        "_type": "EVALUATION",
                        "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                        "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                        "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                        "subject": { "_type": "PARTY_SELF" },
                        "data": {
                            "_type": "ITEM_TREE",
                            "archetype_node_id": "at0001",
                            "name": { "_type": "DV_TEXT", "value": "Tree" },
                            "items": [ {
                                "_type": "ELEMENT",
                                "archetype_node_id": "at0002",
                                "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                                "value": { "_type": "DV_TEXT", "value": "Asthma" }
                            } ]
                        }
                    } ]
                })),
                (serde_json::Value = "application/xml"),
                (serde_json::Value = "application/openehr.wt.flat+json"),
                (serde_json::Value = "application/openehr.wt.structured+json")
            )
        ),
        (status = 204, description = "The addressed COMPOSITION version is \
                                      logically deleted, so there is no \
                                      representation to serve: a logical delete \
                                      commits a new version whose `data` is \
                                      removed and whose `lifecycle_state` is \
                                      `523|deleted|` (RM common master06 \
                                      §\"Logical Deletion\": \"create a new \
                                      Version …; delete its `data` …; set the \
                                      `lifecycle_state` value to the code for \
                                      `deleted`\"), and RM common \
                                      `org.openehr.rm.common.version` types \
                                      `ORIGINAL_VERSION.data` `0..1`. This server \
                                      answers `204` for ALL THREE addressing \
                                      forms — the latest version, the version \
                                      extant at `version_at_time`, and an \
                                      explicit `version_uid` naming a deleted \
                                      version. Only the middle one is covered by \
                                      released text (ITS-REST \
                                      `specifications/responses/204_deleted_at_time.yaml`: \
                                      \"`204 No Content` is returned when the \
                                      resource identified by the request \
                                      parameters (at specified `version_at_time`) \
                                      time has been deleted\"); the explicit-\
                                      version and implicit-latest branches have no \
                                      released branch assignment at all, so \
                                      extending the same outcome to them is OUR \
                                      OWN reading, carried as an \
                                      ambiguity-register entry in the conformance \
                                      catalogue. The VERSION envelope routes below \
                                      answer differently — `200` with a data-less \
                                      `523|deleted|` ORIGINAL_VERSION."),
        (status = 400, description = "`ehr_id` is not a UUID, `uid_based_id` is \
                                      neither a well-formed OBJECT_VERSION_ID nor \
                                      a UUID version-container id, or \
                                      `version_at_time` is not an extended ISO \
                                      8601 datetime (`Requests_and_responses.md` \
                                      §\"HTTP status codes\", the `400` row: \
                                      \"malformed request syntax, syntactically \
                                      invalid content\"). A syntactically valid \
                                      but unknown id is `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`; no COMPOSITION with \
                                      `uid_based_id` in this EHR; no version at \
                                      the requested `version_at_time`; or an \
                                      explicit `version_uid` whose full \
                                      three-part identity names no stored VERSION \
                                      (a fabricated `creating_system_id`).",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: a \
                                      COMPOSITION is served as canonical \
                                      `application/json` / `application/xml` or as \
                                      `application/openehr.wt.flat+json` / \
                                      `application/openehr.wt.structured+json` \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\"/§\"Simplified Formats\": \"If the \
                                      service cannot fulfill this aspect of the \
                                      request, it MUST respond with HTTP status \
                                      code `406 Not Acceptable`\").",
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
/// Addresses the version CONTAINER and takes the superseded version in a
/// mandatory `If-Match`, so — unlike the `DELETE` — the precondition is not
/// in the path (`Requests_and_responses.md` §"If-Match and accidental
/// overwrites"). The committal headers `openehr-version` /
/// `openehr-audit-details` are accepted and merged into the commit
/// (§openehr-version-and-audit-details).
#[utoipa::path(
    put, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("uid_based_id" = String, Path,
         description = "The version container to commit the new version into — a \
                        HIER_OBJECT_ID taken from VERSIONED_OBJECT.uid.value \
                        (`Resources.md` §\"Identifier types\"). Unlike the `GET`, \
                        the update takes ONLY this form; the version being \
                        superseded travels in `If-Match`. A body \
                        `COMPOSITION.uid`, if present, must identify the same \
                        versioned object — its `object_id` part must equal this \
                        segment (RM common master06 §\"Version Identification\": \
                        the `object_id()` part \"is a copy of the `uid` of the \
                        owning VERSIONED_OBJECT\"); a mismatch is `400`, never a \
                        silent write to the path's object.",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21"),
        ("If-Match" = String, Header,
         description = "The latest COMPOSITION version uid (which becomes the new \
                        version's `preceding_version_uid`), double-quoted; the \
                        weak `W/\"…\"` form the server emits is also accepted. \
                        Required — `Requests_and_responses.md` §\"If-Match and \
                        accidental overwrites\": the header is required \"when the \
                        `preceding_version_uid` is not part of the endpoint path \
                        segment\", which is exactly this operation; \"When the \
                        service expects `If-Match` for an operation, but the \
                        client does not provide it, the service SHOULD respond \
                        with `400 Bad Request`\", and a non-matching value MUST be \
                        `412`.",
         example = "W/\"df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1\""),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`Requests_and_responses.md` §\"Representation details \
                        negotiation\"). Exactly one of the three tokens: \
                        `return=minimal` — no body, `204 No Content`; \
                        `return=identifier` — the body is only \
                        `{ \"uid\": \"<new version uid>\" }` at `200 OK`, never \
                        `204` (§\"Prefer only identifier\": \"a variant of \
                        preference that implies minimal response semantics, but \
                        with a non-empty response body\"); `return=representation` \
                        — the committed COMPOSITION at `200 OK`. An absent header \
                        means `return=minimal`; the token actually applied is \
                        echoed in `Preference-Applied`. A Simplified-Format \
                        `Accept` always answers with the committed COMPOSITION in \
                        that form, so the applied preference is then \
                        `return=representation` whatever was asked for.",
         example = "return=representation"),
        ("openehr-version" = Option<String>, Header,
         description = "Committal metadata for the new COMPOSITION VERSION, as an \
                        attribute-path list — e.g. \
                        `lifecycle_state.code_string=\"553\"` (`553|incomplete|` \
                        relaxes the template's lower-cardinality limits, RM common \
                        master06 §\"Version Lifecycle\"); the default is \
                        `532|complete|`. Merged with the server defaults \
                        (`Requests_and_responses.md` §\"openehr-version and \
                        openehr-audit-details\": whatever is provided \"MUST be \
                        merged with the default VERSION and \
                        VERSION.audit_details attributes on commit runtime\")."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the CONTRIBUTION this update \
                        commits, as an attribute-path list; the header MAY repeat \
                        — e.g. `description.value=\"Diagnosis corrected\"`, \
                        `committer.name=\"John Doe\",\
                        committer.external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\",\
                        committer.external_ref.namespace=\"demographic\",\
                        committer.external_ref.type=\"PERSON\"`, \
                        `system_id=\"example.openehr.systemid\"`. \
                        `change_type` defaults to `251|modification|` and is \
                        client-overridable to any audit_change_type code \
                        consistent with a content-carrying new version of an \
                        existing object — `250|amendment|`, `252|synthesis|`, \
                        `253|unknown|`, `816|restoration|`, \
                        `817|format conversion|` — but never `249|creation|`, \
                        `523|deleted|` or `666|attestation|`, which belong to \
                        other operations (RM common master06 §Contributions); an \
                        incompatible legal code is `400`. `time_committed` \"is always set by the \
                        server\", and an omitted `system_id` defaults to the \
                        server's configured identifier."),
        ("openehr-template-id" = Option<String>, Header,
         description = "The operational-template id the committed content is \
                        validated against. `Requests_and_responses.md` \
                        §openehr-template-id: the header \"MUST be used whenever \
                        committing COMPOSITION (via `PUT` or `POST` methods) \
                        using a Simplified Format which does not support \
                        TEMPLATE_ID value under an equivalent \
                        `LOCATABLE.archetype_details.template_id` attribute of \
                        contained data\". A canonical JSON/XML body carries its \
                        own `archetype_details.template_id`; a simplified commit \
                        without the header is `422`.",
         example = "problem_list.v1"),
        ("openehr-item-tag" = Option<String>, Header,
         description = "Item tags to set on the VERSIONED_COMPOSITION \
                        (VERSIONED_OBJECT-level target); an empty value removes \
                        all (`Requests_and_responses.md` §\"openehr-item-tag and \
                        openehr-version-item-tag\", \"Usage in Requests\"). MAY \
                        be echoed back in the response header of the same name."),
        ("openehr-version-item-tag" = Option<String>, Header,
         description = "Item tags to set on the new COMPOSITION VERSION; an empty \
                        value removes all (same section, \"Usage in Requests\"). \
                        MAY be echoed back in the response header of the same \
                        name.")
    ),
    request_body(
        content(
            (serde_json::Value = "application/json", example = json!({
                "_type": "COMPOSITION",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" },
                "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                "archetype_details": {
                    "_type": "ARCHETYPED",
                    "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                    "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                    "rm_version": "1.2.0"
                },
                "name": { "_type": "DV_TEXT", "value": "Encounter" },
                "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                "category": {
                    "_type": "DV_CODED_TEXT", "value": "event",
                    "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                },
                "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                "context": {
                    "_type": "EVENT_CONTEXT",
                    "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                    "setting": {
                        "_type": "DV_CODED_TEXT", "value": "other care",
                        "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                    }
                },
                "content": [ {
                    "_type": "EVALUATION",
                    "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                    "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                    "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                    "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                    "subject": { "_type": "PARTY_SELF" },
                    "data": {
                        "_type": "ITEM_TREE",
                        "archetype_node_id": "at0001",
                        "name": { "_type": "DV_TEXT", "value": "Tree" },
                        "items": [ {
                            "_type": "ELEMENT",
                            "archetype_node_id": "at0002",
                            "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                            "value": { "_type": "DV_TEXT", "value": "Asthma — well controlled" }
                        } ]
                    }
                } ]
            })),
            (serde_json::Value = "application/xml"),
            (serde_json::Value = "application/openehr.wt.flat+json"),
            (serde_json::Value = "application/openehr.wt.structured+json")
        ),
        description = "The new COMPOSITION, in canonical JSON/XML or one of the \
                       two Simplified Formats per `Content-Type` (`Resources.md` \
                       §\"Simplified Formats\"). A body `uid` is optional; when \
                       present its `object_id` part must equal the path \
                       `uid_based_id`. A simplified body additionally requires the \
                       `openehr-template-id` request header."
    ),
    responses(
        (
            status = 200, description = "Updated, with a body: the committed \
                                        COMPOSITION for \
                                        `Prefer: return=representation` (the \
                                        `representation` example) or the \
                                        single-`uid` object for \
                                        `return=identifier` (the `identifier` \
                                        example) — `Requests_and_responses.md` \
                                        §\"Prefer minimal, identifier or full \
                                        representation response\". `ETag`, \
                                        `Last-Modified` and `Location` describe \
                                        the newly committed version, and \
                                        `Preference-Applied` declares the token \
                                        honoured.",
            headers(
                ("ETag" = String,
                 description = "The weak entity tag `W/\"<new version uid>\"` \
                                (§\"ETag and Last-Modified\": the value \"is \
                                usually taken from e.g. … VERSION.uid.value\" and \
                                \"changes as soon as the resource changes\")."),
                ("Location" = String,
                 description = "The URL of the newly committed version, \
                                `<base_path>/ehr/<ehr_id>/composition/<version_uid>` \
                                (§\"Prefer minimal, identifier or full \
                                representation response\": the response \"SHOULD \
                                include a `Location` header pointing to the newly \
                                created or updated resource\")."),
                ("Last-Modified" = String,
                 description = "The commit instant of the new version as an \
                                HTTP-date — \"derived from \
                                VERSION.commit_audit.time_committed.value\" \
                                (§\"ETag and Last-Modified\")."),
                ("Preference-Applied" = String,
                 description = "`return=minimal` | `return=identifier` | \
                                `return=representation` — the preference the \
                                service honoured (§\"Representation details \
                                negotiation\")."),
                ("openehr-item-tag" = String,
                 description = "Echo of the ITEM_TAG list now stored on the \
                                VERSIONED_COMPOSITION, present only when the \
                                request carried the header (§\"openehr-item-tag \
                                and openehr-version-item-tag\", \"Usage in \
                                Responses\")."),
                ("openehr-version-item-tag" = String,
                 description = "Echo of the ITEM_TAG list now stored on the new \
                                COMPOSITION VERSION, present only when the request \
                                carried the header (same section, \"Usage in \
                                Responses\")."),
            ),
            content(
                (serde_json::Value = "application/json", examples(
                    ("representation" = (summary = "Prefer: return=representation — the committed COMPOSITION",
                     value = json!({
                        "_type": "COMPOSITION",
                        "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
                        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                        "archetype_details": {
                            "_type": "ARCHETYPED",
                            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                            "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                            "rm_version": "1.2.0"
                        },
                        "name": { "_type": "DV_TEXT", "value": "Encounter" },
                        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                        "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                        "category": {
                            "_type": "DV_CODED_TEXT", "value": "event",
                            "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                        },
                        "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                        "context": {
                            "_type": "EVENT_CONTEXT",
                            "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                            "setting": {
                                "_type": "DV_CODED_TEXT", "value": "other care",
                                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                            }
                        },
                        "content": [ {
                            "_type": "EVALUATION",
                            "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                            "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                            "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                            "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                            "subject": { "_type": "PARTY_SELF" },
                            "data": {
                                "_type": "ITEM_TREE",
                                "archetype_node_id": "at0001",
                                "name": { "_type": "DV_TEXT", "value": "Tree" },
                                "items": [ {
                                    "_type": "ELEMENT",
                                    "archetype_node_id": "at0002",
                                    "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                                    "value": { "_type": "DV_TEXT", "value": "Asthma — well controlled" }
                                } ]
                            }
                        } ]
                     }))),
                    ("identifier" = (summary = "Prefer: return=identifier — only the new version uid",
                     value = json!({ "uid": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" })))
                )),
                (serde_json::Value = "application/xml"),
                (serde_json::Value = "application/openehr.wt.flat+json"),
                (serde_json::Value = "application/openehr.wt.structured+json")
            )
        ),
        (status = 204, description = "Updated with no body — the default \
                                      `Prefer: return=minimal` \
                                      (`Requests_and_responses.md` §\"Prefer \
                                      minimal, identifier or full representation \
                                      response\": \"If no response body is \
                                      returned, the service SHOULD use `204 No \
                                      Content`\"). The version headers are carried \
                                      exactly as on the `200`.",
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<new version uid>\"` \
                             (§\"ETag and Last-Modified\")."),
             ("Location" = String,
              description = "The URL of the newly committed version, \
                             `<base_path>/ehr/<ehr_id>/composition/<version_uid>` \
                             (§\"Prefer minimal, identifier or full \
                             representation response\")."),
             ("Last-Modified" = String,
              description = "The commit instant of the new version as an \
                             HTTP-date (§\"ETag and Last-Modified\")."),
             ("Preference-Applied" = String,
              description = "`return=minimal` — the preference the service \
                             honoured (§\"Representation details \
                             negotiation\")."),
             ("openehr-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the \
                             VERSIONED_COMPOSITION, present only when the request \
                             carried the header (§\"openehr-item-tag and \
                             openehr-version-item-tag\", \"Usage in \
                             Responses\")."),
             ("openehr-version-item-tag" = String,
              description = "Echo of the ITEM_TAG list now stored on the new \
                             COMPOSITION VERSION, present only when the request \
                             carried the header (same section, \"Usage in \
                             Responses\")."),
         )),
        (status = 400, description = "`ehr_id` is not a UUID, `uid_based_id` is \
                                      not a well-formed version-container id, the \
                                      COMPOSITION payload could not be parsed, \
                                      `If-Match` is missing/empty/not a \
                                      well-formed OBJECT_VERSION_ID, a body \
                                      `COMPOSITION.uid` names a different \
                                      versioned object than the path, or a \
                                      committal `change_type` names a legal \
                                      audit_change_type code that contradicts an \
                                      update (`Requests_and_responses.md` §\"HTTP \
                                      status codes\", the `400` row: \"malformed \
                                      request syntax, syntactically invalid \
                                      content\"; §\"If-Match and accidental \
                                      overwrites\" for the missing-`If-Match` \
                                      case).",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`; no COMPOSITION with \
                                      `uid_based_id` in this EHR; or the \
                                      COMPOSITION's latest version is already \
                                      logically deleted, so there is nothing to \
                                      supersede (RM common master06 §\"Logical \
                                      Deletion\").",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: a \
                                      COMPOSITION is served as canonical \
                                      `application/json` / `application/xml` or as \
                                      `application/openehr.wt.flat+json` / \
                                      `application/openehr.wt.structured+json` \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\"/§\"Simplified Formats\": an \
                                      unfulfillable `Accept` MUST be `406`).",
         body = serde_json::Value),
        (status = 409, description = "The EHR is not modifiable \
                                      (`EHR_STATUS.is_modifiable = false`), so its \
                                      contents cannot be updated (RM ehr master04 \
                                      §\"EHR Active Status\": the flag \"is used to \
                                      indicate whether the contents of an EHR are \
                                      modifiable\"). The refusal is spec-required; \
                                      the status code is OUR OWN DESIGN — no \
                                      released ITS-REST text assigns a branch to \
                                      it — chosen for the `409` row's \"conflict\" \
                                      meaning (§\"HTTP status codes\").",
         body = serde_json::Value),
        (status = 412, description = "`If-Match` does not name the latest \
                                      COMPOSITION version, so the update was not \
                                      performed (`Requests_and_responses.md` \
                                      §\"If-Match and accidental overwrites\": the \
                                      service \"MUST NOT perform the requested \
                                      method\" and \"MUST respond with HTTP status \
                                      code `412 Precondition Failed`\").",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<current latest version \
                             uid>\"` — the same section's SHOULD: the service \
                             \"SHOULD return also latest `version_uid` in the \
                             `ETag` response headers\"."),
             ("Last-Modified" = String,
              description = "The commit instant of that current latest version as \
                             an HTTP-date, carried alongside the `ETag` \
                             (§\"ETag and Last-Modified\")."),
         )),
        (status = 415, description = "The request `Content-Type` is not one this \
                                      resource can process — a media type outside \
                                      canonical JSON/XML and the two Simplified \
                                      Formats (`Resources.md` §\"Simplified \
                                      Formats\": \"If the service cannot process \
                                      the request payload as the simplified format \
                                      is not supported, it MUST respond with HTTP \
                                      status code `415 Unsupported Media Type`\"; \
                                      §\"XML Format\"/§\"JSON Format\" carry the \
                                      same MUST for the canonical types).",
         body = serde_json::Value),
        (status = 422, description = "The request was well-formed and converted to \
                                      a COMPOSITION, but cannot be followed: the \
                                      named operational template is unknown or \
                                      does not validate the content, an RM \
                                      class-invariant or terminology-binding check \
                                      failed, a committal `lifecycle_state` is not \
                                      a member of its openEHR terminology group, a \
                                      Simplified-Format body arrived without the \
                                      `openehr-template-id` header, or the body \
                                      declares a DIFFERENT `template_id` than the \
                                      stored composition it supersedes \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `422` row; ITS-REST \
                                      `specifications/responses/422.yaml`: \"the \
                                      underlying template is not known or is not \
                                      validating the supplied resource\"). The \
                                      template-change rejection specifically is \
                                      OUR OWN CONVENTION: no released text forbids \
                                      changing `archetype_details.template_id` \
                                      across the versions of one \
                                      VERSIONED_COMPOSITION, and the conformance \
                                      catalogue carries the rule as a \
                                      register-backed, reported-only expectation.",
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
/// Deletion is LOGICAL: a new `523|deleted|` version with no `data` is
/// committed (RM common master06 §"Logical Deletion"), and the `204` reports
/// THAT version in `ETag`/`Last-Modified`. The path segment carries the
/// superseded version's uid, which is why no `If-Match` is taken
/// (`Requests_and_responses.md` §"If-Match and accidental overwrites" — the
/// header is required only when the `preceding_version_uid` is not a path
/// segment).
/// The committal headers `openehr-version` / `openehr-audit-details` are
/// accepted and merged into the deletion commit
/// (`Requests_and_responses.md` §openehr-version-and-audit-details).
#[utoipa::path(
    delete, path = "/ehr/{ehr_id}/composition/{uid_based_id}", tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("uid_based_id" = String, Path,
         description = "The LATEST version's uid — an OBJECT_VERSION_ID taken \
                        from VERSION.uid.value, the `preceding_version_uid` the \
                        deletion supersedes (`Resources.md` §\"Identifier \
                        types\"). A bare HIER_OBJECT_ID container id is `400`. \
                        Because the preceding version travels IN THE PATH, this \
                        operation takes no `If-Match`: \
                        `Requests_and_responses.md` §\"If-Match and accidental \
                        overwrites\" requires the header only \"when the \
                        `preceding_version_uid` is not part of the endpoint path \
                        segment\" — the path IS the precondition here, and a \
                        non-latest uid is `409`, not `412`.",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2"),
        ("openehr-version" = Option<String>, Header,
         description = "Accepted for uniformity — `Requests_and_responses.md` \
                        §\"openehr-version and openehr-audit-details\" requires \
                        the committal headers on `PUT`, `POST` **and** `DELETE`. \
                        The deleting version's `lifecycle_state` is not client-\
                        selectable, though: a logical deletion sets it to \
                        `523|deleted|` by definition (RM common master06 \
                        §\"Logical Deletion\"), so a supplied `lifecycle_state` \
                        does not override it."),
        ("openehr-audit-details" = Option<String>, Header,
         description = "Committal AUDIT_DETAILS for the CONTRIBUTION this \
                        deletion commits, as an attribute-path list; the header \
                        MAY repeat — e.g. `description.value=\"Committed to the \
                        wrong record\"`, `committer.name=\"John Doe\"`, \
                        `system_id=\"example.openehr.systemid\"`. `change_type` \
                        is constrained to `523|deleted|` (RM common master06 \
                        §Contributions: a deletion commits a deleted version); \
                        `time_committed` is always server-set, and an omitted \
                        `system_id` defaults to the server's configured \
                        identifier.")
    ),
    responses(
        (status = 204, description = "Logically deleted. Deletion is never \
                                      physical: a NEW version is committed whose \
                                      `data` is removed and whose \
                                      `lifecycle_state` — and whose \
                                      `commit_audit.change_type` — is \
                                      `523|deleted|` (RM common master06 \
                                      §\"Logical Deletion\": \"create a new \
                                      Version in the normal way; delete its \
                                      `data`…; set the `lifecycle_state` value to \
                                      the code for `deleted`; commit in the normal \
                                      way\"), inside a new CONTRIBUTION. No \
                                      `Location`: `Requests_and_responses.md` \
                                      §\"HTTP headers\" records that \"the \
                                      `Location` response header was deprecated \
                                      from responses of `DELETE` methods\".",
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<new deleted version \
                             uid>\"` — the identity of the version the deletion \
                             just committed, not the superseded one named in the \
                             path. The released text never says which of the two \
                             the header carries, so this choice is OURS, \
                             register-documented in the conformance catalogue; it \
                             follows from §\"ETag and Last-Modified\" (the tag \
                             \"changes as soon as the resource changes\", and the \
                             resource's current version is now the deleted one)."),
             ("Last-Modified" = String,
              description = "The deleting version's commit instant as an \
                             HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\"); a logical delete is a \
                             commit, so it IS the resource's last modification."),
         )),
        (status = 400, description = "`ehr_id` is not a UUID, `uid_based_id` is \
                                      not a well-formed OBJECT_VERSION_ID (a bare \
                                      container id is rejected here), or the \
                                      addressed COMPOSITION is ALREADY logically \
                                      deleted — ITS-REST \
                                      `specifications/responses/400_already_deleted.yaml`: \
                                      `400` is returned \"when the resource \
                                      identified by the request parameters is \
                                      already deleted\" (`Requests_and_responses.md` \
                                      §\"HTTP status codes\", the `400` row). The \
                                      already-deleted branch is evaluated BEFORE \
                                      the not-latest `409`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no COMPOSITION with \
                                      this `uid_based_id` in this EHR.",
         body = serde_json::Value),
        (status = 409, description = "The deletion was refused as conflicting \
                                      with the resource's current state. Two \
                                      triggers: (1) `uid_based_id` does not name \
                                      the latest version — ITS-REST \
                                      `specifications/responses/409_COMPOSITION_with_uid_based_id.yaml`: \
                                      \"`409 Conflict` is returned when supplied \
                                      `uid_based_id` doesn't match the latest \
                                      version. Returns also latest `version_uid` \
                                      in the `ETag` header\" (the full three-part \
                                      identity is compared, so a fabricated \
                                      `creating_system_id` on the right version \
                                      tree does not match); (2) the EHR is not \
                                      modifiable (`EHR_STATUS.is_modifiable = \
                                      false`, RM ehr master04 §\"EHR Active \
                                      Status\") — the refusal is spec-required but \
                                      its status code is OUR OWN DESIGN, no \
                                      released text assigning it a branch.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<current latest version \
                             uid>\"` (the `409` response definition's \"Returns \
                             also latest `version_uid` in the `ETag` header\")."),
             ("Last-Modified" = String,
              description = "The commit instant of that current latest version as \
                             an HTTP-date, carried alongside the `ETag` \
                             (§\"ETag and Last-Modified\")."),
         ))
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value — a HIER_OBJECT_ID in \
                        UUID form (`Resources.md` §\"Identifier types\": \"a \
                        `versioned_object_uid` for identifying a \
                        VERSIONED_OBJECT (i.e. a version container), stored under \
                        VERSIONED_OBJECT.uid.value, in a form of a \
                        HIER_OBJECT_ID\"). Unlike `/composition/{uid_based_id}`, \
                        this segment accepts only the container form — \
                        §\"Multiple identifiers for the same resource\" is scoped \
                        to that route.",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21")
    ),
    responses(
        (status = 200, description = "The VERSIONED_COMPOSITION container \
                                      (canonical JSON/XML per `Accept`): the \
                                      versioned-object identity, its `owner_id` \
                                      (the owning EHR) and `time_created` — the \
                                      commit time of the object's FIRST version \
                                      (RM common master06 §\"Versioned \
                                      Objects\") — not the version content. No \
                                      `Location`: `Requests_and_responses.md` \
                                      §Location forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag \
                             `W/\"<versioned_object_uid>\"` (§\"ETag and \
                             Last-Modified\": the value \"is usually taken from \
                             e.g. VERSIONED_OBJECT.uid.value\"; the `W/` weakness \
                             indicator is required since Release 1.1.0)."),
             ("Last-Modified" = String,
              description = "The commit instant of the container's most recent \
                             version as an HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\": both headers \"SHOULD \
                             be included in responses for VERSION, \
                             VERSIONED_OBJECT, or other resources that have \
                             versioning or unique state identifiers\")."),
         ),
         example = json!({
             "_type": "VERSIONED_COMPOSITION",
             "uid": { "_type": "HIER_OBJECT_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21" },
             "owner_id": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "EHR",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "7d44b88c-4199-4bad-97dc-d78268e01398" }
             },
             "time_created": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" }
         })),
        (status = 400, description = "`ehr_id` or `versioned_object_uid` is not a \
                                      UUID (`Requests_and_responses.md` §\"HTTP \
                                      status codes\", the `400` row: \"malformed \
                                      request syntax, syntactically invalid \
                                      content\"). A syntactically valid but \
                                      unknown id is `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no \
                                      VERSIONED_COMPOSITION with \
                                      `versioned_object_uid` in this EHR.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the VERSIONED_COMPOSITION container has only \
                                      the canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"; the Simplified Formats are \
                                      defined for templated COMPOSITION content, \
                                      not for the version container).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value — a HIER_OBJECT_ID in \
                        UUID form (`Resources.md` §\"Identifier types\").",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21")
    ),
    responses(
        (status = 200, description = "The REVISION_HISTORY of the \
                                      VERSIONED_COMPOSITION (canonical JSON/XML \
                                      per `Accept`): one REVISION_HISTORY_ITEM \
                                      per committed version, most recent LAST \
                                      (RM common \
                                      `org.openehr.rm.common.revision_history.adoc`, \
                                      `REVISION_HISTORY.items`: \"The items in \
                                      this history in most-recent-last order\"). \
                                      Each item's `audits[0]` is that version's \
                                      commit audit; any post-committal \
                                      ATTESTATION on the version appears as a \
                                      further element of the same `audits` list \
                                      (RM common master06 §Attestation), never as \
                                      an extra history item. No `Location`: \
                                      `Requests_and_responses.md` §Location \
                                      forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag \
                             `W/\"<versioned_object_uid>\"` of the container the \
                             history belongs to (§\"ETag and Last-Modified\": the \
                             value \"is usually taken from e.g. \
                             VERSIONED_OBJECT.uid.value\")."),
             ("Last-Modified" = String,
              description = "The commit instant of the most recent revision as \
                             an HTTP-date — \"derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "REVISION_HISTORY",
             "items": [
                 {
                     "_type": "REVISION_HISTORY_ITEM",
                     "version_id": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" },
                     "audits": [ {
                         "_type": "AUDIT_DETAILS",
                         "system_id": "openEHRSys.example.com",
                         "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                         "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                         "change_type": {
                             "_type": "DV_CODED_TEXT",
                             "value": "creation",
                             "defining_code": {
                                 "_type": "CODE_PHRASE",
                                 "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                                 "code_string": "249"
                             }
                         }
                     } ]
                 },
                 {
                     "_type": "REVISION_HISTORY_ITEM",
                     "version_id": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
                     "audits": [ {
                         "_type": "AUDIT_DETAILS",
                         "system_id": "openEHRSys.example.com",
                         "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                         "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                         "change_type": {
                             "_type": "DV_CODED_TEXT",
                             "value": "modification",
                             "defining_code": {
                                 "_type": "CODE_PHRASE",
                                 "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                                 "code_string": "251"
                             }
                         }
                     } ]
                 }
             ]
         })),
        (status = 400, description = "`ehr_id` or `versioned_object_uid` is not a \
                                      UUID (`Requests_and_responses.md` §\"HTTP \
                                      status codes\", the `400` row: \"malformed \
                                      request syntax, syntactically invalid \
                                      content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`, or no \
                                      VERSIONED_COMPOSITION with \
                                      `versioned_object_uid` in this EHR.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the REVISION_HISTORY resource has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": an unfulfillable `Accept` MUST be \
                                      `406`; the Simplified Formats are not \
                                      defined for this resource).",
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
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value — a HIER_OBJECT_ID in \
                        UUID form (`Resources.md` §\"Identifier types\").",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21"),
        ("version_at_time" = Option<String>, Query,
         description = "A time in the extended ISO 8601 format; the VERSION \
                        extant at that time is returned. Absent means the latest \
                        VERSION. The timezone is optional — server-local when \
                        omitted (`Resources.md` §\"Datetime format\": query \
                        parameters \"MUST always use the _extended_ ISO 8601 \
                        format\" and \"Timezone SHOULD be only supplied when \
                        needed, otherwise the local timezone is assumed\").",
         example = "2026-07-26T09:12:44.512Z")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION envelope of the \
                                      COMPOSITION extant at that time (canonical \
                                      JSON/XML per `Accept`): the version \
                                      identity, its CONTRIBUTION reference, the \
                                      commit audit, the lifecycle state, and the \
                                      COMPOSITION itself under `data`. A \
                                      logically deleted version is served here as \
                                      a `200` whose `lifecycle_state` is \
                                      `523|deleted|` and which carries NO `data` \
                                      (RM common master06 §\"Logical Deletion\"; \
                                      `ORIGINAL_VERSION.data` is `0..1` in RM \
                                      common `org.openehr.rm.common.version`) — \
                                      unlike the bare `/composition` route, which \
                                      answers `204`. No `Location`: \
                                      `Requests_and_responses.md` §Location \
                                      forbids it on a `GET` (\"It MUST NOT be used \
                                      to indicate an alternate representation of \
                                      an existing resource\").",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\"; the `W/` \
                             weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "The version's own `commit_audit.time_committed` as \
                             an HTTP-date — \"For openEHR resources, this value \
                             should be derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "ORIGINAL_VERSION",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
             "preceding_version_uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" },
             "contribution": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "CONTRIBUTION",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "0826851c-c4c2-4d61-92b9-410fb8275ff0" }
             },
             "commit_audit": {
                 "_type": "AUDIT_DETAILS",
                 "system_id": "openEHRSys.example.com",
                 "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                 "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                 "change_type": {
                     "_type": "DV_CODED_TEXT",
                     "value": "modification",
                     "defining_code": {
                         "_type": "CODE_PHRASE",
                         "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                         "code_string": "251"
                     }
                 }
             },
             "lifecycle_state": {
                 "_type": "DV_CODED_TEXT",
                 "value": "complete",
                 "defining_code": {
                     "_type": "CODE_PHRASE",
                     "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                     "code_string": "532"
                 }
             },
             "data": {
                 "_type": "COMPOSITION",
                 "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
                 "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                 "archetype_details": {
                     "_type": "ARCHETYPED",
                     "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                     "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                     "rm_version": "1.2.0"
                 },
                 "name": { "_type": "DV_TEXT", "value": "Encounter" },
                 "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                 "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                 "category": {
                     "_type": "DV_CODED_TEXT", "value": "event",
                     "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                 },
                 "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                 "context": {
                     "_type": "EVENT_CONTEXT",
                     "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                     "setting": {
                         "_type": "DV_CODED_TEXT", "value": "other care",
                         "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                     }
                 },
                 "content": [ {
                     "_type": "EVALUATION",
                     "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                     "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                     "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                     "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                     "subject": { "_type": "PARTY_SELF" },
                     "data": {
                         "_type": "ITEM_TREE",
                         "archetype_node_id": "at0001",
                         "name": { "_type": "DV_TEXT", "value": "Tree" },
                         "items": [ {
                             "_type": "ELEMENT",
                             "archetype_node_id": "at0002",
                             "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                             "value": { "_type": "DV_TEXT", "value": "Asthma — well controlled" }
                         } ]
                     }
                 } ]
             }
         })),
        (status = 400, description = "`ehr_id` or `versioned_object_uid` is not a \
                                      UUID, or `version_at_time` is not an \
                                      extended ISO 8601 datetime \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\").",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`; no VERSIONED_COMPOSITION \
                                      with `versioned_object_uid` in this EHR; or \
                                      the container holds no VERSION at the \
                                      requested `version_at_time` (an instant \
                                      before its first commit).",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the ORIGINAL_VERSION envelope has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": an unfulfillable `Accept` MUST be \
                                      `406`; the Simplified Formats describe the \
                                      COMPOSITION content, not the VERSION \
                                      envelope).",
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
///
/// Both id segments are load-bearing: the addressed version must belong to
/// the container in the path (RM common `org.openehr.rm.common.version`
/// invariant `Owner_id_valid`; `Resources.md` §"Identifier types"), and the
/// `version_uid` must match the served version's full three-part identity.
#[utoipa::path(
    get,
    path = "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
    tag = "COMPOSITION",
    params(
        ("ehr_id" = String, Path,
         description = "EHR identifier, taken from EHR.ehr_id.value — a UUID \
                        (`Resources.md` §\"Identifier types\").",
         example = "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ("versioned_object_uid" = String, Path,
         description = "VERSIONED_COMPOSITION identifier, taken from \
                        VERSIONED_COMPOSITION.uid.value — a HIER_OBJECT_ID in \
                        UUID form (`Resources.md` §\"Identifier types\"). It must \
                        be the container that owns `version_uid`: the two \
                        segments are coherent only when the version's `object_id` \
                        part equals this segment.",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21"),
        ("version_uid" = String, Path,
         description = "VERSION identifier, taken from VERSION.uid.value — an \
                        OBJECT_VERSION_ID \
                        `{object_id}::{creating_system_id}::{version_tree_id}` \
                        (`Resources.md` §\"Identifier types\"). The addressed uid \
                        must name the served version's FULL three-part identity; \
                        a fabricated `creating_system_id` names no VERSION here \
                        and is `404`.",
         example = "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2")
    ),
    responses(
        (status = 200, description = "The ORIGINAL_VERSION envelope of the \
                                      COMPOSITION identified by `version_uid` \
                                      (canonical JSON/XML per `Accept`): the \
                                      version identity, its CONTRIBUTION \
                                      reference, the commit audit, the lifecycle \
                                      state, and the COMPOSITION itself under \
                                      `data`. A logically deleted version is \
                                      served here as a `200` whose \
                                      `lifecycle_state` is `523|deleted|` and \
                                      which carries NO `data` (RM common master06 \
                                      §\"Logical Deletion\"; \
                                      `ORIGINAL_VERSION.data` is `0..1` in RM \
                                      common `org.openehr.rm.common.version`). No \
                                      `Location`: `Requests_and_responses.md` \
                                      §Location forbids it on a `GET`.",
         body = serde_json::Value,
         headers(
             ("ETag" = String,
              description = "The weak entity tag `W/\"<version_uid>\"` \
                             (§\"ETag and Last-Modified\": the value \"is usually \
                             taken from e.g. … VERSION.uid.value\"; the `W/` \
                             weakness indicator is required since Release \
                             1.1.0)."),
             ("Last-Modified" = String,
              description = "The version's own `commit_audit.time_committed` as \
                             an HTTP-date — \"For openEHR resources, this value \
                             should be derived from \
                             VERSION.commit_audit.time_committed.value\" \
                             (§\"ETag and Last-Modified\")."),
         ),
         example = json!({
             "_type": "ORIGINAL_VERSION",
             "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
             "preceding_version_uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::1" },
             "contribution": {
                 "_type": "OBJECT_REF",
                 "namespace": "local",
                 "type": "CONTRIBUTION",
                 "id": { "_type": "HIER_OBJECT_ID", "value": "0826851c-c4c2-4d61-92b9-410fb8275ff0" }
             },
             "commit_audit": {
                 "_type": "AUDIT_DETAILS",
                 "system_id": "openEHRSys.example.com",
                 "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                 "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-26T11:04:02.880114Z" },
                 "change_type": {
                     "_type": "DV_CODED_TEXT",
                     "value": "modification",
                     "defining_code": {
                         "_type": "CODE_PHRASE",
                         "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                         "code_string": "251"
                     }
                 }
             },
             "lifecycle_state": {
                 "_type": "DV_CODED_TEXT",
                 "value": "complete",
                 "defining_code": {
                     "_type": "CODE_PHRASE",
                     "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                     "code_string": "532"
                 }
             },
             "data": {
                 "_type": "COMPOSITION",
                 "uid": { "_type": "OBJECT_VERSION_ID", "value": "df58b2ee-30bd-4b2c-9b7d-3a0f8e5c6d21::openEHRSys.example.com::2" },
                 "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                 "archetype_details": {
                     "_type": "ARCHETYPED",
                     "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
                     "template_id": { "_type": "TEMPLATE_ID", "value": "problem_list.v1" },
                     "rm_version": "1.2.0"
                 },
                 "name": { "_type": "DV_TEXT", "value": "Encounter" },
                 "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                 "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
                 "category": {
                     "_type": "DV_CODED_TEXT", "value": "event",
                     "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" }
                 },
                 "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Jane Roe" },
                 "context": {
                     "_type": "EVENT_CONTEXT",
                     "start_time": { "_type": "DV_DATE_TIME", "value": "2026-07-26T09:12:44.512331Z" },
                     "setting": {
                         "_type": "DV_CODED_TEXT", "value": "other care",
                         "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" }
                     }
                 },
                 "content": [ {
                     "_type": "EVALUATION",
                     "archetype_node_id": "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                     "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis" },
                     "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
                     "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
                     "subject": { "_type": "PARTY_SELF" },
                     "data": {
                         "_type": "ITEM_TREE",
                         "archetype_node_id": "at0001",
                         "name": { "_type": "DV_TEXT", "value": "Tree" },
                         "items": [ {
                             "_type": "ELEMENT",
                             "archetype_node_id": "at0002",
                             "name": { "_type": "DV_TEXT", "value": "Problem/Diagnosis name" },
                             "value": { "_type": "DV_TEXT", "value": "Asthma — well controlled" }
                         } ]
                     }
                 } ]
             }
         })),
        (status = 400, description = "`ehr_id` or `versioned_object_uid` is not a \
                                      UUID, or `version_uid` is not a well-formed \
                                      OBJECT_VERSION_ID \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `400` row: \"malformed request \
                                      syntax, syntactically invalid content\"). A \
                                      syntactically valid but unknown id is \
                                      `404`, not `400`.",
         body = serde_json::Value),
        (status = 404, description = "Unknown `ehr_id`; no VERSIONED_COMPOSITION \
                                      with `versioned_object_uid` in this EHR; no \
                                      VERSION with `version_uid`; or the two id \
                                      segments are INCOHERENT — a `version_uid` \
                                      whose `object_id` part does not name the \
                                      container in the path. The version then \
                                      belongs to a different VERSIONED_OBJECT and \
                                      this URL identifies nothing: RM common \
                                      `org.openehr.rm.common.version` invariant \
                                      `Owner_id_valid` \
                                      (`owner_id.value.is_equal (uid.object_id.value)`) \
                                      and `Resources.md` §\"Identifier types\" \
                                      (\"the `object_id` matches the \
                                      VERSIONED_OBJECT identifier, taken from \
                                      VERSIONED_OBJECT.uid.value\").",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the ORIGINAL_VERSION envelope has only the \
                                      canonical `application/json` / \
                                      `application/xml` representations \
                                      (`Resources.md` §\"XML Format\"/§\"JSON \
                                      Format\": an unfulfillable `Accept` MUST be \
                                      `406`; the Simplified Formats describe the \
                                      COMPOSITION content, not the VERSION \
                                      envelope).",
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
