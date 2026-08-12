// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The EHR-detail **Status history** tab: the `VERSIONED_EHR_STATUS` family.
//!
//! Four surfaces over the one versioned object, in the order an operator reads
//! them:
//!
//! 1. the **versioned-object card** — the container's own facts plus the
//!    selected VERSION's envelope facts (`GET …/versioned_ehr_status` +
//!    `GET …/versioned_ehr_status/version[/{uid}]`);
//! 2. the **revision history** table (`GET …/versioned_ehr_status/revision_history`),
//!    newest-first, each row opening that version;
//! 3. the **at-time lookup** (`GET …/versioned_ehr_status/version?version_at_time=…`),
//!    which resolves an instant to a version and opens it;
//! 4. the **pinned document** — that version's `EHR_STATUS`, read by its
//!    `OBJECT_VERSION_ID` (`GET /ehr/{ehr_id}/ehr_status/{version_uid}`).
//!
//! One reader per claim (crate `CLAUDE.md`): this tab never reads
//! `GET /ehr/{ehr_id}/ehr_status` — the current-status document belongs to the
//! Status tab. The split within the tab mirrors the composition viewer's:
//! document CONTENT ← the `EHR_STATUS` resource at that version, commit history ←
//! the revision history, the VERSION's envelope facts (lifecycle state,
//! preceding version, contribution, signature) ← the direct VERSION read.
//!
//! Every resource is created ONCE in `status_history_section` — never inside a
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
use crate::pages::ehr_detail::status::{
    VersionedStatusDetails, fetch_ehr_status_version, fetch_status_revision_history,
    fetch_status_version_at_time, fetch_versioned_status,
};

/// The revision-history resource: the versioned object's commits, newest-first.
type HistoryResource = Resource<Result<Vec<VersionEntry>, AdminUiError>>;

/// The versioned-object + VERSION-envelope resource.
type VersionedResource = Resource<Result<Option<VersionedStatusDetails>, AdminUiError>>;

/// The pinned-version document resource (`None` while no version is pinned).
type DocumentResource = Resource<Result<Option<String>, AdminUiError>>;

