//! Generated `OpenAPI` document for the server's **extension surface** — every
//! path this server actually serves that the vendored ITS-REST `OpenAPI` bundles
//! (`openehr_its::rest::VENDORED_OAS`, served by [`crate::extensions::openapi`])
//! do **not** describe.
//!
//! **No openEHR spec governs an OAS-serving endpoint, nor most of what is
//! documented here — this is our own operational + extension surface.** The
//! vendored bundles are the authoritative contract for the standardised API
//! groups (EHR / COMPOSITION / DIRECTORY / CONTRIBUTION / DEMOGRAPHIC /
//! DEFINITION / QUERY / ADMIN-delete). This document covers the remainder:
//!
//! - the public operational endpoints (`/status`, health) —
//!   [`crate::overview::status`];
//! - the management surface (`/management/*`) — [`crate::extensions::management`];
//! - the SMART service-discovery document — [`crate::smart::discovery`]
//!   (operation semantics cited from ITS-REST `smart_app_launch/master04`);
//! - the `OpenAPI`/Swagger discoverability endpoints —
//!   [`crate::extensions::openapi`];
//! - the `/terminology` extension wire — [`crate::extensions::terminology`]
//!   (operation semantics cited from SM `master12` `I_TERMINOLOGY_SERVICE`; the
//!   wire shape is ours);
//! - the `PARTY_RELATIONSHIP` demographic extension —
//!   [`crate::api::demographic`] `RELATIONSHIP_ROUTES` (SM-3; no ITS-REST
//!   contract);
//! - the enterprise extensions with no spec governance: event-subscription CRUD
//!   ([`crate::extensions::event_subscription`]), multi-tenancy admin
//!   ([`crate::extensions::tenant_routes`]), and the FHIR R4 connector
//!   ([`crate::extensions::fhir`]).
//!
//! Config-gated surfaces are documented **unconditionally** (the document
//! describes the product surface, not one deployment's live routing); each such
//! operation's `description` names the `EHRBASE_REST_*` flag that mounts it.
//!
//! ## Why documentation-only stubs, not handler annotations
//!
//! The extension handlers are not 1:1 with operations: the terminology / FHIR /
//! event-subscription / tenancy / relationship groups each route a whole
//! `*_ROUTES` table through **one** multiplexed `dispatch(state, op, parts)`
//! function, and `/status` / SMART discovery / Swagger are served by closures.
//! `#[utoipa::path]` is one-per-operation, so the annotations live on dedicated
//! documentation stubs here (bodies empty; they exist only to carry the
//! attribute). No handler behaviour is touched. Each stub's `path`/`method`/
//! status set was read off the owning handler; request/response bodies that are
//! openEHR RM canonical JSON (or otherwise free-form) are typed
//! `serde_json::Value` with the shape described in prose rather than fabricating
//! an RM schema.
//!
//! The document is exposed by [`crate::extensions::openapi`] as a spec-selector
//! entry beside the vendored bundles.
//!
//! The functions below are documentation stubs: they carry a `#[utoipa::path]`
//! attribute (which generates the path metadata the `#[derive(OpenApi)]` gathers)
//! and are otherwise unused, hence the module-level `dead_code` allow.
#![allow(dead_code)]

use utoipa::OpenApi;

