# Memory index — leptos-reviewer

- [thaw::Field random id](thaw-field-random-id.md) — Field mints Uuid::new_v4() id; §8 hydration hazard unless an explicit stable id is passed
- [thaw::Input name forwarding OK](thaw-input-name-forwarding-ok.md) — Input forwards explicit `name` to the real <input>; no-JS ActionForm submit works, don't flag
- [Polled resource needs Transition](polled-resource-needs-transition.md) — interval-refetched resource under <Suspense> flashes fallback; §6 wants <Transition>
- [Internal nav uses plain <a>](internal-nav-uses-plain-anchor.md) — W2 uses <a href>/thaw NavItem, not leptos_router <A>; §9 full-page-reload deviation
- [W2 confirmed-good patterns](w2-confirmed-good-patterns.md) — auth guards, .into_any() erasure, theme Effect, no-usize/unwrap — verified correct, don't re-flag
# Leptos reviewer memory index

- [thaw hydration hazards](thaw-hydration-hazards.md) — thaw::Field emits a random UUID id (banned); thaw::Upload id is an optional prop (safe); verify widget source before approving `for=`/`id=`
- [Tabbed screen pattern](tabbed-screen-pattern.md) — always-mounted bodies + class:hidden + tab-gated resource sources (ehr_detail exemplar); template_detail diverged (eager fetch)
