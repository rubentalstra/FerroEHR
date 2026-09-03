// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The CDR's **tenant registry** surface the viewer consumes.
//!
//! The availability probe the registry screen and its nav entry are gated on,
//! the five registry operations (list / create / read-by-id / update / delete),
//! and the read-only "which tenant does this session resolve to" context.
//!
//! NOTE: no openEHR spec governs multi-tenancy — our own enterprise extension;
//! the whole group (paths, payloads, status codes) is the CDR's own design.
//!
//! **The viewer never CHOOSES a tenant.** Tenancy resolves per request from
//! the caller's credential, and the CDR additionally honours a dev-only header
//! override that wins over the claim, so a viewer-side switcher would need
//! either viewer-local state (banned outright) or that header, which is an
//! authorization bypass. The context card reads `GET admin/tenant/current` and
//! RENDERS it; nothing here changes which tenant a request runs on.
//!
//! **Probe-and-hide.** The group is config-gated on the CDR
//! (`[tenancy] enabled`, off by default) and answers `404` for every route as if
//! unmounted while it is off, so the viewer discovers it
//! ([`probe_tenant_registry`]) before offering any of it. Capability is not
//! authorization: a mounted-but-refused group (`401`/`403`) still counts as
//! present, and the refusal surfaces as copy on the screen that asked.
//!
//! Every `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first (a server fn is a
//! publicly reachable HTTP endpoint) and keeps the CDR credential server-side.

#![allow(
    clippy::disallowed_types,
    reason = "the viewer consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694); the carriers here are ssr-only, so #[expect] would be unfulfilled on the \
              hydrate target"
)]

use leptos::prelude::*;
use leptos::server;
use serde::{Deserialize, Serialize};

use crate::error::ViewerError;

/// One tenant as the registry serves it (`{id, name, system_id, created_at}`).
///
/// Every field is a string, including the id and the timestamp: the record
/// crosses the server-fn boundary and is rendered, never computed with (no
/// `usize`, and no parsing the viewer would have to keep in step with the
/// CDR's own formatting).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TenantRow {
    /// The tenant's UUID, as the CDR spelled it.
    pub id: String,
    /// The unique tenant name — the key a credential's claim resolves by.
    pub name: String,
    /// The tenant's openEHR `system_id`.
    pub system_id: String,
    /// The registry row's creation instant, verbatim from the wire.
    pub created_at: String,
}

/// The `GET admin/tenant/current` answer: the tenant THIS session's credential
/// resolves to.
///
/// `default: true` with no record means the request ran unscoped on the
/// reserved default tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentTenant {
    /// Whether the session runs unscoped on the reserved default tenant.
    pub default: bool,
    /// The resolved registry record; absent on the default tenant.
    pub tenant: Option<TenantRow>,
}

impl Default for CurrentTenant {
    /// The unscoped answer: no registry record, running on the reserved
    /// default tenant. A derived `false` here would be the one combination the
    /// wire never produces — no tenant AND not the default.
    fn default() -> Self {
        Self {
            default: true,
            tenant: None,
        }
    }
}

/// Whether the CDR serves its tenant registry.
///
/// Carries only fixed-size, client-safe data — it crosses the server-fn
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantAvailability {
    /// The group answered: it is mounted, so the registry screen is offered.
    /// Whether THIS session may use it is a per-request answer.
    Available,
    /// The group is not mounted (`[tenancy] enabled = false`) — the CDR
    /// answered `404` as if the routes did not exist.
    Disabled,
}

