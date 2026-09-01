---
name: redirect-needs-ssrmode-async
description: <Redirect> from a route body sets a real 302 only because the authenticated routes are SsrMode::Async (first chunk = whole document); under out-of-order streaming the header would be too late
metadata:
  type: reference
---

`leptos_router::components::Redirect` is `#[component(transparent)]`: its body
runs at view-construction time, and on the server it calls the
`ServerRedirectFunction` (`leptos_router-0.8.14/src/components.rs:563-604`),
which `leptos_axum` provides as `leptos_axum::redirect`
(`leptos_axum-0.8.10/src/lib.rs:965` → `:227-257`).

Two conditions make it work, and both must be checked in review:

1. **`Accept: text/html`** — `redirect()` inserts `Location` always but sets
   `302 Found` only when the request accepts `text/html`; otherwise it just adds
   a custom redirect header (so server-fn callers can still read the payload).
   Browser navigations always qualify; scripted clients get `200` + `Location`.
2. **Rendering mode** — status/headers are applied only after the FIRST chunk of
   the stream (`leptos_integration_utils-0.8.8/src/lib.rs`, "wait for the first
   chunk of the stream, then set the status and headers"). Under
   `SsrMode::Async` the first chunk is the ENTIRE document
   (`leptos_axum/src/lib.rs:1153-1171`, `async_stream_builder` collects to a
   `String`), so a redirect decided anywhere in the tree still lands. Under the
   default **out-of-order streaming the head is flushed first and the redirect
   would be lost** — a `<Redirect>` decided from a route body is therefore
   coupled to the route's `ssr=SsrMode::Async` (viewer: the authenticated
   `ParentRoute` in `src/app.rs`). Require that coupling in a written comment.

Hydration: a redirect branch chosen purely from the URL is deterministic, not a
§8 `cfg!` structure branch, and the browser never hydrates it after a document
load (it follows the 302). Client-side arrivals take `Redirect`'s
`use_navigate()` branch.

Nuance seen in the viewer: the shell's session gate (`/login`) and a child
route's `<Redirect>` can both write `Location` in one pass; `insert_header`
replaces, so the last one to resolve wins — worst case one extra hop, no leak
(every `#[server]` fn guards independently).

Related: [[leptos-router-form-interception]], [[redirect-path-must-be-percent-encoded]]
