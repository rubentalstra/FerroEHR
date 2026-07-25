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

## Error feedback: toast vs inline (one rule, 2026-07-25)

- **A mutation toasts on success AND on failure.** Every action that writes to
  the CDR (create / commit / update / save / upload / delete) reports both
  outcomes as a toast. The failure toast carries the **actionable** copy —
  name the object, name what went wrong (the CDR's own diagnostic verbatim),
  name the next action: `feedback::write_failure_copy` for writes,
  `admin::delete_failure_copy` for the destructive admin ops.
- **A detailed inline `MessageBar` may stay BESIDE the failure toast** where
  the diagnostic is worth reading line by line (template-upload validation, a
  rejected composition body, the directory conflict banner) — the toast is
  the notification, the inline bar is the detail. Never inline-only: a
  transient success toast paired with a silent failure below the fold reads
  as "nothing happened".
- **Pure reads render inline errors only** — `format_view::inline_error` in
  the section whose data failed (a screen says so where the data would be),
  and never a toast. A first-class empty/absent state (`Ok(None)` from a
  `404`) is not an error at all.

## One reader per claim (owner adjudication, 2026-07-25)

- **No two console surfaces may read the same claim from two endpoints.** Where
  the CDR exposes one fact on more than one endpoint, the console picks ONE
  reader and every other screen cross-links to it. Live cases: the topbar pill
  reads the status document (`/ehrbase/rest/status` — API up + version) while
  the operations panel's health card reads `/health/readiness` (dependency
  indicators), and the screen states the split; the redacted effective
  configuration is served identically by `/management/env` and `/admin/config`,
  so the ONE viewer lives on `/system` (the API base URL is always configured;
  the management surface may sit on an unreachable internal port) and
  `/operations` links to it.
- **Optional CDR surfaces are probe-and-hide.** An affordance for a surface the
  CDR may not serve is gated on a probe (`crate::admin` for the admin group via
  the System API manifest, `crate::management` for the management surface via
  `GET /management/info`): a `404` hides the affordance/nav entry entirely, and
  every other answer counts as present — capability is not authorization, so a
  `401`/`403` refusal surfaces as actionable copy on the screen that asked.

## Gates

`/ui-gates`: clippy on **native and wasm32** targets, `cargo nextest run -p
ehrbase-admin-ui`, leptosfmt + cargo fmt, cargo-leptos build; E2E journeys
(`tests/e2e_*.rs`, skip-with-reason via `UI_E2E_BASE_URL`) merge-gate in CI;
a UI-visual change re-captures the `website/book` screenshots
(`ui-screenshot-guard`).
