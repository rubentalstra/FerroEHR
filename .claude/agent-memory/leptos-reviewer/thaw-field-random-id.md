---
name: thaw-field-random-id
description: thaw::Field mints a Uuid::new_v4() id at setup — a §8 hydration hazard whenever a Field/Input is used without an explicit stable id
metadata:
  type: project
---

`thaw::Field` generates its label/input id with `Uuid::new_v4()` at component
setup (pinned thaw main `0726a3d`, `thaw/src/field/field.rs:26`). It renders
that uuid onto the label as a **static** `attr:for` and provides it (via
`FieldInjection`) to the child `thaw::Input`, which renders it as a
**reactive** `id=Signal`. So SSR emits uuid X, hydration mints uuid Y; the
static label `for` keeps X while the reactive input `id` can become Y →
label↔input association breaks post-hydration and it can trip a hydration
attribute-mismatch browser-console warning (the J2 E2E gate fails on any console
error).

**Rule:** `.claude/rules/leptos-ui.md` §8 — "No non-determinism in initial
render (random ids …) that differs between the server pass and client
hydration."

**How to apply:** flag every `<thaw::Field>` / `<thaw::Input>` that does NOT
pass an explicit stable `id` (and `name`). `Field::use_id_and_name` returns an
explicit `id` prop before falling back to the uuid, so passing
`id="login-username"` etc. is the fix. Confirmed in W2 `login.rs`
`basic_login_form`. See [[thaw-input-name-forwarding-ok]].
