# UI-2 — Admin console: design overhaul + 100% feature completeness

Working plan for the UI-2 worklist row (owner directive 2026-07-17: a real
design, and no missing features). Deleted in the PR that lands it; the
durable record is `docs/PROGRESS.md` + `CHANGELOG.md` + the book chapter.
No openEHR spec governs an admin UI — our own design/extension; the wire it
consumes stays ITS-REST-bound. Rules: `.claude/rules/leptos-ui.md` (all
mandates unchanged: Rust-only, REST-only boundary, server-fn auth,
`.into_any()` sections, E2E as the merge gate).

Grounding: the 2026-07-18 screen survey — zero design-token layer (3-line
Tailwind entry, stock thaw theme), page skeletons + tables + inputs
hand-duplicated ~6×, and the feature absences listed in part B.

## A0. Broken-layout defects (seen in the 2026-07-18 captures — fix first)

- **Grid children float/overlap**: builder + template-detail cards sit at
  odd vertical offsets (grid cells centering instead of `items-start`/
  stretch) and overlap adjacent rows; the dashboard chart card renders
  narrow and centered. Every grid gets explicit alignment + spans.
- **Unstyled flash of the thaw chrome**: the queries capture shows the nav
  as one unstyled inline run ("DashboardTemplatesQueries…") — thaw injects
  its CSS at runtime, so pre-hydration paint (and any no-JS view) collapses.
  Layout-critical chrome (shell, nav, topbar, footer, cards) moves to
  static Tailwind classes; thaw stays for interactive widgets only.
- **Bare "Loading..." text** where a skeleton belongs (dashboard chart).
- **Raw AQL column indexes (`#0`–`#3`) as table headers** on the EHR
  detail compositions tab — name the columns (Composition, Name,
  Template, Time); the audit card on the composition viewer renders
  narrow/centered instead of full-width.

## A. The design system (before any screen work)

**The look (binding direction):** a calm, clinical, information-dense
operator console — closer to a modern observability tool than a marketing
site. Deep **teal** as the single brand accent (`#0d9488` light /
`#2dd4bf` dark) used sparingly: primary buttons, active nav, focus rings,
links, chart strokes. Everything else lives on a **slate** neutral ramp.
Surfaces are layered (page < card < raised) with hairline borders and a
single soft shadow level — no heavy drop shadows, no gradients. Type:
system UI stack; a clear scale (12/14/16/20/28) with `tabular-nums` for
every metric and `font-mono` for ids/paths/AQL. Radius 8px cards / 6px
controls. Data first: tables and trees get the space; chrome stays quiet.

1. **Design tokens** — Tailwind v4 `@theme` block in `style/tailwind.css`:
   the palette above as semantic tokens (`--color-surface`,
   `--color-surface-raised`, `--color-edge`, `--color-ink` /
   `--color-ink-muted`, `--color-accent` + hover/subtle variants,
   success/warn/danger), radius + spacing + shadow scale. Dark mode = the
   same semantic tokens redefined under `.dark`, so pages stop
   hand-writing `dark:` per element and both themes stay in lockstep.
2. **thaw theme** — customize `thaw::Theme` (brand color into the
   light/dark themes) so thaw widgets and Tailwind utilities share one
   palette; keep the fixed `theme_id`.
3. **Shared component kit** (`src/components/`): `PageHeader` (title,
   description, breadcrumbs, action slot), `DataTable` (styled table +
   sticky header + pagination footer with page size/total), `field`
   inputs (text/select/textarea — one styled definition replacing the
   repeated raw markup), `StatCard` (icon, delta accent), `EmptyState`
   (icon + hint + action), `Toolbar`, toast feedback via `thaw::Toaster`
   (mounted once in the shell). Icons: `icondata` 0.7.0 + `leptos_icons`
   0.7.1 SVG icons (pure Rust; live-verified on crates.io 2026-07-18).
4. **Shell** — wordmark/logo (inline SVG mark), nav items with icons +
   section labels, clear active state, refined topbar (health pill →
   proper status chip), consistent footer.
5. **Every screen restyled** onto the kit: login (branded, centered card
   with the mark), dashboard (accented stat cards, trend deltas, fuller
   chart), system, templates + detail, EHRs + detail, composition viewer,
   queries hub, builder, raw AQL. Consistent page rhythm: header →
   toolbar → content cards.

