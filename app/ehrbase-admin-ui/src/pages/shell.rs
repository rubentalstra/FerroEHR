//! The application shell: the session guard plus the persistent chrome
//! (responsive nav drawer, topbar with the CDR health pill, user menu +
//! scopes drawer, dark-mode toggle, footer) around the routed `<Outlet/>`.
//!
//! The shell is a layout, not a route: it is the `view` of the guarded
//! `ParentRoute` in [`crate::app`], and it renders the matched child screen
//! through `<Outlet/>`. Access is gated on a live console session — with no
//! session the guard redirects to `/login`.

use leptos::prelude::*;
use leptos_router::components::{Outlet, Redirect};
use leptos_use::use_interval_fn;
// The `view!` macro rejects a module-pathed name in slot position, so the
// slot component alone is imported directly (a plain import, not a rename).
use thaw::PopoverTrigger;

use crate::auth::{Logout, SessionInfo, current_session, fetch_status};

/// How often (ms) the topbar re-polls the CDR `/rest/status` health endpoint.
const HEALTH_POLL_MS: u64 = 30_000;

/// The browser `localStorage` key the dark-mode preference persists under.
const THEME_STORAGE_KEY: &str = "ehrbase-admin-theme";

/// Maps a full URL path to the top-level nav key it belongs under, so the nav
/// drawer highlights the active section after a (full-page) navigation. Pure
/// logic, unit-tested below.
fn nav_key(path: &str) -> &'static str {
    if path.starts_with("/templates") {
        "/templates"
    } else if path.starts_with("/queries") {
        "/queries"
    } else if path.starts_with("/ehrs") {
        "/ehrs"
    } else if path.starts_with("/system") {
        "/system"
    } else {
        "/"
    }
}

