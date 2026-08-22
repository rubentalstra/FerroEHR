---
name: internal-nav-uses-plain-anchor
description: CORRECTED — plain <a href> IS intercepted by leptos_router's window-level click handler, so internal anchors are client-side navigations, not full-page reloads
metadata:
  type: project
---

The console navigates internal routes with plain `<a href>` and thaw `NavItem
href=…` rather than `leptos_router::components::A`.

**The earlier claim in this memory — that those do FULL-PAGE reloads — was
WRONG.** Verified first-hand against the locked leptos_router 0.8.15 source:

- `src/location/history.rs:161` — `BrowserUrl::init` installs
  `window_event_listener(ev::click, …)` unconditionally.
- `src/location/mod.rs:298` `handle_anchor_click` walks `ev.composed_path()`
  for ANY `HtmlAnchorElement` and calls `ev.prevent_default()` + a History-API
  navigation. It bails out only for: a modified click (button != 0, meta/alt/
  ctrl/shift), a non-empty `target`, `download`, `rel="external"`, a
  cross-origin href, or a path outside the router base.

So a plain `<a href="/x">` is a client-side navigation exactly like `<A>`; the
only thing `<A>` adds is `aria-current`/active-class handling. Do NOT flag
plain internal anchors as full-page reloads, and DO treat every internal
anchor as a client-side nav when reasoning about remount semantics —
see [[router-same-route-param-nav]], which is what makes that load-bearing.

`rel="external"` on an anchor to a BFF axum route (the OIDC login link) is
still required and correct (leptos-ui.md §4).
