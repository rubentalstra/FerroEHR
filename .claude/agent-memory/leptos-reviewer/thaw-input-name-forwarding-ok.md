---
name: thaw-input-name-forwarding-ok
description: thaw::Input DOES forward an explicit `name` to the real <input>, so ActionForm progressive-enhancement submit works without WASM — confirmed, don't flag
metadata:
  type: project
---

`thaw::Input` renders `name=name` on the underlying `<input>` element
(pinned thaw main `0726a3d`, `thaw/src/input/mod.rs:166`), and
`Field::use_id_and_name` honours an explicit `name` prop first. So a
`<thaw::Input name="username">` inside an `<ActionForm>` submits its field
under progressive enhancement (no WASM) — the no-JS Basic-login path (J6) is
sound as written.

**Rule:** `.claude/rules/leptos-ui.md` §5 (ActionForm field names must match
server-fn arg names and submit without WASM).

**How to apply:** do NOT flag missing name-forwarding on thaw::Input when an
explicit `name` is passed. Still verify the `name` string matches the
`#[server]` fn argument exactly. Related hydration caveat:
[[thaw-field-random-id]].
