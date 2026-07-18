---
name: chartistry-chart-hydration
description: leptos-chartistry 0.2.3 Chart is hydration-safe (client-measured gate + deterministic ids); verified against vendored source
metadata:
  type: project
---

Verified against the pinned `leptos-chartistry-0.2.3` source
(`~/.cargo/registry/.../leptos-chartistry-0.2.3/src/`):

- **`<Chart>` is hydration-safe regardless of `AspectRatio`.** `chart.rs`
  gates the SVG behind `<Show when=have_dimensions fallback=|| <p>"Loading..."</p>>`,
  where `have_dimensions = watch.bounds.get().is_some()` and `bounds` comes
  from `use_watched_node` (client-only `getBoundingClientRect`). So the SERVER
  render and the CLIENT's FIRST hydration render both emit
  `<div class="_chartistry">…<p>Loading...</p></div>` — structures match; the
  chart draws only after a post-mount effect measures the node (normal
  post-hydration reactivity). This holds even for a fixed
  `AspectRatio::from_outer_ratio(w,h)` — the `have_dimensions` gate still
  applies. (leptos-ui.md §8 — deterministic initial render.)
- **No random ids.** Series ids are a deterministic `next_id: usize` counter
  (`series/mod.rs`) assigned when `Series::new().line(...)` is built in the
  component body, identical on server and client. No `Uuid`/`rand`.

**Why:** the dashboard trend chart (`pages/dashboard.rs::trend_chart`) claimed
hydration stability; confirmed true.
**How to apply:** a `<Chart>` needs no `<Suspense>`/placeholder wrapper of its
own for hydration — the widget self-gates. Don't flag a chartistry chart as a
hydration hazard on the basis of client measurement alone.
