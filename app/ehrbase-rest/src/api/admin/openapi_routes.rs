//! Native `utoipa-axum` routing for the Admin API group (physical EHR delete).
//!
//! Operation semantics are the ITS-REST Admin API (`docs/specs/openehr/ITS-REST`,
//! DEVELOPMENT status); no openEHR spec governs the OAS layout. Each handler
//! forwards to the group dispatcher through [`guarded_dispatch`], so the wire
//! behaviour is identical to the former table-driven `mount` adapter.
//!
//! PORT NOTE (path): the generated `admin_ehr_delete_all` route carries an
//! RFC 6570 query-expansion suffix (`/admin/ehr/all{?ehr_id*}`) that is not part
//! of the resource path; the mounted/documented path is the plain
//! `/admin/ehr/all` (the `ehr_id` list is read from the query string), matching
//! the normalisation the former adapter applied.

use axum::extract::State;
use axum::response::Response;
use ehrbase_sm::Platform;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The Admin API group as a native `utoipa-axum` router (group-relative paths).
pub(crate) fn routes<S: Platform>() -> OpenApiRouter<AppState<S>> {
    OpenApiRouter::new()
        .routes(routes!(admin_ehr_delete_all))
        .routes(routes!(admin_ehr_delete))
}

/// Physically delete every EHR named in the `ehr_id` query parameter.
#[utoipa::path(
    delete, path = "/admin/ehr/all", tag = "admin",
    params(("ehr_id" = Vec<String>, Query, description = "The EHR ids to delete.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn admin_ehr_delete_all<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "admin_ehr_delete_all",
        parts,
        super::dispatch::dispatch::<S>,
    )
    .await
}

/// Physically delete one EHR by id.
#[utoipa::path(
    delete, path = "/admin/ehr/{ehr_id}", tag = "admin",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn admin_ehr_delete<S: Platform>(
    State(state): State<AppState<S>>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "admin_ehr_delete",
        parts,
        super::dispatch::dispatch::<S>,
    )
    .await
}
