---
name: router-same-route-param-nav
description: leptos_router only updates the params signal when a navigation matches the SAME <Route> — the component body does not re-run, so a path param read untracked at setup goes stale
metadata:
  type: project
---

Verified first-hand in the locked leptos_router **0.8.15** source.

**Declaration order decides the match.** `src/matching/nested/tuples.rs:299`
(`impl MatchNestedRoutes for ($($ty,)*)`) is
`$(if let (Some(..), remaining) = $ty.match_nested(path) { return … })*` — the
first tuple element that matches wins. So declaring
`demographics/relationship` before `demographics/:kind` really does keep the
literal from being read as a param. Route ids are safe under >16 children
too: `NestedRoute.id` comes from a global counter
(`src/matching/nested/mod.rs:92`, `ROUTE_ID.fetch_add`), and the Either's
`as_id()` delegates to the inner match, so nesting never collides ids.

**A same-route navigation does NOT re-run the component body.**
`src/nested_router.rs:849-851`: *"a unique ID for each route … if two IDs are
the same, we do not rerender, but only update the params"*, and the same-id
branch (~:1026) is only
`current.matched.set(…); current.params.set(…); current.url.set(…)`.
Two paths matching the same `<Route>` (`/x/person` vs `/x/organisation` under
`path!("x/:kind")`) share one id → params update, view is kept.

**Review rule:** a **path** param may be read with
`with_untracked`/`get_untracked` at setup ONLY if no reachable in-app anchor
or `navigate()` reaches another value of it under the same `<Route>`. Because
plain `<a>` is intercepted too ([[internal-nav-uses-plain-anchor]]), a
"kind switcher" of anchors is exactly such a navigation. The console's own
exemplars (`pages/ehr_detail/mod.rs:263`, `pages/composition.rs:568`) make
EVERY path param a `Signal::derive`; the only sanctioned untracked reads are
of QUERY params that arrive solely by full document load (`ehrs.rs:378`
`?find=`, whose doc comment states the rule correctly).
