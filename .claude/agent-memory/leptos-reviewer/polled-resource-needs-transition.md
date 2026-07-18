---
name: polled-resource-needs-transition
description: A Resource refetched on an interval (health poll) must be read under <Transition>, not <Suspense>, or it flashes the fallback every poll
metadata:
  type: project
---

The shell's CDR-health pill reads a `Resource` that `use_interval_fn`
refetches every 30 s. Read under `<Suspense>`, each refetch reverts to
pending → the pill flashes its "checking…" fallback on every poll.

**Rule:** `.claude/rules/leptos-ui.md` §6 (book `async/12`) — reloading
data uses `<Transition>` to keep old data visible instead of flashing the
fallback.

**How to apply:** whenever a resource is periodically refetched (interval
poll) OR reloaded on a filter/param change, its read site must be
`<Transition>`, not `<Suspense>`. Confirmed in W2 `pages/shell.rs`
`authed_shell` health_pill.