impl TenantAvailability {
    /// Whether the viewer may offer the tenant registry.
    #[must_use]
    pub fn usable(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Classify the probe's HTTP status: only a `404` means "not mounted".
///
/// A `401`/`403` is a mounted group refusing THIS session (the registry sits
/// under `/admin`, which the coarse RBAC gate classes as admin work) — the
/// surface exists, which is what the nav gate asks about.
#[must_use]
pub fn availability_of_status(status: http::StatusCode) -> TenantAvailability {
    if status == http::StatusCode::NOT_FOUND {
        TenantAvailability::Disabled
    } else {
        TenantAvailability::Available
    }
}

/// The single predicate every tenancy-gated affordance uses: render only for a
/// probe that succeeded and found the registry mounted.
///
/// A failed probe (CDR unreachable, expired session) hides it — never a nav
/// link to a screen that cannot work.
#[must_use]
pub fn renders_tenant_registry(probe: &Result<TenantAvailability, ViewerError>) -> bool {
    probe
        .as_ref()
        .copied()
        .is_ok_and(TenantAvailability::usable)
}

/// The tenancy gate: one probe [`Resource`].
///
/// Created in component setup — never inside a `Suspend` closure, which re-runs
/// and would re-create the resource.
#[must_use]
pub fn tenant_gate() -> Resource<Result<TenantAvailability, ViewerError>> {
    Resource::new(|| (), |()| async move { probe_tenant_registry().await })
}

/// Render `affordance` only when the gate found the registry mounted;
/// otherwise render nothing at all (probe-and-hide).
///
/// The probe is resolved INSIDE the `<Suspense>`: an SSR'd `ErrorBoundary`
/// fallback mismatches at hydration in leptos 0.8, and a render-time resource
/// read is itself a hydration mismatch. `affordance` creates no resources, so
/// re-runs are safe, and it is shared through an `Arc` because the `Suspend`
/// closure must not consume its environment.
#[must_use]
pub fn when_tenant_registry_usable(
    gate: Resource<Result<TenantAvailability, ViewerError>>,
    affordance: impl Fn() -> AnyView + Send + Sync + 'static,
) -> AnyView {
    let affordance: std::sync::Arc<dyn Fn() -> AnyView + Send + Sync> =
        std::sync::Arc::new(affordance);
    view! {
        <Suspense fallback=|| ()>
            {move || {
                let affordance = std::sync::Arc::clone(&affordance);
                Suspend::new(async move {
                    if renders_tenant_registry(&gate.await) { affordance() } else { ().into_any() }
                })
            }}
        </Suspense>
    }
    .into_any()
}

/// The one sentence the context card renders after "This session's credential
/// resolves to".
///
/// Pure and unit-tested, so the copy is the same on the server pass and at
/// hydration and the screen stays a thin renderer.
#[must_use]
pub fn context_line(current: &CurrentTenant) -> String {
    match current.tenant.as_ref() {
        Some(tenant) => format!("{} ({})", tenant.name, tenant.system_id),
        None => "the reserved default tenant".to_owned(),
    }
}

/// Whether a `{name, system_id}` draft is complete enough to send.
///
/// Both fields are required and non-empty after trimming — the CDR's own
/// `400` rule, checked here so the create/save button is inert until the form
/// can actually succeed.
#[must_use]
pub fn draft_is_complete(name: &str, system_id: &str) -> bool {
    !name.trim().is_empty() && !system_id.trim().is_empty()
}

/// Actionable copy for a refused tenant operation — a write, a delete, or a
/// read the RBAC gate turned down.
///
/// Names the object, carries the CDR's own diagnostic verbatim, and names the
/// next action. The registry's `409` is either the reserved default tenant or a
/// tenant that still owns data, so this family gets its own copy rather than
/// [`delete_failure_copy`](crate::admin::delete_failure_copy)'s wording.
#[must_use]
pub fn tenant_failure_copy(object: &str, error: &ViewerError) -> String {
    match error {
        ViewerError::Cdr { message, .. }
            if error.status_code() == Some(http::StatusCode::CONFLICT) =>
        {
            format!(
                "The CDR refused the change to {object}: {message}. Resolve that first, then \
                 retry."
            )
        }
        ViewerError::Cdr { message, .. }
            if error.status_code() == Some(http::StatusCode::NOT_FOUND) =>
        {
            format!(
                "{object} is not in the registry any more ({message}) — another operator may have \
                 deleted it. Reload this screen."
            )
        }
        ViewerError::Forbidden(message) => format!(
            "This session may not administer {object} ({message}). Sign in with an ADMIN-role \
             account and retry."
        ),
        other => crate::feedback::write_failure_copy(object, other),
    }
}

/// Probe the CDR for its tenant registry (`GET admin/tenant`).
///
/// The list operation is the availability signal: a `404` means the group is
/// not mounted for this deployment (`[tenancy] enabled = false`), any other
/// answer means it is.
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::CdrUnreachable`] on transport failure.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn probe_tenant_registry() -> Result<TenantAvailability, ViewerError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("admin/tenant");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    Ok(availability_of_status(response.status))
}

/// List the registered tenants (`GET admin/tenant`).
///
/// `Ok(None)` is the CDR answering `404` — the tenancy extension is disabled,
/// which is an absent surface rather than an error, and the screen renders it
/// as its own first-class state.
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::CdrUnauthorized`] / [`ViewerError::Forbidden`] / [`ViewerError::Cdr`] /
/// [`ViewerError::CdrUnreachable`] from the CDR; [`ViewerError::Internal`]
/// when the body is not valid JSON.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn list_tenants() -> Result<Option<Vec<TenantRow>>, ViewerError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("admin/tenant");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    if response.is(http::StatusCode::NOT_FOUND) {
        return Ok(None);
    }
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let value = serde_json::from_str::<serde_json::Value>(&body)
        .map_err(|e| ViewerError::Internal(format!("tenant list JSON: {e}")))?;
    let rows = value
        .as_array()
        .map(|items| items.iter().map(tenant_row).collect())
        .unwrap_or_default();
    Ok(Some(rows))
}