/// The composed `OpenAPI` document for the extension surface. Serve via
/// [`ExtensionsApiDoc::openapi()`].
#[derive(OpenApi)]
#[openapi(
    info(
        title = "EHRbase-RS — extension surface",
        description = "Operational + extension endpoints this server serves that the vendored \
                       ITS-REST `OpenAPI` bundles do not describe: status/health, the management \
                       surface, SMART discovery, the `OpenAPI` endpoints, and the config-gated \
                       terminology / PARTY_RELATIONSHIP / event-subscription / multi-tenancy / \
                       FHIR-connector extensions. No openEHR spec governs these — our own \
                       operational + extension design."
    ),
    paths(
        // Operational / status
        status_status,
        status_root_health,
        status_status_health,
        // SMART discovery
        smart_configuration,
        // `OpenAPI` / Swagger
        openapi_identity_json,
        swagger_ui_index,
        // Management
        management_health,
        management_liveness,
        management_readiness,
        management_info,
        management_prometheus,
        management_metrics_list,
        management_metrics_detail,
        management_env,
        management_loggers_get,
        management_loggers_post,
        management_loggers_delete,
        // Terminology extension
        terminology_ids,
        terminology_description,
        terminology_get_term,
        terminology_subsumes,
        terminology_value_set,
        terminology_value_set_validate,
        // PARTY_RELATIONSHIP demographic extension
        relationship_create,
        relationship_get,
        relationship_update,
        relationship_delete,
        versioned_relationship_get,
        relationship_revision_history,
        relationship_version_at_time,
        relationship_version_by_id,
        // Event-subscription extension
        event_subscription_list,
        event_subscription_create,
        event_subscription_get,
        event_subscription_update,
        event_subscription_delete,
        // Multi-tenancy admin extension
        tenant_list,
        tenant_create,
        tenant_get,
        tenant_update,
        tenant_delete,
        // FHIR R4 connector extension
        fhir_ingest,
        fhir_search,
        fhir_mapping_list,
        fhir_mapping_create,
        fhir_mapping_get,
        fhir_mapping_update,
        fhir_mapping_delete,
    ),
    tags(
        (name = "status", description = "Public operational status + health (unauthenticated)."),
        (name = "smart", description = "SMART App Launch service discovery (config-gated: EHRBASE_REST_SMART__ENABLED)."),
        (name = "openapi", description = "`OpenAPI` document + Swagger UI discoverability (config-gated: EHRBASE_REST_SWAGGER_UI)."),
        (name = "management", description = "Operational management surface (config-gated: EHRBASE_REST_MANAGEMENT__*); each endpoint opt-in via its access level."),
        (name = "terminology", description = "Terminology extension wire — SM I_TERMINOLOGY_SERVICE (config-gated: EHRBASE_REST_TERMINOLOGY__ENABLED)."),
        (name = "demographic-relationship", description = "PARTY_RELATIONSHIP demographic extension (SM-3; no ITS-REST contract)."),
        (name = "event-subscription", description = "Event-subscription CRUD extension (config-gated: EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED)."),
        (name = "tenancy", description = "Multi-tenancy admin extension (config-gated: EHRBASE_REST_TENANCY__ENABLED)."),
        (name = "fhir", description = "FHIR R4 inbound connector + mapping store (config-gated: EHRBASE_REST_FHIR__ENABLED)."),
    )
)]
#[derive(Debug)]
pub struct ExtensionsApiDoc;

// The full served path is the configured base path (default
// `/ehrbase/rest/openehr/v1`) for API-nested routes, and the REST root
// (`/ehrbase/rest`) for the operational endpoints. Paths below use the defaults;
// a non-default `base_path` shifts them uniformly.

// ── Operational / status (crate::overview::status) ───────────────────────────

/// Server status: reports the server version and the tested ITS-REST contract
/// identity. Unauthenticated. Body: `{status, server_version,
/// openehr_rest_api_version, timestamp}`.
#[utoipa::path(
    get, path = "/ehrbase/rest/status", tag = "status",
    responses((status = 200, description = "Server up.", body = serde_json::Value))
)]
fn status_status() {}

/// Liveness text probe (`OK`). Unauthenticated.
#[utoipa::path(
    get, path = "/health", tag = "status",
    responses((status = 200, description = "Server process alive.", body = String))
)]
fn status_root_health() {}

/// Health text probe under the REST root (`OK`). Unauthenticated.
#[utoipa::path(
    get, path = "/ehrbase/rest/status/health", tag = "status",
    responses((status = 200, description = "Server process alive.", body = String))
)]
fn status_status_health() {}

// ── SMART discovery (crate::smart::discovery) ────────────────────────────────

/// SMART App Launch service-discovery document (ITS-REST
/// `smart_app_launch/master04` §Service Discovery). Unauthenticated,
/// `application/json`. Config-gated: `EHRBASE_REST_SMART__ENABLED` (absent →
/// 404 when SMART is disabled).
#[utoipa::path(
    get, path = "/ehrbase/rest/.well-known/smart-configuration", tag = "smart",
    responses((status = 200, description = "The SMART configuration document.", body = serde_json::Value))
)]
fn smart_configuration() {}

// ── `OpenAPI` / Swagger (crate::extensions::openapi) ───────────────────────────

