# Admin console — audited remaining scope (tracker issue #152)

- Status: in-progress
- Started: 2026-07-20 (supersedes the 2026-07-18 plan after a full two-track
  code audit at develop HEAD `651ff8d5d`+)
- Consumes: `.claude/rules/leptos-ui.md` (all mandates unchanged: Rust-only,
  REST-only boundary, server-fn auth, e2e as the merge gate). No openEHR
  spec governs an admin UI — our own design/extension; the wire it consumes
  stays ITS-REST-bound.

Deleted in the PR that closes #152. The prior plan generations and the two
design studies (the admin-UI feature catalog + wireframes, and the
query-builder UX study) were deleted with this consolidation and are
retrievable from git history; everything from them that remains unbuilt is
itemized below.

## Audit baseline (2026-07-20): what is VERIFIED SHIPPED

Do not re-plan these — the audit confirmed each in code:

- **Design system**: semantic Tailwind-v4 tokens
  (surface/raised/sunken/edge/ink/accent), class-driven dark mode with
  thaw theme + tokens switched together (no OS-media divergence), the
  component kit (PageHeader with breadcrumbs, table_shell, StatCard,
  EmptyState, toast helpers), static-Tailwind shell chrome, all eleven
  screens restyled. Token discipline verified: zero raw palette utilities,
  zero inline styles (one exception, T16 below).
- **Features**: CSV+JSON export (BFF `POST /export/aql`, no-WASM form
  POST); EHR creation (plain + subject-bound); composition commit
  (JSON/XML/FLAT with template-id header) and update (If-Match new
  version, 412 handled); builder ORDER BY, column/alias picker, cohort
  (`Ehrs`) shape, LIMIT, live AQL preview; tab-state-in-URL on both detail
  screens; success toasts on every mutation; subject-id EHR lookup;
  version time-travel (composition + directory); contributions tab
  (paged list + by-uid lookup); scopes drawer; ITI-81 audit browser with
  first-class disabled/empty states; full directory experience (tree
  editor, history/restore, folder templates); runtime-config +
  repository-usage panels; full-stack OIDC; image-mode e2e
  (`UI_E2E_IMAGE=1`).
- **Rule compliance**: REST-only boundary (deps: `openehr-query`, `openehr-its` — no app crates); every CDR-touching
  `#[server]` fn session-guards first (sampled exhaustively); `/login`
  no-JS via `SsrMode::Async` with written rationale.

## Remaining scope (every item audit-verified, with location)

### T1–T2 correctness / rule violations

- [ ] **T1** Directory tree `<For>` keyed by positional index
  (`src/pages/ehr_detail/directory/tree.rs:547`,
  `format!("{parent}#{idx}")`) — sibling delete shifts identity (the code
  comment at `tree.rs:460` admits it). Key by stable node identity
  (object_id / archetype path), not folder-relative position.
- [ ] **T2** Template-id link segment interpolated without
  percent-encoding (`src/pages/templates.rs:338` TODO). Fix now with
  `urlencoding` (owner hard rule: never hand-roll or defer percent
  codecs), delete the TODO.

### T3–T13 missing features

- [ ] **T3** Template delete in the console — CDR
  `DELETE /admin/template/{id}` exists (409-with-count on referenced
  templates); two-step confirm; hidden when the CDR admin API is off
  (probe like the sibling admin surfaces).
- [ ] **T4** Stored-query delete in the console — CDR
  `DELETE /admin/query/{name}/{version}` exists; same admin gating. Note:
  the current `/queries` Delete button (`queries.rs:487`) deletes a
  console-local *group*, not a stored query — label the two clearly.
- [ ] **T5** Stored-query loaded-mode into the point-and-click builder:
  `?load=name@version` exists only on the raw editor
  (`query_aql.rs:51-82`). Add an AQL→`BuilderQuery` reverse-lift for the
  subset the builder can express (template-first FROM/CONTAINS chain,
  typed WHERE tree, columns/order/limit); queries outside the subset keep
  the raw-editor fallback with a visible "opened in raw mode" note.
- [ ] **T6** Grouped-result charting: group data-value results by path →
  multi-series `leptos-chartistry` view with a series legend (today:
  single series over the first mostly-numeric column,
  `query_builder.rs:1593-1629`). Table|Chart toggle stays.
- [ ] **T7** Syntax-highlighted document viewer + copy button: the shared
  `DocumentPane` (`components/format_view.rs`) is a plain `<pre>`. Add a
  pure-Rust token-class highlighter for JSON/XML (no JS highlighter — rule
  file) + a copy affordance.
- [ ] **T8** Pagination on the three unpaged tables — templates
  (`templates.rs:286-332`), stored queries (`queries.rs:141-162`), query
  groups (`queries.rs:422`) — unified on the table_shell footer idiom the
  other tables use.
- [ ] **T9** Scope previewer (feature-gated): resolve the design point
  first — lift the master08 scope grammar out of `ferroehr-rest` into a
  shared spec-side crate, or add a CDR debug endpoint; then a "what would
  this scope grant" panel in the scopes drawer. If the design point
  resolves against (grammar lift rejected + no debug endpoint), adjudicate
  out with the reasoning recorded on #152.