/// Read the tenant this session's credential resolves to
/// (`GET admin/tenant/current`).
///
/// `Ok(None)` is the disabled-extension `404`, exactly as in [`list_tenants`].
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::CdrUnauthorized`] / [`ViewerError::Forbidden`] / [`ViewerError::Cdr`] /
/// [`ViewerError::CdrUnreachable`] from the CDR; [`ViewerError::Internal`]
/// when the body is not valid JSON.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn fetch_current_tenant() -> Result<Option<CurrentTenant>, ViewerError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("admin/tenant/current");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    if response.is(http::StatusCode::NOT_FOUND) {
        return Ok(None);
    }
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let value = serde_json::from_str::<serde_json::Value>(&body)
        .map_err(|e| ViewerError::Internal(format!("current tenant JSON: {e}")))?;
    Ok(Some(current_tenant(&value)))
}

/// Register a tenant (`POST admin/tenant`, body `{name, system_id}`).
///
/// The CDR's `400` (missing/empty field), `409` (duplicate name) and `415`
/// (wrong content type) diagnostics surface verbatim through
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::Invalid`] when a field is blank;
/// [`ViewerError::CdrUnauthorized`] / [`ViewerError::Forbidden`] / [`ViewerError::Cdr`] /
/// [`ViewerError::CdrUnreachable`] from the CDR; [`ViewerError::Internal`]
/// when the created record is not valid JSON.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn create_tenant(
    /// The tenant name (unique in the registry).
    name: String,
    /// The tenant's openEHR `system_id`.
    system_id: String,
) -> Result<TenantRow, ViewerError> {
    let session = crate::session::require_session().await?;
    if !draft_is_complete(&name, &system_id) {
        return Err(ViewerError::Invalid(
            "a tenant needs both a name and a system_id".to_owned(),
        ));
    }
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("admin/tenant");
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            tenant_definition(&name, &system_id),
        )
        .await?;
    stored_record(&crate::cdr::CdrClient::expect_success(response)?.body)
}