/// The identity `OpenAPI` JSON document (API metadata + contract provenance).
/// Config-gated: `EHRBASE_REST_SWAGGER_UI` (default on).
#[utoipa::path(
    get, path = "/ehrbase/rest/api-docs/openapi.json", tag = "openapi",
    responses((status = 200, description = "The identity `OpenAPI` document.", body = serde_json::Value))
)]
fn openapi_identity_json() {}

/// The Swagger UI (HTML). The vendored ITS-REST bundles and this extension
/// document are offered in the UI's spec selector. Config-gated:
/// `EHRBASE_REST_SWAGGER_UI` (default on).
#[utoipa::path(
    get, path = "/ehrbase/rest/swagger-ui", tag = "openapi",
    responses((status = 200, description = "The Swagger UI index.", content_type = "text/html"))
)]
fn swagger_ui_index() {}

// ── Management surface (crate::extensions::management) ────────────────────────
// All config-gated by EHRBASE_REST_MANAGEMENT__ENABLED + each endpoint's own
// access level; served on the management base path (default `/management`), or
// on a separate management port when configured.

/// Aggregate health (all registered indicators). 200 when UP/DEGRADED, 503 when
/// DOWN. Access-level gated.
#[utoipa::path(
    get, path = "/management/health", tag = "management",
    responses(
        (status = 200, description = "Aggregate health UP or DEGRADED.", body = serde_json::Value),
        (status = 503, description = "Aggregate health DOWN.", body = serde_json::Value)
    )
)]
fn management_health() {}

/// Kubernetes-style liveness probe (public when probes are enabled).
#[utoipa::path(
    get, path = "/management/health/liveness", tag = "management",
    responses((status = 200, description = "Process alive.", body = serde_json::Value))
)]
fn management_liveness() {}

/// Kubernetes-style readiness probe (public when probes are enabled). 503 when
/// not ready.
#[utoipa::path(
    get, path = "/management/health/readiness", tag = "management",
    responses(
        (status = 200, description = "Ready to serve.", body = serde_json::Value),
        (status = 503, description = "Not ready.", body = serde_json::Value)
    )
)]
fn management_readiness() {}

/// Build/spec provenance (`/info`): version, git, spec pins. Access-level gated.
#[utoipa::path(
    get, path = "/management/info", tag = "management",
    responses((status = 200, description = "Build + spec provenance.", body = serde_json::Value))
)]
fn management_info() {}

/// Prometheus text exposition. 503 when the recorder is not installed.
/// Access-level gated.
#[utoipa::path(
    get, path = "/management/prometheus", tag = "management",
    responses(
        (status = 200, description = "Prometheus exposition text.", content_type = "text/plain"),
        (status = 503, description = "Metrics recorder not installed.", body = serde_json::Value)
    )
)]
fn management_prometheus() {}

/// Actuator-style JSON list of known metric names. 503 when the recorder is not
/// installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/metrics", tag = "management",
    responses(
        (status = 200, description = "Known metric names.", body = serde_json::Value),
        (status = 503, description = "Metrics recorder not installed.", body = serde_json::Value)
    )
)]
fn management_metrics_list() {}

/// Actuator-style JSON detail for one metric. 404 when the metric is unknown,
/// 503 when the recorder is not installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/metrics/{name}", tag = "management",
    params(("name" = String, Path, description = "The metric name.")),
    responses(
        (status = 200, description = "The metric's current value(s).", body = serde_json::Value),
        (status = 404, description = "Unknown metric.", body = serde_json::Value)
    )
)]
fn management_metrics_detail() {}

/// The redacted effective-configuration snapshot (`/env`). Access-level gated.
#[utoipa::path(
    get, path = "/management/env", tag = "management",
    responses((status = 200, description = "Redacted effective configuration.", body = serde_json::Value))
)]
fn management_env() {}

