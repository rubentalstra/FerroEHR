//! `OpenAPI` document + Swagger UI (discoverability).
//!
//! No openEHR spec governs an OAS-serving endpoint; this is our own surface.
//! What it exposes, though, is the authoritative contract itself: the
//! **vendored development-edition** ITS-REST `OpenAPI` bundles at
//! `crates/openehr-its/vendor/rest-oas/` — openEHR `specifications-ITS-REST`
//! `master` @ `e8a093e9…` (the `-codegen` variant `emit-rest` consumes to
//! generate the served routes). Serving Swagger for that same tree keeps the
//! documented API and the implemented contract one identity, not two.
//!
//! `utoipa` is used only for the Swagger UI shell and as the seam for a future
//! code→OAS drift-check against the vendored bundles; the vendored OAS is the
//! source of truth, never this generated document (which carries the API
//! metadata; handlers are dispatched generically from the generated `ROUTES`
//! tables rather than annotated per operation, so its live path set is the
//! generated contract).

use axum::Router;
use ehrbase_sm::Platform;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

/// The served `OpenAPI` document metadata. The `version` mirrors the tested
/// contract identity [`crate::extensions::provenance::ITS_REST`]
/// (`development@e8a093e`) — the development edition the vendored OAS pins, not
/// the retired `1.0.3` release label. (`utoipa`'s `info` attribute takes a
/// string literal, so this cannot reference the const directly; keep the two in
/// sync on a spec-pin bump.)
#[derive(OpenApi)]
#[openapi(info(
    title = "EHRbase-RS — openEHR ITS-REST",
    version = "development@e8a093e",
    description = "openEHR-spec-conformant CDR. The authoritative contract is the \
                   vendored development-edition ITS-REST OpenAPI (master @ e8a093e9, \
                   the tree emit-rest generates from); this document is served for \
                   discoverability only."
))]
#[derive(Debug)]
pub struct ApiDoc;

/// Build the Swagger UI router (serves the UI and the `OpenAPI` JSON).
pub(crate) fn swagger_router<S: Platform>(ui_path: &str, json_path: &str) -> Router<AppState<S>> {
    Router::new()
        .merge(SwaggerUi::new(ui_path.to_owned()).url(json_path.to_owned(), ApiDoc::openapi()))
}
