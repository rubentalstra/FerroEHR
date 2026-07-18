//! The `/templates/{template_id}` screen — template detail: OPT/WT/example tabs +
//! path catalog.
//!
//! Three tabs over one operational template: **WT** (the Web Template path
//! catalog as an expandable tree + a node inspector), **OPT** (the raw
//! canonical-XML operational template), and **Example** (the CDR-generated
//! example composition, format-switchable). No openEHR spec governs an admin
//! UI — our own design / product extension; the `WebTemplate` shape it renders
//! is `openehr-flat`'s (built from the CDR's OPT), per the ITS-REST
//! Simplified Formats spec (`master04`).
//!
//! Discipline (rules §0/§1/§6/§8): each `#[server]` fn guards the session
//! first and keeps CDR credentials server-side; the view is composed from
//! `.into_any()`-erased sections; the catalog tree is a recursive component
//! that returns `AnyView` at every level (type erasure also breaks the
//! infinite-type recursion); refetching resources render under `<Transition>`.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::builder::catalog::CatalogNode;
use crate::error::AdminUiError;
use crate::format::ReprFormat;

/// Fetch the raw OPT 1.4 operational template (canonical XML).
///
/// GET `definition/template/adl1.4/{template_id}` with
/// `Accept: application/xml`; the `template_id` path segment is percent-encoded
/// server-side.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Cdr`] (e.g. `404` for an unknown template) /
/// [`AdminUiError::Forbidden`] / [`AdminUiError::CdrUnreachable`] from the CDR.
#[server]
pub async fn fetch_template_opt(template_id: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "definition/template/adl1.4/{}",
        urlencoding::encode(&template_id)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/xml")
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// Fetch the OPT, build its Web Template, and distil the browser-side path
/// catalog (the same [`CatalogNode`] tree the Query Builder navigates).
///
/// The OPT XML is parsed with
/// [`openehr_its::opt14::from_xml`](openehr_its::opt14::from_xml) — the OPT 1.4
/// canonical-XML parse entry (root `<template>` = `OPERATIONAL_TEMPLATE`) —
/// then [`openehr_flat::webtemplate::build_web_template`] produces the Web
/// Template, and [`crate::builder::catalog::from_web_template`] the slim
/// serializable tree.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR errors as
/// above; [`AdminUiError::Internal`] when the OPT fails to parse or the Web
/// Template fails to build (the diagnostic named, never a panic).
#[server]
pub async fn fetch_template_catalog(template_id: String) -> Result<CatalogNode, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "definition/template/adl1.4/{}",
        urlencoding::encode(&template_id)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/xml")
        .await?;
    let xml = crate::cdr::CdrClient::expect_success(response)?.body;
    let opt = openehr_its::opt14::from_xml(&xml)
        .map_err(|e| AdminUiError::Internal(format!("OPT 1.4 parse: {e}")))?;
    let web_template = openehr_flat::webtemplate::build_web_template(&opt)
        .map_err(|e| AdminUiError::Internal(format!("WebTemplate build: {e}")))?;
    Ok(crate::builder::catalog::from_web_template(&web_template))
}

