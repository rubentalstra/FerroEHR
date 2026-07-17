//! The root Leptos application: HTML shell, router, and (for now) the W0
//! thaw smoke-test page — replaced by the real §7A shell in W2.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
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

/// The root component: meta context, theme provider, routes.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/ehrbase-admin-ui.css" />
        <Title text="ehrbase-admin" />
        <thaw::ConfigProvider>
            <Router>
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=SmokeTest />
                </Routes>
            </Router>
        </thaw::ConfigProvider>
    }
}

/// W0 smoke test: exercises every thaw component family the §7A screen
/// catalog depends on (Layout/NavDrawer, Table, Tree, TabList, Upload,
/// MessageBar, Skeleton, Button/Input/Field) against `thaw =0.5.0-beta` on
/// Leptos 0.8 — the pinned-beta risk is retired here, before screen work.
#[component]
fn SmokeTest() -> impl IntoView {
    // NOTE: each section is type-erased with `.into_any()` — under plain
    // `cargo build`/`cargo test` (no `erase_components` cfg, which only
    // cargo-leptos dev builds pass) the fully structural thaw view types
    // nest deep enough to blow rustc's layout recursion depth. Every screen
    // keeps this section-boundary erasure pattern.
    let clicks = RwSignal::new(0u32);
    let banner = view! {
        <thaw::MessageBar>
            <thaw::MessageBarBody>"thaw 0.5.0-beta smoke test"</thaw::MessageBarBody>
        </thaw::MessageBar>
    }
    .into_any();
    let counter = view! {
        <thaw::Button on_click=move |_| {
            clicks.update(|c| *c += 1);
        }>{move || format!("clicked {}", clicks.get())}</thaw::Button>
    }
    .into_any();
    let skeleton = view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem />
        </thaw::Skeleton>
    }
    .into_any();
    let table = view! {
        <thaw::Table>
            <thaw::TableHeader>
                <thaw::TableRow>
                    <thaw::TableHeaderCell>"column"</thaw::TableHeaderCell>
                </thaw::TableRow>
            </thaw::TableHeader>
            <thaw::TableBody>
                <thaw::TableRow>
                    <thaw::TableCell>"cell"</thaw::TableCell>
                </thaw::TableRow>
            </thaw::TableBody>
        </thaw::Table>
    }
    .into_any();
    view! { <thaw::Layout>{banner}{counter}{skeleton}{table}</thaw::Layout> }
}
