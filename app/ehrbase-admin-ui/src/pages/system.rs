//! The `/system` screen: a four-card operational panel
//! over the CDR — server status, SMART service-discovery, the CDR's own
//! served `OpenAPI` document (rendered by our own grouped-endpoint component,
//! never a Swagger embed), and the activity log.
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound: the SMART discovery document follows
//! `docs/specs/openehr/ITS-REST/docs/smart_app_launch/master04-service_discovery.adoc`.
//!
//! Each card is an `.into_any()`-erased section local (rules §1) with its own
//! [`Resource`] + `<Suspense>` skeleton + `<ErrorBoundary>` error bar, so one
//! failing card never blanks the page. Every co-located `#[server]` fn guards
//! with [`require_session`](crate::session::require_session) first — server
//! functions are a public HTTP API (rules §0), and the CDR credential never
//! reaches client-visible state.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;

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
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn SystemPage() -> impl IntoView {
    let status = status_card();
    let smart = smart_card();
    let openapi = openapi_card();
    let activity = activity_log_card();

    view! {
        <Title text="System" />
        <div class="p-4">
            <h1 class="text-xl font-semibold mb-4">"System"</h1>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {status} {smart} {openapi} {activity}
            </div>
        </div>
    }
}

/// A uniform card shell: a titled [`thaw::Card`] wrapping an already-erased
/// body. `full_width` spans both grid columns.
fn card_shell(title: &'static str, full_width: bool, body: AnyView) -> AnyView {
    let class = if full_width { "lg:col-span-2" } else { "" };
    view! {
        <thaw::Card class=class>
            <thaw::CardHeader>
                <div class="text-sm font-semibold">{title}</div>
            </thaw::CardHeader>
            {body}
        </thaw::Card>
    }
    .into_any()
}

/// The `<Suspense>` fallback shared by every data-backed card.
fn card_skeleton() -> impl IntoView {
    view! {
        <div class="p-4">
            <thaw::Skeleton>
                <thaw::SkeletonItem class="h-4 mb-2" />
                <thaw::SkeletonItem class="h-4 mb-2" />
                <thaw::SkeletonItem class="h-4" />
            </thaw::Skeleton>
        </div>
    }
}

