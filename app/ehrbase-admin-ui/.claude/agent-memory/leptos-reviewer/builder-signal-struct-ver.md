---
name: builder-signal-struct-ver
description: The query-builder's single-RwSignal<BuilderQuery> + struct_ver gating pattern for focus-preserving deep-tree editors
metadata:
  type: project
---

`pages/query_builder.rs` (branch claude/admin-ui) is the validated exemplar for
a deeply-nested editable tree that must NOT destroy input focus on edit:

- **One `RwSignal<BuilderQuery>`** holds all editable state; a separate
  `struct_ver: RwSignal<u32>` is bumped ONLY on structural edits
  (add/remove/regroup/toggle/shape). The tree/output render closures read
  `struct_ver.get()` (tracked) then `query.get_untracked()` — so a text-field
  keystroke (`query.update` with NO bump) does not re-render the tree, and the
  `<input>` keeps focus.
- The live AQL preview is a `Memo` reading `query.with(to_aql)` TRACKED, so it
  (and each leaf's readable "sentence" text node) updates on every keystroke
  while the editor DOM is untouched.
- Leaf editors seed fresh local `RwSignal`s from the snapshot on each
  structural re-render and write the rebuilt `CriterionKind` back via
  `query.update(|q| set_leaf_kind(...))`. No `Effect` anywhere — all writes are
  `on:input`/`on:click` listeners, so there are zero signal-writes-signal
  effects (leptos-ui.md §2).
- Tree mutation is pure `Vec<usize>`-path helpers (`node_at_mut`, `add_leaf_at`,
  `remove_at`, …), all unit-tested out of components (leptos-ui.md §10). Radio
  `name`/input `id`s use `path_key(&[usize])` — deterministic, not random.

**Why:** the standard "each row is an ArcRwSignal in a `<For>`" pattern doesn't
fit an n-ary AND/OR tree; this design threads one Copy `BuilderCtx` bundle
instead and gates re-render explicitly.
**How to apply:** when reviewing a similar tree editor, confirm the bump signal
is bumped ONLY on structural change (never on text edit), render closures read
the query UNTRACKED, and the tracked subscribers are just the preview/sentence
text nodes — that combination is what preserves focus. No stale-index risk
because every mutation bumps → full rebuild with fresh paths.
