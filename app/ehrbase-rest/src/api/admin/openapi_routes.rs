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
//! ## Prose-vs-OAS reconciliations (documented real wire)
//!
//! - **Disabled admin API → `405 Method Not Allowed`**: the group is
//!   config-gated (`AppConfig::admin.enabled`, default false); when off,
//!   every admin route answers `405` with the openEHR error body — the
//!   status the OAS itself declares for a disabled admin operation
//!   (`admin_ehr_delete_all.yaml` + `responses/405.yaml`), applied
//!   uniformly across the group (`admin/dispatch.rs`).
//! - **Synchronous deletes → `204`** (never `202`): the OAS permits an async
//!   `202 Accepted`, but every delete here is synchronous, so success is always
//!   `204 No Content` (bodyless); the `202` path is not produced.
//! - **`admin_ehr_delete_all` `ehr_id`**: optional; an absent/empty list means
//!   "delete ALL EHRs" (`admin_ehr_delete_all.yaml` + `ehr_id_Admin.yaml`). Both
//!   the repeated (`?ehr_id=a&ehr_id=b`) and comma-separated (`?ehr_id=a,b`)
//!   forms are accepted.
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

/// The redacted effective configuration as a JSON tree
/// (`GET /admin/config`).
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
        (status = 200, description = "The redacted effective configuration \
                                      (every secret leaf already `***` by its \
                                      `Secret`/`SecretUrl` type).",
         body = serde_json::Value),
        (status = 401, description = "Unauthenticated (auth enabled, no valid \
                                      principal).",
         body = serde_json::Value),
        (status = 403, description = "Authenticated but not in the Admin class.",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this \
                                      server (the status the OAS declares for \
                                      a disabled admin operation: \
                                      `admin_ehr_delete_all.yaml` + \
                                      `responses/405.yaml`).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn admin_config(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "admin_config", parts, super::dispatch::dispatch).await
}

/// Physically delete multiple EHRs, or all of them
/// (`DELETE /admin/ehr/all`).
///
/// Cascades every owned resource (`COMPOSITION`, `EHR_STATUS`, `ITEM_TAG`,
/// CONTRIBUTION + historical versions) — a permanent physical delete (e.g. for
/// GDPR erasure). Intended for development/testing.
#[utoipa::path(
    delete, path = "/admin/ehr/all", tag = "EHR",
    params(
        ("ehr_id" = Option<Vec<String>>, Query,
         description = "The EHR ids (UUIDs) to delete, as repeated `ehr_id=…` or \
                        a comma-separated `ehr_id=a,b`. OPTIONAL — an absent or \
                        empty list deletes ALL EHRs.")
    ),
    responses(
        (status = 204, description = "Deleted (synchronous; bodyless)."),
        (status = 404, description = "An `ehr_id` in the list does not exist \
                                      (nothing is deleted).",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this \
                                      server (the status the OAS declares for \
                                      a disabled admin operation: \
                                      `admin_ehr_delete_all.yaml` + \
                                      `responses/405.yaml`).",
         body = serde_json::Value)
    )
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

/// Physically delete one EHR by id (`DELETE /admin/ehr/{ehr_id}`).
///
/// Cascades every owned resource (`COMPOSITION`, `EHR_STATUS`, `ITEM_TAG`,
/// CONTRIBUTION + historical versions) — a permanent physical delete (e.g. for
/// GDPR erasure).
#[utoipa::path(
    delete, path = "/admin/ehr/{ehr_id}", tag = "EHR",
    params(
        ("ehr_id" = String, Path,
         description = "The EHR id (a UUID), taken from EHR.ehr_id.value.")
    ),
    responses(
        (status = 204, description = "Deleted (synchronous; bodyless)."),
        (status = 404, description = "No EHR with `ehr_id`.",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this \
                                      server (the status the OAS declares for \
                                      a disabled admin operation: \
                                      `admin_ehr_delete_all.yaml` + \
                                      `responses/405.yaml`).",
         body = serde_json::Value)
    )
)]
pub(crate) async fn admin_ehr_delete(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let parts = crate::api::into_parts(request).await;
    guarded_dispatch(state, "admin_ehr_delete", parts, super::dispatch::dispatch).await
}

/// Physically delete one operational template by its `template_id`
/// (`DELETE /admin/template/{template_id}`).
///
/// OUR OWN EXTENSION — no openEHR spec governs this operation (the ITS-REST
/// Admin API defines only EHR deletes). Refuses with `409` while any committed
/// version still references the template (physical deletes never orphan clinical
/// data).
#[utoipa::path(
    delete, path = "/admin/template/{template_id}", tag = "ADMIN",
    params(
        ("template_id" = String, Path,
         description = "The `template_id` (the wire address of the OPT).")
    ),
    responses(
        (status = 204, description = "Deleted (bodyless)."),
        (status = 404, description = "No template with `template_id`.",
         body = serde_json::Value),
        (status = 409, description = "A committed version still references the \
                                      template (deletion would orphan clinical \
                                      data).",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this \
                                      server (the status the OAS declares for \
                                      a disabled admin operation: \
                                      `admin_ehr_delete_all.yaml` + \
                                      `responses/405.yaml`).",
         body = serde_json::Value)
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

/// Physically delete one stored-query version — a single `(name, version)` row
/// (`DELETE /admin/query/{qualified_query_name}/{version}`).
///
/// OUR OWN EXTENSION — no openEHR spec governs this operation (the ITS-REST
/// Admin API defines only EHR deletes). The qualified name is matched
/// case-insensitively (as on the stored-query PUT); the version is exact.
#[utoipa::path(
    delete, path = "/admin/query/{qualified_query_name}/{version}", tag = "ADMIN",
    params(
        ("qualified_query_name" = String, Path,
         description = "The qualified query name \
                        (`[{namespace}::]{query-name}`; matched \
                        case-insensitively)."),
        ("version" = String, Path,
         description = "The exact stored SEMVER version to delete.")
    ),
    responses(
        (status = 204, description = "Deleted (bodyless)."),
        (status = 404, description = "No stored query at that `(name, version)`.",
         body = serde_json::Value),
        (status = 405, description = "The admin API is disabled on this \
                                      server (the status the OAS declares for \
                                      a disabled admin operation: \
                                      `admin_ehr_delete_all.yaml` + \
                                      `responses/405.yaml`).",
         body = serde_json::Value)
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
