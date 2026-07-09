//! `OpenAPI` document + Swagger UI.
//!
//! Per ADR-005 the **vendored** ITS-REST `OpenAPI` is the authoritative contract;
//! `utoipa` here serves a Swagger UI and a served `OpenAPI` document for
//! discoverability, and is the seam for a future code→OAS drift-check against
//! the vendored spec. It is not the source of truth.

use axum::Router;
use ehrbase_sm::Platform;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

/// The served `OpenAPI` document. Handlers are dispatched generically from the
/// generated `ROUTES` tables rather than annotated individually, so this doc
/// carries the API metadata; the full path set is the vendored OAS.
#[derive(OpenApi)]
#[openapi(info(
    title = "EHRbase-RS — openEHR ITS-REST",
    version = "1.0.3",
    description = "openEHR-spec-conformant CDR (ITS-REST 1.0.3). The authoritative \
                   contract is the vendored OpenAPI; this document is served for \
                   discoverability (ADR-005)."
))]
pub struct ApiDoc;

/// Build the Swagger UI router (serves the UI and the `OpenAPI` JSON).
pub(crate) fn swagger_router<S: Platform>(ui_path: &str, json_path: &str) -> Router<AppState<S>> {
    Router::new()
        .merge(SwaggerUi::new(ui_path.to_owned()).url(json_path.to_owned(), ApiDoc::openapi()))
}
