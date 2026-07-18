# Leptos reviewer memory index

- [thaw hydration hazards](thaw-hydration-hazards.md) — thaw::Field emits a random UUID id (banned); thaw::Upload id is an optional prop (safe); verify widget source before approving `for=`/`id=`
- [Tabbed screen pattern](tabbed-screen-pattern.md) — always-mounted bodies + class:hidden + tab-gated resource sources (ehr_detail exemplar); template_detail diverged (eager fetch)
- [chartistry Chart hydration](chartistry-chart-hydration.md) — Chart self-gates on client-measured have_dimensions → server + client-initial both render `<p>Loading...</p>`; deterministic next_id, no random ids; hydration-safe with any AspectRatio
- [Builder signal + struct_ver](builder-signal-struct-ver.md) — query_builder's one RwSignal<BuilderQuery> + struct_ver bump-on-structural-edit-only; render reads query untracked so inputs keep focus; no effects; pure tested path helpers
