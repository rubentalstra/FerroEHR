# Memory index — leptos-reviewer

- [thaw::Field random id](thaw-field-random-id.md) — Field mints Uuid::new_v4() id; §8 hydration hazard unless an explicit stable id is passed
- [thaw hydration hazards](thaw-hydration-hazards.md) — thaw::Field emits random UUID id (banned); thaw::Upload id is optional prop (safe); verify widget source before approving `for=`/`id=`
- [thaw::Input name forwarding OK](thaw-input-name-forwarding-ok.md) — Input forwards explicit `name` to the real <input>; no-JS ActionForm submit works, don't flag
- [Polled resource needs Transition](polled-resource-needs-transition.md) — interval-refetched resource under <Suspense> flashes fallback; §6 wants <Transition>
- [Plain <a> IS client-side nav](internal-nav-uses-plain-anchor.md) — CORRECTION: leptos_router intercepts every same-origin anchor via a window click handler; not a full reload
- [Same-route param nav keeps the view](router-same-route-param-nav.md) — declaration-order first match; same `<Route>` id ⇒ params update only, body never re-runs ⇒ untracked path params go stale
- [W2 confirmed-good patterns](w2-confirmed-good-patterns.md) — auth guards, .into_any() erasure, theme Effect, no-usize/unwrap — verified correct, don't re-flag
- [Tabbed screen pattern](tabbed-screen-pattern.md) — always-mounted bodies + class:hidden + tab-gated resource sources (ehr_detail exemplar); template_detail diverged (eager fetch)
- [chartistry Chart hydration](chartistry-chart-hydration.md) — Chart self-gates on client-measured have_dimensions; deterministic next_id, no random ids; hydration-safe with any AspectRatio
- [Builder signal + struct_ver](builder-signal-struct-ver.md) — query_builder's one RwSignal<BuilderQuery> + struct_ver bump-on-structural-edit-only; render reads query untracked so inputs keep focus; no effects; pure tested path helpers
- [Router never intercepts form submits](leptos-router-form-interception.md) — plain `<form>` is safe; `<Form method=GET>` to the same path is a trap (rebuild short-circuits on unchanged path)
- [Redirect needs SsrMode::Async](redirect-needs-ssrmode-async.md) — headers apply after the first chunk; Async = whole doc, out-of-order would lose the 302
- [Redirect paths must be encoded](redirect-path-must-be-percent-encoded.md) — leptos_axum::redirect `expect`s HeaderValue::from_str → remotely triggerable panic without `urlencoding::encode`
- [No-JS journeys must click](no-js-journeys-must-click.md) — the "inert `<template>` fragments" excuse is stale (authed routes are Async); source-substring assertions don't prove the contract
- [Directory tree editor](directory-tree-editor.md) — positional-path For keys + reactive-by-path reads: content/delete correct but collapse/rename state bleeds (§4); editor re-seeded on every refetch → 412 discards edits + Suspense (not Transition) flashes skeleton on write
- [Seed-once form idiom](seed-once-form-idiom.md) — long-lived form signals above the Transition + `seeded_uid` no-op + success-only refetch memo: the viewer's accepted shape, don't re-flag
- [default-style guard blind spot](default-style-guard-untracked-blindspot.md) — single-reader `DEFAULT_*` rule is `--all`-only over git-tracked files: new untracked consts pass locally, fail CI
