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
use bytes::Bytes;
use utoipa::OpenApi;
use utoipa::openapi::security::{
    Http, HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::{Config, SwaggerFile, Url};

use crate::config::AppConfig;
use crate::extensions::management;
use crate::overview::status;
use crate::smart::discovery as smart_discovery;
use crate::state::AppState;
use ehrbase::config::auth::AuthConfig;

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
        // ITS-REST resource tags (categorised exactly as the vendored group OAS
        // documents do — one tag per RM resource, cross-referenced by path).
        (name = "EHR", description = "Management of EHRs — create/retrieve; physical delete (Admin API)."),
        (name = "EHR_STATUS", description = "Management of EHR_STATUS and its VERSIONED_EHR_STATUS reads."),
        (name = "COMPOSITION", description = "Management of COMPOSITION and its VERSIONED_COMPOSITION reads."),
        (name = "DIRECTORY", description = "Management of the directory (FOLDER) tree."),
        (name = "CONTRIBUTION", description = "Management of CONTRIBUTION (EHR + demographic)."),
        (name = "ITEM_TAG", description = "Management of ITEM_TAG sub-resources (EHR + demographic)."),
        (name = "PERSON", description = "Management of the demographic PERSON (ITS-REST, DEVELOPMENT)."),
        (name = "AGENT", description = "Management of the demographic AGENT (ITS-REST, DEVELOPMENT)."),
        (name = "GROUP", description = "Management of the demographic GROUP (ITS-REST, DEVELOPMENT)."),
        (name = "ORGANISATION", description = "Management of the demographic ORGANISATION (ITS-REST, DEVELOPMENT)."),
        (name = "ROLE", description = "Management of the demographic ROLE (ITS-REST, DEVELOPMENT)."),
        (name = "VERSIONED_PARTY", description = "Management of the VERSIONED_PARTY reads (ITS-REST, DEVELOPMENT)."),
        (name = "ADL1.4", description = "Management of AOM/ADL 1.4 operational templates."),
        (name = "ADL2", description = "Management of AOM2/ADL 2 templates."),
        (name = "Query", description = "Ad-hoc + stored AQL execution and stored-query definitions (ITS-REST)."),
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
pub fn extensions_document(cfg: &AppConfig) -> utoipa::openapi::OpenApi {
    let mut doc = ExtensionsInfo::openapi();

    // Public (no lock): operational status/health, SMART discovery, the OAS
    // meta-endpoints.
    doc.merge(status::openapi());
    doc.merge(smart_discovery::openapi());
    doc.merge(meta_openapi());

    // Authenticated: the management surface + the entire API surface (every
    // ITS-REST standard group + the own-design extension groups, all behind the
    // auth layer). Paths for the API groups are nested under the configured base
    // path so they read as full server paths.
    let mut protected = management::openapi();
    protected.merge(crate::api::api_doc(&cfg.server.base_path));

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

/// The served `OpenAPI` JSON document — the complete server surface
/// (`GET /ehrbase/rest/api-docs/openapi.json`).
///
/// No openEHR spec governs an OAS-serving endpoint — our own discoverability
/// surface. Public (no auth requirement) and always `200 application/json`.
/// Config-gated by the Swagger UI: when it is disabled the route is not mounted
/// and the path is absent (a router `404`). This handler carries the
/// `#[utoipa::path]` metadata that puts the endpoint into the composed document
/// (via [`meta_openapi`]); the **live** route serves the document pre-serialized
/// once at assembly ([`prebuild_docs`], [`swagger_router`]), so this body runs
/// only if the endpoint is ever mounted directly.
#[utoipa::path(
    get, path = "/ehrbase/rest/api-docs/openapi.json", tag = "openapi",
    responses((status = 200, description = "The complete server-surface `OpenAPI` document (JSON).", body = serde_json::Value))
)]
async fn openapi_json(State(state): State<AppState>) -> Response {
    let doc = extensions_document(state.config());
    let body = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".to_owned());
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// The Swagger UI index (`GET /ehrbase/rest/swagger-ui`).
///
/// No openEHR spec governs a Swagger UI — our own discoverability surface.
/// Public (no auth requirement); config-gated by the Swagger UI (absent — a
/// router `404` — when disabled). Serves the embedded `index.html`
/// (`200 text/html`); the sibling asset route (`{*file}`, a plain route, not
/// documented here) answers `404` for an unknown asset. The spec selector
/// offers one document per API family — the standardised ITS-REST groups
/// (`openEHR — …`, selected by resource path) and the server's own extensions
/// (`EHRbase — …`, selected by tag) — each filtered from the one
/// server-generated composed document, plus that complete document last.
/// Nothing vendored is served.
#[utoipa::path(
    get, path = "/ehrbase/rest/swagger-ui", tag = "openapi",
    responses((status = 200, description = "The Swagger UI index page.", content_type = "text/html"))
)]
async fn swagger_ui_index(State(state): State<AppState>) -> Response {
    let config = swagger_config(state.config());
    serve_ui_file("", &config)
}

