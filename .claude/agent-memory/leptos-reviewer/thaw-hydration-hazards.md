---
name: thaw-hydration-hazards
description: Which thaw 0.5 (git main) widgets are hydration-safe vs. emit random ids; verified against vendored thaw source
metadata:
  type: project
---

Verified against the pinned thaw git checkout source (`~/.cargo/git/checkouts/thaw-*`):

- **`thaw::Field` — BANNED here.** `field.rs` does `StoredValue::new(Uuid::new_v4().to_string())`
  and wires its `<label for>` to that per-render UUID → SSR/hydrate id mismatch
  (leptos-ui.md §5/§8). The codebase pattern is a plain `<label r#for="stable-id">`
  + explicit `id=` on `thaw::Input`. Already applied in ehrs.rs / ehr_detail.rs /
  composition.rs.
- **`thaw::Upload` — SAFE.** Its `id` is `#[prop(optional, into)] MaybeProp<String>`;
  with no id passed it renders no id attribute (`id=move || id.get()` → None). No
  auto-generated UUID. Its file-read + trigger wiring is client-only (`Effect::new`,
  `mount_style`), so the SSR `<input type="file">` is deterministic. templates.rs
  usage is clean.

**Why:** hydration determinism (leptos-ui.md §8 — "no random ids that differ between
server pass and client hydration").
**How to apply:** when a review touches a thaw widget that renders a `for=`/`id=`
association, check the widget's source for `Uuid::new_v4`/random id generation before
approving; don't assume all thaw widgets share the Field defect.
