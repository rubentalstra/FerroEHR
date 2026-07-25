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

use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, LABEL, TEXTAREA};
use crate::components::page_header::{Crumb, PageHeader};
use crate::components::surface::CARD_PAD;
use crate::components::toast::toast_success;
use crate::error::AdminUiError;
use crate::pages::ehrs::{ResultPage, table_skeleton};
use crate::pages::query_builder::{export_forms, paging_buttons, results_view, save_as_fields};
use crate::queries_api::{fetch_stored_query, run_aql, store_query, validate_aql};
use crate::query_namespace::{qualify, split_qualified, split_query_ref};

/// Build the link INTO this screen that pre-fills the editor with `aql`.
///
/// The AQL becomes one query-string value, percent-encoded with the
/// `urlencoding` crate (owner rule: all percent-coding goes through that
/// crate). AQL is full of URL-reserved characters — a space, the `/` in every
/// path, `#`, `&`, `=`, `'`, `%` — any one of which would truncate the value or
/// split it into extra parameters. The router percent-DEcodes query params
/// (`ParamsMap::insert` → `Url::unescape`), which is why
/// [`QueryAqlPage`]'s `?aql=` read needs no decode of its own.
/// NOTE: no openEHR spec governs an admin UI's internal links — our own
/// design/extension.
pub(crate) fn aql_href(aql: &str) -> String {
    format!("/queries/aql?aql={}", urlencoding::encode(aql))
}

