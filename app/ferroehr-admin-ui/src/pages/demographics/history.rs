// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The **History** tab of a demographic object — the versioned family, shared
//! by parties and relationships.
//!
//! Four surfaces over one versioned object, in the order an operator reads
//! them:
//!
//! 1. the **versioned-object card** — the container's own facts plus the
//!    selected VERSION's envelope facts (`GET /demographic/{family}/{uid}` +
//!    `…/version[/{version_uid}]`), the contribution linked through to its
//!    own viewer;
//! 2. the **revision history** table (`…/revision_history`), newest-first,
//!    each row opening that version;
//! 3. the **at-time lookup** (`…/version?version_at_time=…`), which resolves an
//!    instant to a version and opens it;
//! 4. the **pinned document** — that version of the object itself, read by its
//!    `OBJECT_VERSION_ID` through the RESOURCE route.
//!
//! One reader per claim (crate `CLAUDE.md`): this tab never reads the current
//! object — that belongs to the Party (or Relationship) surface. The split
//! within the tab is the composition viewer's: document CONTENT ← the resource
//! at that version, commit history ← the revision history, the VERSION's
//! envelope facts (lifecycle state, preceding version, contribution, signature)
//! ← the direct VERSION read.
//!
//! Every resource is created ONCE in [`history_section`] — never inside a
//! `Suspend` (rules §4) — and gated on the tab being active, so an unopened tab
//! fetches nothing (rules §6).

use leptos::prelude::*;

use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell, table_skeleton};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_SECONDARY, INPUT, LABEL};
use crate::components::format_view::{DocumentPane, inline_error};
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::error::AdminUiError;
use crate::pages::composition::VersionEntry;
use crate::pages::demographics::party::fact_row;
use crate::pages::demographics::{
    DemographicResource, VersionedObjectFacts, contribution_href,
    fetch_demographic_revision_history, fetch_demographic_version_document, fetch_versioned_object,
    resolve_demographic_version_at_time,
};

/// The revision-history resource: the object's commits, newest-first.
type HistoryResource = Resource<Result<Vec<VersionEntry>, AdminUiError>>;

/// The container + VERSION-envelope resource.
type VersionedResource = Resource<Result<Option<VersionedObjectFacts>, AdminUiError>>;

/// The pinned-version document resource (`None` while no version is pinned).
type DocumentResource = Resource<Result<Option<String>, AdminUiError>>;

