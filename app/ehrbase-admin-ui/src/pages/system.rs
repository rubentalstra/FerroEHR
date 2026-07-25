//! The `/system` screen: an operational panel over the CDR — server status,
//! SMART service-discovery, repository usage, the CDR's own served `OpenAPI`
//! document (rendered by our own grouped-endpoint component, never a Swagger
//! embed), the redacted runtime configuration, and a link into the `/audit`
//! activity browser.
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound: the SMART discovery document follows
//! `docs/specs/openehr/ITS-REST/docs/smart_app_launch/master04-service_discovery.adoc`.
//!
//! Each card is an `.into_any()`-erased section local (rules §1) with its own
//! [`Resource`] + `<Suspense>` skeleton that resolves its `Result` inside the
//! suspense (rendering an error bar on failure) rather than through an
//! `<ErrorBoundary>` — an SSR'd `ErrorBoundary` fallback mismatches at hydration
//! in leptos 0.8 — so one failing card never blanks the page. Every co-located
//! `#[server]` fn guards with [`require_session`](crate::session::require_session)
//! first — server functions are a public HTTP API (rules §0), and the CDR
//! credential never reaches client-visible state.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;

use leptos_router::components::A;

use crate::components::field::BTN_SECONDARY;
use crate::components::page_header::PageHeader;
use crate::components::surface::titled_card;
use crate::error::AdminUiError;

/// The CDR's SMART service-discovery document, or `None` when the CDR
/// advertises none (a `404` from `/.well-known/smart-configuration` is a
/// first-class "SMART disabled" state, not an error).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx, non-404 CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_smart_config() -> Result<Option<String>, AdminUiError> {
    crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    // Served relative to the platform base URL, not the ITS-REST base
    // (master04-service_discovery.adoc §"the configuration endpoint").
    let url = state.cdr.origin_url(".well-known/smart-configuration");
    let response = state.cdr.get_public(&url, "application/json").await?;
    if response.status == 404 {
        return Ok(None);
    }
    Ok(Some(crate::cdr::CdrClient::expect_success(response)?.body))
}

/// The CDR's own natively served OpenAPI document, raw JSON.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_openapi() -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    // NOTE: no openEHR spec governs an OAS-serving endpoint — our own design /
    // product extension. The CDR serves ONLY its own natively generated
    // document (never a vendored OAS) at this default path
    // ("/ehrbase/rest/api-docs/openapi.json", configurable CDR-side),
    // outside auth as a public discoverability surface.
    let url = state.cdr.origin_url("ehrbase/rest/api-docs/openapi.json");
    let response = state.cdr.get_public(&url, "application/json").await?;
    // Public surface; if a deployment happens to gate it, retry with the
    // session credential before giving up.
    let response = if response.status == 401 {
        state
            .cdr
            .get(&session.credential, &url, "application/json")
            .await?
    } else {
        response
    };
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// The `/system` screen: four independent, individually-failing cards.
/// The redacted effective CDR configuration (`GET /admin/config` — the
/// CDR's own extension endpoint; secrets are redacted structurally
/// server-side, this fn only relays). A `404` means the admin API is
/// disabled and `401`/`403` that the session lacks the ADMIN role — both
/// first-class rendered states for the panel, not failures.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR
/// transport errors pass through; non-2xx answers normalize via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_admin_config() -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("admin/config");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    Ok(serde_json::from_str::<serde_json::Value>(&body)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or(body))
}

/// Per-template composition counts ("repo usage"; measured ~0.3 s per
/// count AQL — plain AQL suffices, no CDR stats endpoint). Bounded to the
/// first 25 templates, sorted by count descending.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR errors
/// normalized by the underlying calls.
#[server]
pub async fn template_usage() -> Result<Vec<(String, i64)>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let templates = crate::pages::templates::list_templates().await?;
    let mut usage = Vec::new();
    for row in templates.into_iter().take(25) {
        let aql = format!(
            "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c \
             WHERE c/archetype_details/template_id/value = '{}'",
            row.template_id.replace('\'', "''")
        );
        let body = crate::pages::ehrs::aql_request_body(&aql, &serde_json::json!({}), 0);
        let url = state.cdr.rest_v1("query/aql");
        let response = state
            .cdr
            .post(
                &session.credential,
                &url,
                "application/json",
                "application/json",
                &[],
                body,
            )
            .await?;
        let response = crate::cdr::CdrClient::expect_success(response)?;
        let count = serde_json::from_str::<serde_json::Value>(&response.body)
            .ok()
            .and_then(|v| {
                v.get("rows")?
                    .as_array()?
                    .first()?
                    .as_array()?
                    .first()?
                    .as_i64()
            })
            .unwrap_or(0);
        usage.push((row.template_id, count));
    }
    usage.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    Ok(usage)
}

