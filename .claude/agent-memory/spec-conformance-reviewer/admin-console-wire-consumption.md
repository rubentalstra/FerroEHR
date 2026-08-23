---
name: admin-console-wire-consumption
description: Confirmed defect patterns when auditing app/ferroehr-admin-ui as an ITS-REST client (AQL escaping, datetime-local UTC, 204 branches, strict DTOs, misattributed OAS-vs-docs-text citations)
metadata:
  type: project
---

Confirmed 2026-08-23 first-hand (issue #319 close-out audit); re-verify before citing.

- **AQL string escaping is BACKSLASH, never SQL doubling.** `AqlLexer.g4`
  `ESCAPE_SEQ: '\\' ['"?abfnrtv\\]` and `STRING: '\'' (ESCAPE_SEQ|…|~('\\'|'\''))* '\''`
  — `''` closes and reopens the literal. `openehr_query::printer::escape_string`
  is the correct in-repo escaper (`\'`). The console's `pages/system.rs`
  `template_usage` hand-rolls `.replace('\'', "''")` — the ONLY interpolation
  site; everything else binds `query_parameters` (`composition_filter.rs` is
  the model: compile-time fragments + bindings + `*`/`?`/`\` escaping per
  QUERY master03-syntax.adoc line 332).
- **`CdrResponse` exposes only status/content_type/body** (`cdr.rs`) — no
  header map, so `ETag`, `Location`, `Last-Modified`, `Preference-Applied`
  are unreadable by construction. Every `If-Match` is rebuilt from the
  served body's `uid.value` instead. Works, but any finding about header
  round-trip must start here.
- **Two CDR error-body shapes**: `{message, validationErrors[]}` (the released
  `schemas/others/Error.yaml`, only for `ApiError::ValidationFailed`) and
  `{error, message}` for everything else (`ferroehr-rest/src/overview/error.rs`
  `ErrorBody`). `Requests_and_responses.md` §HTTP status codes shows a THIRD
  shape (`{message, code, errors[DV_CODED_TEXT]}`) — unadjudicated. The console's
  `cdr.rs::diagnostic_of` reads `message`/`error` only → per-path
  `validationErrors` are dropped on every 422.
- **Citation trap: OAS artifacts vs docs text.** "The format is always an
  `version_uid` identifier enclosed by double quotes" is
  `parameters/header/If-Match.yaml`, NOT `Requests_and_responses.md`
  §If-Match. "returned when `If-Match` … doesn't match the latest version" is
  `responses/412_*.yaml`. FLAT/STRUCTURED MIME types are
  `docs/simplified_formats/master02-overview.adoc` §MIME Types (NOT master05);
  `application/openehr.wt+json` is `docs/overview/Resources.md` §Data
  representation. Console files misattribute all four.
- **`composition_get` declares `204` (`204_deleted_at_time`); `ehr_status_get_at_time`
  does NOT.** The CDR returns 204 for a deleted composition version
  (`api/ehr/composition.rs` `if body.is_null()`); the console's
  `fetch_composition` doesn't distinguish it → blank pane.
- **`CompositionDeleteParams` has no `if_match` field** — the CDR never
  evaluates `If-Match` on `composition_delete` (the OAS declares none, matching
  `person_delete`). The console sends one anyway, equal to the path segment,
  so it is inert.
- **Stored-query POST body**: `schemas/query/Query.yaml` marks
  `offset`/`fetch`/`query_parameters` `required`, but the docs text
  (`docs/query/Request.md` §Common Headers) gives all three defaults — already
  adjudicated docs-text-wins in the generated `Query` DTO (all `Option`), so a
  `{query_parameters}`-only body is conformant. Do not re-report.
- `Options.yaml` (System API manifest) marks EVERY property optional; the
  console deserializes into a struct with no container `#[serde(default)]` →
  a spec-legal manifest omitting one field is an Internal error.
- The typed-status guard (`scripts/checks/typed-status.sh`) greps only
  `[=!]=` forms — it does NOT see `match response.status { 204 => … }` or
  `AdminUiError::Cdr { status: 409, .. }` patterns, of which the console has ~40.
