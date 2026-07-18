//! Native `utoipa-axum` routing for the Admin API group.
//!
//! The EHR-delete operations' semantics are the ITS-REST Admin API
//! (`docs/specs/openehr/ITS-REST`, DEVELOPMENT status); no openEHR spec governs
//! the OAS layout. Each handler forwards to the group dispatcher through
//! [`guarded_dispatch`], so the wire behaviour is identical to the former
//! table-driven `mount` adapter.
//!
//! The **template delete** and **stored-query-version delete** are OUR OWN
//! EXTENSION — no openEHR spec governs them (the ITS-REST Admin API defines only
//! EHR deletes). They mirror `admin_ehr_delete` end to end: same admin gate
//! (`AppConfig::admin.enabled` → 404 when off; RBAC Admin class by the `/admin/`
//! path), `204` on success, `404` for an unknown id, plus a `409` when a
//! template delete would orphan committed clinical data.
//!
//! NOTE (path): the generated `admin_ehr_delete_all` route carries an
//! RFC 6570 query-expansion suffix (`/admin/ehr/all{?ehr_id*}`) that is not part
//! of the resource path; the mounted/documented path is the plain
//! `/admin/ehr/all` (the `ehr_id` list is read from the query string), matching
//! the normalisation the former adapter applied.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::guarded_dispatch;
use crate::state::AppState;

/// The Admin API group as a native `utoipa-axum` router (group-relative paths).
pub(crate) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(admin_ehr_delete_all))
        .routes(routes!(admin_ehr_delete))
        .routes(routes!(admin_template_delete))
        .routes(routes!(admin_query_delete))
        .routes(routes!(admin_config))
}

/// The redacted effective configuration as a JSON tree.
///
/// OUR OWN EXTENSION — no openEHR spec governs configuration (the ITS-REST
/// Admin API defines only EHR deletes). Same admin gate as the sibling deletes
/// (`AppConfig::admin.enabled` → `404` when off; RBAC Admin class by the
/// `/admin/` path → `401` unauthenticated / `403` non-admin). Every
/// secret-bearing leaf is redacted structurally by its `Secret`/`SecretUrl`
/// type before it ever reaches this handler (see
/// [`ehrbase::config::EhrbaseConfig::to_redacted_json`]).
#[utoipa::path(
    get, path = "/admin/config", tag = "ADMIN",
    responses(
        (status = 200, description = "The redacted effective configuration.", body = serde_json::Value),
        (status = 404, description = "Admin API disabled.")
    )
)]
pub(crate) async fn admin_config(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "admin_config", parts, super::dispatch::dispatch).await
}

/// Physically delete every EHR named in the `ehr_id` query parameter.
#[utoipa::path(
    delete, path = "/admin/ehr/all", tag = "EHR",
    params(("ehr_id" = Vec<String>, Query, description = "The EHR ids to delete.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn admin_ehr_delete_all(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "admin_ehr_delete_all",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Physically delete one EHR by id.
#[utoipa::path(
    delete, path = "/admin/ehr/{ehr_id}", tag = "EHR",
    params(("ehr_id" = String, Path, description = "The EHR id.")),
    responses((status = 204, description = "Deleted."))
)]
pub(crate) async fn admin_ehr_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "admin_ehr_delete", parts, super::dispatch::dispatch).await
}

/// Physically delete one operational template by its `template_id`.
///
/// OUR OWN EXTENSION — no openEHR spec governs this operation (the ITS-REST
/// Admin API defines only EHR deletes). Refuses with `409` while any committed
/// version still references the template (physical deletes never orphan clinical
/// data).
#[utoipa::path(
    delete, path = "/admin/template/{template_id}", tag = "ADMIN",
    params(("template_id" = String, Path, description = "The template id (wire address).")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 404, description = "Unknown template.", body = serde_json::Value),
        (status = 409, description = "Still referenced by a committed version.", body = serde_json::Value)
    )
)]
pub(crate) async fn admin_template_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "admin_template_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}

/// Physically delete one stored-query version (a single `(name, version)` row).
///
/// OUR OWN EXTENSION — no openEHR spec governs this operation (the ITS-REST
/// Admin API defines only EHR deletes). The qualified name is matched
/// case-insensitively (as on the stored-query PUT); the version is exact.
#[utoipa::path(
    delete, path = "/admin/query/{qualified_query_name}/{version}", tag = "ADMIN",
    params(
        ("qualified_query_name" = String, Path, description = "The qualified query name."),
        ("version" = String, Path, description = "The exact stored SEMVER.")
    ),
    responses(
        (status = 204, description = "Deleted."),
        (status = 404, description = "Unknown query name/version.", body = serde_json::Value)
    )
)]
pub(crate) async fn admin_query_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(
        state,
        "admin_query_delete",
        parts,
        super::dispatch::dispatch,
    )
    .await
}