#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn SystemPage() -> impl IntoView {
    let status = status_card();
    let smart = smart_card();
    let openapi = openapi_card();
    let activity = activity_log_card();
    let usage = usage_card();
    let config = config_card();

    view! {
        <Title text="System" />
        <div class="p-6">
            <PageHeader
                title="System"
                subtitle="CDR status, SMART discovery, repository usage, the served OpenAPI surface, and the redacted runtime configuration."
            />
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-start">
                {status} {smart} {usage} {openapi} {config} {activity}
            </div>
        </div>
    }
}

/// The repo-usage card: per-template composition counts.
fn usage_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { template_usage().await });
    let body = view! {
        <Suspense fallback=|| {
            view! {
                <thaw::Skeleton>
                    <thaw::SkeletonItem class="h-24" />
                </thaw::Skeleton>
            }
        }>
            {move || {
                Suspend::new(async move {
                    match resource.await {
                        Ok(rows) if rows.is_empty() => {
                            view! {
                                <p class="text-sm text-ink-muted">
                                    "No templates yet — usage appears once compositions are committed."
                                </p>
                            }
                                .into_any()
                        }
                        Ok(rows) => {
                            let body = rows
                                .into_iter()
                                .map(|(template_id, count)| {
                                    view! {
                                        <tr class=crate::components::data_table::ROW>
                                            <td class=crate::components::data_table::CELL_MONO>
                                                {template_id}
                                            </td>
                                            <td class="px-3 py-2 text-right tabular-nums">{count}</td>
                                        </tr>
                                    }
                                        .into_any()
                                })
                                .collect_view()
                                .into_any();
                            crate::components::data_table::table_shell(
                                &["Template", "Compositions"],
                                body,
                            )
                        }
                        Err(e) => crate::components::format_view::inline_error(&e),
                    }
                })
            }}
        </Suspense>
    }
    .into_any();
    titled_card("Repository usage", false, body)
}

/// The runtime-configuration card: the CDR's redacted effective config
/// (read-only). Admin-API-off (404) and insufficient-role (401/403) render
/// as first-class states.
fn config_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { fetch_admin_config().await });
    let body = view! {
        <Suspense fallback=|| {
            view! {
                <thaw::Skeleton>
                    <thaw::SkeletonItem class="h-24" />
                </thaw::Skeleton>
            }
        }>
            {move || {
                Suspend::new(async move {
                    match resource.await {
                        Ok(config) => {
                            view! {
                                <pre class="max-h-96 overflow-auto rounded-card border border-edge bg-sunken p-3 font-mono text-xs leading-relaxed text-ink">
                                    {config}
                                </pre>
                            }
                                .into_any()
                        }
                        Err(AdminUiError::Cdr { status: 404, .. }) => {
                            view! {
                                <p class="text-sm text-ink-muted">
                                    "The CDR's admin API is disabled — enable [admin] to expose the configuration view."
                                </p>
                            }
                                .into_any()
                        }
                        Err(AdminUiError::Forbidden(_)) => {
                            view! {
                                <p class="text-sm text-ink-muted">
                                    "This session lacks the ADMIN role — the configuration view needs an admin sign-in."
                                </p>
                            }
                                .into_any()
                        }
                        Err(e) => crate::components::format_view::inline_error(&e),
                    }
                })
            }}
        </Suspense>
    }
    .into_any();
    titled_card("Runtime configuration (redacted)", true, body)
}

/// The `<Suspense>` fallback shared by every data-backed card.
fn card_skeleton() -> impl IntoView {
    view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4" />
        </thaw::Skeleton>
    }
}

