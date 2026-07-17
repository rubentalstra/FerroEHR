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

/// WASM entry point: attaches interactivity to the server-rendered DOM.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
