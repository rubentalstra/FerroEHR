//! `OpenAPI` documents + Swagger UI (discoverability).
//!
//! No openEHR spec governs an OAS-serving endpoint; this is our own surface.
//! What it exposes is the authoritative contract itself: the **vendored
//! development-edition** ITS-REST `OpenAPI` bundles
//! (`openehr_its::rest::VENDORED_OAS` — openEHR `specifications-ITS-REST`
//! `master` @ `e8a093e9…`, the same pinned tree `emit-rest` generates the
//! served routes from). Serving Swagger for that tree keeps the documented
//! API and the implemented contract one identity, not two.
//!
//! The UI assets are served through [`utoipa_swagger_ui::serve`] directly
//! rather than [`utoipa_swagger_ui::SwaggerUi`]'s router: the router answers
//! the bare mount path with a `303` to the trailing-slash form, which the
//! serve-time `NormalizePathLayer` strips again before routing — an infinite
//! redirect loop. Serving `index.html` for the bare path outright has no
//! redirect to fight.

use std::sync::Arc;

use axum::Router;
use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use ehrbase_sm::Platform;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerFile, Url};

use crate::state::AppState;

/// The identity document: API metadata + the contract provenance. The
/// `version` mirrors the tested contract identity
/// [`crate::extensions::provenance::ITS_REST`] (`development@e8a093e`).
/// (`utoipa`'s `info` attribute takes a string literal, so this cannot
/// reference the const directly; keep the two in sync on a spec-pin bump.)
#[derive(OpenApi)]
#[openapi(info(
    title = "EHRbase-RS — openEHR ITS-REST",
    version = "development@e8a093e",
    description = "openEHR-spec-conformant CDR. The API-group documents in the \
                   spec selector are the vendored development-edition ITS-REST \
                   OpenAPI bundles (master @ e8a093e9) — the authoritative \
                   contract the server's routes are generated from."
))]
#[derive(Debug)]
pub struct ApiDoc;

/// Build the docs router: the Swagger UI (loop-free), the identity JSON, and
/// one route per vendored API-group bundle (served verbatim as YAML).
pub(crate) fn swagger_router<S: Platform>(ui_path: &str, json_path: &str) -> Router<AppState<S>> {
    // `json_path` is `{rest_root}/api-docs/openapi.json`; the group bundles
    // live beside it.
    let api_docs_root = json_path
        .rsplit_once('/')
        .map_or("/api-docs", |(dir, _)| dir)
        .to_owned();

    let mut router = Router::new();

    // The vendored contract, one URL per API group + the identity doc.
    let mut urls: Vec<Url<'static>> = Vec::new();
    for (group, yaml) in openehr_its::rest::VENDORED_OAS {
        let route = format!("{api_docs_root}/openehr-{group}.openapi.yaml");
        urls.push(Url::new(group, route.clone().leak()));
        router = router.route(
            &route,
            get(|| async { ([(header::CONTENT_TYPE, "application/yaml")], *yaml) }),
        );
    }
    urls.push(Url::new("identity", json_path.to_owned().leak()));
    let identity = serde_json::to_string(&ApiDoc::openapi()).unwrap_or_else(|_| "{}".to_owned());
    router = router.route(
        json_path,
        get(move || async move {
            (
                [(header::CONTENT_TYPE, "application/json")],
                identity.clone(),
            )
        }),
    );

    // The UI itself: assets straight from the embedded dist. The bare mount
    // path serves index.html (serve() maps "" to it) — no redirect, no loop.
    let config = Arc::new(Config::new(urls));
    let cfg_index = Arc::clone(&config);
    router
        .route(
            ui_path,
            get(move || {
                let cfg = Arc::clone(&cfg_index);
                async move { serve_ui_file("", &cfg) }
            }),
        )
        .route(
            &format!("{ui_path}/{{*file}}"),
            get(move |Path(file): Path<String>| {
                let cfg = Arc::clone(&config);
                async move { serve_ui_file(&file, &cfg) }
            }),
        )
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