/// The effective log-filter directives + boot filter. 503 when no reloadable
/// filter is installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/loggers", tag = "management",
    responses(
        (status = 200, description = "Effective + boot log filter.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
fn management_loggers_get() {}

/// Swap the live log filter. Body: `{"filter": "ehrbase=debug,sqlx=warn"}`. 400
/// on a parse error, 503 when no reloadable filter is installed. Access-level
/// gated.
#[utoipa::path(
    post, path = "/management/loggers", tag = "management",
    request_body(content = serde_json::Value, description = "`{\"filter\": \"<env-filter directives>\"}`"),
    responses(
        (status = 200, description = "Filter applied.", body = serde_json::Value),
        (status = 400, description = "Malformed filter directives.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
fn management_loggers_post() {}

/// Reset the log filter to the boot filter. 503 when no reloadable filter is
/// installed. Access-level gated.
#[utoipa::path(
    delete, path = "/management/loggers", tag = "management",
    responses(
        (status = 200, description = "Filter reset to boot value.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
fn management_loggers_delete() {}

// ── Terminology extension (crate::extensions::terminology) ────────────────────
// SM master12 I_TERMINOLOGY_SERVICE semantics; our own wire shape. Config-gated:
// EHRBASE_REST_TERMINOLOGY__ENABLED.

/// Every terminology id the server knows (`get_terminology_ids`). Body:
/// `{"terminology_ids": [..]}`.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology", tag = "terminology",
    responses((status = 200, description = "The known terminology ids.", body = serde_json::Value))
)]
fn terminology_ids() {}

/// One terminology's descriptor (`get_terminology_description`; also the
/// `has_terminology` existence check). 404 when unknown.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology/{terminology_id}", tag = "terminology",
    params(("terminology_id" = String, Path, description = "The terminology id.")),
    responses(
        (status = 200, description = "The terminology descriptor.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology.", body = serde_json::Value)
    )
)]
fn terminology_description() {}

/// A term definition (`get_term`). Optional `at_date`. 404 when unknown.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology/{terminology_id}/term/{code}", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("code" = String, Path, description = "The term code."),
        ("at_date" = Option<String>, Query, description = "Optional ISO-8601 effective date.")
    ),
    responses(
        (status = 200, description = "The term extract.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology or code.", body = serde_json::Value)
    )
)]
fn terminology_get_term() {}

/// Strict subsumption test (`subsumes`). Body: `{"subsumes": bool}`. 400 when a
/// required query parameter is missing.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology/{terminology_id}/subsumes", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("ref_code" = String, Query, description = "The reference (ancestor-candidate) code."),
        ("candidate" = String, Query, description = "The candidate (descendant) code.")
    ),
    responses(
        (status = 200, description = "The subsumption result.", body = serde_json::Value),
        (status = 400, description = "Missing required query parameter.", body = serde_json::Value)
    )
)]
fn terminology_subsumes() {}

/// A value set's extract (`get_value_set`; also the `has_value_set` existence
/// check). 404 when unknown.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("value_set_id" = String, Path, description = "The value set id.")
    ),
    responses(
        (status = 200, description = "The value set extract.", body = serde_json::Value),
        (status = 404, description = "Unknown terminology or value set.", body = serde_json::Value)
    )
)]
fn terminology_value_set() {}

/// Value-set membership test (`value_set_validate`). Body: `{"valid": bool}`.
/// 400 when `candidate_code` is missing.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}/validate", tag = "terminology",
    params(
        ("terminology_id" = String, Path, description = "The terminology id."),
        ("value_set_id" = String, Path, description = "The value set id."),
        ("candidate_code" = String, Query, description = "The candidate code to test for membership."),
        ("at_date" = Option<String>, Query, description = "Optional ISO-8601 effective date.")
    ),
    responses(
        (status = 200, description = "The membership result.", body = serde_json::Value),
        (status = 400, description = "Missing required query parameter.", body = serde_json::Value)
    )
)]
fn terminology_value_set_validate() {}

// ── PARTY_RELATIONSHIP demographic extension (crate::api::demographic) ─────────
// SM-3; no ITS-REST contract. Always mounted (not config-gated). Request/response
// bodies are RM canonical JSON (PARTY_RELATIONSHIP / VERSIONED_OBJECT / VERSION).

