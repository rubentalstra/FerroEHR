---
name: directory-tree-editor
description: The directory FOLDER tree editor design (pages/ehr_detail/directory) — positional-path For keys + reactive-by-path reads; content correct but UI-state bleeds; editor re-seeded on every refetch
metadata:
  type: project
---

`pages/ehr_detail/directory/` (tree.rs + edit.rs + panels.rs + mod.rs), branch
`claude/testperf-framework-rewrite` HEAD d048ef3, reviewed 2026-07-18.

Design (one `RwSignal<serde_json::Value>` working tree, no per-row local state):
- `<For>` over folders/items is keyed by the **positional path** (`key_of` →
  `"0/1"`, items `"0#1"`) — i.e. by index, not a stable logical id. FOLDER
  children carry no uid, so no data-stable key exists.
- Every rendered datum reads the tree reactively by that path
  (`ed.tree.with(|t| node_name(t,&path))`), and every mutation goes through
  pure `edit::*` helpers (unit-tested — rules §10 satisfied).
- Because display AND the captured mutation index are BOTH positional,
  **content and delete-targeting stay correct on reorder/delete** (row at key
  "0" always shows and deletes the folder currently at index 0). Verified by
  hand — no wrong-delete bug.
- BUT collapse/expand (`collapsed: HashSet<String>`) and in-progress rename
  (`renaming`) are keyed by the same positional path → on delete/reorder this
  UI state **bleeds to the sibling that shifts into that position** (delete a
  folder → a sibling randomly appears collapsed). The §4 index-key symptom,
  bounded to ancillary view state. should-fix.

Structural gotcha (root cause of several findings): the tree editor
(`tree_editor`) is built INSIDE the main directory `<Suspense>`'s `Suspend`
closure. It creates only signals (no Resources — the picker Resource is created
outside and passed in, so rules §4 resource-in-Suspend is NOT violated). But
the directory Resource source includes `write_version` (create/update/delete/
restore `.version()`), so **every write refetches → Suspend re-runs →
tree_editor re-seeds a fresh working tree from the server body**. Consequences:
- Verified in reactive_graph-0.2.14 action.rs:290 that `Action::version`
  increments on EVERY completion incl. `Err`, and Resource value updates with
  no equality guard → a **412 discards the user's unsaved edits** (re-seed to
  server state) while the toast says "reload and try again". Advertised "412
  recovery" is broken. should-fix.
- Every successful save also resets collapse/advanced-JSON/rename UI state.
- Main content uses `<Suspense fallback=table_skeleton>` not `<Transition>`, so
  each write refetch flashes the skeleton (§6). The four panels correctly use
  `<Transition>`.

Confirmed-good (don't re-flag): all 9 `#[server]` fns call
`crate::session::require_session().await?` before any CDR call; no usize/isize
in server-fn args/returns or serialized types (DirectoryVersion uses i32); no
authored JS / onxxx strings; icondata_lu icons only (the `·`/`…` in strings are
established punctuation across the whole viewer — app.rs/shell.rs/dashboard.rs
etc. — NOT icon-substitute glyphs, do not flag); `<textarea>` has child text +
prop:value; every section `.into_any()`-erased; no thaw::Field (plain `<label
r#for>` + explicit id); ul/li trees (no invalid tables); error arms render
`inline_error` explicitly (no ErrorBoundary-in-Suspense).