/// Status history tab: the versioned-object card, the revision-history table,
/// the at-time lookup, and the pinned version's document.
pub(in crate::pages::ehr_detail) fn status_history_section(
    ehr_id: Signal<String>,
    selected: Memo<String>,
) -> AnyView {
    // Which version the tab is showing. Empty = none pinned yet: the card then
    // reports the LATEST version (the versioned-object read's default) and the
    // document pane invites the operator to open one from the table.
    let pinned = RwSignal::new(String::new());
    let at_time_input = RwSignal::new(String::new());
    let active = Memo::new(move |_| selected.get() == "status-history");

    // The at-time lookup is an Action, not a resource: it is a one-shot the
    // operator triggers, and its answer (an OBJECT_VERSION_ID) feeds the shared
    // selection rather than being rendered on its own.
    let at_time = Action::new(|(ehr_id, at_time): &(String, String)| {
        let (ehr_id, at_time) = (ehr_id.clone(), at_time.clone());
        async move { fetch_status_version_at_time(ehr_id, at_time).await }
    });

    let history: HistoryResource = Resource::new(
        move || active.get().then(|| ehr_id.get()),
        |target| async move {
            match target {
                Some(id) => fetch_status_revision_history(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let versioned: VersionedResource = Resource::new(
        move || active.get().then(|| (ehr_id.get(), pinned.get())),
        |target| async move {
            match target {
                Some((id, version)) => fetch_versioned_status(id, version).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let document: DocumentResource = Resource::new(
        move || {
            let version = pinned.get();
            (active.get() && !version.is_empty()).then(|| (ehr_id.get(), version))
        },
        |target| async move {
            match target {
                Some((id, version)) => fetch_ehr_status_version(id, version).await.map(Some),
                None => Ok(None),
            }
        },
    );

    // Sync a resolved at-time lookup into the shared selection. This is the
    // async-load-into-local-state case (the composition viewer's precedent): the
    // Effect reads ONLY the action's value and writes ONLY `pinned`, so there is
    // no reactive loop, and Effects never run on the server (no hydration
    // divergence). A failure leaves the selection untouched — the lookup's own
    // note renders it.
    Effect::new(move |_| {
        if let Some(Ok(resolved)) = at_time.value().get()
            && !resolved.is_empty()
        {
            pinned.set(resolved);
        }
    });

    let card = versioned_section(versioned);
    let table = history_section(history, pinned);
    let lookup = lookup_section(ehr_id, at_time, at_time_input);
    let pane = document_section(document, pinned);
    view! { <div class="flex flex-col gap-4">{card} {table} {lookup} {pane}</div> }.into_any()
}

/// The versioned-object card: the `VERSIONED_EHR_STATUS` container's own facts
/// plus the pinned VERSION's envelope facts.
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
                    Ok(Some(details)) => versioned_card(&details),
                    Ok(None) => ().into_any(),
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the container + selected-VERSION facts as a card.
fn versioned_card(details: &VersionedStatusDetails) -> AnyView {
    let signature = if details.signed {
        "present".to_owned()
    } else {
        "none".to_owned()
    };
    view! {
        <section class=CARD_PAD id="versioned-ehr-status">
            <h2 class=CARD_TITLE>"Versioned EHR_STATUS"</h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 items-start gap-x-4 gap-y-2 text-sm">
                {fact_row("versioned object", "object-uid", details.object_uid.clone())}
                {fact_row("owner EHR", "owner", details.owner_id.clone())}
                {fact_row("created", "created", details.time_created.clone())}
                {fact_row("version", "version", details.version_id.clone())}
                {fact_row("lifecycle", "lifecycle", details.lifecycle_state.clone())}
                {fact_row("preceding version", "preceding", details.preceding_version_uid.clone())}
                {fact_row("contribution", "contribution", details.contribution_uid.clone())}
                {fact_row("signature", "signature", signature)}
            </div>
        </section>
    }
    .into_any()
}

/// One label/value line of the versioned-object card. `hook` is the row's
/// `data-versioned-fact` value — the stable E2E hook; an absent value shows an
/// em dash.
fn fact_row(label: &'static str, hook: &'static str, value: String) -> AnyView {
    let shown = if value.is_empty() {
        "—".to_owned()
    } else {
        value
    };
    view! {
        <div>
            <span class="font-medium text-ink-muted mr-1">{label}":"</span>
            <span class="font-mono break-all text-ink" data-versioned-fact=hook>
                {shown}
            </span>
        </div>
    }
    .into_any()
}

/// The revision-history table: one row per committed version, newest-first, each
/// row opening that version in the document pane below.
fn history_section(history: HistoryResource, pinned: RwSignal<String>) -> AnyView {
    let table = view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match history.await {
                    Ok(entries) if entries.is_empty() => {
                        view! {
                            <EmptyState
                                icon=icondata_lu::LuHistory
                                message="No EHR_STATUS versions"
                                hint="Every EHR is created with a first EHR_STATUS version; if none is listed, the CDR did not report a revision history for this EHR."
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
                "Newest first. Open a version to read the EHR_STATUS document exactly as it stood at that commit."
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
                    data-status-version=hook
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
    ehr_id: Signal<String>,
    at_time: Action<(String, String), Result<String, AdminUiError>>,
    at_time_input: RwSignal<String>,
) -> AnyView {
    let on_go = move |_| {
        let requested = at_time_input.get();
        if !requested.trim().is_empty() {
            at_time.dispatch((ehr_id.get(), requested));
        }
    };
    let note = move || {
        match at_time.value().get() {
        Some(Err(AdminUiError::Cdr { status: 404, .. })) => view! { <p class="mt-2 text-sm text-ink-muted">"No EHR_STATUS version existed at that time."</p> }
        .into_any(),
        Some(Err(error)) => inline_error(&error),
        _ => ().into_any(),
    }
    };
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"EHR_STATUS at a point in time"</h2>
            <div class="flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="status-at-time">
                        "Date and time (interpreted as UTC)"
                    </label>
                    <input
                        id="status-at-time"
                        type="datetime-local"
                        class=INPUT
                        prop:value=move || at_time_input.get()
                        on:input:target=move |ev| at_time_input.set(ev.target().value())
                    />
                </div>
                <button
                    id="status-at-time-go"
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

/// The pinned version's `EHR_STATUS` document, read by its `OBJECT_VERSION_ID`.
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
                            <div id="status-version-document">
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
