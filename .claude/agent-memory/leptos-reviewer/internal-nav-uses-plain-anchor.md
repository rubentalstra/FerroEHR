---
name: internal-nav-uses-plain-anchor
description: W2 admin-ui navigates internal routes with plain <a href> (and thaw NavItem href), not leptos_router <A> — full-page reloads, §9
metadata:
  type: project
---

The W2 shell/app never import or use `leptos_router::components::A`. Internal
links use plain `<a href="/">` (e.g. `app.rs` NotFound) and the nav drawer
uses `thaw::NavItem href=…`, both of which do full-page navigations instead
of client-side routing.

**Rule:** `.claude/rules/leptos-ui.md` §9 — "Navigation uses `<A>`/router
APIs, never window.location."

**How to apply:** hand-written internal links should be `<A href=…>`
(should-fix). The `thaw::NavDrawer`/`NavItem` full-page nav is a thaw
constraint (it renders raw `<a href>`, not integrated with leptos_router) —
the `selected`-nav-seeded-from-URL comment in shell.rs acknowledges it; treat
as a recorded deviation, not a fresh finding, unless a client-side-routing
NavItem wrapper is introduced. Plain `<a>` to a BFF axum route (OIDC login) is
correct and must stay a plain anchor.
