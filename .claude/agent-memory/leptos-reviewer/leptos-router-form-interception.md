---
name: leptos-router-form-interception
description: leptos_router 0.8.14 never intercepts native form submits (click-on-anchor + popstate only), so a plain <form> is the right no-JS tool; <Form method="GET"> to the SAME path is a trap
metadata:
  type: reference
---

Verified in `~/.cargo/registry/src/index.crates.io-*/leptos_router-0.8.14`:

- The Router installs exactly TWO global listeners: `window_event_listener(ev::click, …)`
  and `ev::popstate` (`src/location/history.rs:161` + `:200`). There is **no
  window/document `submit` listener anywhere in the crate.**
- The click handler acts only when an `HtmlAnchorElement` appears in
  `ev.composed_path()` (`src/location/mod.rs:321-329`), and bails on
  `target`/`download`/`rel=external`/cross-origin. A click on
  `<button type="submit">` is therefore ignored.
- Submit interception exists ONLY as `.on(ev::submit, on_submit)` that the
  `Form`/`ActionForm` component attaches to its own element (`src/form.rs:309`).

⇒ A plain `<form method="GET" action="/x">` + a Rust `on:submit` that calls
`ev.prevent_default()` is safe and correct for progressive enhancement
(leptos-ui.md §5 uncontrolled + §9 state-in-URL): native GET pre-WASM,
client-side navigation once hydrated.

**Trap:** `leptos_router::components::Form method="GET"` pointed at the SAME
route path does a client-side navigation, and `NestedRoutesView::rebuild`
short-circuits — "if the path is the same, we do not need to re-route, we can
just update the search query" (`src/nested_router.rs:152-162`). The route
component does NOT re-run, so anything decided from the query at setup
(untracked read) silently no-ops. Corollary for reviews: a query param a route
body reads **untracked** only works on cross-path navigation and full loads;
same-path query changes need a reactive read (`Memo`/`Signal` over
`use_query_map`), which is why `/ehrs`'s `?offset=` is a `Signal`.

Path params ARE percent-decoded on read (`ParamsMap::insert` calls
`Url::unescape`, `src/params.rs:29`), so encoding an id into a link round-trips.

Related: [[redirect-needs-ssrmode-async]], [[redirect-path-must-be-percent-encoded]]
