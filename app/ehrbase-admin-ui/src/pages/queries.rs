//! The `/queries` screen: the CDR's stored queries (left) and the
//! console-local query groups (right).
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The stored-query surface is the ITS-REST Definition/Query API
//! (`docs/specs/openehr/ITS-REST/docs/definition/`); groups are console-local
//! (no ITS-REST resource exists for them). All data flows through the
//! [`crate::queries_api`] server fns — each guards the session itself; this
//! screen adds no new server fn.
//!
//! Discipline (rules §1/§4/§6/§8): the view is composed from `.into_any()`-
//! erased section locals; the stored-query list is a `<For>` keyed by
//! `name@version`; refetched data (the groups list, the on-demand AQL detail)
//! reads under `<Transition>`; the table emits an explicit `<tbody>`; internal
//! navigation uses `<A>`; there is zero authored JavaScript (`on:` Rust
//! listeners only).

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::format_view::DocumentPane;
use crate::error::AdminUiError;
use crate::pages::dashboard::split_query_ref;
use crate::pages::ehrs::{error_bar, table_skeleton};
use crate::queries_api::{
    QueryGroup, StoredQueryRow, fetch_stored_query, list_groups, list_stored_queries, save_group,
};

/// The `/queries` screen: a stored-queries table with an on-demand AQL detail,
/// alongside the console-local query-group editor.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn QueriesPage() -> impl IntoView {
    // Stored queries load once and feed both the table (left) and the group
    // form's member checkboxes (right); a Resource is Copy so both read it.
    let stored = Resource::new(|| (), |()| async move { list_stored_queries().await });
    // The group save/delete action; its version drives the groups refetch.
    let save = Action::new(|input: &(String, Vec<String>)| {
        let (name, members) = input.clone();
        async move { save_group(name, members).await }
    });

    let table = stored_queries_panel(stored);
    let groups = groups_panel(stored, save);

    view! {
        <Title text="Stored queries · ehrbase-admin" />
        <div class="p-4">
            <div class="flex items-center justify-between mb-4">
                <h1 class="text-xl font-semibold">"Stored queries"</h1>
                <A
                    href="/queries/builder"
                    attr:class="inline-flex items-center rounded bg-blue-600 text-white px-3 py-1.5 text-sm hover:bg-blue-700"
                >
                    "New query (builder)"
                </A>
            </div>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">{table} {groups}</div>
        </div>
    }
}

// ── Stored queries (left) ────────────────────────────────────────────────

/// The stored-queries panel: the list table plus the single-select AQL detail
/// shown below it.
fn stored_queries_panel(stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>) -> AnyView {
    // The row (name, version) whose AQL detail is currently expanded.
    let selected = RwSignal::new(Option::<(String, String)>::None);
    // The selected query's AQL, fetched on demand; `None` source → no call.
    let detail = Resource::new(
        move || selected.get(),
        |key| async move {
            match key {
                Some((name, version)) => fetch_stored_query(name, version).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let table = stored_table(stored, selected);
    let detail_view = stored_detail(detail);
    view! {
        <section>
            <h2 class="text-sm font-semibold text-neutral-500 mb-2">"Queries"</h2>
            {table}
            {detail_view}
        </section>
    }
    .into_any()
}

/// The stored-queries table, read under `<Suspense>` (loads once).
fn stored_table(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    view! {
        <Suspense fallback=table_skeleton>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let rows = stored.await?;
                    Ok::<_, AdminUiError>(stored_rows_view(rows, selected))
                })}
            </ErrorBoundary>
        </Suspense>
    }
    .into_any()
}

/// Render the stored-query rows (or the empty state). The row list is a `<For>`
/// keyed by `name@version` (rules §4 — a stable, data-derived key).
fn stored_rows_view(
    rows: Vec<StoredQueryRow>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    if rows.is_empty() {
        return view! {
            <thaw::MessageBar intent=thaw::MessageBarIntent::Info>
                <thaw::MessageBarBody>
                    "No stored queries — create one from the query builder."
                </thaw::MessageBarBody>
            </thaw::MessageBar>
        }
        .into_any();
    }
    let body = view! {
        <For each=move || rows.clone() key=|row| format!("{}@{}", row.name, row.version) let:row>
            {stored_row(row, selected)}
        </For>
    };
    view! {
        <div class="overflow-x-auto">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="border-b border-neutral-200 dark:border-neutral-700">
                        <th class="text-left font-medium text-neutral-500 py-1 pr-4">"name"</th>
                        <th class="text-left font-medium text-neutral-500 py-1 pr-4">"version"</th>
                        <th class="text-left font-medium text-neutral-500 py-1 pr-4">"saved"</th>
                    </tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
        </div>
    }
    .into_any()
}