/// Build the link INTO this screen that seeds the editor from the stored query
/// `name@version` (the `?load=` hand-off, parsed back by
/// [`split_query_ref`]).
///
/// The qualified reference is ONE query-string value, so it is encoded as a
/// whole: a stored-query name may carry `/`, `&`, `=` or `#`, which would
/// otherwise truncate the value or forge extra parameters.
/// NOTE: no openEHR spec governs an admin UI's internal links — our own
/// design/extension.
pub(crate) fn load_href(name: &str, version: &str) -> String {
    format!(
        "/queries/aql?load={}",
        urlencoding::encode(&format!("{name}@{version}"))
    )
}

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
    // The two halves of the stored query's qualified name (`namespace::name`,
    // the namespace optional — see `crate::query_namespace`).
    let save_namespace = RwSignal::new(String::new());
    let save_name = RwSignal::new(String::new());
    let offset = RwSignal::new(0_u32);
    let ran = RwSignal::new(None::<(String, String)>);

    // The "open in editor" hand-off from /queries: `?load=name@version` fetches
    // the stored query and seeds the editor. `load` is URL-derived, identical on
    // the server pass and the client hydration (hydration-safe), so the status
    // section only renders when the param is actually present.
    let has_load = query_map.with_untracked(|m| m.get("load").is_some_and(|s| !s.is_empty()));
    let load_resource: Resource<Result<Option<(String, String)>, AdminUiError>> = Resource::new(
        move || query_map.with(|m| m.get("load").unwrap_or_default()),
        |load| async move {
            match split_query_ref(&load) {
                Some((name, version)) => {
                    let text = fetch_stored_query(name.clone(), version).await?;
                    Ok(Some((name, text)))
                }
                None => Ok(None),
            }
        },
    );
    // Seed the editor from the loaded query exactly once, client-side (Effects
    // never run on the server, so there is no SSR/hydration divergence), and
    // only while the editor is still untouched so back-navigation never
    // clobbers edits. This is the async-load-into-editable-local-state case
    // (rules §2); the one-shot `StoredValue` guard keeps it from re-firing.
    // Loading the qualified name back into the save fields versions the query
    // on re-save: it is SPLIT into its namespace + bare name, which is what
    // pre-fills the namespace field from a `namespace::` prefix.
    let seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if seeded.get_value() {
            return;
        }
        if let Some(Ok(Some((qualified, text)))) = load_resource.get() {
            if aql.with_untracked(std::string::String::is_empty) {
                aql.set(text);
            }
            let (namespace, name) = split_qualified(&qualified);
            save_namespace.set(namespace);
            save_name.set(name);
            seeded.set_value(true);
        }
    });

    let validate_action: Action<String, Result<(), AdminUiError>> = Action::new(|aql: &String| {
        let aql = aql.clone();
        async move { validate_aql(aql).await }
    });
    let save_action: Action<(String, String), Result<(), AdminUiError>> =
        Action::new(|input: &(String, String)| {
            let (name, aql) = input.clone();
            async move { store_query(name, aql).await }
        });
    // Both outcomes toast (rules: Effect = sync with the outside world; no
    // signal is written — and the console's mutation-feedback rule, crate
    // CLAUDE.md). The CDR's diagnostic ALSO stays inline (save_feedback),
    // beside the AQL it rejected.
    let toaster = thaw::ToasterInjection::expect_context();
    Effect::new(move |_| match save_action.value().get() {
        Some(Ok(())) => toast_success(toaster, "Query saved", ""),
        Some(Err(error)) => {
            crate::feedback::toast_write_failure(
                toaster,
                "Save failed",
                "the stored query",
                &error,
            );
        }
        None => {}
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

    let load_status = if has_load {
        load_status_section(load_resource)
    } else {
        ().into_any()
    };
    let editor = editor_section(aql, validate_action);
    let parameters = parameters_section(params);
    let run_save = run_save_section(
        aql,
        params,
        save_namespace,
        save_name,
        save_action,
        ran,
        offset,
    );
    // Export tracks the editor + parameter panes — the same signals Run uses.
    let export_aql = Signal::derive(move || aql.get());
    let export_params = Signal::derive(move || params.get());
    let results_pane = results_section(results, offset, export_aql, export_params);

    view! {
        <Title text="AQL editor" />
        <div class="p-6 space-y-4">
            <PageHeader
                title="Raw AQL"
                subtitle="Write and run AQL directly, bind parameters, and save it as a stored query."
                crumbs=vec![Crumb::new("Queries", "/queries")]
            />
            {load_status}
            {editor}
            {parameters}
            {run_save}
            {results_pane}
        </div>
    }
}

/// The `?load=` hand-off status: a house-pattern `<Transition>` reporting the
/// loaded query (or the CDR error inline). The editor itself is seeded by the
/// one-shot effect in the page component; this only surfaces load progress.
fn load_status_section(
    load_resource: Resource<Result<Option<(String, String)>, AdminUiError>>,
) -> AnyView {
    view! {
        <Transition fallback=move || {
            view! { <p class="text-sm text-ink-muted">"Loading stored query…"</p> }
        }>
            {move || Suspend::new(async move {
                match load_resource.await {
                    Ok(Some((qualified, _))) => {
                        view! {
                            <p class="text-sm text-ink-muted">
                                "Loaded stored query "
                                <span class="font-mono text-ink">{qualified}</span> "."
                            </p>
                        }
                            .into_any()
                    }
                    Ok(None) => ().into_any(),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
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
        <section class=CARD_PAD>
            <div class="space-y-2">
                <label class=LABEL r#for="aql-editor">
                    "AQL"
                </label>
                <textarea
                    id="aql-editor"
                    class=format!("{TEXTAREA} h-40")
                    placeholder="SELECT c FROM EHR e CONTAINS COMPOSITION c"
                    prop:value=move || aql.get()
                    on:input:target=move |ev| aql.set(ev.target().value())
                >
                    {aql.get_untracked()}
                </textarea>
                <div class="flex items-center gap-3">
                    <button
                        type="button"
                        class=BTN_SECONDARY
                        disabled=Signal::derive(move || aql.with(std::string::String::is_empty))
                        on:click=validate_click
                    >
                        "Validate"
                    </button>
                    {validate_feedback(validate_action)}
                </div>
            </div>
        </section>
    }
    .into_any()
}

/// The Validate action's inline result (parser diagnostic on failure).
fn validate_feedback(validate_action: Action<String, Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || validate_action.pending().get()>
                <span class="text-ink-muted">"Validating…"</span>
            </Show>
            {move || match validate_action.value().get() {
                Some(Ok(())) => {
                    view! {
                        <span class="rounded-control bg-ok-subtle px-2 py-0.5 text-ok">
                            "AQL is valid."
                        </span>
                    }
                        .into_any()
                }
                Some(Err(error)) => crate::components::format_view::inline_error(&error),
                None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The parameter-bindings pane: a JSON object bound as AQL `query_parameters`.
fn parameters_section(params: RwSignal<String>) -> AnyView {
    view! {
        <section class=CARD_PAD>
            <div class="space-y-2">
                <label class=LABEL r#for="aql-params">
                    "Parameters (JSON object)"
                </label>
                <textarea
                    id="aql-params"
                    class=format!("{TEXTAREA} h-24")
                    placeholder="{\"ehr_id\": \"...\"}"
                    prop:value=move || params.get()
                    on:input:target=move |ev| params.set(ev.target().value())
                >
                    {params.get_untracked()}
                </textarea>
            </div>
        </section>
    }
    .into_any()
}

/// The Run + Save surface: Run executes the AQL with the parameter bindings at
/// the first page; Save stores it under the qualified name composed from the
/// namespace + name fields ([`save_as_fields`]).
fn run_save_section(
    aql: RwSignal<String>,
    params: RwSignal<String>,
    save_namespace: RwSignal<String>,
    save_name: RwSignal<String>,
    save_action: Action<(String, String), Result<(), AdminUiError>>,
    ran: RwSignal<Option<(String, String)>>,
    offset: RwSignal<u32>,
) -> AnyView {
    let empty_aql = Signal::derive(move || aql.with(std::string::String::is_empty));
    // The namespace stays optional (the spec makes it optional), so only the
    // AQL and the query name gate the Save button.
    let save_disabled = Signal::derive(move || {
        aql.with(std::string::String::is_empty) || save_name.with(std::string::String::is_empty)
    });
    let run_click = move |_| {
        ran.set(Some((aql.get_untracked(), params.get_untracked())));
        offset.set(0);
    };
    let save_click = move |_| {
        save_action.dispatch((
            qualify(&save_namespace.get_untracked(), &save_name.get_untracked()),
            aql.get_untracked(),
        ));
    };
    let save_fields = save_as_fields("aql", save_namespace, save_name);
    view! {
        <section class=CARD_PAD>
            <div class="space-y-3">
                <div class="flex flex-wrap items-end gap-3">
                    <button type="button" class=BTN_PRIMARY disabled=empty_aql on:click=run_click>
                        "Run"
                    </button> {save_fields}
                    <button
                        type="button"
                        class=BTN_PRIMARY
                        disabled=save_disabled
                        on:click=save_click
                    >
                        "Save"
                    </button>
                </div>
                {save_feedback(save_action)}
            </div>
        </section>
    }
    .into_any()
}

/// The Save action's inline feedback: a pending hint and the CDR error verbatim.
/// Success is reported as a toast (dispatched from the page component), so it
/// renders nothing here.
fn save_feedback(save_action: Action<(String, String), Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || save_action.pending().get()>
                <span class="text-ink-muted">"Saving…"</span>
            </Show>
            {move || match save_action.value().get() {
                Some(Err(error)) => crate::components::format_view::inline_error(&error),
                Some(Ok(())) | None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The result table under a `<Transition>`, with local prev/next paging.
fn results_section(
    results: Resource<Result<Option<ResultPage>, AdminUiError>>,
    offset: RwSignal<u32>,
    current_aql: Signal<String>,
    params: Signal<String>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match results.await {
                    Ok(None) => ().into_any(),
                    Ok(Some(page)) => {
                        let controls = paging_buttons(offset, page.rows.len());
                        let body = results_view(&page, false);
                        let export = export_forms(current_aql, params);
                        // Resolve inside the Transition: an SSR'd ErrorBoundary fallback
                        // mismatches at hydration in leptos 0.8 (E2E console gate).
                        view! {
                            <section class=CARD_PAD>
                                <div class="flex items-center justify-between gap-2 flex-wrap mb-3">
                                    <h2 class="text-sm font-semibold text-ink">"Results"</h2>
                                    {export}
                                </div>
                                {body}
                                {controls}
                            </section>
                        }
                            .into_any()
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use crate::pages::query_aql::{aql_href, load_href};
    use crate::query_namespace::split_query_ref;

    #[test]
    fn load_href_leaves_an_unreserved_qualified_ref_alone() {
        assert_eq!(
            load_href("my_query", "1.0.0"),
            "/queries/aql?load=my_query%401.0.0"
        );
    }

    #[test]
    fn load_href_escapes_the_qualified_ref_as_one_value() {
        // A qualified stored-query name carries `/` and `::`.
        assert_eq!(
            load_href("org.example::c/name/value", "1.2.3"),
            "/queries/aql?load=org.example%3A%3Ac%2Fname%2Fvalue%401.2.3"
        );
        // A `&` or `=` in the name must not become an extra parameter.
        assert_eq!(load_href("a&b=c", "1"), "/queries/aql?load=a%26b%3Dc%401");
    }

    #[test]
    fn load_href_round_trips_back_through_split_query_ref() {
        // The router decodes `?load=` before this screen reads it, so decoding
        // the emitted value must hand `split_query_ref` the original pair.
        for (name, version) in [
            ("my_query", "1.0.0"),
            ("org.example::c/name/value", "1.2.3"),
            ("a&b=c", "1"),
            ("blodtryk_målinger", "2.0.0"),
        ] {
            let href = load_href(name, version);
            let value = href
                .strip_prefix("/queries/aql?load=")
                .expect("the builder always emits /queries/aql?load=<value>");
            let decoded = urlencoding::decode(value).expect("valid UTF-8 percent-encoding");
            assert_eq!(
                split_query_ref(&decoded),
                Some((name.to_owned(), version.to_owned()))
            );
        }
    }

    #[test]
    fn aql_href_escapes_every_reserved_character_in_the_query_text() {
        assert_eq!(aql_href("Aa0-_.~"), "/queries/aql?aql=Aa0-_.~");
        // Space, ampersand, equals, percent, hash, question mark, plus, slash.
        assert_eq!(
            aql_href(" &=%#?+/"),
            "/queries/aql?aql=%20%26%3D%25%23%3F%2B%2F"
        );
        // Multi-byte UTF-8 escapes per byte, uppercase hex.
        assert_eq!(aql_href("é"), "/queries/aql?aql=%C3%A9");
    }

    #[test]
    fn aql_href_round_trips_a_real_query() {
        let aql = "SELECT c/uid/value FROM COMPOSITION c WHERE c/name/value = 'a b'";
        let href = aql_href(aql);
        assert_eq!(
            href,
            "/queries/aql?aql=SELECT%20c%2Fuid%2Fvalue%20FROM%20COMPOSITION%20c%20WHERE%20c%2Fname%2Fvalue%20%3D%20%27a%20b%27"
        );
        let value = href
            .strip_prefix("/queries/aql?aql=")
            .expect("the builder always emits /queries/aql?aql=<value>");
        // Nothing that could truncate the value or forge a second parameter.
        assert!(!value.contains(['?', '&', '=', '/', ' ', '#']));
        assert_eq!(
            urlencoding::decode(value).expect("valid UTF-8 percent-encoding"),
            aql
        );
    }
}
