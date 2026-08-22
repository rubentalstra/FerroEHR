// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `FerroEHR` admin console — Leptos SSR web app + BFF over the ITS-REST API.
//!
//! No openEHR spec governs an
//! admin UI — our own design / product extension. The three binding
//! mandates: Rust only (zero authored JavaScript), the CDR is reached ONLY
//! over ITS-REST (never `app/ferroehr` / `app/ferroehr-rest` in-process), and
//! every `#[server]` fn is a publicly reachable endpoint that enforces the
//! console's own session auth itself.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
// Every `#[component]` in this crate expands `#[derive(TypedBuilder)]` on its
// generated props struct, and that derive emits an inherent `builder()` whose
// name matches a trait method already in scope — so `same_name_method` fires
// once per component with the macro invocation as its only span, never on
// hand-written code. Crate-level because there is no smaller item to scope it
// to: the finding does not exist in this crate's source
// (https://docs.rs/leptos/0.8/leptos/attr.component.html).
#![allow(
    clippy::same_name_method,
    reason = "emitted only by leptos's TypedBuilder derive inside #[component]; no hand-written method in this crate shadows a trait method"
)]
// `hydrate` (wasm client) and `ssr` (server) are mutually exclusive build
// modes — cargo-leptos always builds them separately. Guarded per the Cargo
// book's prescription for genuinely exclusive features
// (https://doc.rust-lang.org/cargo/reference/features.html#mutually-exclusive-features);
// CI's workspace lanes therefore run `--exclude ferroehr-admin-ui` under
// `--all-features` and lint/test this crate per-feature instead.
#[cfg(all(feature = "hydrate", feature = "ssr"))]
compile_error!("features \"hydrate\" and \"ssr\" cannot be enabled at the same time");

pub mod activity;
pub mod adl2;
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
pub mod fhir;
pub mod format;
pub mod highlight;
pub mod management;
pub mod pages;
pub mod queries_api;
pub mod query_namespace;
pub mod scopes;
pub mod system_api;
pub mod tenants;
pub mod terminology;
pub mod theme;
pub mod uid;

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
