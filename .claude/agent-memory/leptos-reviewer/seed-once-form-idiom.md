---
name: seed-once-form-idiom
description: The console's accepted edit-form shape — long-lived signals above the Transition, seeded_uid idempotent re-seed, success-only refetch memo, controlled textarea with prop:value + get_untracked child text
metadata:
  type: project
---

Exemplar: `pages/ehr_detail/status/{mod.rs:549, edit.rs:62-139}`. Replicated
verbatim by the demographics party/relationship edit cards. **Do not re-flag
any of these:**

- Form state is a `Copy` struct of `RwSignal`s created in the SETUP fn, above
  the `<Transition>`, so a Suspend re-run cannot re-create it (leptos-ui.md §4).
- `seed(form, &state)` returns early while `form.seeded_uid == state.version_uid`
  — idempotent per loaded version, so a re-run for the same version never
  overwrites edits in progress. Writing those signals from inside the `Suspend`
  is therefore accepted here.
- `let saved = Memo::new(|prev: Option<&usize>| …)` holds the previous
  `Action::version()` unless `value()` is `Some(Ok(_))`, and the resource source
  carries `saved` — so only a SUCCESSFUL write refetches (`Action::version`
  increments on `Err` too). `usize` here is local only, never serialized.
- Controlled textarea = `prop:value=move || sig.get()` + `on:input:target` +
  `{sig.get_untracked()}` as child text. The child expression is evaluated at
  view CONSTRUCTION (before the Suspend runs), so it is the same empty string
  on the server pass and at hydration — no mismatch (leptos-ui.md §5/§8).
- `RwSignal::new(pretty)` created inside a `Suspend` to feed `DocumentPane` is
  the exemplar's own shape (`status/mod.rs::document_section`) — a signal, not
  a Resource, so §4 is not violated.

Standing UX caveat (affects the exemplar too, so it is not a new-code finding):
between hydration and the first seed the textarea is empty and editable, and
the seed then overwrites whatever was typed. The E2E suites work around it with
a "wait until `prop("value")` is non-empty" helper rather than fixing the UI;
the real fix is disabling the card until `seeded_uid` is `Some`.