/// The OAS meta-endpoints' `OpenAPI` (paths at the default REST root; a
/// non-default base path shifts them uniformly). Public (no auth requirement).
fn meta_openapi() -> utoipa::openapi::OpenApi {
    OpenApiRouter::<AppState>::new()
        .routes(routes!(openapi_json))
        .routes(routes!(swagger_ui_index))
        .into_openapi()
}

// ── The spec-selector families ────────────────────────────────────────────────

/// How a spec-selector family picks its operations out of the one composed
/// server document.
///
/// The standardised ITS-REST groups categorise by **resource path** (their
/// operations carry per-resource tags — `EHR`/`EHR_STATUS`/`COMPOSITION`/…,
/// exactly as the vendored group OAS documents tag them — so a shared tag no
/// longer identifies a group; the base-path-relative path root does). The
/// server's own extensions have no shared path root and are still identified by
/// **tag**.
enum Members {
    /// Operations whose base-path-relative path starts with `include` (on a
    /// segment boundary) and starts with none of `exclude`.
    Path {
        include: &'static str,
        exclude: &'static [&'static str],
    },
    /// Operations carrying one of these tags.
    Tags(&'static [&'static str]),
}

/// Every API family offered as its own selector document:
/// `(selector name, url slug, membership criterion)`. ALL of them are filtered
/// from the one server-generated composed document — nothing vendored is served
/// (the vendored ITS-REST bundles are reference material for authoring our
/// schemas, never a served artifact) — so each family document inherits the
/// config-driven security scheme and can never drift from the router.
const FAMILIES: &[(&str, &str, Members)] = &[
    // The standardised ITS-REST groups (our generated wire), by resource path.
    (
        "openEHR — EHR",
        "ehr",
        Members::Path {
            include: "/ehr",
            exclude: &[],
        },
    ),
    (
        "openEHR — Query",
        "query",
        Members::Path {
            include: "/query",
            exclude: &[],
        },
    ),
    (
        "openEHR — Definition",
        "definition",
        Members::Path {
            include: "/definition",
            exclude: &[],
        },
    ),
    (
        "openEHR — Demographic",
        "demographic",
        Members::Path {
            include: "/demographic",
            // The own-design PARTY_RELATIONSHIP extension shares the /demographic
            // root but is its own family (below).
            exclude: &["/demographic/party_relationship"],
        },
    ),
    (
        "openEHR — Admin",
        "admin",
        Members::Path {
            include: "/admin/ehr",
            exclude: &[],
        },
    ),
    // The server's own extension families, by tag.
    (
        "EHRbase — Status & Management",
        "management",
        Members::Tags(&["status", "management", "openapi"]),
    ),
    (
        "EHRbase — Terminology",
        "terminology",
        Members::Tags(&["terminology"]),
    ),
    (
        "EHRbase — Party Relationships",
        "relationships",
        Members::Tags(&["demographic-relationship"]),
    ),
    (
        "EHRbase — Event Subscriptions",
        "events",
        Members::Tags(&["event-subscription"]),
    ),
    (
        "EHRbase — Multi-tenancy",
        "tenancy",
        Members::Tags(&["tenancy"]),
    ),
    ("EHRbase — FHIR Connector", "fhir", Members::Tags(&["fhir"])),
    (
        "EHRbase — SMART Discovery",
        "smart",
        Members::Tags(&["smart"]),
    ),
];