/// Status card: `fetch_status` JSON → a definition list. A transport failure
/// surfaces through the `<ErrorBoundary>` as an explicit DOWN state.
fn status_card() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { crate::auth::fetch_status().await });
    let body = view! {
        <Suspense fallback=card_skeleton>
            <ErrorBoundary fallback=move |errors| {
                view! {
                    <div class="p-4">
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>
                                <span class="font-semibold">"● DOWN — "</span>
                                {move || {
                                    errors
                                        .get()
                                        .into_iter()
                                        .map(|(_, e)| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                }}
                            </thaw::MessageBarBody>
                        </thaw::MessageBar>
                    </div>
                }
            }>
                {move || Suspend::new(async move {
                    let body = resource.await?;
                    status_body(&body)
                })}
            </ErrorBoundary>
        </Suspense>
    }
    .into_any();
    card_shell("Status", false, body)
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
                <dt class="font-medium text-neutral-500">{k}</dt>
                <dd class="font-mono break-all">{v}</dd>
            }
        })
        .collect::<Vec<_>>();
    let (pill, pill_class) = if up {
        ("● UP", "mb-3 font-semibold text-emerald-600")
    } else {
        ("● DOWN", "mb-3 font-semibold text-red-600")
    };
    Ok(view! {
        <div class="p-4">
            <div class=pill_class>{pill}</div>
            <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">{rows}</dl>
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
            <ErrorBoundary fallback=move |errors| {
                view! {
                    <div class="p-4">
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>
                                {move || {
                                    errors
                                        .get()
                                        .into_iter()
                                        .map(|(_, e)| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                }}
                            </thaw::MessageBarBody>
                        </thaw::MessageBar>
                    </div>
                }
            }>
                {move || Suspend::new(async move {
                    let config = resource.await?;
                    Ok::<_, AdminUiError>(smart_body(config))
                })}
            </ErrorBoundary>
        </Suspense>
    }
    .into_any();
    card_shell("SMART", false, body)
}

/// Render the SMART discovery document. `None` is the neutral disabled state;
/// `Some(json)` renders the authentication endpoints as out-links and the
/// advertised capability lists as chips (master04-service_discovery.adoc
/// §"Authentication Endpoints").
fn smart_body(config: Option<String>) -> AnyView {
    let Some(raw) = config else {
        return view! {
            <div class="p-4">
                <thaw::MessageBar intent=thaw::MessageBarIntent::Info>
                    <thaw::MessageBarBody>
                        "SMART: disabled — the CDR advertises no service-discovery document."
                    </thaw::MessageBarBody>
                </thaw::MessageBar>
            </div>
        }
        .into_any();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return view! {
            <div class="p-4">
                <thaw::MessageBar intent=thaw::MessageBarIntent::Warning>
                    <thaw::MessageBarBody>
                        "SMART configuration returned, but it is not valid JSON."
                    </thaw::MessageBarBody>
                </thaw::MessageBar>
            </div>
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
                            class="text-sm text-blue-600 hover:underline"
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
                            view! { <thaw::Tag>{s}</thaw::Tag> }
                        })
                        .collect::<Vec<_>>();
                    let label = *label;
                    view! {
                        <div class="mt-2">
                            <div class="text-xs font-medium text-neutral-500 mb-1">{label}</div>
                            <div class="flex flex-wrap gap-1">{tags}</div>
                        </div>
                    }
                })
        })
        .collect::<Vec<_>>();

    view! {
        <div class="p-4">
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
            <ErrorBoundary fallback=move |errors| {
                view! {
                    <div class="p-4">
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>
                                {move || {
                                    errors
                                        .get()
                                        .into_iter()
                                        .map(|(_, e)| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                }}
                            </thaw::MessageBarBody>
                        </thaw::MessageBar>
                    </div>
                }
            }>
                {move || Suspend::new(async move {
                    let doc_str = resource.await?;
                    let doc = serde_json::from_str::<serde_json::Value>(&doc_str)
                        .map_err(|e| AdminUiError::Internal(format!("openapi JSON: {e}")))?;
                    Ok::<_, AdminUiError>(openapi_body(&doc))
                })}
            </ErrorBoundary>
        </Suspense>
    }
    .into_any();
    card_shell("Served OpenAPI", true, body)
}

/// Render the served `OpenAPI` document as a grouped, scrollable endpoint list.
fn openapi_body(doc: &serde_json::Value) -> AnyView {
    let groups = group_openapi_paths(doc);
    if groups.is_empty() {
        return view! { <div class="p-4 text-sm text-neutral-500">"No paths advertised."</div> }
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
                            <code class="font-mono font-semibold shrink-0">{method}</code>
                            <span class="font-mono break-all">{path}</span>
                        </li>
                    }
                })
                .collect::<Vec<_>>();
            view! {
                <section class="mb-3">
                    <h3 class="text-xs font-semibold uppercase tracking-wide text-neutral-500 mb-1">
                        {header}
                    </h3>
                    <ul>{items}</ul>
                </section>
            }
        })
        .collect::<Vec<_>>();
    view! { <div class="p-4 overflow-x-auto max-h-96 overflow-y-auto">{sections}</div> }.into_any()
}

/// Activity log card. The CDR exposes no system-log read surface today, so
/// this renders an informative placeholder rather than inventing an endpoint.
fn activity_log_card() -> AnyView {
    // TODO: the CDR has no system-log / ATNA read endpoint — audit is a
    // write-side concern (ATNA audit middleware in `app/ehrbase-rest`) and
    // audit rows are folded into version reads, with no GET surface listing
    // recent events. When a read-only system-log endpoint lands, add a
    // `fetch_activity_log` #[server] fn and render events in a `thaw::Table`
    // here (with an explicit `<tbody>`).
    let body = view! {
        <div class="p-4">
            <thaw::MessageBar intent=thaw::MessageBarIntent::Info>
                <thaw::MessageBarBody>
                    "No activity-log read surface is exposed by the CDR. ATNA audit is captured on the write path; there is currently no endpoint to list recent events."
                </thaw::MessageBarBody>
            </thaw::MessageBar>
        </div>
    }
    .into_any();
    card_shell("Activity log", true, body)
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
