//! `OpenAPI` document + Swagger UI (discoverability).
//!
//! No openEHR spec governs an OAS-serving endpoint; this is our own surface. The
//! Swagger UI's spec selector shows **only documents this server generates
//! itself** — never a vendored OAS. There is exactly one entry, `ehrbase-rest`,
//! and it is the **complete server surface**, composed natively from every
//! `#[utoipa::path]` handler via `utoipa-axum`'s [`OpenApiRouter`]
//! (`.into_openapi()` per area, merged here):
//!
//! - the standardised **ITS-REST API groups** — EHR / COMPOSITION / CONTRIBUTION
//!   / DIRECTORY / DEMOGRAPHIC / DEFINITION / QUERY / ADMIN
//!   ([`crate::api::api_doc`]);
//! - the own-design extension groups (terminology, `PARTY_RELATIONSHIP`,
//!   event-subscription, multi-tenancy, FHIR connector);
//! - the operational endpoints (`/status`, health), the management surface, the
//!   SMART discovery document, and these `OpenAPI` endpoints.
//!
//! Route and `OpenAPI` path are single-sourced from one `#[utoipa::path]` handler —
//! the document cannot drift from the router. The vendored ITS-REST OAS bundles
//! are only the **codegen input** for the generated group contract (`openehr-its`
//! `emit-rest`); they are never served (owner rule: we serve only what we
//! generate, never a vendored OAS).
//!
//! **Authentication in the document is config-driven** (owner requirement): the
//! server accepts Basic *or* OAuth2/OIDC bearer per [`AuthConfig`], and the
//! served document declares exactly the one scheme in effect (`openehr_auth`,
//! bearer JWT when OIDC is configured, else HTTP Basic) — never both, and none
//! when auth is disabled — so the Swagger "Authorize" dialog and the per-endpoint
//! padlocks match the running server. The lock is applied to the authenticated
//! surfaces (the API-nested extension groups + the management surface); the
//! public endpoints (`/status`, health, SMART discovery, these OAS endpoints)
//! carry no requirement.
//!
//! The UI assets are served through [`utoipa_swagger_ui::serve`] directly rather
//! than [`utoipa_swagger_ui::SwaggerUi`]'s router: the router answers the bare
//! mount path with a `303` to the trailing-slash form, which the serve-time
//! `NormalizePathLayer` strips again before routing — an infinite redirect loop.
//! Serving `index.html` for the bare path outright has no redirect to fight.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use ehrbase_sm::Platform;
use utoipa::OpenApi;
use utoipa::openapi::security::{
    Http, HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::{Config, SwaggerFile, Url};

use crate::config::AppConfig;
use crate::extensions::access::authn::config::AuthConfig;
use crate::extensions::management;
use crate::overview::status;
use crate::smart::discovery as smart_discovery;
use crate::state::AppState;

/// The logical name of the single advertised security scheme. One name, one
/// scheme kind chosen by config, so the Swagger "Authorize" dialog shows exactly
/// what the server accepts.
const SECURITY_SCHEME: &str = "openehr_auth";

/// Info + tags carrier for the served document. It declares **no paths** — every
/// path comes from a `#[utoipa::path]` handler collected through `utoipa-axum`
/// (see [`extensions_document`]) — so there is no hand-maintained operation list
/// to drift.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "EHRbase-RS — openEHR ITS-REST + extensions",
        description = "The complete surface this server serves, generated natively from its \
                       handlers: the standardised ITS-REST API groups (EHR / COMPOSITION / \
                       CONTRIBUTION / DIRECTORY / DEMOGRAPHIC / DEFINITION / QUERY / ADMIN), the \
                       own-design extensions (terminology, PARTY_RELATIONSHIP, event-subscription, \
                       multi-tenancy, FHIR connector), and the operational endpoints \
                       (status/health, management, SMART discovery, the OpenAPI endpoints)."
    ),
    tags(
        (name = "ehr", description = "EHR API — EHR, EHR_STATUS, COMPOSITION, DIRECTORY, CONTRIBUTION, item tags (ITS-REST)."),
        (name = "demographic", description = "Demographic API — PERSON/AGENT/GROUP/ORGANISATION/ROLE, versioned reads, contributions, tags (ITS-REST, DEVELOPMENT)."),
        (name = "definition", description = "Definition API — ADL 1.4 / ADL 2 templates + stored AQL queries (ITS-REST)."),
        (name = "query", description = "Query API — ad-hoc + stored AQL execution (ITS-REST)."),
        (name = "admin", description = "Admin API — physical EHR delete (ITS-REST, DEVELOPMENT)."),
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
struct ExtensionsInfo;

/// Compose the served extension-surface `OpenAPI` document from the per-area
/// `utoipa-axum` routers, then declare the config-appropriate security scheme.
///
/// Public surfaces (status/health, SMART discovery, these OAS endpoints) carry
/// no auth requirement; the authenticated surfaces (the API-nested extension
/// groups + the management surface) get the single `openehr_auth` requirement
/// (a per-endpoint padlock) whenever authentication is enabled. The scheme kind
/// (bearer JWT vs HTTP Basic) is chosen by [`advertised_scheme`].
#[must_use]
pub fn extensions_document<S: Platform>(cfg: &AppConfig) -> utoipa::openapi::OpenApi {
    let mut doc = ExtensionsInfo::openapi();

    // Public (no lock): operational status/health, SMART discovery, the OAS
    // meta-endpoints.
    doc.merge(status::openapi::<S>());
    doc.merge(smart_discovery::openapi::<S>());
    doc.merge(meta_openapi::<S>());

    // Authenticated: the management surface + the entire API surface (every
    // ITS-REST standard group + the own-design extension groups, all behind the
    // auth layer). Paths for the API groups are nested under the configured base
    // path so they read as full server paths.
    let mut protected = management::openapi();
    protected.merge(crate::api::api_doc::<S>(&cfg.server.base_path));

    let scheme = advertised_scheme(&cfg.auth);
    if scheme.is_some() {
        require_auth(&mut protected);
    }
    doc.merge(protected);

    if let Some(scheme) = scheme {
        doc.components
            .get_or_insert_with(Default::default)
            .add_security_scheme(SECURITY_SCHEME, scheme);
    }

    doc
}

/// The single security scheme the running server advertises, or `None` when
/// authentication is disabled. Bearer JWT when OIDC is configured, else HTTP
/// Basic — never both (owner requirement: one clean scheme per config). No
/// openEHR spec governs the authorization scheme (ITS-REST places it out of
/// band) — our own operational choice.
fn advertised_scheme(auth: &AuthConfig) -> Option<SecurityScheme> {
    if !auth.enabled {
        return None;
    }
    if auth.oidc.is_some() {
        Some(SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .build(),
        ))
    } else if auth.basic.is_some() {
        Some(SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)))
    } else {
        None
    }
}

