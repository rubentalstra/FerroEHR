//! `EHRbase` admin console — Leptos SSR web app + BFF over the ITS-REST API.
//!
//! Governing plan: `docs/design/ehrbase-admin-ui.md` (deleted at close; the
//! durable record is `docs/architecture.md`). No openEHR spec governs an
//! admin UI — our own design / product extension. The three binding
//! mandates: Rust only (zero authored JavaScript), the CDR is reached ONLY
//! over ITS-REST (never `app/ehrbase` / `app/ehrbase-rest` in-process), and
//! every `#[server]` fn is a publicly reachable endpoint that enforces the
//! console's own session auth itself.

pub mod app;
pub mod auth;
pub mod builder;
pub mod components;
pub mod error;
pub mod format;
pub mod pages;
pub mod queries_api;
pub mod theme;
pub mod urlq;

#[cfg(feature = "ssr")]
pub mod groups;

#[cfg(feature = "ssr")]
pub mod folder_templates;

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