/// Create a `PARTY_RELATIONSHIP` (RM canonical JSON body). 201 with the created
/// resource; ETag/Location headers.
#[utoipa::path(
    post, path = "/ehrbase/rest/openehr/v1/demographic/party_relationship", tag = "demographic-relationship",
    request_body(content = serde_json::Value, description = "An RM PARTY_RELATIONSHIP (canonical JSON)."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
fn relationship_create() {}

/// Read a `PARTY_RELATIONSHIP` by uid-based id. 404 when absent.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    responses(
        (status = 200, description = "The relationship (RM canonical JSON).", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
fn relationship_get() {}

/// Update a `PARTY_RELATIONSHIP` (If-Match required; RM canonical JSON body).
#[utoipa::path(
    put, path = "/ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    request_body(content = serde_json::Value, description = "The updated RM PARTY_RELATIONSHIP (canonical JSON)."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
fn relationship_update() {}

/// Delete a `PARTY_RELATIONSHIP` (If-Match required).
#[utoipa::path(
    delete, path = "/ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}", tag = "demographic-relationship",
    params(("uid_based_id" = String, Path, description = "The relationship uid-based id.")),
    responses((status = 204, description = "Deleted."))
)]
fn relationship_delete() {}

/// Read the `VERSIONED_PARTY_RELATIONSHIP` container.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}", tag = "demographic-relationship",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses((status = 200, description = "The VERSIONED_PARTY_RELATIONSHIP (RM canonical JSON).", body = serde_json::Value))
)]
fn versioned_relationship_get() {}

/// The relationship's `REVISION_HISTORY`.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history", tag = "demographic-relationship",
    params(("versioned_object_uid" = String, Path, description = "The versioned-object uid.")),
    responses((status = 200, description = "The REVISION_HISTORY (RM canonical JSON).", body = serde_json::Value))
)]
fn relationship_revision_history() {}

/// The relationship VERSION at a point in time (`?version_at_time=`).
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path, description = "The versioned-object uid."),
        ("version_at_time" = Option<String>, Query, description = "Optional ISO-8601 instant; latest when omitted.")
    ),
    responses((status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value))
)]
fn relationship_version_at_time() {}

/// A specific relationship VERSION by version uid.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}", tag = "demographic-relationship",
    params(
        ("versioned_object_uid" = String, Path, description = "The versioned-object uid."),
        ("version_uid" = String, Path, description = "The OBJECT_VERSION_ID.")
    ),
    responses((status = 200, description = "The VERSION (RM canonical JSON).", body = serde_json::Value))
)]
fn relationship_version_by_id() {}

// ── Event-subscription extension (crate::extensions::event_subscription) ──────
// No openEHR spec governs eventing. Config-gated:
// EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED. Mounted under /admin (Admin RBAC class).

/// List every event subscription.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/event_subscription", tag = "event-subscription",
    responses((status = 200, description = "The subscription records.", body = serde_json::Value))
)]
fn event_subscription_list() {}

/// Create a subscription. Body: `{name, kind?, change_type?, template_id?,
/// archetype?, enabled?}`.
#[utoipa::path(
    post, path = "/ehrbase/rest/openehr/v1/admin/event_subscription", tag = "event-subscription",
    request_body(content = serde_json::Value, description = "The subscription definition."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
fn event_subscription_create() {}

/// Read one subscription by id. 404 when absent.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    responses(
        (status = 200, description = "The subscription record.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
fn event_subscription_get() {}

/// Replace one subscription's predicates + enabled flag.
#[utoipa::path(
    put, path = "/ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    request_body(content = serde_json::Value, description = "The updated subscription definition."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
fn event_subscription_update() {}

/// Delete one subscription.
#[utoipa::path(
    delete, path = "/ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}", tag = "event-subscription",
    params(("subscription_id" = String, Path, description = "The subscription UUID.")),
    responses((status = 204, description = "Deleted."))
)]
fn event_subscription_delete() {}

// ── Multi-tenancy admin extension (crate::extensions::tenant_routes) ──────────
// No openEHR spec governs tenancy. Config-gated: EHRBASE_REST_TENANCY__ENABLED.

/// List every tenant.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/tenant", tag = "tenancy",
    responses((status = 200, description = "The tenant records.", body = serde_json::Value))
)]
fn tenant_list() {}

/// Create a tenant. Body: `{name, system_id}`.
#[utoipa::path(
    post, path = "/ehrbase/rest/openehr/v1/admin/tenant", tag = "tenancy",
    request_body(content = serde_json::Value, description = "The tenant definition."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
fn tenant_create() {}

/// Read one tenant by id. 404 when absent.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    responses(
        (status = 200, description = "The tenant record.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
fn tenant_get() {}

/// Update one tenant's name/`system_id`.
#[utoipa::path(
    put, path = "/ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    request_body(content = serde_json::Value, description = "The updated tenant definition."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
fn tenant_update() {}

/// Delete one tenant (only when empty and not the reserved default).
#[utoipa::path(
    delete, path = "/ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}", tag = "tenancy",
    params(("tenant_id" = String, Path, description = "The tenant UUID.")),
    responses((status = 204, description = "Deleted."))
)]
fn tenant_delete() {}