- [ ] **T10** i18n groundwork: keys-not-literals with a single `en`
  catalog (no translations yet). All strings are inline English today
  (`<html lang="en">` fixed, `app.rs:18`).
- [x] **T11** System-page activity-log placeholder
  (`system.rs:571-589` + its TODO): ITI-81 landed and `/audit` is the
  read surface — fold the card into a link/summary onto `/audit` or drop
  it; delete the stale TODO either way.
- [x] **T12** EHR finder no-JS fallback (`ehrs.rs:510` TODO): a GET
  `<Form>` to a redirect route so find-by-id works pre-WASM. Landed as a
  PLAIN `<form method="GET" action="/ehrs">` + `?find=` → `<Redirect>` on the
  same screen: the router's `<Form method="GET">` would navigate to the same
  path, and a same-path navigation only updates the search query without
  re-running the route component, so the hydrated lookup would no-op.
- [x] **T13** Error-feedback pattern unified: most CDR failures render
  inline only (toast_error is wired solely on group actions). Rule:
  validation/document errors stay inline verbatim; action failures
  (delete/save/commit transport errors) also toast. Apply everywhere.

### T14–T18 design polish

- [x] **T14** `scope="col"` on the shared table_shell `<th>` (already implemented — src/components/data_table.rs; ticked 2026-08-22 when the #308 build found the stale box)
  (`components/data_table.rs:30`) — one edit fixes every table.
- [ ] **T15** `aria-label` on the four icon-only remove buttons
  (`query_builder.rs:503/616/1254/1323`).
- [ ] **T16** Tokenize the one raw color: `bg-black/40` modal scrim
  (`directory/tree.rs:801`) → an overlay/scrim token.
- [ ] **T17** EmptyState kit on the ~11 hand-rolled bare-text voids:
  `query_builder.rs:1672`, `compositions.rs:317`, `tree.rs:871`,
  `templates.rs:329`, `ehrs.rs:577`, `system.rs:538`, `dashboard.rs:440`,
  `composition.rs:629/877`, `queries.rs:359`, `directory/panels.rs:186`.
  (Genuinely inline hints may stay text — judge per site.)
- [ ] **T18** Shared skeleton component replacing bare "Loading…" text at
  8 sites (`query_builder.rs:185`, `queries.rs:215/343/406`,
  `query_aql.rs:149`, `tree.rs:786`, `panels.rs:179/352/427`) — the
  dashboard's `StatTileSkeletons` (`dashboard.rs:250`) is the pattern.
- [ ] **T18b** Residual thaw in layout chrome: the user-menu
  `thaw::Popover` (`shell.rs:320`) is the remaining pre-hydration-flash
  surface — convert to a static disclosure if flash is observed in
  captures; otherwise record the adjudication.

### T19–T20 e2e coverage

- [ ] **T19** Assertive journeys for the shipped-but-untested features:
  export (CSV/JSON download), composition commit + update, stored-query
  save + groups CRUD, the time-travel picker, the contributions tab, EHR
  creation as a first-class assertion, find-by-subject, the scopes
  drawer. (Today these are only screenshot-captured or used as setup.)
- [ ] **T20** Dark-mode capture coverage beyond the single dashboard
  shot; reduce the seed-conditional silent SKIPs in docs-shots where the
  battery env can guarantee seeds.

### T21–T23 CDR-side consistency follow-ups (carried from the prior plan)

- [ ] **T21** `delete_opt` (SM-UUID path): the friendly 409-with-count
  refusal instead of leaking the raw FK constraint string.
- [ ] **T22** Contribution list `change_type`: decide rubric-beside-code
  (`creation` + `249`) presentation, record the decision.
- [ ] **T23** ADL1.4 template list: verify the OAS `filter_version` query
  parameter is honored; fix or adjudicate with citation.

### T24 close-out

- [ ] **T24** Spec-compliance audit of every console-touched surface
  (openEHR ITS-REST/RM + IHE ITI-81 where relevant) — the final phase
  before close.

## Exit criteria

- [ ] Every T-item above checked (implemented, or adjudicated on #152
  with reasoning)
- [ ] `/ui-gates` green (both-target clippy, nextest, leptosfmt,
  cargo-leptos build); full e2e battery + docs-shots green
- [ ] Book pages + screenshots updated (`ui-screenshot-guard`); changelog
  entry for user-visible console changes
- [ ] ECC zero-drift run at close
- [ ] This file deleted in the closing PR

## Delivery

Branch `feat/admin-ui-completion` off develop. Orchestrator holds the
builder reverse-lift (T5) design + review; bounded slices fan out to
`ui-implementer` (max 2 concurrent), `leptos-reviewer` gates each slice.
Suggested slicing: [T1,T2,T14–T16] (correctness+a11y quick sweep) ·
[T3,T4,T8,T13] (admin actions + tables) · [T5] · [T6,T7] · [T9–T12] ·
[T17,T18] · [T19,T20] · [T21–T23] · [T24 + close].