/// Status card: `fetch_status` JSON → a definition list. A transport or parse
/// failure resolves inside the `<Suspense>` (an SSR'd `ErrorBoundary` fallback
/// mismatches at hydration in leptos 0.8) as an explicit DOWN state.
fn status_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { crate::auth::fetch_status().await });
    let body = view! {
        <Suspense fallback=card_skeleton>
            {move || Suspend::new(async move {
                match resource.await.and_then(|body| status_body(&body)) {
                    Ok(view) => view,
                    Err(e) => {
                        // Resolve inside the Suspense (E2E console gate): render the parsed
                        // status, or the explicit DOWN chip on transport/parse failure.
                        view! {
                            <div>
                                <span class="inline-flex items-center gap-1.5 rounded-full border border-edge bg-raised px-2.5 py-1 text-xs font-medium text-danger">
                                    <span class="h-1.5 w-1.5 rounded-full bg-danger"></span>
                                    "DOWN"
                                </span>
                                <p class="mt-2 text-sm text-danger">{e.to_string()}</p>
                            </div>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
    .into_any();
    titled_card("Status", false, body)
}

/// The uniform card error state: the domain-error message in an error
/// `MessageBar` (the same shape the removed per-card `ErrorBoundary` fallbacks
/// rendered).
fn card_error(error: &AdminUiError) -> AnyView {
    let message = error.to_string();
    view! {
        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
            <thaw::MessageBarBody>{message}</thaw::MessageBarBody>
        </thaw::MessageBar>
    }
    .into_any()
}

/// Render the parsed `/ehrbase/rest/status` document: an UP/DOWN pill plus every
/// scalar field as a definition list.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
fn status_body(body: &str) -> Result<AnyView, AdminUiError> {
    let doc = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|e| AdminUiError::Internal(format!("status JSON: {e}")))?;
    let up = doc
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|s| s.eq_ignore_ascii_case("UP"));
    let rows = scalar_rows(&doc)
        .into_iter()
        .map(|(k, v)| {
            view! {
                <dt class="font-medium text-ink-muted">{k}</dt>
                <dd class="font-mono break-all text-ink">{v}</dd>
            }
        })
        .collect::<Vec<_>>();
    let (dot, text_cls, label) = if up {
        ("bg-ok", "text-ink", "UP")
    } else {
        ("bg-danger", "text-danger", "DOWN")
    };
    Ok(view! {
        <div>
            <span class=format!(
                "inline-flex items-center gap-1.5 rounded-full border border-edge bg-raised px-2.5 py-1 text-xs font-medium {text_cls}",
            )>
                <span class=format!("h-1.5 w-1.5 rounded-full {dot}")></span>
                {label}
            </span>
            <dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">{rows}</dl>
        </div>
    }
    .into_any())
}

/// SMART card: `fetch_smart_config` → a "disabled" state, or the advertised
/// endpoints (out-links) plus capability chips.
fn smart_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { fetch_smart_config().await });
    let body = view! {
        <Suspense fallback=card_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(config) => smart_body(config),
                    Err(e) => card_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any();
    titled_card("SMART", false, body)
}

/// Render the SMART discovery document. `None` is the neutral disabled state;
/// `Some(json)` renders the authentication endpoints as out-links and the
/// advertised capability lists as chips (master04-service_discovery.adoc
/// §"Authentication Endpoints").
fn smart_body(config: Option<String>) -> AnyView {
    let Some(raw) = config else {
        return view! {
            <thaw::MessageBar intent=thaw::MessageBarIntent::Info>
                <thaw::MessageBarBody>
                    "SMART: disabled — the CDR advertises no service-discovery document."
                </thaw::MessageBarBody>
            </thaw::MessageBar>
        }
        .into_any();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return view! {
            <thaw::MessageBar intent=thaw::MessageBarIntent::Warning>
                <thaw::MessageBarBody>
                    "SMART configuration returned, but it is not valid JSON."
                </thaw::MessageBarBody>
            </thaw::MessageBar>
        }
        .into_any();
    };

    // Advertised authentication endpoints → out-links (present ones only).
    let endpoints = [
        ("authorization_endpoint", "Authorization"),
        ("token_endpoint", "Token"),
        ("jwks_uri", "JWKS"),
        ("introspection_endpoint", "Introspection"),
    ];
    let links = endpoints
        .iter()
        .filter_map(|(key, label)| {
            doc.get(*key)
                .and_then(serde_json::Value::as_str)
                .map(|url| {
                    let href = url.to_owned();
                    let label = *label;
                    view! {
                        <a
                            href=href
                            target="_blank"
                            rel="noreferrer"
                            class="text-sm text-accent hover:underline"
                        >
                            {label}
                            " ↗"
                        </a>
                    }
                })
        })
        .collect::<Vec<_>>();

    // Advertised capability lists → chips.
    let chip_groups = [
        ("grant_types_supported", "Grant types"),
        ("code_challenge_methods_supported", "PKCE methods"),
        ("scopes_supported", "Scopes"),
    ];
    let chips = chip_groups
        .iter()
        .filter_map(|(key, label)| {
            doc.get(*key)
                .and_then(serde_json::Value::as_array)
                .map(|arr| {
                    let tags = arr
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(|s| {
                            let s = s.to_owned();
                            view! { <span class="rounded-full bg-accent-subtle px-2 py-0.5 text-xs text-accent-ink">{s}</span> }
                        })
                        .collect::<Vec<_>>();
                    let label = *label;
                    view! {
                        <div class="mt-2">
                            <div class="text-xs font-medium text-ink-muted mb-1">{label}</div>
                            <div class="flex flex-wrap gap-1">{tags}</div>
                        </div>
                    }
                })
        })
        .collect::<Vec<_>>();

    view! {
        <div>
            <div class="flex flex-wrap gap-3">{links}</div>
            {chips}
        </div>
    }
    .into_any()
}

/// Served-OpenAPI card: `fetch_openapi` → our own grouped endpoint list.
fn openapi_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { fetch_openapi().await });
    let body = view! {
        <Suspense fallback=card_skeleton>
            {move || Suspend::new(async move {
                let rendered = resource
                    .await
                    .and_then(|doc_str| {
                        let doc = serde_json::from_str::<serde_json::Value>(&doc_str)
                            .map_err(|e| AdminUiError::Internal(format!("openapi JSON: {e}")))?;
                        Ok::<_, AdminUiError>(openapi_body(&doc))
                    });
                match rendered {
                    Ok(view) => view,
                    Err(e) => card_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any();
    titled_card("Served OpenAPI", true, body)
}

/// Render the served `OpenAPI` document as a grouped, scrollable endpoint list.
fn openapi_body(doc: &serde_json::Value) -> AnyView {
    let groups = group_openapi_paths(doc);
    if groups.is_empty() {
        return view! { <div class="text-sm text-ink-muted">"No paths advertised."</div> }
            .into_any();
    }
    let sections = groups
        .into_iter()
        .map(|(group, rows)| {
            let header = format!("{group} ({})", rows.len());
            let items = rows
                .into_iter()
                .map(|(method, path)| {
                    view! {
                        <li class="flex gap-2 py-0.5 text-sm">
                            <code class="font-mono font-semibold shrink-0 text-accent">
                                {method}
                            </code>
                            <span class="font-mono break-all text-ink">{path}</span>
                        </li>
                    }
                })
                .collect::<Vec<_>>();
            view! {
                <section class="mb-3">
                    <h3 class="text-xs font-semibold uppercase tracking-wide text-ink-muted mb-1">
                        {header}
                    </h3>
                    <ul>{items}</ul>
                </section>
            }
        })
        .collect::<Vec<_>>();
    view! { <div class="overflow-x-auto max-h-96 overflow-y-auto">{sections}</div> }.into_any()
}

/// Activity-log card: a compact pointer into the `/audit` screen, which is
/// the console's real activity browser (the CDR's local Audit Record
/// Repository over the RESTful-ATNA ITI-81 retrieval). This card deliberately
/// fetches nothing — duplicating a filterable, paged browser inside a panel
/// tile would be a second, worse audit surface.
fn activity_log_card() -> AnyView {
    let body = view! {
        <div class="flex flex-col items-start gap-3">
            <p class="text-sm text-ink-muted">
                "Who accessed what, with what outcome: the ATNA security audit trail, "
                "filterable by time window, patient, principal, outcome and action."
            </p>
            <A href="/audit" attr:class=BTN_SECONDARY>
                <leptos_icons::Icon icon=icondata_lu::LuActivity width="14" height="14" />
                " Open audit browser"
            </A>
        </div>
    }
    .into_any();
    titled_card("Audit log", false, body)
}

/// Every top-level scalar field of a JSON object as `(key, text)`, sorted by
/// key for a stable (hydration-safe) render order.
fn scalar_rows(doc: &serde_json::Value) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    if let Some(obj) = doc.as_object() {
        for (key, value) in obj {
            let text = match value {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            };
            if let Some(text) = text {
                rows.push((key.clone(), text));
            }
        }
    }
    rows.sort();
    rows
}

/// Group an `OpenAPI` document's operations by their first `tag` (falling back
/// to the first path segment, then `"default"`), as sorted `(group, rows)`
/// where each row is `(METHOD, path)`. Sorted throughout so server and client
/// render the identical structure (rules §8 — no non-deterministic order).
fn group_openapi_paths(doc: &serde_json::Value) -> Vec<(String, Vec<(String, String)>)> {
    const METHODS: [&str; 8] = [
        "get", "put", "post", "delete", "patch", "options", "head", "trace",
    ];
    let mut groups: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    let Some(paths) = doc.get("paths").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    for (path, item) in paths {
        let Some(methods) = item.as_object() else {
            continue;
        };
        for method in METHODS {
            let Some(op) = methods.get(method) else {
                continue;
            };
            let group = op
                .get("tags")
                .and_then(serde_json::Value::as_array)
                .and_then(|tags| tags.first())
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    path.trim_start_matches('/')
                        .split('/')
                        .next()
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "default".to_owned());
            groups
                .entry(group)
                .or_default()
                .push((method.to_uppercase(), path.clone()));
        }
    }
    let mut out: Vec<(String, Vec<(String, String)>)> = groups.into_iter().collect();
    for (_, rows) in &mut out {
        rows.sort();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{group_openapi_paths, scalar_rows};

    #[test]
    fn groups_by_tag_then_path_segment_sorted() {
        let doc = serde_json::json!({
            "paths": {
                "/ehr/{ehr_id}": {
                    "put": {"tags": ["EHR"]},
                    "get": {"tags": ["EHR"]}
                },
                "/query/aql": {"post": {"tags": ["Query"]}},
                "/misc": {"get": {}},
                "/misc/thing": {"parameters": []}
            }
        });
        let groups = group_openapi_paths(&doc);
        let names: Vec<String> = groups.iter().map(|(g, _)| g.clone()).collect();
        // BTreeMap keys sorted: "EHR", "Query", then the path-segment fallback "misc".
        assert_eq!(names, vec!["EHR", "Query", "misc"]);
        let ehr = groups
            .iter()
            .find(|(g, _)| g == "EHR")
            .map(|(_, r)| r.clone())
            .expect("EHR group present");
        // Methods sorted within a path (GET before PUT).
        assert_eq!(
            ehr,
            vec![
                ("GET".to_owned(), "/ehr/{ehr_id}".to_owned()),
                ("PUT".to_owned(), "/ehr/{ehr_id}".to_owned()),
            ]
        );
        // A path item with only non-operation keys contributes nothing.
        assert!(
            groups
                .iter()
                .all(|(_, rows)| rows.iter().all(|(_, p)| p != "/misc/thing"))
        );
    }

    #[test]
    fn empty_document_yields_no_groups() {
        assert!(group_openapi_paths(&serde_json::json!({})).is_empty());
        assert!(group_openapi_paths(&serde_json::json!({"paths": {}})).is_empty());
    }

    #[test]
    fn scalar_rows_are_scalar_only_and_key_sorted() {
        let doc = serde_json::json!({
            "status": "UP",
            "server_version": "3.1.1",
            "count": 5,
            "flag": true,
            "nested": {"x": 1},
            "list": [1, 2]
        });
        let rows = scalar_rows(&doc);
        assert_eq!(
            rows,
            vec![
                ("count".to_owned(), "5".to_owned()),
                ("flag".to_owned(), "true".to_owned()),
                ("server_version".to_owned(), "3.1.1".to_owned()),
                ("status".to_owned(), "UP".to_owned()),
            ]
        );
    }
}
