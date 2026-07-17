//! The `/queries/aql` screen: the raw AQL editor.
//!
//! A plain-text AQL editor with BFF-local validation, an optional JSON
//! parameter-bindings pane, a paged result table, and a save-as-stored-query
//! row. Seeds its editor from the `?aql=` query parameter (the "open in raw
//! editor" hand-off from the point-and-click builder), which the router decodes
//! for us. No openEHR spec governs an admin UI — our own design / product
//! extension; the wire it drives IS spec-bound (ITS-REST Query/Definition APIs).
//!
//! Discipline (rules §0/§1/§5/§6/§8): no new `#[server]` fn — the screen reuses
//! [`validate_aql`], [`run_aql`] and [`store_query`], each of which guards its
//! own session. The editor is a controlled `<textarea>` (child text +
//! `prop:value`); the result table renders under `<Transition>` and reuses the
//! shared render helpers from [`crate::pages::query_builder`] and
//! [`crate::pages::ehrs`]. The view is composed from `.into_any()`-erased
//! sections.

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use crate::error::AdminUiError;
use crate::pages::ehrs::{ResultPage, error_bar, table_skeleton};
use crate::pages::query_builder::{paging_buttons, results_view};
use crate::queries_api::{run_aql, store_query, validate_aql};

/// The raw AQL editor screen.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn QueryAqlPage() -> impl IntoView {
    let query_map = use_query_map();
    // Deterministic from the URL (decoded by the router), so hydration-safe: the
    // server pass and the client hydration read the same `?aql=` value.
    let initial_aql = query_map.with_untracked(|m| m.get("aql").unwrap_or_default());
    let aql = RwSignal::new(initial_aql);
    let params = RwSignal::new(String::new());
    let save_name = RwSignal::new(String::new());
    let offset = RwSignal::new(0_u32);
    let ran = RwSignal::new(None::<(String, String)>);

    let validate_action: Action<String, Result<(), AdminUiError>> = Action::new(|aql: &String| {
        let aql = aql.clone();
        async move { validate_aql(aql).await }
    });
    let save_action: Action<(String, String), Result<(), AdminUiError>> =
        Action::new(|input: &(String, String)| {
            let (name, aql) = input.clone();
            async move { store_query(name, aql).await }
        });
    let results: Resource<Result<Option<ResultPage>, AdminUiError>> = Resource::new(
        move || (ran.get(), offset.get()),
        |(ran, off)| async move {
            match ran {
                Some((aql, parameters)) => run_aql(aql, parameters, off).await.map(Some),
                None => Ok(None),
            }
        },
    );

    let editor = editor_section(aql, validate_action);
    let parameters = parameters_section(params);
    let run_save = run_save_section(aql, params, save_name, save_action, ran, offset);
    let results_pane = results_section(results, offset);

    view! {
        <Title text="AQL editor" />
        <div class="p-4 space-y-4">
            <h1 class="text-xl font-semibold">"AQL editor"</h1>
            {editor}
            {parameters}
            {run_save}
            {results_pane}
        </div>
    }
}

