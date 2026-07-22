# `ehrbase-admin-ui` — the Leptos admin console (BFF over ITS-REST)

A standalone full-stack Leptos 0.8 app (cargo-leptos: `ssr` server binary +
`hydrate` WASM client) and its own OCI image. It is a **REST client** of the
CDR — no openEHR spec governs an admin UI (our own design / product
extension); the wire it consumes IS spec-bound (`docs/specs/openehr/ITS-REST/`
+ the served `openapi.json`).

## The three binding mandates (owner, 2026-07-13)

1. **Rust only — zero authored JavaScript** (the wasm-bindgen bootstrap is
   generated, never touched; styling is Tailwind v4 standalone, no Node).
2. **The CDR is reached ONLY over ITS-REST.** Never depend on `app/ehrbase`
   or `app/ehrbase-rest`; allowed deps: `crates/openehr-*` + the network.
3. **Every `#[server]` fn is a publicly reachable HTTP endpoint** — each one
   enforces the console's own session auth itself; "only my UI calls this"
   is never assumed.

## Discipline

- Rules file: `.claude/rules/leptos-ui.md` (hydration hard rules, `<For>`
  keys, forms, server fns); Leptos questions → `/leptos-lookup` (the book is
  the oracle, never memory).
- Business logic (query-builder AST lowering, criteria validation) lives in
  component-free plain-Rust modules with ordinary unit tests; components
  stay thin.
- Pins (root `[workspace.dependencies]`): `leptos`/`leptos_axum`/`leptos_meta`/
  `leptos_router` 0.8.x · `thaw` at a pinned **git rev of main** (the crates.io
  `0.5.0-beta` lacks `#![recursion_limit = "256"]` and fails plain,
  non-`erase_components` codegen on rustc 1.96; re-pin to crates.io at 0.5
  stable) · `leptos-use` 0.19 · `leptos-struct-table` 0.19 · `leptos-chartistry`
  0.2.3 · `leptos_icons` 0.7.1.
- **Views are built in `.into_any()`-erased sections** (rules §1): plain cargo
  builds have no `erase_components`, and monolithic thaw view trees blow rustc's
  layout-recursion depth in `cargo test` codegen.

## Gates

`/ui-gates`: clippy on **native and wasm32** targets, `cargo nextest run -p
ehrbase-admin-ui`, leptosfmt + cargo fmt, cargo-leptos build; E2E journeys
(`tests/e2e_*.rs`, skip-with-reason via `UI_E2E_BASE_URL`) merge-gate in CI;
a UI-visual change re-captures the `website/book` screenshots
(`ui-screenshot-guard`).