/// The History tab: the versioned-object card, the revision-history table, the
/// at-time lookup, and the pinned version's document.
///
/// `tab` is the `?tab=` value that activates this section on its screen, so the
/// same code serves the party detail and the relationship detail.
pub(super) fn history_section(
    object: DemographicResource,
    uid: Signal<String>,
    selected: Memo<String>,
    tab: &'static str,
) -> AnyView {
    // Which version the tab is showing. Empty = none pinned yet: the card then
    // reports the LATEST version (the version read's own default) and the
    // document pane invites the operator to open one from the table.
    let pinned = RwSignal::new(String::new());
    let at_time_input = RwSignal::new(String::new());
    let active = Memo::new(move |_| selected.get() == tab);
    let family = object.family().segment();

    // The at-time lookup is an Action, not a resource: it is a one-shot the
    // operator triggers, and its answer (an OBJECT_VERSION_ID) feeds the shared
    // selection rather than being rendered on its own. A failure leaves the
    // selection untouched — the lookup's own note renders it.
    let at_time: Action<(String, String), Result<String, AdminUiError>> =
        Action::new(move |(uid, at): &(String, String)| {
            let (uid, at) = (uid.clone(), at.clone());
            async move {
                let resolved =
                    resolve_demographic_version_at_time(family.to_owned(), uid, at).await;
                // NOTE: the write rides the dispatched event's own continuation,
                // so it is an event write rather than an Effect write (rules §2).
                if let Ok(version) = &resolved
                    && !version.is_empty()
                {
                    pinned.set(version.clone());
                }
                resolved
            }
        });

    let history: HistoryResource = Resource::new(
        move || active.get().then(|| uid.get()),
        move |target| async move {
            match target {
                Some(id) => fetch_demographic_revision_history(family.to_owned(), id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let versioned: VersionedResource = Resource::new(
        move || active.get().then(|| (uid.get(), pinned.get())),
        move |target| async move {
            match target {
                Some((id, version)) => fetch_versioned_object(family.to_owned(), id, version)
                    .await
                    .map(Some),
                None => Ok(None),
            }
        },
    );
    let segment = object.segment();
    let document: DocumentResource = Resource::new(
        move || {
            let version = pinned.get();
            (active.get() && !version.is_empty()).then_some(version)
        },
        move |target| async move {
            match target {
                Some(version) => fetch_demographic_version_document(segment.to_owned(), version)
                    .await
                    .map(Some),
                None => Ok(None),
            }
        },
    );

    let card = versioned_section(versioned);
    let table = history_table_section(history, pinned);
    let lookup = lookup_section(uid, at_time, at_time_input);
    let pane = document_section(document, pinned);
    view! { <div class="flex flex-col gap-4">{card} {table} {lookup} {pane}</div> }.into_any()
}

/// The versioned-object card: the container's own facts plus the pinned
/// VERSION's envelope facts.
///
/// A `<Transition>` so switching version keeps the previous facts visible
/// (rules §6), with the `Result` resolved inside it (rules §4).
fn versioned_section(versioned: VersionedResource) -> AnyView {
    view! {
        <Transition fallback=|| {
            view! {
                <thaw::Skeleton>
                    <thaw::SkeletonItem class="h-24" />
                </thaw::Skeleton>
            }
        }>
            {move || Suspend::new(async move {
                match versioned.await {
                    Ok(Some(facts)) => versioned_card(&facts),
                    Ok(None) => ().into_any(),
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the container + selected-VERSION facts as a card, with the
/// contribution linked to its own viewer.
fn versioned_card(facts: &VersionedObjectFacts) -> AnyView {
    let signature = if facts.signed {
        "present".to_owned()
    } else {
        "none".to_owned()
    };
    let contribution = (!facts.contribution_uid.is_empty()).then(|| {
        let href = contribution_href(&facts.contribution_uid);
        let shown = facts.contribution_uid.clone();
        view! {
            <div>
                <span class="font-medium text-ink-muted mr-1">"contribution:"</span>
                <leptos_router::components::A
                    href=href
                    attr:class="font-mono break-all text-accent hover:underline"
                    attr:data-demographic-fact="contribution"
                >
                    {shown}
                </leptos_router::components::A>
            </div>
        }
        .into_any()
    });
    view! {
        <section class=CARD_PAD id="demographic-versioned">
            <h2 class=CARD_TITLE>"Versioned object"</h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 items-start gap-x-4 gap-y-2 text-sm">
                {fact_row("versioned object", "object-uid", facts.object_uid.clone())}
                {fact_row("created", "created", facts.time_created.clone())}
                {fact_row("version", "version-id", facts.version_id.clone())}
                {fact_row("lifecycle", "lifecycle", facts.lifecycle_state.clone())}
                {fact_row("preceding version", "preceding", facts.preceding_version_uid.clone())}
                {contribution} {fact_row("signature", "signature", signature)}
            </div>
        </section>
    }
    .into_any()
}

/// The revision-history table: one row per committed version, newest-first,
/// each row opening that version in the document pane below.
fn history_table_section(history: HistoryResource, pinned: RwSignal<String>) -> AnyView {
    let table = view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match history.await {
                    Ok(entries) if entries.is_empty() => {
                        view! {
                            <EmptyState
                                icon=icondata_lu::LuHistory
                                message="No versions"
                                hint="Every demographic object is created with a first version; if none is listed, the CDR reported no revision history for this id."
                            />
                        }
                            .into_any()
                    }
                    Ok(entries) => history_table(entries, pinned),
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any();
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"Revision history"</h2>
            <p class="mb-3 text-xs text-ink-muted">
                "Newest first. Open a version to read the document exactly as it stood at that commit."
            </p>
            {table}
        </section>
    }
    .into_any()
}

/// Render the history rows in the shared table kit. `<For>` keyed on the
/// version's own `OBJECT_VERSION_ID` — stable, unique, data-derived (rules §4).
fn history_table(entries: Vec<VersionEntry>, pinned: RwSignal<String>) -> AnyView {
    let rows = view! {
        <For each=move || entries.clone() key=|entry| entry.version_id.clone() let:entry>
            {history_row(&entry, pinned)}
        </For>
    }
    .into_any();
    table_shell(
        &["Version", "Committed", "Change type", "Committer", ""],
        rows,
    )
}

/// One revision-history row plus its "Open" action.
fn history_row(entry: &VersionEntry, pinned: RwSignal<String>) -> AnyView {
    let version_id = entry.version_id.clone();
    let hook = version_id.clone();
    let target = version_id.clone();
    let shown = version_id.clone();
    // The pinned row is tinted. A computed class string (not `class:`) because
    // the accent token carries a hyphen, which the `class:name=` shorthand
    // cannot spell.
    let row_class = move || {
        if pinned.with(|current| current.as_str() == version_id.as_str()) {
            format!("{ROW} bg-accent-subtle")
        } else {
            ROW.to_owned()
        }
    };
    let committed = entry.committed.clone();
    let change_type = entry.change_type.clone();
    let committed_by = entry.committer.clone();
    view! {
        <tr class=row_class>
            <td class=CELL_MONO>{shown}</td>
            <td class=CELL>{committed}</td>
            <td class=CELL>{change_type}</td>
            <td class=CELL>{committed_by}</td>
            <td class=CELL>
                <button
                    type="button"
                    class=BTN_SECONDARY
                    data-demographic-version=hook
                    on:click=move |_| pinned.set(target.clone())
                >
                    <leptos_icons::Icon icon=icondata_lu::LuEye width="14" height="14" />
                    "Open"
                </button>
            </td>
        </tr>
    }
    .into_any()
}

/// The at-time lookup: a `datetime-local` input plus a button that resolves the
/// VERSION extant at that instant and pins it.
///
/// The `404` answer ("no version at that time") is a neutral note beside the
/// control, not an error bar — it is the answer to the question asked. Any other
/// failure renders through the normal inline-error path (a pure read, so no
/// toast — the console's feedback rule).
fn lookup_section(
    uid: Signal<String>,
    at_time: Action<(String, String), Result<String, AdminUiError>>,
    at_time_input: RwSignal<String>,
) -> AnyView {
    let on_go = move |_| {
        let requested = at_time_input.get();
        if !requested.trim().is_empty() {
            at_time.dispatch((uid.get(), requested));
        }
    };
    let note = move || match at_time.value().get() {
        Some(Err(AdminUiError::Cdr { status: 404, .. })) => {
            view! { <p class="mt-2 text-sm text-ink-muted">"No version existed at that time."</p> }
                .into_any()
        }
        Some(Err(error)) => inline_error(&error),
        _ => ().into_any(),
    };
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"Version at a point in time"</h2>
            <div class="flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="demographic-at-time">
                        "Date and time (interpreted as UTC)"
                    </label>
                    <input
                        id="demographic-at-time"
                        type="datetime-local"
                        class=INPUT
                        prop:value=move || at_time_input.get()
                        on:input:target=move |ev| at_time_input.set(ev.target().value())
                    />
                </div>
                <button
                    id="demographic-at-time-go"
                    type="button"
                    class=BTN_SECONDARY
                    disabled=Signal::derive(move || at_time.pending().get())
                    on:click=on_go
                >
                    <leptos_icons::Icon icon=icondata_lu::LuClock width="14" height="14" />
                    "Open that version"
                </button>
            </div>
            {note}
        </section>
    }
    .into_any()
}

/// The pinned version's document, read by its `OBJECT_VERSION_ID`.
///
/// A `<Transition>` so switching version keeps the previous document visible
/// (rules §6). Nothing pinned is a first-class empty state, not an error.
fn document_section(document: DocumentResource, pinned: RwSignal<String>) -> AnyView {
    let heading = move || {
        let version = pinned.get();
        if version.is_empty() {
            "Version document".to_owned()
        } else {
            format!("Version {version}")
        }
    };
    let body = view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match document.await {
                    Ok(Some(body)) => {
                        let doc = RwSignal::new(body);
                        view! {
                            <div id="demographic-version-document">
                                <DocumentPane body=doc />
                            </div>
                        }
                            .into_any()
                    }
                    Ok(None) => {
                        view! {
                            <EmptyState
                                icon=icondata_lu::LuFileClock
                                message="No version opened"
                                hint="Open a version from the revision history above, or resolve one by date and time."
                            />
                        }
                            .into_any()
                    }
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any();
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>{heading}</h2>
            {body}
        </section>
    }
    .into_any()
}
