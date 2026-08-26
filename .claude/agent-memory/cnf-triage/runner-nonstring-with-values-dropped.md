---
name: runner-nonstring-with-values-dropped
description: HISTORICAL (fixed) — numeric/bool `with:` values used to be dropped from URL/header templates; merge_with_vars now promotes every scalar
metadata:
  type: project
---

**STATUS: FIXED — verified 2026-07-28.** `merge_with_vars` now matches
`Value::String | Value::Number | Value::Bool` and promotes each as wire text
(the in-code comment cites this very triage: "a number-typed `url_fetch: 4`
must reach a `${url_fetch?}` URL slot"). Objects/arrays still stay out, which
is correct — they have no scalar wire text. Keep this entry only as the shape
to re-check after driver refactors; do NOT attribute a live row to it without
re-reading the function.

Historical description follows.

`HttpDriver::merge_with_vars` (Veredictum's `src/exec/driver.rs`) promotes a
step's `with:` entries into the template VarStore **only when the JSON value is a
`Value::String`**. A numeric/bool `with:` value (e.g. `url_fetch: 4`) is therefore
unbound when `build_url`/`build_headers` render `"${url_fetch?}"`; the ref is
optional, so the parameter is **silently omitted** from the wire — no error, no
inconclusive row. The structured-BODY path does not have this bug
(`select_body` → `RequestBody::Structured` promotes non-strings as
`Captured::Body`), so body forms of the same value work — the asymmetry is the
tell.

Why most numeric query params still work: `build_url` has a second loop that
backfills `with.get(<query-param-name>)` (stringifying any JSON scalar). So a case
binding `fetch: 100` reaches `?fetch=100` **by name coincidence**, while a case
binding `url_fetch: 4` for a query slot named `fetch` sends nothing.

**Why:** first confirmed 2026-07-27 triaging
`I_QUERY_SERVICE.execute_ad_hoc_query-post_fetch_url_only` (row count 10 != 4) —
attribution: runner machinery, not the app; the app's
`merge_body_and_url_i64` (`app/ferroehr-rest/src/api/query/response.rs`) reads
`?fetch=` on both POST arms correctly.

**How to apply:** on any red row where the SUT "ignored" a URL/header parameter,
first check whether the case's `with:` key name differs from the binding's
parameter name AND the value is non-string — that combination means the parameter
never left the runner. Same fault silently hollows
`execute_stored_query-protocol_fetch_reserved` (its URL `fetch=5` is never sent;
the case passes on the body half alone). Related:
[[runner-driver-gaps]].