/// Whether `rel` equals `prefix` or continues past it on a path-segment boundary
/// (so `/ehr` matches `/ehr` and `/ehr/{id}`, never a hypothetical `/ehrx`).
fn on_segment_boundary(rel: &str, prefix: &str) -> bool {
    rel == prefix
        || rel
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}

/// A copy of `doc` keeping only the paths whose base-path-relative form is under
/// `include` (segment-boundary) and under none of `exclude`. Whole path items
/// are kept or dropped — an ITS-REST resource path belongs entirely to one group.
fn filter_by_path(
    doc: &utoipa::openapi::OpenApi,
    base_path: &str,
    include: &str,
    exclude: &[&str],
) -> utoipa::openapi::OpenApi {
    let mut out = doc.clone();
    out.paths.paths.retain(|path, _| {
        let Some(rel) = path.strip_prefix(base_path) else {
            return false;
        };
        on_segment_boundary(rel, include) && !exclude.iter().any(|ex| on_segment_boundary(rel, ex))
    });
    prune_tags(&mut out);
    out
}

/// A copy of `doc` keeping only the operations tagged with one of `tags`
/// (paths whose every operation is filtered away are dropped entirely).
fn filter_by_tags(doc: &utoipa::openapi::OpenApi, tags: &[&str]) -> utoipa::openapi::OpenApi {
    let mut out = doc.clone();
    out.paths.paths.retain(|_, item| {
        for op in [
            &mut item.get,
            &mut item.put,
            &mut item.post,
            &mut item.delete,
            &mut item.options,
            &mut item.head,
            &mut item.patch,
            &mut item.trace,
        ] {
            if let Some(inner) = op
                && !inner
                    .tags
                    .as_ref()
                    .is_some_and(|t| t.iter().any(|t| tags.contains(&t.as_str())))
            {
                *op = None;
            }
        }
        item.get.is_some()
            || item.put.is_some()
            || item.post.is_some()
            || item.delete.is_some()
            || item.options.is_some()
            || item.head.is_some()
            || item.patch.is_some()
            || item.trace.is_some()
    });
    prune_tags(&mut out);
    out
}

/// Drop tag declarations no retained operation carries, so a family document's
/// tag list shows exactly the resource groups it contains.
fn prune_tags(doc: &mut utoipa::openapi::OpenApi) {
    let mut used = std::collections::BTreeSet::new();
    for item in doc.paths.paths.values() {
        for op in [
            &item.get,
            &item.put,
            &item.post,
            &item.delete,
            &item.options,
            &item.head,
            &item.patch,
            &item.trace,
        ] {
            if let Some(inner) = op
                && let Some(tags) = &inner.tags
            {
                for t in tags {
                    used.insert(t.clone());
                }
            }
        }
    }
    if let Some(doc_tags) = &mut doc.tags {
        doc_tags.retain(|t| used.contains(&t.name));
    }
}

/// Every served `OpenAPI` document, pre-serialized **once** at router assembly.
///
/// The composed document and every family document are pure functions of the
/// static [`AppConfig`], so building them per request re-ran the full
/// `utoipa` reflection (all API groups + the auth walk) and, for each family,
/// an additional whole-document deep clone + filter. They are computed once here
/// and served as ready [`Bytes`], so a request is a clone-free body write.
struct PrebuiltDocs {
    /// The complete composed document as serialized JSON.
    full: Bytes,
    /// One filtered family document per [`FAMILIES`] entry, in the same order.
    families: Vec<Bytes>,
}

/// Build the composed document and every family document once, serializing each
/// to [`Bytes`] (the filter machinery — [`filter_by_path`]/[`filter_by_tags`] —
/// is applied here rather than per request).
fn prebuild_docs(cfg: &AppConfig) -> PrebuiltDocs {
    let full_doc = extensions_document(cfg);
    let full = to_json_bytes(&full_doc);

    let families = FAMILIES
        .iter()
        .map(|(name, _, members)| {
            let mut doc = match members {
                Members::Path { include, exclude } => {
                    filter_by_path(&full_doc, &cfg.server.base_path, include, exclude)
                }
                Members::Tags(tags) => filter_by_tags(&full_doc, tags),
            };
            doc.info.title = (*name).to_string();
            to_json_bytes(&doc)
        })
        .collect();

    PrebuiltDocs { full, families }
}

