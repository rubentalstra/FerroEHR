//! The root Leptos application: the HTML document shell, the theme provider,
//! and the §7A route tree. `/login` is public; every other screen is nested
//! under the session-guarded [`crate::pages::shell::AppShell`] layout, which
//! renders the matched child through its `<Outlet/>`.
//!
//! Screens not yet built (dashboard, templates, queries, EHRs and their
//! detail routes) render a small [`Placeholder`] card naming the stage that
//! delivers them (W3 browse surfaces, W4 query-builder + dashboard). Every
//! routed view — placeholders included — sets its own `<Title/>`.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

/// The full HTML document shell rendered by the server.
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// The root component: meta context, the `thaw` theme/config provider, and the
/// router. The `theme_id` is fixed (never the component's default random
/// UUID) so the server pass and client hydration emit an identical
/// `data-thaw-id` and generated-style selector — a hydration-determinism
/// requirement (rules §8).
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/ehrbase-admin-ui.css" />
        <Title text="ehrbase-admin" />
        <thaw::ConfigProvider theme_id="ehrbase-admin".to_owned()>
            <Router>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/login") view=crate::pages::login::LoginPage />
                    <ParentRoute path=path!("") view=crate::pages::shell::AppShell>
                        <Route
                            path=path!("")
                            view=|| view! { <Placeholder title="Dashboard" stage="W4" /> }
                        />
                        <Route
                            path=path!("templates")
                            view=|| view! { <Placeholder title="Templates" stage="W3" /> }
                        />
                        <Route
                            path=path!("templates/:template_id")
                            view=|| view! { <Placeholder title="Template detail" stage="W3" /> }
                        />
                        <Route
                            path=path!("queries")
                            view=|| view! { <Placeholder title="Stored queries" stage="W4" /> }
                        />
                        <Route
                            path=path!("queries/builder")
                            view=|| view! { <Placeholder title="Query builder" stage="W4" /> }
                        />
                        <Route
                            path=path!("queries/aql")
                            view=|| view! { <Placeholder title="Raw AQL" stage="W4" /> }
                        />
                        <Route
                            path=path!("ehrs")
                            view=|| view! { <Placeholder title="EHRs" stage="W3" /> }
                        />
                        <Route
                            path=path!("ehrs/:ehr_id")
                            view=|| view! { <Placeholder title="EHR detail" stage="W3" /> }
                        />
                        <Route
                            path=path!("ehrs/:ehr_id/compositions/:uid")
                            view=|| view! { <Placeholder title="Composition viewer" stage="W3" /> }
                        />
                        <Route path=path!("system") view=crate::pages::system::SystemPage />
                    </ParentRoute>
                </Routes>
            </Router>
        </thaw::ConfigProvider>
    }
}

/// Interim screen for a route whose real UI lands in a later stage. Renders a
/// `thaw` Card naming the screen and its delivering stage, and sets the page
/// `<Title/>` so the routed-page title rule holds even before the screen
/// exists.
#[component]
fn Placeholder(
    /// The screen's display name (also the page title stem).
    #[prop(into)]
    title: String,
    /// The delivery stage label (`"W3"` browse surfaces, `"W4"` builder +
    /// dashboard).
    #[prop(into)]
    stage: String,
) -> impl IntoView {
    let page_title = format!("{title} · ehrbase-admin");
    let heading = view! {
        <thaw::CardHeader>
            <thaw::Body1>{title}</thaw::Body1>
        </thaw::CardHeader>
    }
    .into_any();
    let note = view! { <p class="text-sm opacity-70">{format!("This screen is delivered in stage {stage}.")}</p> }
    .into_any();
    view! {
        <Title text=page_title />
        <div class="p-6">
            <thaw::Card>{heading}{note}</thaw::Card>
        </div>
    }
}

/// The router's real 404 fallback: sets a distinct title and offers a link
/// back to the dashboard.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <Title text="Not found · ehrbase-admin" />
        <div class="p-6">
            <thaw::Card>
                <thaw::CardHeader>
                    <thaw::Body1>"Page not found"</thaw::Body1>
                </thaw::CardHeader>
                <p class="text-sm opacity-70">
                    "The page you requested does not exist. "
                    <leptos_router::components::A href="/" attr:class="underline">
                        "Return to the dashboard."
                    </leptos_router::components::A>
                </p>
            </thaw::Card>
        </div>
    }
}
