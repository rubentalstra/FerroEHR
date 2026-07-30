//! `EHRbase` admin console — Leptos SSR web app + BFF over the ITS-REST API.
//!
//! No openEHR spec governs an
//! admin UI — our own design / product extension. The three binding
//! mandates: Rust only (zero authored JavaScript), the CDR is reached ONLY
//! over ITS-REST (never `app/ehrbase` / `app/ehrbase-rest` in-process), and
//! every `#[server]` fn is a publicly reachable endpoint that enforces the
//! console's own session auth itself.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
// `hydrate` (wasm client) and `ssr` (server) are mutually exclusive build
// modes — cargo-leptos always builds them separately. Guarded per the Cargo
// book's prescription for genuinely exclusive features
// (https://doc.rust-lang.org/cargo/reference/features.html#mutually-exclusive-features);
// CI's workspace lanes therefore run `--exclude ehrbase-admin-ui` under
// `--all-features` and lint/test this crate per-feature instead.
#[cfg(all(feature = "hydrate", feature = "ssr"))]
compile_error!("features \"hydrate\" and \"ssr\" cannot be enabled at the same time");

pub mod activity;
pub mod admin;
pub mod app;
pub mod aql_text;
pub mod auth;
pub mod builder;
pub mod chart_model;
pub mod clinical;
pub mod components;
pub mod error;
pub mod feedback;
pub mod format;
pub mod highlight;
pub mod management;
pub mod pages;
pub mod queries_api;
pub mod query_namespace;
pub mod scopes;
pub mod system_api;
pub mod theme;

#[cfg(feature = "ssr")]
pub mod cdr;
#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod export;
#[cfg(feature = "ssr")]
pub mod oidc;
#[cfg(feature = "ssr")]
pub mod session;
#[cfg(feature = "ssr")]
pub mod state;

/// WASM entry point: attaches interactivity to the server-rendered DOM.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