/// The session-guarded application shell.
///
/// Loads the current session with a [`Resource`]; under `<Suspense>` a missing
/// session renders a [`Redirect`] to `/login`, and a present session renders
/// the full chrome + `<Outlet/>`. The topbar health pill reads a second
/// resource over `fetch_status`, re-polled every [`HEALTH_POLL_MS`] via
/// [`use_interval_fn`] (a no-op on the server; the effect-safe browser timer
/// pattern). Dark mode flips the `thaw` theme signal imperatively on click and
/// persists to `localStorage`; the persisted choice is re-applied after
/// hydration inside an [`Effect`], keeping the initial render deterministic
/// (rules §8).
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[allow(clippy::too_many_lines)] // resource/effect setup plus the guarded view — one cohesive component
#[component]
pub fn AppShell() -> impl IntoView {
    let session = Resource::new(|| (), |()| current_session());
    let status = Resource::new(|| (), |()| fetch_status());
    let theme = thaw::ConfigInjection::expect_context().theme;
    let is_dark = RwSignal::new(false);
    let nav_open = RwSignal::new(false);
    let scopes_open = RwSignal::new(false);
    let logout_action = ServerAction::<Logout>::new();

    // Active nav section, seeded once from the request URL (identical on the
    // server pass and on hydration — the URL is the same on both).
    let selected = RwSignal::new(Some(
        nav_key(
            &leptos_router::hooks::use_location()
                .pathname
                .get_untracked(),
        )
        .to_owned(),
    ));

    // Browser-only: re-apply the persisted theme after hydration. Reads the
    // outside world (localStorage), so an Effect is the correct home (rules
    // §2/§8); it never runs on the server.
    Effect::new(move |_| {
        if let Ok(Some(storage)) = window().local_storage()
            && let Ok(Some(pref)) = storage.get_item(THEME_STORAGE_KEY)
        {
            let dark = pref == "dark";
            is_dark.set(dark);
            theme.set(if dark {
                thaw::Theme::dark()
            } else {
                thaw::Theme::light()
            });
        }
    });

    // Browser-only: poll the CDR health endpoint on an interval.
    let _pausable = use_interval_fn(
        move || {
            status.refetch();
        },
        HEALTH_POLL_MS,
    );

    view! {
        <Suspense fallback=|| {
            view! {
                <div class="min-h-screen flex items-center justify-center">
                    <thaw::Spinner />
                </div>
            }
        }>
            {move || {
                Suspend::new(async move {
                    match session.await {
                        Ok(Some(info)) => {
                            authed_shell(
                                info,
                                status,
                                theme,
                                is_dark,
                                nav_open,
                                scopes_open,
                                selected,
                                logout_action,
                            )
                        }
                        _ => view! { <Redirect path="/login" /> }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

/// Builds the authenticated chrome (topbar, nav, footer, scopes drawer) around
/// the routed `<Outlet/>`. Split out of the component body so each section is a
/// `.into_any()`-erased local (the section-boundary erasure rule, §1) and so
/// the identity/scopes from the resolved session flow in as owned data.
#[allow(clippy::too_many_arguments)] // one flat call carrying the shell's shared reactive state; a wrapper struct would not aid clarity
#[allow(clippy::too_many_lines)] // one screen assembled from `.into_any()`-erased section locals (rules §1) — deliberately one function
fn authed_shell(
    info: SessionInfo,
    status: Resource<Result<String, crate::error::AdminUiError>>,
    theme: RwSignal<thaw::Theme>,
    is_dark: RwSignal<bool>,
    nav_open: RwSignal<bool>,
    scopes_open: RwSignal<bool>,
    selected: RwSignal<Option<String>>,
    logout_action: ServerAction<Logout>,
) -> AnyView {
    let SessionInfo {
        identity,
        method,
        scopes,
    } = info;
    let scope_count = scopes.len();
    let trigger_label = format!("{identity} ▾");

    // <Transition>, not <Suspense>: the 30 s poll refetches the resource and
    // Transition keeps the previous pill visible during reload instead of
    // flashing the fallback every poll (leptos-ui.md §6, book async/12).
    let health_pill = view! {
        <Transition fallback=|| {
            view! { <span class="text-xs opacity-60">"checking…"</span> }
        }>
            {move || {
                Suspend::new(async move {
                    match status.await {
                        Ok(body) => {
                            let doc = serde_json::from_str::<serde_json::Value>(&body).ok();
                            let (label, cls) = match doc
                                .as_ref()
                                .and_then(|v| v.get("status"))
                                .and_then(serde_json::Value::as_str)
                            {
                                Some("UP") => ("UP", "text-xs text-green-600"),
                                _ => ("DEGRADED", "text-xs text-amber-600"),
                            };
                            let version = doc
                                .as_ref()
                                .and_then(|v| v.get("server_version"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or_default();
                            view! {
                                <span class=cls>{format!("● CDR {label} · v{version}")}</span>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! { <span class="text-xs text-red-600">"● CDR DOWN"</span> }
                                .into_any()
                        }
                    }
                })
            }}
        </Transition>
    }
    .into_any();

    let dark_toggle = view! {
        <thaw::Button
            appearance=thaw::ButtonAppearance::Subtle
            on_click=move |_| {
                let now_dark = !is_dark.get_untracked();
                is_dark.set(now_dark);
                theme.set(if now_dark { thaw::Theme::dark() } else { thaw::Theme::light() });
                if let Ok(Some(storage)) = window().local_storage() {
                    let _ = storage
                        .set_item(THEME_STORAGE_KEY, if now_dark { "dark" } else { "light" })
                        .ok();
                }
            }
        >
            {move || if is_dark.get() { "☀" } else { "☾" }}
        </thaw::Button>
    }
    .into_any();

    let user_menu = view! {
        <thaw::Popover trigger_type=thaw::PopoverTriggerType::Click>
            <PopoverTrigger slot>
                <thaw::Button appearance=thaw::ButtonAppearance::Subtle>
                    {trigger_label}
                </thaw::Button>
            </PopoverTrigger>
            <div class="flex flex-col gap-2 min-w-44">
                <span class="text-sm font-medium">{identity}</span>
                <span class="text-xs opacity-60">{method}</span>
                <thaw::Button
                    appearance=thaw::ButtonAppearance::Subtle
                    on_click=move |_| scopes_open.set(true)
                >
                    "View scopes"
                </thaw::Button>
                <ActionForm action=logout_action>
                    <thaw::Button
                        button_type=thaw::ButtonType::Submit
                        appearance=thaw::ButtonAppearance::Secondary
                        block=true
                    >
                        "Sign out"
                    </thaw::Button>
                </ActionForm>
            </div>
        </thaw::Popover>
    }
    .into_any();

    let scopes_body = if scopes.is_empty() {
        view! {
            <p class="text-sm opacity-70">
                "No scopes — Basic authentication grants full console access."
            </p>
        }
        .into_any()
    } else {
        let items = scopes
            .iter()
            .map(|s| view! { <li class="text-sm">{s.clone()}</li> })
            .collect_view();
        view! { <ul class="list-disc pl-5 flex flex-col gap-1">{items}</ul> }.into_any()
    };

    let scopes_drawer = view! {
        <thaw::OverlayDrawer open=scopes_open position=thaw::DrawerPosition::Right>
            <thaw::DrawerHeader>
                <thaw::DrawerHeaderTitle>"Access scopes"</thaw::DrawerHeaderTitle>
            </thaw::DrawerHeader>
            <thaw::DrawerBody>{scopes_body}</thaw::DrawerBody>
        </thaw::OverlayDrawer>
    }
    .into_any();

    let nav = view! {
        <aside class="md:block border-r shrink-0" class:hidden=move || !nav_open.get()>
            <thaw::NavDrawer selected_value=selected>
                <thaw::NavItem value="/" href="/">
                    "Dashboard"
                </thaw::NavItem>
                <thaw::NavItem value="/templates" href="/templates">
                    "Templates"
                </thaw::NavItem>
                <thaw::NavItem value="/queries" href="/queries">
                    "Queries"
                </thaw::NavItem>
                <thaw::NavItem value="/ehrs" href="/ehrs">
                    "EHRs"
                </thaw::NavItem>
                <thaw::NavItem value="/system" href="/system">
                    "System"
                </thaw::NavItem>
            </thaw::NavDrawer>
        </aside>
    }
    .into_any();

    let footer = view! {
        <footer class="flex items-center gap-2 px-4 h-10 border-t text-xs opacity-70">
            <span>{format!("console v{}", env!("CARGO_PKG_VERSION"))}</span>
            <span>"·"</span>
            <span>{format!("{scope_count} scope(s)")}</span>
        </footer>
    }
    .into_any();

    let topbar = view! {
        <header class="flex items-center gap-3 px-4 h-14 border-b shrink-0">
            <thaw::Button
                class="md:hidden"
                appearance=thaw::ButtonAppearance::Subtle
                on_click=move |_| nav_open.update(|open| *open = !*open)
            >
                "☰"
            </thaw::Button>
            <span class="font-semibold">"ehrbase-admin"</span>
            <div class="flex-1"></div>
            {health_pill}
            {user_menu}
            {dark_toggle}
        </header>
    }
    .into_any();

    view! {
        <div class="flex flex-col min-h-screen">
            {topbar}
            <div class="flex flex-1 min-h-0">
                {nav} <main class="flex-1 min-w-0 overflow-auto">
                    <Outlet />
                </main>
            </div> {footer} {scopes_drawer}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::nav_key;

    #[test]
    fn nav_key_maps_paths_to_top_level_sections() {
        assert_eq!(nav_key("/"), "/");
        assert_eq!(nav_key("/templates"), "/templates");
        assert_eq!(nav_key("/templates/vitals.v1"), "/templates");
        assert_eq!(nav_key("/queries/builder"), "/queries");
        assert_eq!(nav_key("/ehrs/abc/compositions/xyz"), "/ehrs");
        assert_eq!(nav_key("/system"), "/system");
        assert_eq!(nav_key("/unknown"), "/");
    }
}
