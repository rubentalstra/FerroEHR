//! The `/ehrs/{ehr_id}` screen — EHR detail: status / directory / compositions /
//! contributions tabs.
//!
//! Four URL-driven tabs (`?tab=`, rules §9) over one EHR. Each tab's data is a
//! `#[server]` fn co-located with the tab in its own submodule ([`status`],
//! [`directory`], [`compositions`], [`contributions`]); the resources are
//! created once and their sources are gated on the active tab (a `Memo` over
//! the query map), so only the visible tab fetches (rules §6 — never
//! fetch-in-effect). The tab bodies are always mounted and toggled with
//! `class:hidden`, keeping the server and client view structure identical
//! (rules §8 — no `cfg!`-branched structure).
//!
//! This module owns the [`EhrDetailPage`] shell, the shared [`tab_bar`] strip,
//! and the `commit_version_uid` helper shared by the directory and composition
//! commit paths.
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound (ITS-REST EHR + Query APIs). User input
//! NEVER concatenates into AQL — the fixed query is a validated const and the
//! `ehr_id` travels as an AQL `query_parameters` binding; path segments are
//! percent-encoded server-side.
//!
//! Every co-located `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first (rules §0), and
//! the CDR credential never reaches client-visible state.

pub mod compositions;
pub mod contributions;
pub mod directory;
pub mod status;

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;
#[cfg(feature = "ssr")]
use serde_json::Value;

use crate::components::page_header::{Crumb, PageHeader};
use crate::pages::ehr_detail::compositions::compositions_section;
use crate::pages::ehr_detail::contributions::contributions_section;
use crate::pages::ehr_detail::directory::directory_section;
use crate::pages::ehr_detail::status::status_section;

#[cfg(feature = "ssr")]
/// The new version uid of a just-committed versioned object: `uid.value` from
/// the `Prefer: return=representation` body (an `OBJECT_VERSION_ID`). Empty when
/// the CDR returned no representation body — the UI then shows a generic
/// success message rather than a uid.
fn commit_version_uid(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|doc| {
            doc.get("uid")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

/// The `/ehrs/{ehr_id}` screen: the tab bar plus four always-mounted,
/// visibility-toggled tab bodies.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn EhrDetailPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let ehr_id = Signal::derive(move || params.with(|p| p.get("ehr_id").unwrap_or_default()));
    let query = leptos_router::hooks::use_query_map();
    let offset = Signal::derive(move || {
        query
            .with(|q| q.get("offset"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });
    // Tab state lives in the URL (`?tab=`, rules §9): shareable and refresh-safe.
    // A Memo (not an Effect) derives the active tab, defaulting to "status".
    let selected: Memo<String> = Memo::new(move |_| {
        query
            .with(|q| q.get("tab"))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "status".to_owned())
    });

    let status = status_section(ehr_id, selected);
    let directory = directory_section(ehr_id, selected);
    let compositions = compositions_section(ehr_id, offset, selected);
    let contributions = contributions_section(ehr_id, selected);

    let heading = Signal::derive(move || {
        let id = ehr_id.get();
        let short: String = id.chars().take(8).collect();
        format!("EHR {short}…")
    });

    let tabs = tab_bar(ehr_id, selected);

    view! {
        <Title text="EHR detail · ehrbase-admin" />
        <div class="p-6">
            <PageHeader
                title=Signal::derive(move || heading.get())
                crumbs=vec![Crumb::new("EHRs", "/ehrs")]
                mono=true
            />
            {tabs}
            <div class="mt-4">
                <div class:hidden=move || selected.get() != "status">{status}</div>
                <div class:hidden=move || selected.get() != "directory">{directory}</div>
                <div class:hidden=move || selected.get() != "compositions">{compositions}</div>
                <div class:hidden=move || {
                    selected.get() != "contributions"
                }>{contributions}</div>
            </div>
        </div>
    }
}

/// The URL-driven tab bar: four pill anchors (`?tab=…`) replacing the thaw
/// `TabList`. Selected = `bg-accent-subtle text-accent-ink`; idle =
/// `text-ink-muted hover:bg-sunken`. Plain anchors keep the tabs working
/// before hydration (the router intercepts them once WASM loads).
fn tab_bar(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
    let link = move |value: &'static str, label: &'static str| {
        let href = move || format!("/ehrs/{}?tab={value}", ehr_id.get());
        let class = move || {
            if selected.get() == value {
                "rounded-control px-3 py-1.5 text-sm font-medium bg-accent-subtle text-accent-ink"
            } else {
                "rounded-control px-3 py-1.5 text-sm font-medium text-ink-muted hover:bg-sunken"
            }
        };
        view! {
            <a href=href class=class>
                {label}
            </a>
        }
    };
    view! {
        <div class="flex flex-wrap gap-1 border-b border-edge pb-2">
            {link("status", "Status")} {link("directory", "Directory")}
            {link("compositions", "Compositions")} {link("contributions", "Contributions")}
        </div>
    }
    .into_any()
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::commit_version_uid;

    #[test]
    fn commit_version_uid_reads_uid_value_or_empty() {
        let body =
            r#"{"_type":"COMPOSITION","uid":{"_type":"OBJECT_VERSION_ID","value":"7d44::sys::1"}}"#;
        assert_eq!(commit_version_uid(body), "7d44::sys::1");
        // A return=minimal (empty) or non-JSON body yields no uid.
        assert_eq!(commit_version_uid(""), "");
        assert_eq!(commit_version_uid("{}"), "");
    }
}