## B. Feature completeness

Items 11–21 are recovered from the original console design document
(deleted at the ADMIN-UI close; retrieve with
`git show 56166c67a^:docs/design/ehrbase-admin-ui.md` — its §7.1 feature
map and §7A screen catalog remain the binding wireframe reference for
this stream) — designed, promised, and never built.

| # | Feature | Notes |
|---|---------|-------|
| 1 | Query result **export** (CSV + JSON) | Design §7A.6 `export_csv`/`export_json`: BFF axum download route (Content-Disposition; anchors `rel="external"` — works without WASM); server-side RESULT_SET serialization; on the builder AND the raw-AQL results panes. |
| 2 | **EHR creation** | POST /ehr (optional subject id/namespace + queryable/modifiable) from the EHRs screen; toast + navigate to the new EHR. |
| 3 | **Composition commit + update** | Paste/upload canonical JSON/XML/FLAT against a template; POST, and PUT with If-Match from the viewer (new version); CDR validation errors surfaced verbatim. |
| 4 | **Template deletion** (admin API, when enabled) | Two-step confirm; hidden when the CDR's admin API is off. |
| 5 | **Stored-query delete** (admin API, when enabled) | Same gating. |
| 6 | **Tab state in URL** (`?tab=`) on EHR detail + template detail | Shareable/refresh-safe (rules §9). |
| 7 | **Pagination upgrade** | Page size picker + totals where available; unify on the `DataTable` footer; keep URL-driven offsets. |
| 8 | **Toasts** for every mutation | Upload, save, delete, commit, create. |
| 9 | **Breadcrumbs** on nested routes | Templates/:id, EHRs/:id, compositions/:uid. |
| 10 | **Keyboard affordances** | Focus ring discipline; Enter-to-run in the raw-AQL editor (Rust `on:` listeners only). |
| 11 | **Cohort / EHR queries** (design §7.1) | The builder gains an "EHRs" result shape ("which EHRs match these conditions" — SELECT e/ehr_id/value with the same criteria tree) plus a per-EHR boolean probe ("does THIS EHR match?") on the EHR detail screen. |
| 12 | **Chart rendering of grouped results** (design §7.2 step 6) | Data-values results groupable by path → `leptos-chartistry` series view next to the table (table \| chart toggle on the results pane). |
| 13 | **Find EHR by subject id** (design §7A.7) | GET /ehr?subject_id&subject_namespace on the finder (today: ehr_id only). |
| 14 | **Version time-travel** (design §7A.9 `fetch_at_version`) | `version_at_time` picker (⏱) next to the version dropdown; version timeline strip (v1 ── v2 current) instead of the bare `<select>`. |
| 15 | **Contributions listing** (design §7A.8) | List the EHR's contributions as a table (via versioned-object/AQL listing), not just a by-uid lookup box. |
| 16 | **Open a stored query in the builder** (design §7A.5 "loaded mode") | Run ▶ from /queries lands in the builder/raw editor with the query loaded and the result pane active (builder-state lift only where the AQL parses back; otherwise raw editor). |
| 17 | **Syntax-highlighted document viewer + copy** (design §7A.9) | Pure-Rust token-class highlighting for JSON/XML in the shared viewer; copy button (`leptos-use` clipboard). No JS highlighter. |
| 18 | **Session scopes / launch-context panel** (design §7.4/§7A.0) | The user-menu drawer shows the session's scope claims + any ehrId/patient launch context — "what can I do right now". |
| 19 | **Scope previewer** (design §7.4/§7A.10, feature-gated) | master08 scope-grammar preview; requires lifting the grammar out of `ehrbase-rest` into a shared spec crate or a CDR debug endpoint — resolve the design point first; ships behind a gate. |
| 20 | **Activity log view** (design §7.1/§7A.10) | The CDR's ATNA system log is emit-only today — needs a small CDR-side read surface (admin API, our own extension; no openEHR spec governs it) before the tile can render events; "endpoint absent" stays a first-class rendered state. |
| 21 | **i18n layer** (design §7A conventions) | Keys-not-literals with a single `en` catalog at v1 — groundwork, not translations. |