/// One stored-query row. Clicking it toggles the single-select detail below the
/// table (`on:click` is a Rust listener — zero authored JS, rules §0).
fn stored_row(row: StoredQueryRow, selected: RwSignal<Option<(String, String)>>) -> impl IntoView {
    let key_for_class = (row.name.clone(), row.version.clone());
    let key_for_click = (row.name.clone(), row.version.clone());
    let is_selected = move || selected.with(|current| current.as_ref() == Some(&key_for_class));
    let on_click = move |_| {
        let key = key_for_click.clone();
        selected.update(|current| {
            if current.as_ref() == Some(&key) {
                *current = None;
            } else {
                *current = Some(key);
            }
        });
    };
    view! {
        <tr
            class="border-b border-neutral-100 dark:border-neutral-800 cursor-pointer hover:bg-neutral-50 dark:hover:bg-neutral-800/40"
            class=("bg-neutral-100", is_selected)
            on:click=on_click
        >
            <td class="py-1 pr-4 font-mono">{row.name}</td>
            <td class="py-1 pr-4 font-mono">{row.version}</td>
            <td class="py-1 pr-4 text-xs text-neutral-500">{row.saved}</td>
        </tr>
    }
}

/// The AQL detail below the table: the selected query's AQL in a
/// [`DocumentPane`] plus a Run button. Read under `<Transition>` so the old
/// detail stays visible while a new selection loads (rules §6).
fn stored_detail(detail: Resource<Result<Option<String>, AdminUiError>>) -> AnyView {
    view! {
        <div class="mt-3">
            <Transition fallback=|| {
                view! { <p class="text-sm text-neutral-500">"Loading query…"</p> }
            }>
                <ErrorBoundary fallback=error_bar>
                    {move || Suspend::new(async move {
                        let aql = detail.await?;
                        Ok::<
                            _,
                            AdminUiError,
                        >(
                            match aql {
                                Some(text) => detail_panel(text),
                                None => ().into_any(),
                            },
                        )
                    })}
                </ErrorBoundary>
            </Transition>
        </div>
    }
    .into_any()
}

/// The expanded detail for one stored query: its AQL and a Run button that
/// navigates to the raw-AQL screen with the query pre-filled via `?aql=`.
fn detail_panel(aql: String) -> AnyView {
    let navigate = use_navigate();
    let aql_for_run = aql.clone();
    let on_run = move |_| {
        let href = format!("/queries/aql?aql={}", encode_query_value(&aql_for_run));
        navigate(&href, NavigateOptions::default());
    };
    let body = Signal::derive(move || aql.clone());
    view! {
        <div class="rounded border border-neutral-200 dark:border-neutral-700 p-3">
            <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-semibold uppercase tracking-wide text-neutral-500">
                    "AQL"
                </span>
                <thaw::Button
                    appearance=thaw::ButtonAppearance::Primary
                    size=thaw::ButtonSize::Small
                    on_click=on_run
                >
                    "Run"
                </thaw::Button>
            </div>
            <DocumentPane body=body />
        </div>
    }
    .into_any()
}

// ── Query groups (right) ─────────────────────────────────────────────────

/// The groups panel: a create/edit form over the console-local groups, and the
/// list of existing groups with edit/delete controls.
fn groups_panel(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
) -> AnyView {
    // Groups refetch whenever the save action's version bumps (rules §6).
    let groups = Resource::new(
        move || save.version().get(),
        |_| async move { list_groups().await },
    );
    // Form state, shared between the form and the per-group Edit buttons.
    let name = RwSignal::new(String::new());
    let selected = RwSignal::new(Vec::<String>::new());
    // Which existing group is awaiting a delete confirmation (second click).
    let pending_delete = RwSignal::new(Option::<String>::None);

    let form = group_form(stored, save, name, selected);
    let list = groups_list(groups, save, name, selected, pending_delete);
    view! {
        <section>
            <h2 class="text-sm font-semibold text-neutral-500 mb-2">"Groups"</h2>
            {form}
            {list}
        </section>
    }
    .into_any()
}

