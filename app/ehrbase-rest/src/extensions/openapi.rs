//! `OpenAPI` document + Swagger UI (discoverability).
//!
//! No openEHR spec governs an OAS-serving endpoint; this is our own surface.
//! What the Swagger UI exposes here is **our own `ehrbase-rest` endpoints
//! only** — the composed extension-surface document
//! ([`crate::extensions::openapi_extensions::ExtensionsApiDoc`]): the
//! operational + extension endpoints this server serves (status/health, the
//! management surface, SMART discovery, the `OpenAPI` endpoints, and the
//! terminology / `PARTY_RELATIONSHIP` / event-subscription / multi-tenancy /
//! FHIR-connector extensions). No openEHR spec governs those — our own design.
//!
//! The vendored ITS-REST `OpenAPI` bundles (`openehr_its::rest::VENDORED_OAS`)
//! are **not** served through this selector: they are the standardised
//! contract, authoritative in their own right, and this UI is deliberately
//! scoped to what is unique to this server.
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

use crate::extensions::openapi_extensions::ExtensionsApiDoc;
use crate::state::AppState;

/// Build the docs router: the Swagger UI (loop-free) and the single
/// `ehrbase-rest` extension-surface `OpenAPI` JSON document. No vendored
/// bundles are served here.
pub(crate) fn swagger_router<S: Platform>(ui_path: &str, json_path: &str) -> Router<AppState<S>> {
    // One selector entry: our own composed extension-surface document.
    let urls: Vec<Url<'static>> = vec![Url::new("ehrbase-rest", json_path.to_owned().leak())];

    let document =
        serde_json::to_string(&ExtensionsApiDoc::openapi()).unwrap_or_else(|_| "{}".to_owned());
    let router = Router::new().route(
        json_path,
        get(move || async move {
            (
                [(header::CONTENT_TYPE, "application/json")],
                document.clone(),
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