### B3. Cabolabs-parity sweep (owner directive 2026-07-18 — every §Appendix-A
"Adopt" feature lands or carries a written adjudication)

| # | Feature (Appendix A source) | Status |
|---|---|--------|
| 22 | **Audit/activity log over REST** (A.4 "Full audit / activity log") | The CDR gains an **admin-only read endpoint** for the ATNA system log (task UI-2e: an optional DB sink for audit events + `GET /admin/system_log` with paging/filtering, admin-authenticated; our own extension — no openEHR spec governs the read surface). **Design constraints (owner 2026-07-18):** it realizes the read side of the SM System Log component (`I_SYSTEM_LOG` is an empty stub; SM master02 names it "IHE ATNA-compliant") under the `system_log` name; the payload is the **standard DICOM PS3.15 audit-message model we already emit** — never bespoke JSON — so a future openEHR-specified surface migrates at the route/DTO skin only; house REST style identical to the existing `/admin` group (utoipa path, typed errors → openEHR error body, standard paging, admin auth, endpoint-map row, book page, same-PR changelog). |
| 23 | **Directory editing + folder templates** (A.1 "Directory / folders + folder templates") | Console gains directory create/edit (PUT /ehr/{id}/directory — spec-standard) and console-local folder templates (named FOLDER-tree shapes applied on create; our own extension). |
| 24 | **Usage statistics** (A.4 "Dashboard + usage stats") | Per-template composition counts + repo totals on the dashboard/system panel; CDR-side stats endpoint if the AQL route is too slow (measure first). |
| 25 | **Runtime configuration view** (A.4 "Runtime configuration") | Read-only, redacted effective-config panel; needs a CDR admin config endpoint (our own extension, secrets never serialized). |
| — | Query sharing (A.3 `QueryShare`) | **Adjudicated N/A at Stage 1**: stored queries are already CDR-global and the console has no per-user ownership to share between; revisit with Stage-2 RBAC/multi-user. |
| — | Template activate/deactivate (A.2) | **Adjudicated out** (design §7A.3): ITS-REST has no template state; idea-source only. |
| — | SNOMED-expression criteria (A.3) | **Deferred** until the CDR's AQL `TERMINOLOGY()` family lands (no open row). |
| — | Data-value indexing, OPT storage backends, sync/VNA, notifications, commit-log repos, multitenancy/billing (A.1/A.2/A.4) | **CDR-side or Stage-2** per the design §7.3 — not console work. |

Coverage cross-check (Adopt items already landed or in flight): versioned
compositions browse/audit ✓ · EHR list/show/create (create in flight) ·
directory browse ✓ · composition viewer ✓ · template manager
upload/list/inspect ✓ · builder + typed criteria + AND/OR + shapes ✓ ·
stored queries save/run ✓ + export/loaded-mode (in flight) + delete
(task UI-2e) · query groups ✓ · cohort/EHR queries (B11) · grouping→chart
(B12) · raw AQL ✓ · XML+JSON ✓ · dashboard ✓.

Deliberately out of scope (unchanged from the original design §7.3):
GROUP BY/aggregates beyond COUNT, proportion denominator, islands mode,
multitenancy/accounts UI.

## B2. Image-based E2E (owner directive 2026-07-18 — "the true experience")

The battery must also run against the **composed console image**, exactly
as the CDR's conformance suite tests the composed CDR image:
`scripts/ui-e2e.sh` gains `UI_E2E_IMAGE=1` (compose up the existing
`ehrbase-admin-ui` service instead of host-building; same journeys, same
docs-shots), and CI gains an image-mode battery job so every merge
verifies the shipped artifact, not just the host build.

## C. Delivery

- Branch `claude/admin-ui-design-overhaul`; **A lands first** (tokens +
  kit + shell + all screens restyled), then B features ride the kit.
- Orchestrator holds the design tokens/theme/kit + review; bounded screen
  conversions and features fan out to `ui-implementer` (max 2 concurrent),
  `leptos-reviewer` gates each slice.
- Every slice: `/ui-gates` green (both-target clippy, nextest, leptosfmt,
  cargo-leptos build); E2E battery + docs-shots re-run at convergence; all
  book screenshots re-captured (`ui-screenshot-guard`); book pages updated
  for the new features; changelog entry; ECC zero-drift at close.