// ── FHIR R4 connector extension (crate::extensions::fhir) ─────────────────────
// No openEHR spec governs FHIR interop. Config-gated: EHRBASE_REST_FHIR__ENABLED.
// Errors on this surface are FHIR OperationOutcome (application/fhir+json), not
// the openEHR error body.

/// Inbound connector: commit a FHIR R4 resource as an openEHR COMPOSITION. Only
/// the starter set (Patient, Observation, Condition, `DocumentReference`) is
/// supported; anything else is 501. Responses are `application/fhir+json`.
#[utoipa::path(
    post, path = "/ehrbase/rest/openehr/v1/fhir/r4/{resource_type}", tag = "fhir",
    params(("resource_type" = String, Path, description = "The FHIR R4 resource type (starter set only).")),
    request_body(content = serde_json::Value, description = "A FHIR R4 resource (JSON)."),
    responses(
        (status = 201, description = "Committed as a COMPOSITION (informational OperationOutcome + ETag/Location).", content_type = "application/fhir+json"),
        (status = 422, description = "Mapped COMPOSITION failed validation (OperationOutcome).", content_type = "application/fhir+json"),
        (status = 501, description = "Resource type outside the starter set (OperationOutcome).", content_type = "application/fhir+json")
    )
)]
fn fhir_ingest() {}

/// Read façade: a patient-scoped FHIR searchset Bundle of reverse-mapped
/// resources. `patient` is mandatory. Responses are `application/fhir+json`.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/fhir/r4/{resource_type}", tag = "fhir",
    params(
        ("resource_type" = String, Path, description = "The FHIR R4 resource type (starter set only)."),
        ("patient" = String, Query, description = "The patient scope (EHR subject or id) — required."),
        ("_count" = Option<i64>, Query, description = "Optional page size.")
    ),
    responses(
        (status = 200, description = "A FHIR searchset Bundle.", content_type = "application/fhir+json"),
        (status = 400, description = "Missing `patient` scope (OperationOutcome).", content_type = "application/fhir+json"),
        (status = 501, description = "Resource type outside the starter set (OperationOutcome).", content_type = "application/fhir+json")
    )
)]
fn fhir_search() {}

/// List the FHIR mapping artefacts (mapping-as-data).
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/fhir_mapping", tag = "fhir",
    responses((status = 200, description = "The mapping records.", body = serde_json::Value))
)]
fn fhir_mapping_list() {}

/// Create a FHIR mapping artefact.
#[utoipa::path(
    post, path = "/ehrbase/rest/openehr/v1/admin/fhir_mapping", tag = "fhir",
    request_body(content = serde_json::Value, description = "The mapping definition."),
    responses((status = 201, description = "Created.", body = serde_json::Value))
)]
fn fhir_mapping_create() {}

/// Read one FHIR mapping artefact by id. 404 when absent.
#[utoipa::path(
    get, path = "/ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}", tag = "fhir",
    params(("mapping_id" = String, Path, description = "The mapping UUID.")),
    responses(
        (status = 200, description = "The mapping record.", body = serde_json::Value),
        (status = 404, description = "Not found.", body = serde_json::Value)
    )
)]
fn fhir_mapping_get() {}

/// Update one FHIR mapping artefact.
#[utoipa::path(
    put, path = "/ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}", tag = "fhir",
    params(("mapping_id" = String, Path, description = "The mapping UUID.")),
    request_body(content = serde_json::Value, description = "The updated mapping definition."),
    responses((status = 200, description = "Updated.", body = serde_json::Value))
)]
fn fhir_mapping_update() {}

/// Delete one FHIR mapping artefact.
#[utoipa::path(
    delete, path = "/ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}", tag = "fhir",
    params(("mapping_id" = String, Path, description = "The mapping UUID.")),
    responses((status = 204, description = "Deleted."))
)]
fn fhir_mapping_delete() {}