/// Update one tenant's name and `system_id`
/// (`PUT admin/tenant/{tenant_id}`, body `{name, system_id}`).
///
/// The id is percent-encoded with the `urlencoding` crate — the registry serves
/// it as text, and the viewer never hand-rolls a codec for a path segment.
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::Invalid`] for an empty id or a blank field;
/// [`ViewerError::CdrUnauthorized`] / [`ViewerError::Forbidden`] / [`ViewerError::Cdr`] /
/// [`ViewerError::CdrUnreachable`] from the CDR; [`ViewerError::Internal`]
/// when the stored record is not valid JSON.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn update_tenant(
    /// The tenant to update, by registry id.
    tenant_id: String,
    /// The tenant name to store.
    name: String,
    /// The `system_id` to store.
    system_id: String,
) -> Result<TenantRow, ViewerError> {
    let session = crate::session::require_session().await?;
    if tenant_id.trim().is_empty() {
        return Err(ViewerError::Invalid("no tenant id to update".to_owned()));
    }
    if !draft_is_complete(&name, &system_id) {
        return Err(ViewerError::Invalid(
            "a tenant needs both a name and a system_id".to_owned(),
        ));
    }
    let state: crate::state::AppState = expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("admin/tenant/{}", urlencoding::encode(&tenant_id)));
    let response = state
        .cdr
        .put(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            tenant_definition(&name, &system_id),
        )
        .await?;
    stored_record(&crate::cdr::CdrClient::expect_success(response)?.body)
}

/// Delete one tenant (`DELETE admin/tenant/{tenant_id}`).
///
/// The CDR refuses `409` for the reserved default tenant and for a tenant that
/// still owns data; the diagnostic surfaces through [`tenant_failure_copy`].
///
/// # Errors
/// [`ViewerError::Unauthenticated`] without a viewer session;
/// [`ViewerError::Invalid`] for an empty id;
/// [`ViewerError::CdrUnauthorized`] / [`ViewerError::Forbidden`] / [`ViewerError::Cdr`] /
/// [`ViewerError::CdrUnreachable`] from the CDR.
#[server(client = crate::session_client::SessionAwareClient)]
pub async fn delete_tenant(
    /// The tenant to delete, by registry id.
    tenant_id: String,
) -> Result<(), ViewerError> {
    let session = crate::session::require_session().await?;
    if tenant_id.trim().is_empty() {
        return Err(ViewerError::Invalid("no tenant id to delete".to_owned()));
    }
    let state: crate::state::AppState = expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("admin/tenant/{}", urlencoding::encode(&tenant_id)));
    let response = state.cdr.delete(&session.credential, &url, &[]).await?;
    crate::cdr::CdrClient::expect_success(response)?;
    Ok(())
}

/// The `{name, system_id}` request body both writes send, with each field
/// trimmed.
///
/// Built through `serde_json` rather than string formatting so a name carrying
/// a quote or a backslash is escaped by the encoder.
#[cfg(feature = "ssr")]
fn tenant_definition(name: &str, system_id: &str) -> String {
    serde_json::json!({ "name": name.trim(), "system_id": system_id.trim() }).to_string()
}

/// Parse the stored record a create/update answered with.
#[cfg(feature = "ssr")]
fn stored_record(body: &str) -> Result<TenantRow, ViewerError> {
    let value = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|e| ViewerError::Internal(format!("tenant record JSON: {e}")))?;
    Ok(tenant_row(&value))
}

/// Distil one registry element into a [`TenantRow`], reading each field
/// defensively so a missing or renamed field empties that cell rather than
/// dropping the whole row from the listing.
#[cfg(feature = "ssr")]
fn tenant_row(item: &serde_json::Value) -> TenantRow {
    let text = |key: &str| {
        item.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    TenantRow {
        id: text("id"),
        name: text("name"),
        system_id: text("system_id"),
        created_at: text("created_at"),
    }
}

/// Distil the `current` answer: the flag, plus the record when one is present.
///
/// A body without `default` reads as the default tenant — the honest reading of
/// "no scope was reported", and the same shape an unscoped request produces.
#[cfg(feature = "ssr")]
fn current_tenant(body: &serde_json::Value) -> CurrentTenant {
    let tenant = body
        .get("tenant")
        .filter(|value| !value.is_null())
        .map(tenant_row);
    CurrentTenant {
        default: body
            .get("default")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(tenant.is_none()),
        tenant,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CurrentTenant, TenantAvailability, TenantRow, availability_of_status, context_line,
        draft_is_complete, renders_tenant_registry, tenant_failure_copy,
    };
    use crate::error::ViewerError;

    fn row(name: &str, system_id: &str) -> TenantRow {
        TenantRow {
            id: "01a02a79-a92d-70e2-8386-2816600e0c60".to_owned(),
            name: name.to_owned(),
            system_id: system_id.to_owned(),
            created_at: "2026-08-22T17:16:51.373027Z".to_owned(),
        }
    }

    #[test]
    fn only_a_404_means_the_registry_is_not_mounted() {
        assert_eq!(
            availability_of_status(http::StatusCode::NOT_FOUND),
            TenantAvailability::Disabled
        );
        // Mounted-but-refused, and mounted-and-served, are both "it exists".
        for status in [
            http::StatusCode::OK,
            http::StatusCode::UNAUTHORIZED,
            http::StatusCode::FORBIDDEN,
            http::StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            assert_eq!(
                availability_of_status(status),
                TenantAvailability::Available,
                "{status}"
            );
        }
        assert!(TenantAvailability::Available.usable());
        assert!(!TenantAvailability::Disabled.usable());
    }

    #[test]
    fn only_an_available_registry_renders_its_affordances() {
        assert!(renders_tenant_registry(&Ok(TenantAvailability::Available)));
        assert!(!renders_tenant_registry(&Ok(TenantAvailability::Disabled)));
        // A probe that never got an answer hides the entry too.
        assert!(!renders_tenant_registry(&Err(ViewerError::Unauthenticated)));
        assert!(!renders_tenant_registry(&Err(ViewerError::CdrUnreachable(
            "connection refused".to_owned()
        ))));
    }

    #[test]
    fn the_context_line_names_the_resolved_tenant_or_the_default() {
        assert_eq!(
            context_line(&CurrentTenant {
                default: false,
                tenant: Some(row("acme", "acme.example.org")),
            }),
            "acme (acme.example.org)"
        );
        assert_eq!(
            context_line(&CurrentTenant::default()),
            "the reserved default tenant"
        );
    }

    #[test]
    fn a_draft_needs_both_fields_non_blank() {
        assert!(draft_is_complete("acme", "acme.example.org"));
        assert!(!draft_is_complete("", "acme.example.org"));
        assert!(!draft_is_complete("acme", ""));
        // Whitespace is not a value: the CDR refuses it with a 400.
        assert!(!draft_is_complete("  ", "  "));
    }

    #[test]
    fn a_refusal_names_the_tenant_the_diagnostic_and_the_next_action() {
        let conflict = tenant_failure_copy(
            "tenant `default`",
            &ViewerError::Cdr {
                status: 409,
                message: "the reserved default tenant cannot be deleted".to_owned(),
            },
        );
        assert!(conflict.contains("tenant `default`"), "{conflict}");
        assert!(
            conflict.contains("the reserved default tenant cannot be deleted"),
            "{conflict}"
        );
        assert!(conflict.contains("then retry"), "{conflict}");

        let gone = tenant_failure_copy(
            "tenant `acme`",
            &ViewerError::Cdr {
                status: 404,
                message: "not found: tenant 01a0".to_owned(),
            },
        );
        assert!(gone.contains("Reload this screen"), "{gone}");

        let refused = tenant_failure_copy(
            "tenant `acme`",
            &ViewerError::Forbidden("insufficient role".to_owned()),
        );
        assert!(
            refused.contains("ADMIN-role") && refused.contains("insufficient role"),
            "{refused}"
        );

        // Everything else keeps the shared write copy rather than inventing a
        // second vocabulary.
        let rejected = tenant_failure_copy(
            "tenant `acme`",
            &ViewerError::Cdr {
                status: 400,
                message: "`name` is required and non-empty".to_owned(),
            },
        );
        assert_eq!(
            rejected,
            crate::feedback::write_failure_copy(
                "tenant `acme`",
                &ViewerError::Cdr {
                    status: 400,
                    message: "`name` is required and non-empty".to_owned(),
                }
            )
        );
    }

    /// The wire shapes this module parses, pinned against the answers the CDR
    /// actually serves.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_registry_element_distils_into_a_row_and_missing_fields_stay_empty() {
        let item = serde_json::json!({
            "id": "01a02a79-a92d-70e2-8386-2816600e0c60",
            "name": "acme",
            "system_id": "acme.example.org",
            "created_at": "2026-08-22T17:16:51.373027Z",
        });
        assert_eq!(super::tenant_row(&item), row("acme", "acme.example.org"));
        let sparse = serde_json::json!({ "name": "only-a-name" });
        let parsed = super::tenant_row(&sparse);
        assert_eq!(parsed.name, "only-a-name");
        assert!(parsed.id.is_empty() && parsed.system_id.is_empty());
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_current_answer_reads_both_of_its_two_shapes() {
        // Unscoped: the reserved default tenant, with no record.
        let unscoped = super::current_tenant(&serde_json::json!({
            "default": true,
            "tenant": serde_json::Value::Null,
        }));
        assert_eq!(unscoped, CurrentTenant::default());
        assert!(unscoped.default && unscoped.tenant.is_none());

        // Scoped: the flag is false and the registry record travels with it.
        let scoped = super::current_tenant(&serde_json::json!({
            "default": false,
            "tenant": {
                "id": "01a02a79-a92d-70e2-8386-2816600e0c60",
                "name": "acme",
                "system_id": "acme.example.org",
                "created_at": "2026-08-22T17:16:51.373027Z",
            },
        }));
        assert!(!scoped.default);
        assert_eq!(scoped.tenant, Some(row("acme", "acme.example.org")));
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_definition_body_trims_and_escapes_what_it_sends() {
        assert_eq!(
            super::tenant_definition("  acme  ", " acme.example.org "),
            r#"{"name":"acme","system_id":"acme.example.org"}"#
        );
        // A quote in the name is escaped by the encoder, never by hand.
        assert_eq!(
            super::tenant_definition("a\"b", "s"),
            r#"{"name":"a\"b","system_id":"s"}"#
        );
    }
}