/// The create/edit form: a name input, member checkboxes over the stored
/// queries (`name@version`), and Save/Clear buttons dispatching [`save_group`].
fn group_form(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
) -> AnyView {
    let on_save = move |_| {
        let group_name = name.get().trim().to_owned();
        save.dispatch((group_name, selected.get()));
    };
    let on_clear = move |_| {
        name.set(String::new());
        selected.set(Vec::new());
    };
    let checkboxes = member_checkboxes(stored, selected);
    view! {
        <div class="rounded border border-neutral-200 dark:border-neutral-700 p-3 mb-4">
            <input
                type="text"
                class="w-full rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-3 py-1.5 text-sm mb-3"
                placeholder="group name"
                prop:value=move || name.get()
                on:input:target=move |ev| name.set(ev.target().value())
            />
            <div class="text-xs font-medium text-neutral-500 mb-1">"Members"</div>
            <div class="max-h-48 overflow-y-auto mb-3">{checkboxes}</div>
            <div class="flex items-center gap-2">
                <thaw::Button appearance=thaw::ButtonAppearance::Primary on_click=on_save>
                    "Save group"
                </thaw::Button>
                <thaw::Button appearance=thaw::ButtonAppearance::Subtle on_click=on_clear>
                    "Clear"
                </thaw::Button>
                {group_form_feedback(save)}
            </div>
        </div>
    }
    .into_any()
}

/// The save action's inline state: a pending hint, the error verbatim, or a
/// success confirmation.
fn group_form_feedback(save: Action<(String, Vec<String>), Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || save.pending().get()>
                <span class="text-neutral-500">"Saving…"</span>
            </Show>
            {move || match save.value().get() {
                Some(Err(error)) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                Some(Ok(())) => view! { <span class="text-emerald-600">"Saved."</span> }.into_any(),
                None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The member checkbox list, read under `<Suspense>` (the same stored-query
/// load as the table).
fn member_checkboxes(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    selected: RwSignal<Vec<String>>,
) -> AnyView {
    view! {
        <Suspense fallback=|| {
            view! { <p class="text-xs text-neutral-500">"Loading queries…"</p> }
        }>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let rows = stored.await?;
                    Ok::<_, AdminUiError>(checkbox_list(&rows, selected))
                })}
            </ErrorBoundary>
        </Suspense>
    }
    .into_any()
}

/// Render one checkbox per stored query (or an empty hint).
fn checkbox_list(rows: &[StoredQueryRow], selected: RwSignal<Vec<String>>) -> AnyView {
    if rows.is_empty() {
        return view! { <p class="text-xs text-neutral-500">"No stored queries to add."</p> }
            .into_any();
    }
    let items = rows
        .iter()
        .map(|row| member_checkbox(row, selected))
        .collect::<Vec<_>>();
    view! { <div class="flex flex-col gap-1">{items}</div> }.into_any()
}

/// One member checkbox: a controlled checkbox whose `checked` reflects the
/// selected set and whose `on:change` toggles the `name@version` member.
fn member_checkbox(row: &StoredQueryRow, selected: RwSignal<Vec<String>>) -> AnyView {
    let member = format!("{}@{}", row.name, row.version);
    let member_for_check = member.clone();
    let member_for_toggle = member.clone();
    let checked = move || selected.with(|set| set.iter().any(|m| m == &member_for_check));
    let on_change = move |_| {
        let member = member_for_toggle.clone();
        selected.update(|set| {
            if let Some(pos) = set.iter().position(|m| m == &member) {
                set.remove(pos);
            } else {
                set.push(member);
            }
        });
    };
    view! {
        <label class="flex items-center gap-2 text-sm">
            <input type="checkbox" prop:checked=checked on:change=on_change />
            <span class="font-mono">{member}</span>
        </label>
    }
    .into_any()
}

/// The existing groups, read under `<Transition>` (refetched on save).
fn groups_list(
    groups: Resource<Result<Vec<QueryGroup>, AdminUiError>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    view! {
        <Transition fallback=|| {
            view! { <p class="text-sm text-neutral-500">"Loading groups…"</p> }
        }>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let loaded = groups.await?;
                    Ok::<
                        _,
                        AdminUiError,
                    >(groups_list_view(loaded, save, name, selected, pending_delete))
                })}
            </ErrorBoundary>
        </Transition>
    }
    .into_any()
}

