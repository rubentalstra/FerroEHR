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