/// Fetch the CDR-generated example composition for the template, in `format`.
///
/// GET `definition/template/adl1.4/{template_id}/example` with `Accept` set to
/// the selected representation's media type.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Cdr`] / [`AdminUiError::Forbidden`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR.
#[server]
pub async fn fetch_example(
    template_id: String,
    format: ReprFormat,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "definition/template/adl1.4/{}/example",
        urlencoding::encode(&template_id)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, format.media_type())
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// The template detail screen: a header with a back link + tab bar, then the
/// WT / OPT / Example panes (all mounted, toggled by visibility so switching a
/// tab preserves each pane's loaded state).
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn TemplateDetailPage() -> impl IntoView {
    let params = use_params_map();
    let template_id =
        Signal::derive(move || params.with(|map| map.get("template_id").unwrap_or_default()));

    let selected_tab = RwSignal::new(String::from("wt"));
    let selected_node = RwSignal::new(None::<CatalogNode>);
    let example_format = RwSignal::new(ReprFormat::CanonicalJson);

    // Each tab's resource is gated on the tab being active so only the
    // visible pane fetches (the example fetch in particular triggers the
    // CDR's example GENERATOR — never run it for a tab the user hasn't
    // opened). The stable source keeps loaded state on re-show.
    let catalog: Resource<Result<Option<CatalogNode>, AdminUiError>> = Resource::new(
        move || (selected_tab.get() == "wt").then(|| template_id.get()),
        |active| async move {
            match active {
                Some(id) => fetch_template_catalog(id).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let opt: Resource<Result<Option<String>, AdminUiError>> = Resource::new(
        move || (selected_tab.get() == "opt").then(|| template_id.get()),
        |active| async move {
            match active {
                Some(id) => fetch_template_opt(id).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let example: Resource<Result<Option<String>, AdminUiError>> = Resource::new(
        move || {
            (selected_tab.get() == "example").then(|| (template_id.get(), example_format.get()))
        },
        |active| async move {
            match active {
                Some((id, format)) => fetch_example(id, format).await.map(Some),
                None => Ok(None),
            }
        },
    );

    let wt_pane = wt_tab(catalog, selected_node);
    let opt_pane = opt_tab(opt);
    let example_pane = example_tab(example, example_format);

    view! {
        <Title text=move || format!("Template · {}", template_id.get()) />
        <div class="p-4">
            <div class="flex items-center gap-3 mb-3">
                <leptos_router::components::A
                    href="/templates"
                    attr:class="text-sm text-blue-600 hover:underline"
                >
                    "← Templates"
                </leptos_router::components::A>
                <h1 class="text-xl font-semibold font-mono">{move || template_id.get()}</h1>
            </div>
            <thaw::TabList selected_value=selected_tab>
                <thaw::Tab value="wt">"WT"</thaw::Tab>
                <thaw::Tab value="opt">"OPT"</thaw::Tab>
                <thaw::Tab value="example">"Example"</thaw::Tab>
            </thaw::TabList>
            <div class="mt-4">
                <div class:hidden=move || selected_tab.get() != "wt">{wt_pane}</div>
                <div class:hidden=move || selected_tab.get() != "opt">{opt_pane}</div>
                <div class:hidden=move || selected_tab.get() != "example">{example_pane}</div>
            </div>
        </div>
    }
}

/// The WT tab: a two-pane layout — the recursive path-catalog tree (left) and
/// the node inspector (right), the latter driven by the shared selection
/// signal.
fn wt_tab(
    catalog: Resource<Result<Option<CatalogNode>, AdminUiError>>,
    selected: RwSignal<Option<CatalogNode>>,
) -> AnyView {
    view! {
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <thaw::Card>
                <thaw::CardHeader>
                    <div class="text-sm font-semibold">"Path catalog (WT tree)"</div>
                </thaw::CardHeader>
                <div class="p-3 overflow-auto max-h-[70vh]">
                    <Transition fallback=tree_skeleton>
                        {move || Suspend::new(async move {
                            match catalog.await {
                                Ok(None) => ().into_any(),
                                Ok(Some(root)) => {
                                    // Resolve inside the Transition: an SSR'd ErrorBoundary
                                    // fallback mismatches at hydration in leptos 0.8.
                                    view! {
                                        <ul class="text-sm">
                                            <CatalogTreeNode node=root selected=selected depth=0 />
                                        </ul>
                                    }
                                        .into_any()
                                }
                                Err(e) => catalog_error_view(&e),
                            }
                        })}
                    </Transition>
                </div>
            </thaw::Card>
            <thaw::Card>
                <thaw::CardHeader>
                    <div class="text-sm font-semibold">"Node inspector"</div>
                </thaw::CardHeader>
                <div class="p-3">{node_inspector(selected)}</div>
            </thaw::Card>
        </div>
    }
    .into_any()
}

/// The `<Transition>` fallback while the catalog builds.
fn tree_skeleton() -> impl IntoView {
    view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4 mb-2 ml-4" />
            <thaw::SkeletonItem class="h-4 ml-4" />
        </thaw::Skeleton>
    }
}

/// The catalog error state (e.g. a `404` unknown template, or a `WebTemplate`
/// build failure naming the offending node) with a back link to the list. Used
/// by the tabs that resolve their `Result` inside the `<Transition>` — an SSR'd
/// `ErrorBoundary` fallback mismatches at hydration in leptos 0.8 — so the error
/// (with its back link) renders directly from the resolved `Err` branch.
fn catalog_error_view(error: &AdminUiError) -> AnyView {
    let message = error.to_string();
    view! {
        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
            <thaw::MessageBarBody>
                {message} " — "
                <leptos_router::components::A
                    href="/templates"
                    attr:class="text-blue-600 hover:underline"
                >
                    "back to templates"
                </leptos_router::components::A>
            </thaw::MessageBarBody>
        </thaw::MessageBar>
    }
    .into_any()
}

/// One node of the path catalog: a disclosure toggle (for branches), a
/// selectable label + RM-type tag, and — for a branch — its children.
///
/// Returns [`AnyView`] at every level: the type erasure is what lets the
/// component recurse (an un-erased recursive view would be an infinite type)
/// and keeps rustc's layout-recursion depth bounded on plain `cargo` builds
/// (rules §1).
///
/// # Errors
/// Infallible — renders whatever the (already-fetched) catalog node contains.
#[component]
fn CatalogTreeNode(
    /// The catalog node to render (static data; the component runs once).
    node: CatalogNode,
    /// The shared "inspected node" selection, set when a label is clicked.
    selected: RwSignal<Option<CatalogNode>>,
    /// Depth from the root, used to auto-expand the top two levels.
    depth: i32,
) -> AnyView {
    let has_children = !node.children.is_empty();
    let expanded = RwSignal::new(depth < 2);
    let label = node.label.clone();
    let rm_type = node.rm_type.clone();
    let this_path = node.aql_path.clone();
    let select_node = node.clone();

    // One reactive class closure (a `class:` binding cannot reuse a non-`Copy`
    // closure twice), highlighting the row when it is the inspected node.
    let label_class = move || {
        let selected_here =
            selected.with(|current| current.as_ref().is_some_and(|n| n.aql_path == this_path));
        let mut class = String::from(
            "flex items-center gap-2 rounded px-1 text-left hover:bg-neutral-100 dark:hover:bg-neutral-800",
        );
        if selected_here {
            class.push_str(" bg-blue-100 dark:bg-blue-900");
        }
        class
    };

    let child_views = node
        .children
        .clone()
        .into_iter()
        .map(|child| {
            view! { <CatalogTreeNode node=child selected=selected depth=depth + 1 /> }
        })
        .collect::<Vec<_>>();

    let disclosure = if has_children {
        view! {
            <button
                type="button"
                class="w-4 shrink-0 text-neutral-500"
                on:click=move |_| expanded.update(|open| *open = !*open)
            >
                {move || if expanded.get() { "▾" } else { "▸" }}
            </button>
        }
        .into_any()
    } else {
        view! { <span class="inline-block w-4 shrink-0"></span> }.into_any()
    };

    let children_list = if has_children {
        view! {
            <ul class="ml-4" class:hidden=move || !expanded.get()>
                {child_views}
            </ul>
        }
        .into_any()
    } else {
        ().into_any()
    };

    view! {
        <li class="py-0.5">
            <div class="flex items-center gap-1">
                {disclosure}
                <button
                    type="button"
                    class=label_class
                    on:click=move |_| selected.set(Some(select_node.clone()))
                >
                    <span>{label}</span>
                    <span class="font-mono text-xs text-neutral-500">{rm_type}</span>
                </button>
            </div>
            {children_list}
        </li>
    }
    .into_any()
}

/// The node inspector: nothing until a node is picked, then its aqlPath,
/// rmType, node id, selectability, and any unit / code options as chips.
fn node_inspector(selected: RwSignal<Option<CatalogNode>>) -> AnyView {
    view! {
        {move || match selected.get() {
            None => {
                view! {
                    <p class="text-sm text-neutral-500">
                        "Select a node to inspect its path and metadata."
                    </p>
                }
                    .into_any()
            }
            Some(node) => inspector_body(&node),
        }}
    }
    .into_any()
}

/// Render the inspected node's metadata. `inspector_body` runs once per
/// selection (the caller re-invokes it when the selection changes), so the
/// chip sections are built conditionally here rather than via `<Show>` — a
/// `Vec<AnyView>` is not `Clone`, and `<Show>` would re-call its body.
fn inspector_body(node: &CatalogNode) -> AnyView {
    let node_id = if node.node_id.is_empty() {
        "—".to_owned()
    } else {
        node.node_id.clone()
    };
    let selectable = if node.selectable { "yes" } else { "no" };
    let units_section = chip_section("units", &node.unit_options);
    let codes_section = code_chip_section(node);

    view! {
        <dl class="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1 text-sm">
            <dt class="font-medium text-neutral-500">"label"</dt>
            <dd>{node.label.clone()}</dd>
            <dt class="font-medium text-neutral-500">"rmType"</dt>
            <dd class="font-mono">{node.rm_type.clone()}</dd>
            <dt class="font-medium text-neutral-500">"node id"</dt>
            <dd class="font-mono">{node_id}</dd>
            <dt class="font-medium text-neutral-500">"selectable"</dt>
            <dd>{selectable}</dd>
            <dt class="font-medium text-neutral-500">"aqlPath"</dt>
            <dd class="font-mono break-all">{node.aql_path.clone()}</dd>
        </dl>
        {units_section}
        {codes_section}
    }
    .into_any()
}

/// A titled chip row for a list of plain string options; empty → nothing.
fn chip_section(title: &'static str, values: &[String]) -> AnyView {
    if values.is_empty() {
        return ().into_any();
    }
    let chips = values
        .iter()
        .map(|value| {
            let value = value.clone();
            view! { <thaw::Tag>{value}</thaw::Tag> }
        })
        .collect::<Vec<_>>();
    view! {
        <div class="mt-3">
            <div class="text-xs font-medium text-neutral-500 mb-1">{title}</div>
            <div class="flex flex-wrap gap-1">{chips}</div>
        </div>
    }
    .into_any()
}

/// The coded/ordinal options as chips (`label (code)` when they differ);
/// empty → nothing.
fn code_chip_section(node: &CatalogNode) -> AnyView {
    if node.code_options.is_empty() {
        return ().into_any();
    }
    let chips = node
        .code_options
        .iter()
        .map(|option| {
            let text = if option.label == option.code {
                option.code.clone()
            } else {
                format!("{} ({})", option.label, option.code)
            };
            view! { <thaw::Tag>{text}</thaw::Tag> }
        })
        .collect::<Vec<_>>();
    view! {
        <div class="mt-3">
            <div class="text-xs font-medium text-neutral-500 mb-1">"codes"</div>
            <div class="flex flex-wrap gap-1">{chips}</div>
        </div>
    }
    .into_any()
}

/// The OPT tab: the raw canonical-XML operational template in the shared
/// document pane.
fn opt_tab(opt: Resource<Result<Option<String>, AdminUiError>>) -> AnyView {
    view! {
        <Transition fallback=tree_skeleton>
            {move || Suspend::new(async move {
                match opt.await {
                    Ok(None) => ().into_any(),
                    Ok(Some(xml)) => {
                        // Resolve inside the Transition: an SSR'd ErrorBoundary fallback
                        // mismatches at hydration in leptos 0.8 (E2E console gate).
                        view! { <crate::components::format_view::DocumentPane body=xml /> }
                            .into_any()
                    }
                    Err(e) => catalog_error_view(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// The Example tab: a format selector (JSON / XML / FLAT / STRUCTURED) over the
/// shared selection signal — a change refetches the resource — plus the
/// pretty-printed example composition in the document pane.
fn example_tab(
    example: Resource<Result<Option<String>, AdminUiError>>,
    format: RwSignal<ReprFormat>,
) -> AnyView {
    let offered = vec![
        ReprFormat::CanonicalJson,
        ReprFormat::CanonicalXml,
        ReprFormat::Flat,
        ReprFormat::Structured,
    ];
    view! {
        <div class="space-y-3">
            <crate::components::format_view::FormatSelector offered=offered selected=format />
            <Transition fallback=tree_skeleton>
                {move || Suspend::new(async move {
                    match example.await {
                        Ok(None) => ().into_any(),
                        Ok(Some(raw)) => {
                            let pretty = crate::components::format_view::pretty_body(
                                &raw,
                                format.get_untracked(),
                            );
                            // Resolve inside the Transition: an SSR'd ErrorBoundary fallback
                            // mismatches at hydration in leptos 0.8 (E2E console gate).
                            view! { <crate::components::format_view::DocumentPane body=pretty /> }
                                .into_any()
                        }
                        Err(e) => catalog_error_view(&e),
                    }
                })}
            </Transition>
        </div>
    }
    .into_any()
}