/// Render the group cards (or the empty state).
fn groups_list_view(
    groups: Vec<QueryGroup>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    if groups.is_empty() {
        return view! { <p class="text-sm text-neutral-500">"No groups yet."</p> }.into_any();
    }
    let cards = groups
        .into_iter()
        .map(|group| group_card(&group, save, name, selected, pending_delete))
        .collect::<Vec<_>>();
    view! { <div class="flex flex-col gap-2">{cards}</div> }.into_any()
}

/// One group card: the name, member chips, an Edit button (loads the group into
/// the form) and a Delete button that requires a second confirming click before
/// dispatching a delete (a [`save_group`] with no members).
fn group_card(
    group: &QueryGroup,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    let chips = group
        .members
        .iter()
        .map(|member| {
            let (label, version) =
                split_query_ref(member).unwrap_or_else(|| (member.clone(), String::new()));
            view! {
                <thaw::Tag>
                    <span class="font-mono">{label}</span>
                    {(!version.is_empty())
                        .then(|| view! { <span class="opacity-60">" @"{version}</span> })}
                </thaw::Tag>
            }
        })
        .collect::<Vec<_>>();

    let group_name = group.name.clone();
    let name_for_edit = group.name.clone();
    let members_for_edit = group.members.clone();
    let on_edit = move |_| {
        name.set(name_for_edit.clone());
        selected.set(members_for_edit.clone());
    };

    let name_for_delete = group.name.clone();
    let is_pending = {
        let name_for_pending = group.name.clone();
        move || pending_delete.with(|p| p.as_deref() == Some(name_for_pending.as_str()))
    };
    let on_delete = move |_| {
        if pending_delete.with(|p| p.as_deref() == Some(name_for_delete.as_str())) {
            save.dispatch((name_for_delete.clone(), Vec::new()));
            pending_delete.set(None);
        } else {
            pending_delete.set(Some(name_for_delete.clone()));
        }
    };

    view! {
        <div class="rounded border border-neutral-200 dark:border-neutral-700 p-3">
            <div class="flex items-center justify-between mb-2">
                <span class="font-medium">{group_name}</span>
                <div class="flex gap-2">
                    <thaw::Button size=thaw::ButtonSize::Small on_click=on_edit>
                        "Edit"
                    </thaw::Button>
                    <thaw::Button
                        size=thaw::ButtonSize::Small
                        appearance=thaw::ButtonAppearance::Subtle
                        on_click=on_delete
                    >
                        {move || if is_pending() { "Confirm delete" } else { "Delete" }}
                    </thaw::Button>
                </div>
            </div>
            <div class="flex flex-wrap gap-1">{chips}</div>
        </div>
    }
    .into_any()
}

/// Percent-encode a value for use inside a URL query string, per RFC 3986: the
/// unreserved set (`ALPHA` / `DIGIT` / `-` / `_` / `.` / `~`) passes through;
/// every other byte becomes `%XX` (uppercase hex, one escape per UTF-8 byte).
/// The crate's `urlencoding` is server-only, so the browser navigation that
/// carries a stored query into the raw-AQL screen's `?aql=` needs its own
/// encoder.
fn encode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            other => {
                out.push('%');
                out.push(hex_digit(other >> 4));
                out.push(hex_digit(other & 0x0f));
            }
        }
    }
    out
}

/// One uppercase hex digit for a nibble (0–15).
fn hex_digit(nibble: u8) -> char {
    char::from(if nibble < 10 {
        b'0' + nibble
    } else {
        b'A' + (nibble - 10)
    })
}

#[cfg(test)]
mod tests {
    use super::encode_query_value;

    #[test]
    fn encode_query_value_passes_unreserved_and_escapes_the_rest() {
        assert_eq!(encode_query_value("Aa0-_.~"), "Aa0-_.~");
        assert_eq!(encode_query_value("a b"), "a%20b");
        assert_eq!(encode_query_value("c/name/value"), "c%2Fname%2Fvalue");
        // Multi-byte UTF-8 escapes per byte, uppercase hex.
        assert_eq!(encode_query_value("é"), "%C3%A9");
    }
}
