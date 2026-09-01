---
name: tabbed-screen-pattern
description: The established correct pattern for thaw::TabList screens in this viewer (always-mounted bodies + class:hidden + tab-gated resource sources)
metadata:
  type: project
---

The correct multi-tab screen shape in this viewer (exemplar: `pages/ehr_detail.rs`):

- All tab bodies are **always mounted**, toggled with `class:hidden=move || selected.get() != "x"`
  — identical server/client view structure (leptos-ui.md §8, no cfg!-branched structure).
- Each tab's `Resource` **source is gated on the active tab**:
  `move || (selected.get() == "x").then(|| ehr_id.get())`, fetcher returns `Ok(None)` when
  inactive → only the visible tab hits the CDR (leptos-ui.md §6, "tab-gated resources fetch
  only the visible tab").

**Why:** avoids N eager CDR round-trips per page load (especially expensive endpoints like
template example-generation) while keeping hydration structure stable.
**How to apply:** on any new tabbed screen, confirm resource sources are tab-gated, not just
mounted. `pages/template_detail.rs` (as of 2026-07-17, branch claude/admin-ui) DIVERGED — its
catalog/opt/example resources fetch eagerly on mount. Gating still preserves loaded state
(stable source → no refetch on re-show), so eager fetch is a should-fix, not a requirement.
Also: tab selection is a private RwSignal (not URL) on both screens — refresh/deep-link loses
the active tab (leptos-ui.md §9 shareable-state spirit); noted as a standing nit.
