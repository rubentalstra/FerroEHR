---
name: no-lint-suppressions-in-source
description: "Never silence a lint with #[expect]/#[allow] in production code — fix the code; suppressions are for test files only"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 41ab4b7d-4974-4bc0-9a89-c1d7691c6eb6
  modified: 2026-09-10T05:12:43.817Z
---

Owner directive 2026-09-10, stated twice and emphatically: **do not add
`#[expect(lint, …)]` or `#[allow(lint, …)]` to production code.** A lint
finding is solved, not silenced. Suppressions are acceptable only in test
files.

**Why:** a suppression records that nobody dealt with the finding, and it
outlives the reason it was added. The repo's own reliability rules say a rule
without a failing check is a wish — a silenced check is the same thing with
extra steps.

**How to apply:** when a new toolchain's lint set fires, rewrite the code.
Worked examples from the 1.97→1.98.1 bump:
- `clippy::unused_async_trait_impl` on an axum `FromRequestParts` impl → drop
  `async` and return `std::future::ready(...)` from a plain fn (an impl may
  satisfy a trait's `async fn` that way).
- `clippy::result_large_err` on `Result<(), Response>` → `Box<Response>` in the
  `Err` arm and `*resp` at the single call site.

The one case that is not our code to fix: a lint firing **entirely inside a
third-party macro expansion** (leptos `#[component]`'s TypedBuilder derive,
`server_fn`'s `#[server]` on wasm). Those carry a crate-root `#![allow]` whose
reason states the macro and that no hand-written item in the crate matches —
the precedent is in `app/ferroehr-viewer/src/lib.rs`. Say so out loud when
adding one rather than slipping it in.

Related: [[owner-work-style]], [[gate-parity-and-caller-sweeps]].