/// Stamp the single `openehr_auth` security requirement onto every operation in
/// `doc` (the per-endpoint padlock in Swagger).
fn require_auth(doc: &mut utoipa::openapi::OpenApi) {
    let requirement = SecurityRequirement::new(SECURITY_SCHEME, Vec::<String>::new());
    for item in doc.paths.paths.values_mut() {
        for op in [
            item.get.as_mut(),
            item.put.as_mut(),
            item.post.as_mut(),
            item.delete.as_mut(),
            item.options.as_mut(),
            item.head.as_mut(),
            item.patch.as_mut(),
            item.trace.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            op.security = Some(vec![requirement.clone()]);
        }
    }
}

// ── The OAS meta-endpoints (documented + served by real handlers) ─────────────

/// The served extension-surface `OpenAPI` JSON document. Rebuilt from the
/// request state per call (a pure function of configuration).
#[utoipa::path(
    get, path = "/ehrbase/rest/api-docs/openapi.json", tag = "openapi",
    responses((status = 200, description = "The extension-surface `OpenAPI` document.", body = serde_json::Value))
)]
async fn openapi_json<S: Platform>(State(state): State<AppState<S>>) -> Response {
    let doc = extensions_document::<S>(state.config());
    let body = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".to_owned());
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// The Swagger UI (HTML). The vendored ITS-REST bundles and this extension
/// document are offered in the UI's spec selector.
#[utoipa::path(
    get, path = "/ehrbase/rest/swagger-ui", tag = "openapi",
    responses((status = 200, description = "The Swagger UI index.", content_type = "text/html"))
)]
async fn swagger_ui_index<S: Platform>(State(state): State<AppState<S>>) -> Response {
    let config = swagger_config(state.config());
    serve_ui_file("", &config)
}

/// The OAS meta-endpoints' `OpenAPI` (paths at the default REST root; a
/// non-default base path shifts them uniformly). Public (no auth requirement).
fn meta_openapi<S: Platform>() -> utoipa::openapi::OpenApi {
    OpenApiRouter::<AppState<S>>::new()
        .routes(routes!(openapi_json))
        .routes(routes!(swagger_ui_index))
        .into_openapi()
}

// ── The Swagger router (serving; kept on the loop-free `serve()` mechanism) ───

/// Build the docs router: the Swagger UI (loop-free) and the single
/// `ehrbase-rest` extension-surface document (our own natively generated OAS).
/// Config paths come from [`AppConfig`]. No vendored OAS is served (owner rule).
pub(crate) fn swagger_router<S: Platform>(cfg: &AppConfig) -> Router<AppState<S>> {
    let ui_path = cfg.server.swagger_ui_path();
    let json_path = cfg.server.openapi_json_path();

    // Our own extension-surface document (utoipa-composed, config-driven auth).
    let router = Router::new().route(&json_path, get(openapi_json::<S>));

    // The UI itself: assets straight from the embedded dist. The bare mount path
    // serves index.html (serve() maps "" to it) — no redirect, no loop.
    let config = swagger_config(cfg);
    router.route(&ui_path, get(swagger_ui_index::<S>)).route(
        &format!("{ui_path}/{{*file}}"),
        get(move |Path(file): Path<String>| {
            let cfg = Arc::clone(&config);
            async move { serve_ui_file(&file, &cfg) }
        }),
    )
}

/// The Swagger UI spec-selector config: a single `ehrbase-rest` entry (our
/// composed document). The URL must outlive the server, so the derived path is
/// leaked once at construction (the UI config requires `'static`).
fn swagger_config(cfg: &AppConfig) -> Arc<Config<'static>> {
    let json_path = cfg.server.openapi_json_path();
    let urls: Vec<Url<'static>> = vec![Url::new("ehrbase-rest", json_path.leak())];
    Arc::new(Config::new(urls))
}

/// Serve one embedded Swagger UI asset (`""` → `index.html`).
fn serve_ui_file(file: &str, config: &Arc<Config<'static>>) -> Response {
    match utoipa_swagger_ui::serve(file, Arc::clone(config)) {
        Ok(Some(SwaggerFile {
            bytes,
            content_type,
            ..
        })) => ([(header::CONTENT_TYPE, content_type)], bytes.into_owned()).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