/// Serialize an `OpenAPI` document to JSON [`Bytes`] (empty object on the
/// unreachable serialization error, mirroring the previous per-request path).
fn to_json_bytes(doc: &utoipa::openapi::OpenApi) -> Bytes {
    Bytes::from(serde_json::to_vec(doc).unwrap_or_else(|_| b"{}".to_vec()))
}

/// Serve a pre-serialized document: a clone-free (ref-counted) [`Bytes`] body
/// write with the JSON content type.
fn json_document_response(body: Bytes) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

// ── The Swagger router (serving; kept on the loop-free `serve()` mechanism) ───

/// Build the docs router: the Swagger UI (loop-free), the complete
/// `openapi.json`, and one filtered JSON document per API family — ALL
/// server-generated (nothing vendored is served). Config paths come from
/// [`AppConfig`].
pub(crate) fn swagger_router(cfg: &AppConfig) -> Router<AppState> {
    let ui_path = cfg.server.swagger_ui_path();
    let json_path = cfg.server.openapi_json_path();
    let api_docs_root = api_docs_root(&json_path);

    // Build the composed document and every family document ONCE (they are pure
    // functions of static configuration); every route below serves ready
    // [`Bytes`], so a request never re-runs utoipa reflection or a deep clone.
    let docs = prebuild_docs(cfg);

    // The complete composed document (tooling + the selector's full entry).
    let full = docs.full;
    let mut router = Router::new().route(
        &json_path,
        get(move || {
            let body = full.clone();
            async move { json_document_response(body) }
        }),
    );

    // One filtered document per API family (standard groups + extensions).
    // One static route per family: axum path captures span a whole segment,
    // so a `{family}` embedded inside the `ehrbase-….openapi.json` filename
    // cannot be a route parameter.
    for ((_, slug, _), body) in FAMILIES.iter().zip(docs.families) {
        router = router.route(
            &format!("{api_docs_root}/ehrbase-{slug}.openapi.json"),
            get(move || {
                let body = body.clone();
                async move { json_document_response(body) }
            }),
        );
    }

    // The UI itself: assets straight from the embedded dist. The spec-selector
    // config is also config-static — built once and shared by both the bare
    // mount path (index.html; serve() maps "" to it — no redirect, no loop) and
    // the asset path.
    let config = swagger_config(cfg);
    let index_config = Arc::clone(&config);
    router
        .route(
            &ui_path,
            get(move || {
                let config = Arc::clone(&index_config);
                async move { serve_ui_file("", &config) }
            }),
        )
        .route(
            &format!("{ui_path}/{{*file}}"),
            get(move |Path(file): Path<String>| {
                let config = Arc::clone(&config);
                async move { serve_ui_file(&file, &config) }
            }),
        )
}

/// The `api-docs` directory the documents live under (the parent of the
/// configured `openapi.json` path).
fn api_docs_root(json_path: &str) -> String {
    json_path
        .rsplit_once('/')
        .map_or("/api-docs", |(dir, _)| dir)
        .to_owned()
}

/// The Swagger UI spec-selector config: one entry per API family — the
/// standardised groups (`openEHR — …`) and the server extensions
/// (`EHRbase — …`), every one filtered from the server's own generated
/// document — with the complete composed document last. URLs must outlive the
/// server, so the derived paths are leaked once at construction (the UI
/// config requires `'static`).
fn swagger_config(cfg: &AppConfig) -> Arc<Config<'static>> {
    let json_path = cfg.server.openapi_json_path();
    let root = api_docs_root(&json_path);

    let mut urls: Vec<Url<'static>> = Vec::new();
    for (name, slug, _) in FAMILIES {
        let url = format!("{root}/ehrbase-{slug}.openapi.json");
        urls.push(Url::new(name, url.leak()));
    }
    urls.push(Url::new("EHRbase — Complete surface", json_path.leak()));
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
