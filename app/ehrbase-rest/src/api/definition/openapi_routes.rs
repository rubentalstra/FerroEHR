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
//!   store path (the OAS folds that under `400`). A request DECLARING a media
//!   type other than the operation's single `text/plain` body type is refused
//!   `415` before parsing — the same cross-cutting rule that grounds the ADL 1.4
//!   guard (`specifications/docs/overview/Resources.md` §"XML Format"/§"JSON
//!   Format": a payload the service cannot process as the declared format "MUST
//!   respond with HTTP status code `415 Unsupported Media Type`"); an absent
//!   `Content-Type` declares nothing to refuse (the header is a client MAY). The
//!   `at_version` query parameter is `deprecated: true` and is dropped
//!   (spec-permitted); only `Prefer` is honoured.
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
// `definition_template_adl2_version_get` carries `#[deprecated]` — the released
// operation is `deprecated: true` under SPECITS-87
// (`ITS-REST specifications/operations/definition_template_adl2_version_get.yaml`
// + `docs/overview/Amendment_record.md`), and utoipa reflects the Rust attribute
// into the served OpenAPI. It is the only handler in this group so marked; the
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
         description = "\"Pattern for matching `template_id` (supports wildcards \
                        `*`)\" (ITS-REST \
                        `specifications/parameters/query/filter_template_id.yaml`); \
                        omit to match any.",
         example = "vital*"),
        ("concept" = Option<String>, Query,
         description = "\"Pattern for matching `concept` (supports wildcards \
                        `*`)\" (ITS-REST \
                        `specifications/parameters/query/concept.yaml`); omit to \
                        match any.",
         example = "*signs*"),
        ("version" = Option<String>, Query,
         description = "A glob (`*` wildcard) matched against the whole \
                        `template_id`, which is where an ADL 1.4 template's \
                        version lives (`vital_signs.v1`). The ITS-REST DOCS TEXT \
                        defines no such parameter — it exists only in the \
                        stalled OAS \
                        (`specifications/parameters/query/filter_version.yaml`: \
                        \"Filter by version …, taken from `template_id`; if \
                        missing, then only the latest version will be \
                        returned\"), which is codegen input and never a \
                        behavioural oracle — so OUR handling of that silence is: \
                        an ABSENT `version` applies no version filter and every \
                        stored template is listed; there is no implicit \
                        latest-only collapse. All three filters AND together.",
         example = "*.v1"),
        ("offset" = Option<i64>, Query,
         description = "\"The row number in result-set to start result-set from \
                        (`0`-based), default is `0`\" (ITS-REST \
                        `specifications/parameters/query/offset.yaml`). An offset \
                        past the end of the match set yields an empty list, not \
                        an error — the released text fixes no other handling. A \
                        negative value is ignored (read as absent).",
         example = 0),
        ("fetch" = Option<i64>, Query,
         description = "\"Number of rows to fetch (the default depends on the \
                        implementation)\" (ITS-REST \
                        `specifications/parameters/query/fetch.yaml`); absent or \
                        `0` returns every match.",
         example = 10)
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template list is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_TemplateList_adl1_4.yaml`) \
                                        — retrieval, not matching: a filter that \
                                        matches nothing is this `200` with an \
                                        empty array, never a `404`. Each row is a \
                                        `TemplateMetadata` whose `template_id`, \
                                        `concept`, `archetype_id` and \
                                        `created_timestamp` are all required \
                                        (`schemas/definition/TemplateMetadata.yaml`); \
                                        its `version` member is `deprecated: \
                                        true` there, so this server emits it only \
                                        when the stored `template_id` carries a \
                                        derivable version and never as an \
                                        explicit `null`.",
            content((serde_json::Value = "application/json", example = json!([
                {
                    "template_id": "Vital Signs",
                    "concept": "Vital Signs",
                    "archetype_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                    "created_timestamp": "2017-08-14T19:24:56.639Z"
                }
            ])))
        ),
        (status = 400, description = "The released trigger, verbatim: `400 Bad \
                                      Request` \"is returned when the request \
                                      could not be parsed or is invalid (e.g. \
                                      malformed request URL syntax, missing \
                                      required header or parameter, or \
                                      syntactically invalid header, parameter or \
                                      content)\" (ITS-REST \
                                      `specifications/responses/400.yaml`). Here \
                                      the only reachable trigger is a \
                                      syntactically invalid parameter: `offset` \
                                      or `fetch` that is not an integer. The \
                                      released list operation does not enumerate \
                                      `400`; the trigger is the cross-cutting \
                                      one.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the template list has the single \
                                      `application/json` representation the \
                                      released response declares \
                                      (`200_TemplateList_adl1_4.yaml`, and \
                                      `parameters/header/Accept_JSON.yaml` on the \
                                      operation), so an exclusively-XML `Accept` \
                                      is refused (`Resources.md` §\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"). The released operation does \
                                      not enumerate `406`; the MUST is \
                                      cross-cutting.",
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
/// The template arrives as canonical OPT XML (`Content-Type: application/xml`);
/// a request declaring another payload type is `415`
/// (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
/// §XML Format).
#[utoipa::path(
    post, path = "/definition/template/adl1.4", tag = "ADL1.4",
    params(
        ("Content-Type" = Option<String>, Header,
         description = "`application/xml` (or `text/xml`) — the operation's only \
                        declared payload format \
                        (`specifications/operations/definition_template_adl1.4_upload.yaml`, \
                        whose `Content-Type` enum is the single value \
                        `application/xml`). Any other DECLARED type is `415`; an \
                        absent header declares nothing to refuse (`Resources.md` \
                        §\"XML Format\" makes the header a client MAY) and reads \
                        as XML.",
         example = "application/xml"),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`specifications/parameters/header/Prefer.yaml`; \
                        `Requests_and_responses.md` §\"Representation details \
                        negotiation\"). `return=representation` — the stored OPT \
                        XML; `return=identifier` — \"only its unique identifier \
                        is included\" (`responses/201_Template_adl1_4_upload.yaml`), \
                        i.e. the `TemplateIdentifier` object; missing or \
                        `return=minimal` — \"the body is empty\". The token \
                        actually honoured is echoed in `Preference-Applied`.",
         example = "return=representation")
    ),
    request_body(content((String = "application/xml")),
                 description = "The operational template as canonical OPT XML \
                                (`schemas/aom/OperationalTemplate.yaml`); \
                                `required: true` on the released operation."),
    responses(
        (
            status = 201, description = "The released trigger, verbatim: `201 \
                                        Created` \"is returned when the template \
                                        has been successfully uploaded\" \
                                        (ITS-REST \
                                        `specifications/responses/201_Template_adl1_4_upload.yaml`). \
                                        The body follows `Prefer`, per that same \
                                        response: \"If `Prefer` header is \
                                        `return=representation`, the full \
                                        resource is included in the response \
                                        body; if is `return=identifier`, only its \
                                        unique identifier is included. If the \
                                        `Prefer` header is missing or set to \
                                        `return=minimal`, the body is empty.\" A \
                                        template carries no `uid` on this wire, \
                                        so the identifier body is the released \
                                        `TemplateIdentifier` object \
                                        (`schemas/others/TemplateIdentifier.yaml`), \
                                        not the generic `{uid}` shape.",
            headers(
                ("Location" = String,
                 description = "\"The `Location` response header indicates the \
                                URL of the Template resource\" (ITS-REST \
                                `specifications/headers/Location_Template_adl1_4.yaml`, \
                                whose own example percent-encodes the id: \
                                `…/v1/definition/template/adl1.4/Vital%20Signs`) \
                                — set here to \
                                `<base_path>/definition/template/adl1.4/<percent-encoded \
                                template_id>` on every `Prefer` outcome \
                                (`Requests_and_responses.md` §Location: used \
                                \"in `201 Created` responses when a new resource \
                                is successfully created\")."),
                ("ETag" = String,
                 description = "\"The `ETag` (i.e. entity tag) response header is \
                                an identifier of Template\" (ITS-REST \
                                `specifications/headers/ETag_Template_adl1_4.yaml`), \
                                carrying the stored `template_id` in the weak \
                                form the release requires — \"all `ETag` headers \
                                that hold a resource identifier MUST include a \
                                weakness indicator `W/`\" \
                                (`Requests_and_responses.md` §\"ETag and \
                                Last-Modified\"). Shape: `W/\"Vital Signs\"`. \
                                Keyed on the same `template_id` the retrieval \
                                `ETag` uses, so a client's `If-None-Match` \
                                round-trips."),
                ("Preference-Applied" = String,
                 description = "`return=minimal` | `return=identifier` | \
                                `return=representation` — the preference the \
                                service honoured. \"The service MAY include a \
                                `Preference-Applied` header in the response … to \
                                indicate that the client's preference has been \
                                honored\" (`Requests_and_responses.md` \
                                §\"Representation details negotiation\").")
            ),
            content(
                (String = "application/xml", example = r#"<template xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.openehr.org/v1">
    <language>
        <terminology_id><value>ISO_639-1</value></terminology_id>
        <code_string>en</code_string>
    </language>
    <uid><value>b4d7f203-b329-4e89-a58a-c605b19e94de</value></uid>
    <template_id><value>Vital Signs</value></template_id>
    <concept>Vital Signs</concept>
    <definition>
        <rm_type_name>COMPOSITION</rm_type_name>
        <node_id>at0000</node_id>
        ...
    </definition>
</template>
"#),
                (serde_json::Value = "application/json",
                 example = json!({ "template_id": "Vital Signs" }))
            )
        ),
        (status = 400, description = "The released trigger, verbatim: `400 Bad \
                                      Request` \"is returned when the request \
                                      could not be parsed or is invalid (e.g. \
                                      malformed request URL syntax, missing \
                                      required header or parameter, or \
                                      syntactically invalid header, parameter or \
                                      content)\" (ITS-REST \
                                      `specifications/responses/400.yaml`). Here: \
                                      the body is not UTF-8 text, or is not \
                                      well-formed XML (mismatched tags, an empty \
                                      document with no root element) — \
                                      \"syntactically invalid … content\" is \
                                      precisely that branch. Everything past XML \
                                      well-formedness (a document that is not an \
                                      OPT, an AOM2 rule violation) is the \
                                      semantic `422` below.",
         body = serde_json::Value),
        (status = 409, description = "The released trigger, verbatim: `409 \
                                      Conflict` \"is returned when a template \
                                      with same `template_id` already exists\" \
                                      (ITS-REST \
                                      `specifications/responses/409_template_already_exists.yaml`).",
         body = serde_json::Value),
        (status = 415, description = "The request DECLARES a payload media type \
                                      the service cannot process as this \
                                      operation's XML body: \"A client MAY use \
                                      the header `Content-Type: application/xml` \
                                      in the requests to specify the XML payload \
                                      format. If the service cannot process the \
                                      request payload as XML format, it MUST \
                                      respond with HTTP status code `415 \
                                      Unsupported Media Type`\" (`Resources.md` \
                                      §\"XML Format\"). An absent `Content-Type` \
                                      declares nothing to refuse and is accepted. \
                                      The released operation does not enumerate \
                                      `415`; the MUST is cross-cutting \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `415` row).",
         body = serde_json::Value),
        (status = 422, description = "OUR WIRE — the payload is well-formed XML but \
                                      is not a usable operational template: it \
                                      does not decode as an OPT document; a \
                                      foreign or duplicated top-level element; a \
                                      blank `template_id` or an empty `concept` \
                                      (both mandatory `OPERATIONAL_TEMPLATE` \
                                      attributes); or an AOM2 standalone-artefact \
                                      validity-rule violation, the rule code \
                                      carried in `validationErrors` \
                                      (`AM/docs/AOM2/master08-validation.adoc` \
                                      §Validation; `schemas/others/Error.yaml`). No released \
                                      response file covers the semantic branch on \
                                      this operation, so the assignment is the \
                                      overview status table's own `422` row — \
                                      \"the request was well-formed but was \
                                      unable to be followed due to semantic \
                                      errors\" (`Requests_and_responses.md` \
                                      §\"HTTP status codes\") — and \"additional \
                                      status codes MAY be used as long as they do \
                                      not conflict with the predefined codes\". \
                                      Syntactically-unparseable content is the \
                                      `400` above, not this branch.",
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
         description = "\"Template identifier or partial reference. A partial \
                        `template_id` will resolve to “latest” major version of \
                        that template\" (ITS-REST \
                        `specifications/parameters/path/template_id.yaml`, whose \
                        own examples run from the legacy `Vital Signs` to the \
                        HRID `org.highmed::openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0`). \
                        This server resolves an ADL 1.4 id by EXACT match \
                        (case-insensitive): no released text defines a version \
                        grammar for legacy OPT 1.4 identifiers, so the \
                        partial-reference rule has nothing to resolve against \
                        here — OUR OWN handling of that silence, registered as a \
                        conformance boundary.",
         example = "Vital Signs"),
        ("Accept" = Option<String>, Header,
         description = "`application/xml` (the canonical OPT; also the default \
                        for absent/`*/*`), or `application/openehr.wt+json` / \
                        `application/json` (the Web Template document — the only \
                        JSON projection of an OPT). All three are enumerated by \
                        `specifications/parameters/header/Accept_Template.yaml`. \
                        The response `Content-Type` is always the negotiated type \
                        (`Resources.md` §\"JSON Format\": the proper \
                        `Content-Type` \"MUST be present in the response of the \
                        service\").",
         example = "application/xml")
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_Template_adl1_4_retrieved.yaml`). \
                                        The body follows `Accept`: the stored \
                                        canonical OPT XML served back verbatim \
                                        — \"the original (canonical) `XML` based \
                                        OPT format\" \
                                        (`operations/definition_template_adl1.4_get.yaml`) \
                                        — or \"the simplified `JSON`-based \
                                        \u{201c}web template\u{201d} format\" \
                                        under whichever of \
                                        `application/openehr.wt+json` / \
                                        `application/json` was negotiated. \
                                        `application/json` is honoured rather \
                                        than refused because \
                                        `parameters/header/Accept_Template.yaml` \
                                        and `headers/ContentType_Template.yaml` \
                                        both enumerate it while the released \
                                        `200` declares no schema for it; the Web \
                                        Template document is the only JSON \
                                        template representation the release \
                                        names (`Resources.md` §Simplified \
                                        Formats). The Web Template document's \
                                        internal shape follows the Better \
                                        `web-template` model — no openEHR spec \
                                        defines a schema for it, our own \
                                        design/extension.",
            headers(
                ("ETag" = String,
                 description = "\"The `ETag` (i.e. entity tag) response header is \
                                an identifier of Template\" (ITS-REST \
                                `specifications/headers/ETag_Template_adl1_4.yaml`), \
                                carrying the `template_id` in the weak form the \
                                release requires — \"all `ETag` headers that hold \
                                a resource identifier MUST include a weakness \
                                indicator `W/`\" (`Requests_and_responses.md` \
                                §\"ETag and Last-Modified\"). Shape: `W/\"Vital \
                                Signs\"`, identical across all three \
                                representations because \"the `ETag` value is \
                                independent of its resource serialization format \
                                (JSON/XML)\" (same section). No `Last-Modified`: \
                                a stored OPT has no released version or state \
                                identifier to date.")
            ),
            content(
                (String = "application/xml", example = r#"<template xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.openehr.org/v1">
    <language>
        <terminology_id><value>ISO_639-1</value></terminology_id>
        <code_string>en</code_string>
    </language>
    <uid><value>b4d7f203-b329-4e89-a58a-c605b19e94de</value></uid>
    <template_id><value>Vital Signs</value></template_id>
    <concept>Vital Signs</concept>
    <definition>
        <rm_type_name>COMPOSITION</rm_type_name>
        <node_id>at0000</node_id>
        ...
    </definition>
</template>
"#),
                (serde_json::Value = "application/openehr.wt+json"),
                (serde_json::Value = "application/json")
            )
        ),
        (status = 404, description = "The released trigger, verbatim: `404 Not \
                                      Found` \"is returned when a template with \
                                      the specified `template_id` does not \
                                      exist\" (ITS-REST \
                                      `specifications/responses/404_unknown_template_id.yaml`).",
         body = serde_json::Value),
        (status = 406, description = "The released trigger, verbatim: `406 Not \
                                      Acceptable` \"is returned when the service \
                                      cannot produce a response matching `Accept` \
                                      request header, i.e. content type or format \
                                      is not supported\" (ITS-REST \
                                      `specifications/responses/406.yaml`) — here, \
                                      an `Accept` outside `application/xml`, \
                                      `application/openehr.wt+json` and \
                                      `application/json`. Resolved BEFORE storage \
                                      is touched, so an unacceptable `Accept` on \
                                      an unknown template is still this `406`.",
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
         description = "\"Template identifier or partial reference\" (ITS-REST \
                        `specifications/parameters/path/template_id.yaml`), \
                        resolved by EXACT (case-insensitive) match as on the \
                        sibling `_get` — no released text defines a version \
                        grammar for legacy OPT 1.4 identifiers.",
         example = "Vital Signs"),
        ("type" = Option<String>, Query,
         description = "\"Type of use for the data example\" (ITS-REST \
                        `specifications/parameters/query/example_type.yaml`, \
                        enum `[input, output]`, default `input`): \"either of \
                        type `input` (i.e., ready to be submitted to the \
                        repository) or `output` (as it would appear when \
                        retrieved from the repository)\" \
                        (`operations/definition_template_adl1.4_example_get.yaml`).",
         example = "input"),
        ("detail_level" = Option<String>, Query,
         description = "\"The level of data points details of the example\" \
                        (ITS-REST \
                        `specifications/parameters/query/example_detail_level.yaml`, \
                        enum `[required, medium, complete]`, default \
                        `required`). The released meanings, from the operation \
                        text: `required` — \"a minimal composition including \
                        only mandatory data points\"; `medium` — \"a fairly \
                        realistic set of data points, including some optional RM \
                        attributes and elements\"; `complete` — \"a full \
                        representation of all possible data points; not expected \
                        to be committable or realistic\". \"The implementation \
                        and completeness of these examples are not specified, \
                        and vendors may produce different results.\"",
         example = "required"),
        ("Accept" = Option<String>, Header,
         description = "One of `application/json` (default), `application/xml`, \
                        `application/openehr.wt.flat+json`, or \
                        `application/openehr.wt.structured+json` — the four \
                        forms \
                        `specifications/parameters/header/Accept_LOCATABLE.yaml` \
                        enumerates and \
                        `responses/200_Template_example_retrieved.yaml` declares \
                        content for (`Resources.md` §Simplified Formats fixes \
                        the two `openehr.wt.*` media types).",
         example = "application/json")
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_Template_example_retrieved.yaml`) \
                                        — here the GENERATED example data \
                                        instance, an RM COMPOSITION serialized \
                                        per `Accept` (canonical JSON/XML, or one \
                                        of the two Simplified Formats). No \
                                        `ETag`: the example is derived output, \
                                        not a stored resource with an identifier \
                                        (`Requests_and_responses.md` §\"ETag and \
                                        Last-Modified\" scopes the header to \
                                        \"resources that have versioning or \
                                        unique state identifiers\").",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "`type` or `detail_level` is outside its \
                                      enumerated set. The released operation \
                                      leaves the handling open — when the server \
                                      \"does not support the requested `type` or \
                                      `detail_level`, it will fall back to the \
                                      closest supported level, or it may return \
                                      an error (typically `400 Bad Request`)\" — \
                                      and this server takes the error arm for an \
                                      out-of-enum value (an out-of-enum value is \
                                      also \"syntactically invalid … parameter\" \
                                      under `responses/400.yaml`). Which arm of \
                                      that either/or applies is OURS.",
         body = serde_json::Value),
        (status = 404, description = "The released trigger, verbatim: `404 Not \
                                      Found` \"is returned when a template with \
                                      the specified `template_id` does not \
                                      exist\" (ITS-REST \
                                      `specifications/responses/404_unknown_template_id.yaml`).",
         body = serde_json::Value),
        (status = 406, description = "The released trigger, verbatim: `406 Not \
                                      Acceptable` \"is returned when the service \
                                      cannot produce a response matching `Accept` \
                                      request header, i.e. content type or format \
                                      is not supported\" (ITS-REST \
                                      `specifications/responses/406.yaml`) — an \
                                      `Accept` outside the four \
                                      `Accept_LOCATABLE` representations.",
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
         description = "\"Pattern for matching `template_id` (supports wildcards \
                        `*`)\" (ITS-REST \
                        `specifications/parameters/query/filter_template_id.yaml`); \
                        omit to match any. An ADL2 `template_id` is the \
                        artefact's `ARCHETYPE_HRID`.",
         example = "openEHR-EHR-COMPOSITION.t_vital_signs.*"),
        ("concept" = Option<String>, Query,
         description = "\"Pattern for matching `concept` (supports wildcards \
                        `*`)\" (ITS-REST \
                        `specifications/parameters/query/concept.yaml`); omit to \
                        match any.",
         example = "*signs*"),
        ("version" = Option<String>, Query,
         description = "A glob (`*` wildcard) matched against the whole \
                        `template_id`, which for ADL2 is the HRID carrying the \
                        artefact's SEMVER `release_version` \
                        (`…t_vital_signs.v1.0.0`). The ITS-REST DOCS TEXT defines \
                        no such parameter — it exists only in the stalled OAS \
                        (`specifications/parameters/query/filter_version.yaml`), \
                        which is codegen input and never a behavioural oracle — \
                        so OUR handling of that silence is: an ABSENT `version` \
                        applies no version filter and every stored template is \
                        listed; there is no implicit latest-only collapse. All \
                        three filters AND together.",
         example = "*.v1.*"),
        ("offset" = Option<i64>, Query,
         description = "\"The row number in result-set to start result-set from \
                        (`0`-based), default is `0`\" (ITS-REST \
                        `specifications/parameters/query/offset.yaml`). An offset \
                        past the end of the match set yields an empty list, not \
                        an error. A negative value is ignored (read as absent).",
         example = 0),
        ("fetch" = Option<i64>, Query,
         description = "\"Number of rows to fetch (the default depends on the \
                        implementation)\" (ITS-REST \
                        `specifications/parameters/query/fetch.yaml`); absent or \
                        `0` returns every match.",
         example = 10)
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template list is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_TemplateList_adl2.yaml`) \
                                        — retrieval, not matching: a filter that \
                                        matches nothing is this `200` with an \
                                        empty array, never a `404`. Each row is a \
                                        `TemplateMetadata` whose `template_id`, \
                                        `concept`, `archetype_id` and \
                                        `created_timestamp` are all required \
                                        (`schemas/definition/TemplateMetadata.yaml`); \
                                        the schema gives no member descriptions \
                                        and no released text says what \
                                        `archetype_id` holds for an ADL2 \
                                        operational template, so this server \
                                        emits the artefact's own `ARCHETYPE_HRID` \
                                        there (the released example instead shows \
                                        the specialisation parent) — OUR OWN \
                                        reading of that silence. `concept` is \
                                        derived from the HRID's concept segment. \
                                        The `deprecated: true` `version` member \
                                        is not emitted on this interface.",
            content((serde_json::Value = "application/json", example = json!([
                {
                    "template_id": "openEHR-EHR-COMPOSITION.t_clinical_info_ds_sf.v1.0.0",
                    "concept": "t_clinical_info_ds_sf",
                    "archetype_id": "openEHR-EHR-COMPOSITION.t_clinical_info_ds_sf.v1.0.0",
                    "created_timestamp": "2017-08-14T19:24:56.639Z"
                },
                {
                    "template_id": "openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0",
                    "concept": "t_vital_signs",
                    "archetype_id": "openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0",
                    "created_timestamp": "2017-08-14T19:24:56.639Z"
                }
            ])))
        ),
        (status = 400, description = "The released trigger, verbatim: `400 Bad \
                                      Request` \"is returned when the request \
                                      could not be parsed or is invalid (e.g. \
                                      malformed request URL syntax, missing \
                                      required header or parameter, or \
                                      syntactically invalid header, parameter or \
                                      content)\" (ITS-REST \
                                      `specifications/responses/400.yaml`). Here \
                                      the only reachable trigger is a \
                                      syntactically invalid parameter: `offset` \
                                      or `fetch` that is not an integer. The \
                                      released list operation does not enumerate \
                                      `400`; the trigger is the cross-cutting \
                                      one.",
         body = serde_json::Value),
        (status = 406, description = "The `Accept` header cannot be satisfied: \
                                      the template list has the single \
                                      `application/json` representation the \
                                      released response declares \
                                      (`200_TemplateList_adl2.yaml`, and \
                                      `parameters/header/Accept_JSON.yaml` on the \
                                      operation), so an exclusively-XML `Accept` \
                                      is refused (`Resources.md` §\"JSON \
                                      Format\": \"If the service cannot fulfill \
                                      this aspect of the request, it MUST respond \
                                      with HTTP status code `406 Not \
                                      Acceptable`\"). The released operation does \
                                      not enumerate `406`; the MUST is \
                                      cross-cutting.",
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
/// The template arrives as `text/plain` ADL2 source; a request declaring another
/// payload type is `415`
/// (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
/// §"XML Format"/§"JSON Format" — the cross-cutting format rule). The deprecated
/// `at_version` query parameter is dropped (spec-permitted).
#[utoipa::path(
    post, path = "/definition/template/adl2", tag = "ADL2",
    params(
        ("Content-Type" = Option<String>, Header,
         description = "`text/plain` — the operation's only declared payload \
                        format \
                        (`specifications/operations/definition_template_adl2_upload.yaml`, \
                        whose `Content-Type` enum is the single value \
                        `text/plain`). Any other DECLARED type is `415`; an \
                        absent header declares nothing to refuse (`Resources.md` \
                        makes the header a client MAY) and reads as ADL2 source.",
         example = "text/plain"),
        ("Prefer" = Option<String>, Header,
         description = "Response-verbosity preference \
                        (`specifications/parameters/header/Prefer.yaml`; \
                        `Requests_and_responses.md` §\"Representation details \
                        negotiation\"). `return=representation` — the stored ADL2 \
                        source (`text/plain`); `return=identifier` — \"only its \
                        unique identifier is included\" \
                        (`responses/201_Template_adl2_upload.yaml`), i.e. the \
                        `TemplateIdentifier` object; missing or `return=minimal` \
                        — \"the body is empty\". The token actually honoured is \
                        echoed in `Preference-Applied`.",
         example = "return=identifier")
    ),
    request_body(content((String = "text/plain")),
                 description = "The ADL2 operational-template source \
                                (`schemas/aom/OperationalTemplateV2.yaml` or a \
                                plain string); `required: true` on the released \
                                operation."),
    responses(
        (
            status = 201, description = "The released trigger, verbatim: `201 \
                                        Created` \"is returned when the template \
                                        has been successfully uploaded\" \
                                        (ITS-REST \
                                        `specifications/responses/201_Template_adl2_upload.yaml`). \
                                        The body follows `Prefer`, per that same \
                                        response: \"If `Prefer` header is \
                                        `return=representation`, the full \
                                        resource is included in the response \
                                        body; if is `return=identifier`, only its \
                                        unique identifier is included. If the \
                                        `Prefer` header is missing or set to \
                                        `return=minimal`, the body is empty.\" \
                                        The identifier body is the released \
                                        `TemplateIdentifier` object \
                                        (`schemas/others/TemplateIdentifier.yaml`), \
                                        carrying the stored `ARCHETYPE_HRID`.",
            headers(
                ("Location" = String,
                 description = "\"The `Location` response header indicates the \
                                URL of the Template resource\" (ITS-REST \
                                `specifications/headers/Location_Template_adl2.yaml`, \
                                example \
                                `…/v1/definition/template/adl2/openEHR-EHR-COMPOSITION.t_clinical_info_ds_sf.v1.0.0`) \
                                — set here to \
                                `<base_path>/definition/template/adl2/<ARCHETYPE_HRID>` \
                                on every `Prefer` outcome \
                                (`Requests_and_responses.md` §Location: used \
                                \"in `201 Created` responses when a new resource \
                                is successfully created\")."),
                ("ETag" = String,
                 description = "\"The `ETag` (i.e. entity tag) response header is \
                                an identifier of Template\" (ITS-REST \
                                `specifications/headers/ETag_Template_adl2.yaml`), \
                                carrying the stored `ARCHETYPE_HRID` in the weak \
                                form the release requires — \"all `ETag` headers \
                                that hold a resource identifier MUST include a \
                                weakness indicator `W/`\" \
                                (`Requests_and_responses.md` §\"ETag and \
                                Last-Modified\"). The HRID carries the artefact's \
                                SEMVER `release_version`, so the tag changes \
                                whenever the artefact does. Shape: \
                                `W/\"openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0\"`."),
                ("Preference-Applied" = String,
                 description = "`return=minimal` | `return=identifier` | \
                                `return=representation` — the preference the \
                                service honoured. \"The service MAY include a \
                                `Preference-Applied` header in the response … to \
                                indicate that the client's preference has been \
                                honored\" (`Requests_and_responses.md` \
                                §\"Representation details negotiation\").")
            ),
            content(
                (String = "text/plain", example = r#"operational_template (adl_version=2.0.6; rm_release=1.0.2; generated)
    openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0

language
    original_language = <[ISO_639-1::en]>

description
    lifecycle_state = <"unmanaged">
...
"#),
                (serde_json::Value = "application/json", example = json!({
                    "template_id": "openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0"
                }))
            )
        ),
        (status = 400, description = "The released trigger, verbatim: `400 Bad \
                                      Request` \"is returned when the request \
                                      could not be parsed or is invalid (e.g. \
                                      malformed request URL syntax, missing \
                                      required header or parameter, or \
                                      syntactically invalid header, parameter or \
                                      content)\" (ITS-REST \
                                      `specifications/responses/400.yaml`). Here: \
                                      the body is not UTF-8 text, or the source \
                                      fails the ADL2 grammar (S-code syntax \
                                      errors — unparseable content, an empty \
                                      body, a missing mandatory section) — \
                                      \"syntactically invalid … content\" is \
                                      precisely that branch, the same split as \
                                      the ADL 1.4 sibling. AOM2 validation-phase \
                                      failures (V-codes) on a source that parses \
                                      are the semantic `422` below.",
         body = serde_json::Value),
        (status = 409, description = "The released trigger, verbatim: `409 \
                                      Conflict` \"is returned when a template \
                                      with same `template_id` already exists\" \
                                      (ITS-REST \
                                      `specifications/responses/409_template_already_exists.yaml`) \
                                      — here, an ADL2 artefact already stored \
                                      under the same `ARCHETYPE_HRID`. The \
                                      released wire wins over the SM \
                                      `upload_artefact`, which would \"replace\" \
                                      the existing artefact \
                                      (`SM/docs/UML/classes/i_definition_adl2.adoc`: \
                                      \"If an artefact with the same physical \
                                      identifier and namespace exists, replace \
                                      it\").",
         body = serde_json::Value),
        (status = 415, description = "The request DECLARES a payload media type \
                                      the service cannot process as this \
                                      operation's `text/plain` ADL2 source: a \
                                      payload the service cannot process in the \
                                      declared format \"MUST respond with HTTP \
                                      status code `415 Unsupported Media Type`\" \
                                      (`Resources.md` §\"XML Format\" / §\"JSON \
                                      Format\" — the same cross-cutting rule that \
                                      grounds the ADL 1.4 guard). An absent \
                                      `Content-Type` declares nothing to refuse \
                                      and is accepted. The released operation \
                                      does not enumerate `415`; the MUST is \
                                      cross-cutting \
                                      (`Requests_and_responses.md` §\"HTTP status \
                                      codes\", the `415` row).",
         body = serde_json::Value),
        (status = 422, description = "OUR WIRE — the ADL2 source parses but fails \
                                      an AOM2 validation phase (V-codes, e.g. a \
                                      missing description block). The `Error` \
                                      body's `validationErrors` carry the rule-code \
                                      mnemonics with their detail \
                                      (`schemas/others/Error.yaml`). Grammar-level \
                                      S-code failures are the syntactic `400` \
                                      above. No released \
                                      response file covers the semantic branch on \
                                      this operation, so the assignment is the \
                                      overview status table's own `422` row — \
                                      \"the request was well-formed but was \
                                      unable to be followed due to semantic \
                                      errors\" (`Requests_and_responses.md` \
                                      §\"HTTP status codes\") — and \"additional \
                                      status codes MAY be used as long as they do \
                                      not conflict with the predefined codes\"; \
                                      it realizes the SM `invalid_artefact` \
                                      outcome of `upload_artefact`.",
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
         description = "\"Template identifier or partial reference. A partial \
                        `template_id` will resolve to “latest” major version of \
                        that template\" (ITS-REST \
                        `specifications/parameters/path/template_id_adl2.yaml`, \
                        examples \
                        `org.highmed::openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0` \
                        and the partial \
                        `openEHR-EHR-COMPOSITION.t_vital_signs.v1`). An ADL2 id \
                        is an `ARCHETYPE_HRID`, so the partial form resolves \
                        against its SEMVER `release_version` axis (AM \
                        `docs/Identification/master04-versioning.adoc`).",
         example = "openEHR-EHR-COMPOSITION.t_vital_signs.v1"),
        ("Accept" = Option<String>, Header,
         description = "`text/plain` (the ADL2 source; also the default for \
                        absent/`*/*`/`text/*`) or `application/json` (the \
                        `OperationalTemplateV2` canonical JSON) — the two \
                        representations \
                        `specifications/responses/200_Template_adl2_retrieved.yaml` \
                        declares content for. \
                        `specifications/parameters/header/Accept_Template_adl2.yaml` \
                        additionally enumerates `application/xml`, for which \
                        that `200` declares NO body, so an `Accept` naming only \
                        it is a `406`. A `;q=0` on a range rejects it (RFC 9110 \
                        §12.5.1); `text/plain` wins when both are acceptable.",
         example = "text/plain")
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_Template_adl2_retrieved.yaml`), \
                                        for the operation that \"retrieves the \
                                        LATEST version of the ADL2 operational \
                                        template identified by `template_id`\" \
                                        (`operations/definition_template_adl2_get.yaml`). \
                                        The `text/plain` body is the stored ADL2 \
                                        source served back verbatim (the released \
                                        body is `oneOf: [OperationalTemplateV2, \
                                        string]` and its example IS ADL2 source); \
                                        the `application/json` body is the \
                                        `OperationalTemplateV2` canonical-JSON \
                                        projection (the released schema \
                                        `schemas/aom/OperationalTemplateV2.yaml` \
                                        is an opaque object, which the AOM2 \
                                        canonical JSON satisfies).",
            headers(
                ("ETag" = String,
                 description = "A SERVER EXTRA, not a declared header: \
                                `200_Template_adl2_retrieved.yaml` declares only \
                                `Content-Type`, and \"servers MAY add additional \
                                `ETag` response headers, consisting of an opaque \
                                quoted string, possibly prefixed by a weakness \
                                indicator\" (`Requests_and_responses.md` §\"ETag \
                                and Last-Modified\"). It carries the RESOLVED \
                                `ARCHETYPE_HRID` of the artefact actually served \
                                — never the partial id that addressed it — in \
                                the weak form (\"all `ETag` headers that hold a \
                                resource identifier MUST include a weakness \
                                indicator `W/`\", same section), and the HRID \
                                carries the artefact's SEMVER `release_version`, \
                                so the tag changes whenever the served artefact \
                                does. Identical across both representations, the \
                                `ETag` being \"independent of its resource \
                                serialization format\". No `Last-Modified`.")
            ),
            content(
                (String = "text/plain", example = r#"operational_template (adl_version=2.0.6; rm_release=1.0.2; generated)
    openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0

language
    original_language = <[ISO_639-1::en]>

description
    lifecycle_state = <"unmanaged">
...
"#),
                (serde_json::Value = "application/json")
            )
        ),
        (status = 404, description = "The released trigger, verbatim: `404 Not \
                                      Found` \"is returned when a template with \
                                      the specified `template_id` does not \
                                      exist\" (ITS-REST \
                                      `specifications/responses/404_unknown_template_id.yaml`) \
                                      — including a partial id that resolves to \
                                      no stored version.",
         body = serde_json::Value),
        (status = 406, description = "OUR REASONED BRANCH — `Accept` names only \
                                      `application/xml`, which \
                                      `parameters/header/Accept_Template_adl2.yaml` \
                                      enumerates but \
                                      `200_Template_adl2_retrieved.yaml` declares \
                                      no body for. With no XML representation to \
                                      produce, the cross-cutting MUST applies: \
                                      \"If the service cannot fulfill this aspect \
                                      of the request, it MUST respond with HTTP \
                                      status code `406 Not Acceptable`\" \
                                      (`Resources.md` §\"XML Format\"). The \
                                      released operation enumerates no `406`; \
                                      that silence is what makes this ours.",
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
         description = "\"Template identifier or partial reference. A partial \
                        `template_id` will resolve to “latest” major version of \
                        that template\" (ITS-REST \
                        `specifications/parameters/path/template_id_adl2.yaml`) \
                        — an `ARCHETYPE_HRID`, resolved over its SEMVER \
                        `release_version` axis as on the sibling `_get`.",
         example = "openEHR-EHR-COMPOSITION.t_vital_signs.v1"),
        ("type" = Option<String>, Query,
         description = "\"Type of use for the data example\" (ITS-REST \
                        `specifications/parameters/query/example_type.yaml`, \
                        enum `[input, output]`, default `input`): `input` is \
                        ready to be submitted to the repository, `output` is as \
                        the instance would appear when retrieved from it.",
         example = "input"),
        ("detail_level" = Option<String>, Query,
         description = "\"The level of data points details of the example\" \
                        (ITS-REST \
                        `specifications/parameters/query/example_detail_level.yaml`, \
                        enum `[required, medium, complete]`, default \
                        `required`), which \"affects: Number and complexity of \
                        nested elements; Inclusion/exclusion of optional \
                        elements; Depth of data point details\" \
                        (`operations/definition_template_adl2_example_get.yaml`). \
                        The per-level MEANINGS and the fallback-or-`400` \
                        latitude are spelled out only on the ADL 1.4 sibling \
                        operation; this server applies them uniformly to both \
                        interfaces — OUR OWN reading, the released ADL2 text \
                        being silent.",
         example = "required"),
        ("Accept" = Option<String>, Header,
         description = "One of `application/json` (default), `application/xml`, \
                        `application/openehr.wt.flat+json`, or \
                        `application/openehr.wt.structured+json` — the four \
                        forms \
                        `specifications/parameters/header/Accept_LOCATABLE.yaml` \
                        enumerates and \
                        `responses/200_Template_example_retrieved.yaml` declares \
                        content for.",
         example = "application/json")
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_Template_example_retrieved.yaml`) \
                                        — here the GENERATED example data \
                                        instance: the stored ADL2 template is \
                                        compiled to its operational template and \
                                        turned into a Web Template (the am24 \
                                        front end), which the shared example \
                                        generator walks into an RM COMPOSITION, \
                                        serialized per `Accept`. No `ETag`: the \
                                        example is derived output, not a stored \
                                        resource with an identifier.",
            content((serde_json::Value = "application/json"), (serde_json::Value = "application/xml"), (serde_json::Value = "application/openehr.wt.flat+json"), (serde_json::Value = "application/openehr.wt.structured+json"))
        ),
        (status = 400, description = "`type` or `detail_level` is outside its \
                                      enumerated set — the error arm of the \
                                      released either/or (\"fall back to the \
                                      closest supported level, or it may return \
                                      an error (typically `400 Bad Request`)\", \
                                      stated on the ADL 1.4 sibling and applied \
                                      uniformly here), and \"syntactically \
                                      invalid … parameter\" under \
                                      `responses/400.yaml`.",
         body = serde_json::Value),
        (status = 404, description = "The released trigger, verbatim: `404 Not \
                                      Found` \"is returned when a template with \
                                      the specified `template_id` does not \
                                      exist\" (ITS-REST \
                                      `specifications/responses/404_unknown_template_id.yaml`).",
         body = serde_json::Value),
        (status = 406, description = "The released trigger, verbatim: `406 Not \
                                      Acceptable` \"is returned when the service \
                                      cannot produce a response matching `Accept` \
                                      request header, i.e. content type or format \
                                      is not supported\" (ITS-REST \
                                      `specifications/responses/406.yaml`) — an \
                                      `Accept` outside the four \
                                      `Accept_LOCATABLE` representations.",
         body = serde_json::Value),
        (status = 422, description = "OUR WIRE — the stored ADL2 template cannot \
                                      be compiled into the operational template \
                                      the example generator needs. No released \
                                      response file covers this; the assignment \
                                      is the overview status table's `422` row, \
                                      \"the request was well-formed but was \
                                      unable to be followed due to semantic \
                                      errors\" (`Requests_and_responses.md` \
                                      §\"HTTP status codes\").",
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
/// DEPRECATED in the released spec but served: the operation file carries
/// `deprecated: true`
/// (`specifications/operations/definition_template_adl2_version_get.yaml`) under
/// amendment SPECITS-87, "Deprecate ADL2 'Get template at version' and the
/// version query parameter from upload template"
/// (`specifications/docs/overview/Amendment_record.md`) — the only operation in
/// this group so marked, and the only declaration here that carries
/// `deprecated`. It is still released text, so it is still served; the release
/// defines no `Deprecation`/`Sunset` signalling for a deprecated operation, so
/// none is emitted (silence, registered as a conformance boundary).
///
/// It resolves `template_id` + the SEMVER `version` (exact or
/// `{major}[.{minor}]` prefix → highest match) and returns the same
/// representations as `_get` (`text/plain` source / `application/json`
/// `OperationalTemplateV2`).
#[utoipa::path(
    get, path = "/definition/template/adl2/{template_id}/{version}", tag = "ADL2",
    params(
        ("template_id" = String, Path,
         description = "\"Template identifier or partial reference\" (ITS-REST \
                        `specifications/parameters/path/template_id.yaml`), here \
                        resolved WITHIN the addressed `version`.",
         example = "openEHR-EHR-COMPOSITION.t_vital_signs"),
        ("version" = String, Path,
         description = "\"A SEMVER version number. This can be an exact version \
                        (e.g. `1.7.1`), or a pattern as partial prefix, in a form \
                        of `{major}` or `{major}.{minor}` (e.g. `1` or `1.0`), in \
                        which case the highest (latest) version matching the \
                        prefix will be considered\" (ITS-REST \
                        `specifications/parameters/path/version.yaml`). Matching \
                        runs over the NUMERIC `major.minor.patch` axis of the \
                        stored `ARCHETYPE_HRID` only: a pre-release/build suffix \
                        on a stored artefact (`…v2.0.0-rc.1`) is ignored when \
                        matching and ordering, and a `{version}` value that \
                        carries such a suffix therefore matches nothing. \"SEMVER \
                        version number\" is all the released text says, so that \
                        reading of the pre-release axis is OURS.",
         example = "1.0"),
        ("Accept" = Option<String>, Header,
         description = "`text/plain` (the ADL2 source; also the default for \
                        absent/`*/*`/`text/*`) or `application/json` (the \
                        `OperationalTemplateV2` canonical JSON) — the two \
                        representations \
                        `specifications/responses/200_Template_adl2_retrieved.yaml` \
                        declares content for. `application/xml`, which \
                        `parameters/header/Accept_Template_adl2.yaml` also \
                        enumerates, has no declared response body, so an `Accept` \
                        naming only it is a `406`.",
         example = "text/plain")
    ),
    responses(
        (
            status = 200, description = "The released trigger, verbatim: `200 OK` \
                                        \"is returned when the template is \
                                        successfully retrieved\" (ITS-REST \
                                        `specifications/responses/200_Template_adl2_retrieved.yaml`), \
                                        here for the artefact at the addressed \
                                        `version`. Same two representations as \
                                        the version-less sibling: the stored ADL2 \
                                        source verbatim (`text/plain`) or the \
                                        `OperationalTemplateV2` canonical-JSON \
                                        projection (`application/json`).",
            headers(
                ("ETag" = String,
                 description = "A SERVER EXTRA, not a declared header: \
                                `200_Template_adl2_retrieved.yaml` declares only \
                                `Content-Type`, and \"servers MAY add additional \
                                `ETag` response headers, consisting of an opaque \
                                quoted string, possibly prefixed by a weakness \
                                indicator\" (`Requests_and_responses.md` §\"ETag \
                                and Last-Modified\"). It carries the RESOLVED \
                                `ARCHETYPE_HRID` of the artefact actually served \
                                — never the `{major}`/`{major}.{minor}` prefix \
                                that addressed it — in the required weak form \
                                (\"all `ETag` headers that hold a resource \
                                identifier MUST include a weakness indicator \
                                `W/`\"), so a client can see which concrete \
                                version a prefix resolved to. No \
                                `Last-Modified`.")
            ),
            content(
                (String = "text/plain", example = r#"operational_template (adl_version=2.0.6; rm_release=1.0.2; generated)
    openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0

language
    original_language = <[ISO_639-1::en]>

description
    lifecycle_state = <"unmanaged">
...
"#),
                (serde_json::Value = "application/json")
            )
        ),
        (status = 404, description = "The released trigger, verbatim: `404 Not \
                                      Found` \"is returned when a template with \
                                      the specified `template_id` at given \
                                      `version` does not exist\" (ITS-REST \
                                      `specifications/responses/404_unknown_template_id_or_version.yaml`) \
                                      — the template is unknown, or no stored \
                                      version matches the exact value or prefix.",
         body = serde_json::Value),
        (status = 406, description = "OUR REASONED BRANCH — `Accept` names only \
                                      `application/xml`, which \
                                      `parameters/header/Accept_Template_adl2.yaml` \
                                      enumerates but \
                                      `200_Template_adl2_retrieved.yaml` declares \
                                      no body for. With no XML representation to \
                                      produce, the cross-cutting MUST applies: \
                                      \"If the service cannot fulfill this aspect \
                                      of the request, it MUST respond with HTTP \
                                      status code `406 Not Acceptable`\" \
                                      (`Resources.md` §\"XML Format\"). The \
                                      released operation enumerates no `406`; \
                                      that silence is what makes this ours.",
         body = serde_json::Value)
    )
)]
// The Rust `#[deprecated]` attribute is what utoipa reflects into the served
// OpenAPI `deprecated: true` (utoipa 5 reads the attribute; its `path` macro has
// no `deprecated` argument), matching the released operation's own
// `deprecated: true` under SPECITS-87.
#[deprecated = "ITS-REST Release-1.1.0 marks this operation deprecated under \
                SPECITS-87 (definition_template_adl2_version_get.yaml); served \
                for compatibility"]
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