/// The AQL editor: a controlled `<textarea>` plus a BFF-local Validate button
/// and its diagnostic.
fn editor_section(
    aql: RwSignal<String>,
    validate_action: Action<String, Result<(), AdminUiError>>,
) -> AnyView {
    let validate_click = move |_| {
        validate_action.dispatch(aql.get_untracked());
    };
    view! {
        <thaw::Card>
            <div class="p-3 space-y-2">
                <label class="text-sm font-medium" r#for="aql-editor">
                    "AQL"
                </label>
                <textarea
                    id="aql-editor"
                    class="w-full h-40 rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-3 py-2 text-sm font-mono"
                    placeholder="SELECT c FROM EHR e CONTAINS COMPOSITION c"
                    prop:value=move || aql.get()
                    on:input:target=move |ev| aql.set(ev.target().value())
                >
                    {aql.get_untracked()}
                </textarea>
                <div class="flex items-center gap-3">
                    <thaw::Button
                        disabled=Signal::derive(move || aql.with(std::string::String::is_empty))
                        on_click=validate_click
                    >
                        "Validate"
                    </thaw::Button>
                    {validate_feedback(validate_action)}
                </div>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// The Validate action's inline result (parser diagnostic on failure).
fn validate_feedback(validate_action: Action<String, Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || validate_action.pending().get()>
                <span class="text-neutral-500">"Validating…"</span>
            </Show>
            {move || match validate_action.value().get() {
                Some(Ok(())) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Success>
                            <thaw::MessageBarBody>"AQL is valid."</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                Some(Err(error)) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The parameter-bindings pane: a JSON object bound as AQL `query_parameters`.
fn parameters_section(params: RwSignal<String>) -> AnyView {
    view! {
        <thaw::Card>
            <div class="p-3 space-y-2">
                <label class="text-sm font-medium" r#for="aql-params">
                    "Parameters (JSON object)"
                </label>
                <textarea
                    id="aql-params"
                    class="w-full h-24 rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-3 py-2 text-sm font-mono"
                    placeholder="{\"ehr_id\": \"...\"}"
                    prop:value=move || params.get()
                    on:input:target=move |ev| params.set(ev.target().value())
                >
                    {params.get_untracked()}
                </textarea>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// The Run + Save surface: Run executes the AQL with the parameter bindings at
/// the first page; Save stores it as a namespaced stored query.
fn run_save_section(
    aql: RwSignal<String>,
    params: RwSignal<String>,
    save_name: RwSignal<String>,
    save_action: Action<(String, String), Result<(), AdminUiError>>,
    ran: RwSignal<Option<(String, String)>>,
    offset: RwSignal<u32>,
) -> AnyView {
    let empty_aql = Signal::derive(move || aql.with(std::string::String::is_empty));
    let save_disabled = Signal::derive(move || {
        aql.with(std::string::String::is_empty) || save_name.with(std::string::String::is_empty)
    });
    let run_click = move |_| {
        ran.set(Some((aql.get_untracked(), params.get_untracked())));
        offset.set(0);
    };
    let save_click = move |_| {
        save_action.dispatch((save_name.get_untracked(), aql.get_untracked()));
    };
    view! {
        <thaw::Card>
            <div class="p-3 space-y-3">
                <div class="flex flex-wrap items-end gap-3">
                    <thaw::Button
                        appearance=thaw::ButtonAppearance::Primary
                        disabled=empty_aql
                        on_click=run_click
                    >
                        "Run"
                    </thaw::Button>
                    <div class="flex items-end gap-2">
                        <label class="flex flex-col gap-0.5 text-xs">
                            <span class="text-neutral-500">"Save as (namespace::name)"</span>
                            <input
                                id="aql-save-name"
                                type="text"
                                placeholder="org::my_query"
                                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-56"
                                prop:value=move || save_name.get()
                                on:input:target=move |ev| save_name.set(ev.target().value())
                            />
                        </label>
                        <thaw::Button disabled=save_disabled on_click=save_click>
                            "Save"
                        </thaw::Button>
                    </div>
                </div>
                {save_feedback(save_action)}
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// The Save action's inline feedback.
fn save_feedback(save_action: Action<(String, String), Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || save_action.pending().get()>
                <span class="text-neutral-500">"Saving…"</span>
            </Show>
            {move || match save_action.value().get() {
                Some(Ok(())) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Success>
                            <thaw::MessageBarBody>"Query saved."</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                Some(Err(error)) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The result table under a `<Transition>`, with local prev/next paging.
fn results_section(
    results: Resource<Result<Option<ResultPage>, AdminUiError>>,
    offset: RwSignal<u32>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let page = results.await?;
                    Ok::<
                        _,
                        AdminUiError,
                    >(
                        match page {
                            None => ().into_any(),
                            Some(page) => {
                                let controls = paging_buttons(offset, page.rows.len());
                                let body = results_view(&page, false);
                                view! {
                                    <thaw::Card>
                                        <thaw::CardHeader>
                                            <div class="text-sm font-semibold">"Results"</div>
                                        </thaw::CardHeader>
                                        <div class="p-3">{body}{controls}</div>
                                    </thaw::Card>
                                }
                                    .into_any()
                            }
                        },
                    )
                })}
            </ErrorBoundary>
        </Transition>
    }
    .into_any()
}
