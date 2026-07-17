# `ehrbase-admin-ui` — a pure-Rust admin console over the ITS-REST API

- **Status:** design + **build plan** — framework + ecosystem selection,
  architecture, feature map, **screen catalog (§7A)**, **E2E validation
  matrix (§8d)**, and the **orchestrated build plan (§12)**. This file is the
  governing plan for the `ADMIN-UI` row in `docs/plans/WORKLIST.md` and is
  **deleted in the PR that implements it** (owner lifecycle rule 2026-07-17 —
  the ADR layer is abolished; the durable record is `docs/PROGRESS.md`,
  `CHANGELOG.md`, and `docs/architecture.md`).
- **Date:** 2026-07-13 · revised 2026-07-17 · **audited + extended
  2026-07-17b** (all versions re-verified live against crates.io + GitHub;
  stale references fixed; owner decisions 5–8 recorded in §10)
- **Prior art:** [Cabolabs EHRServer](https://github.com/ppazos/cabolabs-ehrserver)
  — the owner cited its **Template Manager**, its point-and-click **Query
  Builder** ("query creation in seconds, no programming needed"), and its
  **XML + JSON** support as the feature target. **The full source was read
  2026-07-13** (Grails/Groovy; controllers, domain model, services, views) —
  the complete feature inventory is **Appendix A**, and §7 maps each feature to
  *adopt / adapt / defer* for our stack.
- **Pairs with:** `docs/architecture.md` (workspace layout, the ITS-REST base
  path) and the live deployment artifacts (`deploy/`, the quickstart compose,
  `.github/workflows/containers.yml`). *(The former
  `docs/design/container-images.md` was deleted in the 2026-07-16 design-doc
  purge — the shipped compose/Helm/CI files are now the packaging reference.)*
- **No openEHR spec governs an admin UI** — this is **our own design /
  product extension**, not a conformance surface. Its one hard spec-facing
  constraint is the boundary in §1.

## Revision 2026-07-17 — platform reality this design now builds on

The CDR moved substantially since 2026-07-13; the deltas below are folded into
the sections that follow (each marked *(rev. 2026-07-17)*):

1. **Simplified Formats are now first-class and spec-exact.** The FLAT /
   STRUCTURED / Web-Template surface was rewritten against the STABLE ITS-REST
   Simplified Formats specification: strict `Accept`/`Content-Type`
   negotiation with RFC 9110 q-values (there is **no `?format=` parameter**),
   media types `application/openehr.wt.flat+json`,
   `…wt.structured+json`, and `application/openehr.wt+json` (the Web-Template
   rendering of a template), the `openehr-template-id` request header on
   simplified commits, and `406`/`415` with diagnostics elsewhere. The console
   gains a **FLAT/STRUCTURED view + commit** dimension beyond the original
   JSON ⇄ XML toggle (§5.3, §6, §7).
2. **The `openehr-flat` crate is now a clean, spec-cited workspace citizen**
   (one internal simplified tree; FLAT/STRUCTURED as pure codecs;
   `convert::`/`webtemplate::`/`validation::` module-pathed API, zero
   crate-root re-exports; the vendor-quirks feature flag is gone). It joins
   the §6 domain-reuse table — and its served WebTemplate document replaces
   OPT-walking for the Query Builder's path catalog (§7.2).
3. **The served OpenAPI is the server's own** (owner hard rule,
   2026-07-17): `ehrbase-rest` serves only documents it generates natively
   from its `#[utoipa::path]` handlers — the vendored ITS-REST OAS is codegen
   input + behavioural oracle, never served. The console's endpoint knowledge
   can therefore be verified against the *running server's* `openapi.json`
   (which advertises the simplified media types), and a **served-OpenAPI
   viewer** is a natural system-panel tile (§7.1).
4. **App layout is three crates** (`ehrbase`, `ehrbase-rest`,
   `ehrbase-server`) — the former `ehrbase-sm` trait catalog is deleted; §5.2's
   dependency ban list is updated accordingly.
5. **Reliability rules are machine-enforced workspace-wide**
   (`.claude/rules/reliability.md`): deny-tier lints (no
   unwrap/expect/panic/dead code in production code), `unsafe_code = forbid`,
   release-profile overflow checks, no dedicated test files under `src/`,
   zero re-exports. The console crate inherits all of it via
   `[lints] workspace = true`. Server-side `EhrId`/`VoId` newtypes are
   wire-transparent (bare UUIDs on the wire) — no console impact.
6. **CDR configuration is one TOML file** (`ehrbase.toml`, the 2026-07-15
   configuration redesign) rather than an `EHRBASE_*` env matrix; §8's
   console-config note is aligned.
7. **Navigation instruments that now exist:** `docs/endpoint-map.md` traces
   every endpoint to its SQL (useful when designing BFF calls), and the
   platform passed a full per-folder rewrite (W-14) — the SM component map in
   `docs/architecture.md` is current.

## Revision 2026-07-17b — audit + build plan (this revision)

1. **Every version fact re-verified live** (crates.io API + GitHub releases,
   2026-07-17): leptos 0.8.20, cargo-leptos 0.3.7, leptos-struct-table
   0.19.0, leptos-chartistry 0.2.3 (leptos-use `^0.18` dep confirmed),
   leptos-use 0.19.0, thirtyfour 0.37.2 — all unchanged and current.
   Corrections: **Tailwind standalone is now v4.3.3** (released 2026-07-16);
   **`thaw` has exactly one 0.8-compatible release on crates.io,
   `0.5.0-beta` (2025-05-03)** — there is no `beta.N` series, and stable
   0.4.8 (2025-08-03) remains Leptos-0.7-only; explicit pins added for
   `leptos_axum` 0.8.10 and `leptosfmt` 0.1.33.
2. **Stale references purged:** the deleted `docs/design/its-rest/smart.md`,
   `container-images.md`, "blueprint row" pointers, and all ADR-flow language
   (the ADR layer was abolished 2026-07-17 — this doc is the plan and dies
   with the implementing PR).
3. **§7A screen catalog added** — per-screen routes, wireframes, component
   trees, `#[server]` fn → CDR-endpoint data tables, loading/empty/error
   states, and the E2E journey that proves each screen.
4. **§8d expanded into the E2E validation matrix** — journey → assertions →
   fixtures, the browser-console failure gate, and **step screenshots as CI
   artifacts** (owner decision 2026-07-17; no pixel-diff assertions).
5. **§12 build plan added** — Fable 5 orchestrates, `ui-implementer`
   subagents implement (max 2 concurrent, owner cap), `leptos-reviewer`
   gates; **one big-bang PR** with a single convergence (owner decision
   2026-07-17).
6. **Prior-art discipline (owner, 2026-07-17):** the Cabolabs EHRServer repo
   is an **idea source only — never a 1:1 port**. No code, markup, schema, or
   copy is translated from it; we take feature *concepts* (Appendix A) and
   design every screen and interaction fresh for our stack. The §11 README
   credit records the inspiration honestly.

## Owner constraints (2026-07-13, binding)

1. **Rust only — zero hand-written JavaScript, frontend included.** The
   deciding filter for every tool below (§2 treats the one honest nuance).
2. **Consume the CDR strictly over its ITS-REST API.** Never the database,
   never the in-process service layer. This is what keeps the console honest to
   the openEHR ITS: an admin tool that reached around the REST contract would be
   testing something the spec does not define. See §5 boundary.
3. **XML *and* JSON.** openEHR ITS defines both a canonical JSON and a canonical
   XML representation, and ITS-REST negotiates between them per request — the
   console must speak both (§5.3).

---

## 1. What it is (and is not)

**Is:** a standalone web application — its own binary, its own OCI image — that
an administrator points at a running `ehrbase-rs` (or any ITS-REST-1.0.3 CDR)
and uses to manage templates, build and run AQL visually, browse EHRs and
compositions, and inspect audit/version history. Mobile-friendly, like the
Cabolabs console.

**Is not:** part of the CDR, part of the conformance surface, or an in-process
component. It is a **client** of the REST API and nothing more. It ships and
versions on the product line but adds no obligations to the spec-compliance
mission — the ECC suite never touches it.

---

## 2. The "no JavaScript" constraint, honestly

Every framework and library selected below is Rust compiled to WebAssembly (for
anything running in the browser) or plain Rust on the server. **You will not
write, maintain, or hand-edit a line of JavaScript.** Two honest nuances so the
constraint is not oversold:

- **A small generated JS bootstrap is unavoidable today.** `wasm-bindgen` emits
  a tiny glue shim to load the `.wasm` module and bridge WASM ↔ DOM APIs, because
  WebAssembly and the DOM cannot yet call each other directly. It is
  **generated by the toolchain, never written or touched by us**, and the
  WebAssembly "host bindings" proposal is designed to remove it. This is
  inherent to *every* Rust-web framework (Leptos, Dioxus, Yew alike), not a
  property of our choice.
  ([wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen))
- **Styling is CSS, not JS.** We use Tailwind, whose **standalone binary**
  (`cargo-leptos` downloads and runs it) needs **no Node.js and no npm** — see
  §4. Tailwind is a CSS generator; nothing about it is JavaScript we author.

Charting is the usual place a "no-JS" claim quietly breaks (most Rust chart
crates wrap ECharts/Plotly, which are JavaScript). The one we pick
(`leptos-chartistry`) is **pure Rust + SVG — explicitly no JS, no canvas** (§4).

---

## 3. Framework selection

**Pick: [Leptos](https://leptos.dev/) 0.8** (latest stable `0.8.20`, verified
live on crates.io 2026-07-13; a `0.9.0-alpha` exists — we stay on the stable
0.8 line). Fine-grained reactive (surgical DOM updates, ideal for a
data-dense admin dashboard), web-first, and — the clincher for *this* repo — its
**0.8 line targets `axum` 0.8**, the exact version the workspace already pins.
Same server stack, same `tower`/`tracing` idioms.
([Leptos 0.8 release](https://github.com/leptos-rs/leptos/releases/tag/v0.8.0) ·
[leptos_axum](https://docs.rs/leptos_axum/latest/leptos_axum/))

| Framework | Version (Jul 2026) | Verdict for this job |
|---|---|---|
| **Leptos** | 0.8.20 | **Chosen.** Web-first, fine-grained, `axum` 0.8 native, richest *pure-Rust* web-widget ecosystem (§4), first-class BFF via server functions (§5). |
| Dioxus | 0.7.3 (0.7.0 shipped 29 Jan 2026) | **Viable alternative.** Cross-platform (web/desktop/mobile from one codebase), superb DX (Rust hot-patching), also `axum`-based fullstack. Choose it **only** if a desktop/mobile build of the console is ever wanted; for web-only it is heavier (virtual-DOM diff vs Leptos's fine-grained updates) and its *web* widget/chart ecosystem is thinner. ([Dioxus 0.7](https://dioxuslabs.com/blog/release-070/)) |
| Yew | 0.21 | **Ruled out.** Mature but slow-moving React-style vDOM; no edge over Leptos here. |
| egui / eframe | current | **Ruled out for this UI.** Immediate-mode, renders to a WebGL/WebGPU **canvas** — no page search, weak mobile text editing, poor accessibility. Wrong for a mobile-friendly admin console (great for internal debug tools, which this is not). ([egui](https://github.com/emilk/egui) · [Rust GUI landscape 2026](https://wrenlearnsrust.com/posts/2026-03-11-rust-gui-landscape-2026.html)) |

---

## 4. The Rust-only stack (all verified pure-Rust / CSS, no authored JS)

| Concern | Choice | Version (re-verified live 2026-07-17) | No-JS status |
|---|---|---|---|
| UI framework | `leptos` (SSR/full-stack) | 0.8.20 (2026-06-25) | Rust → WASM |
| Server integration | `leptos_axum` | 0.8.10 (2026-06-25) | native (the SSR/axum glue; versions on its own line — pin explicitly, don't assume it equals the `leptos` version) |
| Build / bundler | `cargo-leptos` | 0.3.7 (2026-07-03) | Builds both binaries (native server + WASM client); **bundles the Tailwind standalone binary — no Node/npm**. ([cargo-leptos](https://github.com/leptos-rs/cargo-leptos)) |
| Styling | Tailwind CSS v4 (via `cargo-leptos` standalone) | **v4.3.3** (released 2026-07-16; pin with `LEPTOS_TAILWIND_VERSION`) | CSS, no JS |
| Component kit | [`thaw`](https://github.com/thaw-ui/thaw) (Fluent-design) | **`=0.5.0-beta`** (2025-05-03) — leptos req `^0.8.0` (crates.io index, re-verified 2026-07-17) | Rust → WASM. **Owner decision (2026-07-13): use the 0.5 beta.** The "stable" 0.4.8 (2025-08-03) is pinned to Leptos **^0.7.7** and is *not* 0.8-compatible, so the beta is the only published 0.8 line. **There is exactly one beta on crates.io — `0.5.0-beta`, no `beta.N` series — and it is 14 months old**, so pin it exactly (`=0.5.0-beta`) and treat a pinned git rev of `thaw-ui/thaw` main (the active 0.8 branch) as the *likely* real path if the beta fails against Leptos 0.8.20; smoke-test the beta at scaffold time (W0, §12) before any screen work. (Leptonic is stale — last release Feb 2024 — **do not use**.) |
| Data grid / tables | [`leptos-struct-table`](https://docs.rs/leptos-struct-table) | 0.19.0 (2026-06-23), leptos `^0.8`, leptos-use `^0.19` | Rust → WASM. Async data from a REST source, virtualization, pagination, multi-column sort, column hide/reorder, headless (our CSS). Exactly the RESULT_SET / EHR-list widget. |
| Charts | [`leptos-chartistry`](https://github.com/feral-dot-io/leptos-chartistry) | 0.2.3 (2026-01-23), leptos `^0.8` | **Pure Rust + SVG — "no JS, no canvas."** Has an SSR feature. Dashboard tiles. Note: depends on leptos-use `^0.18` while struct-table wants `^0.19` — cargo resolves both (pre-1.0 minors are distinct), at the cost of a duplicated leptos-use in the WASM bundle until chartistry bumps. Accepted. |
| Reactive utils | `leptos-use` | 0.19.0 (2026-06-22) | Rust → WASM (storage, debounce, clipboard, etc.) |
| Server→CDR HTTP | `reqwest` 0.13 (rustls) | workspace pin | Server-side only; the BFF's call into the CDR (§5). |
| Formatter | `leptosfmt` | 0.1.33 (dev tooling, CI + `/ui-gates`) | n/a — formats the `view!` macro; runs alongside `cargo fmt`. |
| E2E driver | `thirtyfour` | 0.37.2 (2026-07-05) | dev-dep only; the §8d WebDriver client. |

Everything above is already the kind of dependency this workspace uses
(`reqwest` 0.13 and `axum` 0.8 are pinned today).

---

## 5. Architecture — a Backend-for-Frontend (BFF) that enforces the REST boundary

Recommended shape: **one Leptos SSR / full-stack binary** (built by
`cargo-leptos`), which is itself an `axum` 0.8 server. It serves the app *and*
hosts the server-side logic that talks to the CDR. One binary → one OCI image,
mirroring the existing app image.

```
Browser (WASM, no JS authored)
      │  HTML + hydration + server-function calls (same origin)
      ▼
ehrbase-admin-ui   ── the Leptos SSR server (axum 0.8) = the BFF
      │  reqwest 0.13, server-side only
      │  Accept: application/json | application/xml
      ▼
ehrbase-rs CDR     ── ITS-REST 1.0.3   /ehrbase/rest/openehr/v1/…
```

### 5.1 Why BFF (not a browser-direct SPA)

- **Credentials stay server-side.** The browser never holds the CDR token; the
  BFF injects `Authorization` on the outbound `reqwest` call.
- **No CORS on the CDR.** Browser talks only to its own origin (the admin
  server); the admin server talks to the CDR.
- **One place for content negotiation** (§5.3) and error normalisation.
- Leptos **server functions** make this natural: a `#[server]` fn runs on the
  axum side and is callable from the component "as if in the browser," with no
  separate hand-written REST layer between UI and BFF.

**Security correction (from the Leptos book, `server/25`):** server functions
are **not** private plumbing — each `#[server]` fn is a *publicly reachable
HTTP endpoint* on the console server. Every server fn that touches the CDR or
session state must therefore enforce the console's own auth (session check /
middleware) inside the BFF; "only my UI calls this" is never assumed. This is
a hard rule in `.claude/rules/leptos-ui.md` §0.

**Rendering mode:** out-of-order streaming SSR (the Leptos default — shell +
`<Suspense>` fallbacks immediately, data streamed as it resolves, resources
begin loading on the server). Data loads via `Resource` → `#[server]` fn;
mutations via `ServerAction` + `<ActionForm>`, which degrade gracefully to
plain HTML forms before WASM loads.

### 5.2 The hard boundary (the one spec-facing rule)

`ehrbase-admin-ui` reaches the CDR **only** over ITS-REST. In workspace terms:

- **May depend on** `crates/openehr-*` (spec types — see §6). These are the
  domain model and serialization; using them is *not* reaching around REST.
- **Must NOT depend on** `app/ehrbase` or `app/ehrbase-rest` *(rev.
  2026-07-17: the former `app/ehrbase-sm` no longer exists — the app is three
  crates)*. Linking the service layer in-process would defeat the entire point
  and couple the console to CDR internals. The dependency arrow is
  `app/ehrbase-admin-ui → crates/openehr-*` and **network → CDR**, never
  `app → app`.

### 5.3 XML + JSON + the Simplified Formats *(rev. 2026-07-17)*

openEHR ITS-REST negotiates representation via `Accept` / `Content-Type` with
`application/json` (canonical JSON) and `application/xml` (canonical XML), plus
`Prefer: return=representation|minimal`. Since the 2026-07-17 rewrite the CDR
additionally implements the **Simplified Formats** spec-exactly:

- `application/openehr.wt.flat+json` (FLAT) and
  `application/openehr.wt.structured+json` (STRUCTURED) on compositions,
  template examples, and CONTRIBUTION inner payloads;
- `application/openehr.wt+json` — the **Web Template** rendering of a
  template (the machine model the Query Builder consumes, §7.2);
- negotiation is strict RFC 9110 q-values; there is **no `?format=`
  parameter**; unsupported types answer `415`/`406` with a body naming the
  supported set;
- a FLAT/STRUCTURED composition **commit requires the `openehr-template-id`
  request header** (`422` without it) — the BFF must send it.

The BFF therefore:

- exposes a **format selector** — canonical JSON, canonical XML, FLAT,
  STRUCTURED — on composition views (and FLAT/STRUCTURED on commit forms;
  FLAT is *the* form-friendly clinician-facing shape: one flat key/value
  object);
- sets the outbound `Accept`/`Content-Type` accordingly and forwards the
  chosen representation;
- when it needs to *render or validate* a payload (not just proxy bytes),
  parses via `openehr-its` (both canonical forms) into `openehr-rm` types,
  and uses `openehr-flat` for FLAT ⇄ STRUCTURED transforms and Web-Template
  introspection (§6).

> Endpoint paths (Definition/Query/EHR/Composition APIs, base
> `/ehrbase/rest/openehr/v1`) must be taken from the **generated ITS-REST
> contract in `openehr-its`** — and can be cross-checked against the **running
> server's own generated `openapi.json`** (owner hard rule 2026-07-17: the
> served document is composed natively from the handlers and advertises the
> simplified media types; the vendored OAS is the behavioural oracle, never
> served). The feature map (§7) names the API *groups*; the exact routes come
> from the contract.

### 5.4 Auth

**v1 ships both Basic and OAuth2/OIDC** (owner decision, §10) — matching the
CDR's Stage-1 auth. The BFF holds the session server-side (`tower-sessions`) and
attaches the right credential to each outbound CDR call: a Basic header, or a
bearer token obtained from the OIDC/Keycloak authorization-code flow
(`openidconnect`/`oauth2`, both already workspace-pinned). Fine-grained RBAC is a
CDR Stage-2 concern, out of scope here.

---

## 6. Domain reuse — the reason to pick Rust at all

Because the console is a workspace member, it consumes the **already-generated**
spec crates instead of re-modelling openEHR in TypeScript. This is the decisive
advantage over a JS/React admin and it directly powers the headline features:

| Crate | Used for |
|---|---|
| `openehr-query` (AQL AST/parser) | **Query Builder** — assemble AQL from point-and-click selections into the *same* AST the engine validates; parse/validate/pretty-print before sending. No string-concatenation of AQL. |
| `openehr-am` (OPT / ADL) | **Template Manager** — parse and introspect uploaded operational templates for display (constraints, occurrences, terminology bindings). |
| `openehr-its` (canonical JSON + XML) | The XML/JSON viewer + any client-side parse/validate; identical serialization to the server, so round-trips are byte-consistent. |
| `openehr-rm` (RM 1.2.0) | Typed rendering of EHR_STATUS, COMPOSITION, CONTRIBUTION, VERSION, etc. |
| `openehr-flat` *(rev. 2026-07-17 — rewritten spec-exact against ITS-REST `simplified_formats`)* | **FLAT/STRUCTURED rendering + form building** (`convert::` — the pure FLAT ⇄ STRUCTURED transforms need no template), the **Web Template model** (`webtemplate::` — deserialize the CDR's `application/openehr.wt+json` document into the typed tree that drives the Query Builder path catalog and form generation), and pre-submit **validation** (`validation::`). Module-pathed API, no crate-root re-exports. |

---

## 7. Feature map — adopt / adapt / defer

The Appendix-A inventory sorts into three buckets against *our* architecture.
The guiding difference: **Cabolabs has no AQL engine** — it invented a private
criteria model executed over bespoke per-datatype index tables. **We already
have an AQL 1.1 engine and node storage.** So we take Cabolabs' *UX* and emit
**AQL**, executed via the CDR's Query API — never a private index. That keeps
the REST-only boundary (§5.2) intact.

### 7.1 Adopt — the console's feature set (all over ITS-REST)

| Feature | ITS-REST API group | UI building blocks |
|---|---|---|
| **Dashboard** | Query + status | Counts (EHRs, compositions, templates) via saved AQL; trend/among tiles via `leptos-chartistry`. |
| **Template Manager** | Definition (ADL 1.4; ADL2 later) | Upload OPT, list, activate/deactivate, **inspect indexed paths** → `openehr-am` introspection; `thaw` + `leptos-struct-table`. |
| **Query Builder** (point-and-click) | Query (ad-hoc) + Definition (stored) | The star feature — §7.2. Emits `openehr-query` AST → AQL. |
| **Saved queries + Query Groups** | Definition (stored queries) | Save/name/share a query; group queries into a **dashboard of counts** (Cabolabs `QueryGroup.executeCount`), each tile a saved AQL. |
| **Cohort / EHR queries** | Query | "Which EHRs match this set of conditions?" and "does *this* EHR match? (boolean)" — Cabolabs `EhrQuery`, expressed as AQL over EHRs. |
| **EHR + Composition browser** | EHR, EHR_STATUS, Composition, Directory, Contribution | Navigate EHRs, folders/directory; view a composition in JSON **or** XML (§5.3); version list. |
| **Version / audit history** | Versioned-object + Contribution | Revision history, `AUDIT_DETAILS`, contributions, time-travel (`version_at_time`). |
| **Format viewer** | all | JSON ⇄ XML toggle on any resource, both canonical (`openehr-its`). |
| **Result export** | Query | Export a result set (CSV/JSON) — Cabolabs `export`. |
| **Activity / system log view** | admin / System Log | Surface the CDR's ATNA system log read-only (Cabolabs `ActivityLog`). |
| **System status** | `/rest/status`, management | Health, versions, config surface (read-only). |
| **i18n + responsive** | — | Multi-language labels (Cabolabs `name: lang→text`), mobile-friendly (`thaw` + Tailwind). |

### 7.2 Adapt — the Query Builder (the amazing feature), our way

Cabolabs' point-and-click flow, and how each step lands on our stack:

1. **Pick a template** → the builder lists activated OPTs (Definition API).
2. **Pick archetype → path** → Cabolabs serves these from its **OPT path index**
   (`getArchetypesInTemplate` → `getArchetypePaths`). *(rev. 2026-07-17)* We get
   the same catalog **directly from the CDR**: `GET
   …/definition/template/adl1.4/{id}` with
   `Accept: application/openehr.wt+json` returns the **Web Template** document
   — every node already carries its `aqlPath`, RM type, multiplicity, and
   typed `inputs` (the per-datatype widget spec, exactly what step 3 needs) —
   deserialized with `openehr_flat::webtemplate`. Walking the raw OPT with
   `openehr-am` remains the fallback for non-templated introspection; no
   separate index table either way.
3. **Typed criteria widget per RM datatype** → Cabolabs' `getCriteriaSpec`
   returns operators/inputs keyed by `DV_*` type (it has a `DataCriteriaDV_*`
   class per datatype: `DV_QUANTITY` magnitude+unit `between`, `DV_CODED_TEXT`
   code-list/terminology, `DV_DATE_TIME` range, `DV_ORDINAL` symbol/value,
   `DV_PROPORTION`, `DV_COUNT`, …). We reproduce this as a **typed criteria
   component per datatype**, but each emits an **AQL WHERE predicate**.
4. **Complex boolean logic** → Cabolabs stores `where` as a binary tree of
   `DataCriteriaExpression` (AND/OR). We build the same tree and lower it to an
   AQL boolean expression via the `openehr-query` AST.
5. **Query type** → Cabolabs `type = composition | datavalue` (return whole
   compositions vs projected data points). Maps to AQL selecting the COMPOSITION
   vs selecting leaf paths (its `select` = `DataGet` list → AQL SELECT columns).
6. **Grouping** → Cabolabs `group = none | composition | path`
   (composition → table rows, path → value series for charts). Drives whether
   the result renders in `leptos-struct-table` or `leptos-chartistry`.
7. **Validate + run** → parse/validate with `openehr-query`, then execute via the
   CDR Query API; render the RESULT_SET.
8. **Raw mode** → Cabolabs has an `hql` escape hatch; ours is a **raw AQL editor**
   (the same builder output, hand-editable).

> **Not adopted here:** Cabolabs' SNOMED-CT-expression criteria
> (`validateSnomedExpression`, the external SNQUERY tool). That maps to our AQL
> **terminology family** (`TERMINOLOGY()` / `matches {uri}`,
> `docs/specs/openehr/QUERY/AQL/`), which is a **CDR** capability with no
> open worklist row today (the blueprint that tracked it was deleted
> 2026-07-16 — register a row if/when wanted) — the builder gains a
> terminology-constraint widget only once/if that lands CDR-side. Recorded,
> not built at v1.

### 7.3 Defer / out of scope — these are CDR or Stage-2 concerns, not the UI

Cabolabs is a whole platform; much of it is *server* responsibility we already
have (differently) or explicitly park:

- **Multitenancy, accounts, subscription plans, API keys, billing**
  (`Organization`/`Account`/`Plan`/`ApiKey`) → CDR **Stage-2 enterprise** track,
  not this console.
- **Instance-to-instance sync / replication** (VNA master/replica —
  `SyncController`, `SyncLog`) → a **CDR** feature if ever wanted; no openEHR
  spec governs it (our own extension). Not a UI concern.
- **Data-value indexing** (`DataValueIndex` + the `Dv*Index` tables) → this is
  Cabolabs' query substrate. **Ours is the node store + AQL engine** — the
  console never owns an index.
- **Commit-log / version blob repositories (FS/S3)** → CDR storage internals.
- **Notifications / webhooks** (`Notification`, `RemoteNotificationsService`) →
  depends on CDR eventing (our own extension, no spec) — optional, Stage-2.

### 7.4 SMART App Launch (openEHR ITS-REST) — what the console surfaces

Not a Cabolabs feature — this comes from the **openEHR spec**. The CDR already
implements the openEHR **SMART App Launch** *resource-server* role
(`app/ehrbase-rest/src/smart/` — `discovery.rs` / `scope.rs` / `enforce.rs`;
spec `docs/specs/openehr/ITS-REST/docs/smart_app_launch/` master02–09).
What exists today, verified in source:

- **`GET /.well-known/smart-configuration`** discovery document (master04),
  served **pre-auth**, **config-gated — off by default** (disabled → `404`,
  zero wire drift);
- the master08 **resource-scope grammar** (`compartment/resource.permission`,
  `*`/`**`/`ns::*`) parsed from the token `scope` claim;
- **scope enforcement** AND-composed onto the ABAC PEP, with `ehrId`/`patient`
  **launch-context** binding (master07/09) and configurable claim names
  (`SmartConfig`: `enabled`, `platform_base_url`, `ehr_id_claim`,
  `patient_claim`, `require_smart_scopes`, `episode`, advertised endpoints).

It deliberately does **not** do registration (master03), token issuance / PKCE
(master06), or the launch UI (master07) — those are Authorization-Server /
Launcher duties.

The console is a **client + operator view** of that, never an Authorization
Server. **Adopt:**

| Feature | CDR surface | Notes |
|---|---|---|
| **SMART configuration viewer** | `GET …/.well-known/smart-configuration` (pre-auth; `404` = SMART off) | Read-only render of advertised authorization/token/jwks/introspection endpoints, `grant_types`, PKCE methods, `scopes_supported`, launch-context claims, episode support — lets an admin verify SMART is wired correctly. |
| **SMART status tile** (system panel) | discovery + `/rest/status` | "SMART: enabled/disabled" + configured platform base + out-links to the advertised Auth-Server endpoints. |
| **Session scope + launch context** | the console's own OAuth2/OIDC token (BFF-held) | Decode and show the current session's `scope` claims and any `ehrId`/`patient`/episode launch context — the "what can I do right now" panel. |
| **Scope previewer** (master08) | grammar only | Parse/preview a scope string and show what it grants — handy when composing scopes for a third-party app. **Open design point:** the master08 grammar lives in `ehrbase-rest` today, which the console **may not depend on** (§5.2). Resolve by lifting the scope grammar into a shared `openehr-*` crate (spec-defined; server + console both benefit) or exposing a CDR debug endpoint. Decide before building this tile. |

**Out of scope** (mirrors the CDR boundary — Auth-Server/Launcher duties): app
**registration** (master03), **token issuance / PKCE** (master06), the
**launch-sequence UI** (master07). The console only *links out* to the
authorization/registration/management endpoints the discovery document
advertises.

> The console's own login (dual Basic + OAuth2/OIDC, §5.4/§10) authenticates an
> **admin/practitioner**, so it requests admin-level scopes — not a
> `launch/patient` patient context. A patient-context launch of the console
> itself is possible but not a v1 goal.

---

## 7A. Screen catalog — routes, wireframes, data, states *(added 2026-07-17b)*

The binding per-screen specification. A `ui-implementer` subagent builds a
screen from its entry here **without inventing layout, data flow, or state
handling**; anything a screen needs that this catalog doesn't answer goes
back to the orchestrator, not into improvisation. Conventions that apply to
**every** screen (stated once):

- **Routing:** `leptos_router` under the shell layout (§7A.0); route params
  in the tables below use `{braces}`. All routes except `/login` sit behind
  the session guard — no session → redirect to `/login?next=…`.
- **Data:** every read is a `Resource` calling a named `#[server]` fn;
  every mutation is a `ServerAction` + `<ActionForm>` (degrades to a plain
  HTML form pre-hydration, §5.1). Server fns are named in each table —
  they are the BFF's complete CDR-facing API and each one **enforces the
  session check itself** (§5.1 security correction). Endpoint paths come
  from the generated `openehr-its` REST contract (§5.3 note); the tables
  name representative operations, base `/ehrbase/rest/openehr/v1`.
- **States:** every data region renders all four: **loading** (`<Suspense>`
  skeleton — `thaw` Skeleton, never blank), **empty** (explanatory empty
  state with the action that fills it), **error** (the BFF's normalized
  error: HTTP status + CDR diagnostic body, rendered in a `thaw`
  MessageBar, never a raw debug string), **forbidden** (401/403 from the
  CDR → distinct "insufficient scope" surface, journey J7).
- **i18n:** all labels through the i18n layer (single `en` catalog at v1;
  keys, not literals, in components).
- **Spec grounding:** the console is our own extension (no openEHR spec
  governs an admin UI); what IS spec-bound is every wire interaction —
  representations and negotiation per
  `docs/specs/openehr/ITS-REST/docs/simplified_formats/` + the vendored
  OAS (`crates/openehr-its/vendor/rest-oas/`), AQL per
  `docs/specs/openehr/QUERY/AQL/`, SMART per
  `docs/specs/openehr/ITS-REST/docs/smart_app_launch/`.

### 7A.0 App shell (layout, not a route)

```
┌──────────────────────────────────────────────────────────────────┐
│ ehrbase-admin   [CDR: https://cdr:8080 ● UP]        user ▾   ☾  │  topbar
├────────────┬─────────────────────────────────────────────────────┤
│ Dashboard  │                                                     │
│ Templates  │                                                     │
│ Queries    │                <Outlet/> — routed screen            │
│ EHRs       │                                                     │
│ System     │                                                     │
│            │                                                     │
├────────────┴─────────────────────────────────────────────────────┤
│ console vX.Y.Z · CDR vX.Y.Z (from /rest/status) · scopes chip    │  footer
└──────────────────────────────────────────────────────────────────┘
```

- Components: `thaw` Layout + NavDrawer (collapses to a hamburger < 768 px
  — the mobile-friendly mandate), topbar CDR-health pill (polled
  `fetch_status` every 30 s via `leptos-use` `use_interval_fn`), user menu
  (session identity, **scopes/launch-context panel** — the §7.4 "what can I
  do right now" drawer — and logout), dark-mode toggle (`thaw` theme +
  `leptos-use` storage persistence).
- Server fns: `fetch_status` (GET `/rest/status` — extension endpoint, no
  openEHR spec governs it), `current_session` (session introspection,
  BFF-local), `logout` (session destroy).
- E2E: every journey traverses the shell; J2 (hydration) asserts on the
  theme toggle specifically.

### 7A.1 `/login`

```
┌───────────────────────────────┐
│         ehrbase-admin         │
│  ┌─────────────────────────┐  │
│  │  Username  [_________]  │  │
│  │  Password  [_________]  │  │
│  │  [ Sign in ]            │  │
│  ├────────── or ───────────┤  │
│  │  [ Sign in with OIDC ]  │  │
│  └─────────────────────────┘  │
│  CDR: https://cdr:8080 ● UP   │
└───────────────────────────────┘
```

- Purpose: dual auth (§5.4/§10). Basic form posts to `login_basic`
  (validates against the CDR by calling `GET /rest/status` — or any cheap
  authenticated op — with the supplied credentials; on success stores them
  in the server-side session). OIDC button starts the authorization-code
  flow (`openidconnect`); the callback route `/auth/callback` is a plain
  axum handler on the BFF, not a Leptos route.
- Components: `thaw` Card + Field + Input + Button; `<ActionForm>` for the
  Basic path (works pre-hydration — journey J6 depends on this).
- States: error = wrong credentials (401 from CDR probe) or OIDC failure,
  shown in a MessageBar; `next` query param honoured on success.
- E2E: **J1** (both variants), **J6** (no-JS Basic login), **J7**
  (unauthenticated redirect lands here).

### 7A.2 `/` — Dashboard

```
┌───────────────────────────────────────────────────────┐
│  EHRs        Compositions     Templates    Queries    │
│  [ 1 284 ]   [ 45 902 ]       [ 37 ]       [ 12 ]     │   stat tiles
├───────────────────────────────────────────────────────┤
│  Query-group tiles (one per saved group, §7.1)        │
│  ┌───────────────┐ ┌───────────────┐ ┌─────────────┐  │
│  │ Diabetics     │ │ Hypertension  │ │ …           │  │
│  │    412        │ │     97        │ │             │  │
│  └───────────────┘ └───────────────┘ └─────────────┘  │
├───────────────────────────────────────────────────────┤
│  Compositions committed (30 d)   ▁▂▄▆█▆▄  chartistry  │
└───────────────────────────────────────────────────────┘
```

- Server fns: `dashboard_counts` (POST `/query/aql` — one aggregate AQL per
  tile, `SELECT COUNT(…)`), `query_group_counts` (executes each saved query
  in the group via the Query API, concurrent server-side), `commit_trend`
  (AQL over `context/start_time` bucketed per day).
- Components: `thaw` Card grid; `leptos-chartistry` line/bar for the trend.
- States: empty = fresh CDR ("no data yet — upload a template to begin",
  links to `/templates`); a failing tile renders its error in-tile, never
  blanks the whole dashboard.
- E2E: **J8** asserts tiles render with numeric content and the chart SVG
  exists.

### 7A.3 `/templates` — Template Manager (list)

```
┌───────────────────────────────────────────────────────┐
│ Templates                    [ Upload OPT ▲ ]         │
│ [filter: ______ ]                                     │
├───────────────┬──────────────┬──────────┬─────────────┤
│ template_id   │ concept      │ created  │ actions     │
├───────────────┼──────────────┼──────────┼─────────────┤
│ vitals.v1     │ Vital signs  │ 2026-…   │ view · wt   │
│ IPS.v1        │ Intl Patient…│ 2026-…   │ view · wt   │
└───────────────┴──────────────┴──────────┴─────────────┘
```

- Server fns: `list_templates` (GET `/definition/template/adl1.4`),
  `upload_template` (POST `/definition/template/adl1.4`,
  `Content-Type: application/xml`, the raw OPT body; surfaces the CDR's
  validation errors verbatim on 400/409/422).
- Components: `leptos-struct-table` (sort/filter/paginate); `thaw` Upload
  dialog for the OPT file (file → server fn as bytes; no JS file handling —
  `web-sys` File API via the component).
- States: empty = "no templates — upload your first OPT"; upload error =
  the CDR diagnostic (e.g. duplicate template id → 409) in the dialog.
- E2E: **J3** (upload corpus OPT → appears in list → open detail).
- Note: ADL 1.4 only at v1 (§10 decision 4); activate/deactivate from
  Cabolabs is **not** in ITS-REST — omitted (idea-source, not 1:1; the CDR
  has no such state).

### 7A.4 `/templates/{template_id}` — Template detail

```
┌───────────────────────────────────────────────────────┐
│ ← Templates    vitals.v1          [OPT] [WT] [Example]│  tab bar
├──────────────────────────┬────────────────────────────┤
│ Path catalog (WT tree)   │  Node inspector            │
│ ▸ vitals                 │  aqlPath: /content[…]      │
│   ▾ body_temperature     │  rmType: DV_QUANTITY       │
│     ▸ any_event          │  card: 0..*                │
│       • temperature ◀    │  inputs: magnitude+unit    │
│       • time             │  units: [°C, °F]           │
└──────────────────────────┴────────────────────────────┘
```

- Server fns: `fetch_template_opt` (GET `…/adl1.4/{id}`,
  `Accept: application/xml` — the raw OPT), `fetch_webtemplate` (same URL,
  `Accept: application/openehr.wt+json` → `openehr_flat::webtemplate`
  typed tree), `fetch_example` (GET `…/{id}/example`, format per the §5.3
  selector).
- Tabs: **OPT** (canonical XML viewer), **WT** (the tree above — the same
  component the Query Builder reuses for path picking), **Example** (the
  CDR-generated example composition, format-switchable).
- Components: `thaw` Tree + Card; the format viewer component (§7A.9's
  viewer, shared).
- States: 404 = unknown template id (link back to list); WT parse failure
  = error state naming the node (never a panic — deny-tier lints).
- E2E: **J3** (path catalog renders, node inspector shows `aqlPath` +
  `rmType` for a known node).

### 7A.5 `/queries` — Stored queries + groups

```
┌───────────────────────────────────────────────────────┐
│ Queries                        [ New query (builder) ]│
├───────────────────┬───────────────────────────────────┤
│ Stored queries    │ Groups                            │
│ name    ver  run  │ ┌──────────────┐ [ New group ]    │
│ q.vitals 1.0  ▶   │ │ chronic-care │ q.diab, q.hyp    │
│ q.diab   2.1  ▶   │ └──────────────┘                  │
└───────────────────┴───────────────────────────────────┘
```

- Server fns: `list_stored_queries` (GET `/definition/query/{qualified_name}`
  + the list form), `fetch_stored_query` (GET
  `/definition/query/{name}/{version}`), `store_query` (PUT same — versioned
  per the contract), `run_stored_query` (GET/POST `/query/{name}/{version}`).
  Groups are **console-local** (BFF session-store/TOML-config persisted —
  ITS-REST has no query-group resource; flagged: our own extension, no
  openEHR spec governs it).
- Components: `leptos-struct-table` for the query list; group cards.
- States: empty = "no stored queries — build one"; run ▶ navigates to the
  builder in *loaded* mode with the RESULT_SET pane active.
- E2E: **J4** (save from builder → appears here → re-run from saved).

### 7A.6 `/queries/builder` — the Query Builder (the star, §7.2)

```
┌────────────────────────────────────────────────────────────────┐
│ 1 Template   2 Paths   3 Criteria   4 Shape      [Builder|AQL] │  stepper + mode
├──────────────────┬─────────────────────────────────────────────┤
│ WT path tree     │  SELECT  [+ column]                         │
│ (7A.4 component, │   • temperature (DV_QUANTITY) magnitude     │
│  checkboxes)     │  WHERE   [+ criterion] [AND|OR group]       │
│                  │   ┌ AND ────────────────────────────┐       │
│                  │   │ temperature.magnitude between    │       │
│                  │   │   [36.0] and [38.5] °C           │       │
│                  │   │ OR ┌ code = [_____] (DV_CODED)  ││       │
│                  │   └──────────────────────────────────┘       │
├──────────────────┴─────────────────────────────────────────────┤
│ AQL preview (read-only in builder mode, editable in AQL mode)  │
│ SELECT c/content[…]… FROM EHR e CONTAINS COMPOSITION c …       │
│ [ Validate ]  [ ▶ Run ]  [ Save as stored query… ]             │
├────────────────────────────────────────────────────────────────┤
│ RESULT_SET   [table | chart]  [Export CSV] [Export JSON]       │
│ (leptos-struct-table over columns … / chartistry when grouped) │
└────────────────────────────────────────────────────────────────┘
```

- Flow (§7.2): pick template → tick paths from the WT tree (each node
  carries `aqlPath` + `rmType` + `inputs`) → per-`DV_*` typed criterion
  widgets (§7.2 step 3 catalog: DV_QUANTITY magnitude+unit range,
  DV_CODED_TEXT code picker, DV_DATE_TIME range, DV_ORDINAL, DV_COUNT,
  DV_PROPORTION, DV_BOOLEAN, DV_TEXT contains/=) → AND/OR tree (§7.2
  step 4) → shape (compositions vs data points; grouping → table vs chart,
  §7.2 steps 5–6).
- **Core discipline:** the builder state lowers to an `openehr-query` AST
  and pretty-prints — **AQL is never string-concatenated** (§6). The
  builder-state → AST lowering module is component-free plain Rust with
  exhaustive unit tests (§8b) and is **orchestrator-built** (§12).
- Server fns: `validate_aql` (parse via `openehr-query`, BFF-local),
  `run_aql` (POST `/query/aql` with query + `query_parameters`;
  `fetch=`/`offset=` paging honoured), `store_query` (§7A.5), `export_csv`
  / `export_json` (BFF streams the RESULT_SET transformed server-side).
- Raw **AQL mode**: same screen, editable text area replaces the builder
  panes (one-way builder→AQL handoff; hand-edited AQL does not lift back
  into builder state at v1 — flagged in-UI).
- States: criteria widget for an unsupported `rmType` = explicit
  "unsupported at v1" chip (never a silent skip); CDR query errors (bad
  AQL → 400 with diagnostics) render under the preview; result paging
  loading state on the table.
- E2E: **J4** end-to-end (template → path → criterion → run → rows →
  save → re-run), plus an AQL-mode validate/run assertion.

### 7A.7 `/ehrs` — EHR finder

```
┌───────────────────────────────────────────────────────┐
│ EHRs      [ehr_id or subject id: ________ ] [Find]    │
├───────────────────────────────────────────────────────┤
│ Recent / listed EHRs (AQL: SELECT e/ehr_id/value …)   │
│ ehr_id                        │ created   │ status    │
│ 7d44…                        │ 2026-…    │ queryable │
└───────────────────────────────────────────────────────┘
```

- Server fns: `find_ehr` (GET `/ehr/{ehr_id}`, or GET
  `/ehr?subject_id=…&subject_namespace=…`), `list_ehrs` (AQL over EHRs —
  ITS-REST has no unpaged EHR list; flagged: listing via AQL is the
  spec-honest route).
- States: not-found = inline "no EHR with that id"; the list pages via
  AQL `fetch`/`offset`.
- E2E: **J5** entry point.

### 7A.8 `/ehrs/{ehr_id}` — EHR detail

```
┌───────────────────────────────────────────────────────┐
│ ← EHRs   EHR 7d44…        [Status|Directory|Comps|Contribs]
├───────────────────────────────────────────────────────┤
│ Status: queryable ✓ modifiable ✓   subject: …         │  Status tab
│ ── or ──                                              │
│ 📁 root  ▸ episodes  ▸ 2026-07  • items…              │  Directory tab
│ ── or ──                                              │
│ compositions table (name, template, time, versions)   │  Compositions tab
│ ── or ──                                              │
│ contributions table (id, time, audit, versions in it) │  Contributions tab
└───────────────────────────────────────────────────────┘
```

- Server fns: `fetch_ehr` + `fetch_ehr_status` (GET `/ehr/{id}`,
  `/ehr/{id}/ehr_status`), `fetch_directory` (GET `/ehr/{id}/directory`,
  FOLDER tree), `list_compositions` (AQL:
  `SELECT c/uid/value, c/name/value, c/archetype_details/template_id/value,
  c/context/start_time/value FROM EHR e[ehr_id/value=$id] CONTAINS
  COMPOSITION c` — paged), `list_contributions` (GET
  `/ehr/{id}/contribution/{uid}` per row after an AQL/versioned listing).
- Components: `thaw` TabList; Tree for directory; `leptos-struct-table`
  for the two tables.
- States: EHR_STATUS with `is_queryable=false` renders a warning banner;
  no directory = empty state (the CDR 404s — render "no directory").
- E2E: **J5** (navigate status → compositions → open one).

### 7A.9 `/ehrs/{ehr_id}/compositions/{versioned_object_uid}` — Composition viewer

```
┌────────────────────────────────────────────────────────────┐
│ ← EHR 7d44…   Vital signs 2026-07-12                       │
│ Format: [JSON][XML][FLAT][STRUCTURED]   Version: [2 ▾] ⏱   │
├────────────────────────────────────────────────────────────┤
│ {                                                          │
│   "_type": "COMPOSITION",                                  │
│   "name": { "_type": "DV_TEXT", "value": "Vital signs" },  │
│   …                                                        │
├────────────────────────────────────────────────────────────┤
│ Version timeline: v1 ── v2(current)   audit: creation by … │
└────────────────────────────────────────────────────────────┘
```

- Server fns: `fetch_composition` (GET
  `/ehr/{id}/composition/{uid}` with the §5.3 `Accept` per selector:
  canonical JSON, canonical XML, `…wt.flat+json`, `…wt.structured+json` —
  the CDR converts; the BFF forwards bytes and pretty-prints),
  `fetch_versions` (GET `/ehr/{id}/versioned_composition/{uid}` +
  `/revision_history`), `fetch_at_version` (GET
  `…/versioned_composition/{uid}/version/{version_uid}` or
  `?version_at_time=`).
- Components: the shared **format viewer** (syntax-highlighted read-only
  view — pure-Rust lexing for JSON/XML token classes, no JS highlighter;
  copy button via `leptos-use` clipboard), `thaw` Toolbar + Select;
  timeline strip; `AUDIT_DETAILS` card per version
  (committer, time, change type, description — RM 1.2.0 types via
  `openehr-rm`).
- States: format 406 (a representation the CDR declines) renders the
  CDR's supported-set diagnostic; deleted version = tombstone state.
- E2E: **J5** asserts the JSON ⇄ XML toggle round-trip (both render,
  content-bearing) and the version dropdown switches content; FLAT
  render asserted on the corpus composition.

### 7A.10 `/system` — System panel

```
┌───────────────────────────────────────────────────────┐
│ System                                                │
│ ┌ Status ───────────┐ ┌ SMART ─────────────────────┐  │
│ │ ● UP  CDR v3.1.1  │ │ enabled ✓  platform: …     │  │
│ │ PG 18.4 · uptime  │ │ auth endpoints ↗ jwks ↗    │  │
│ └───────────────────┘ └────────────────────────────┘  │
│ ┌ Served OpenAPI ───────────────────────────────────┐ │
│ │ (rendered from the CDR's own /…/openapi.json)     │ │
│ └───────────────────────────────────────────────────┘ │
│ ┌ Scope previewer ──────────┐ ┌ Activity log ──────┐  │
│ │ [scope string ______ ] ▶  │ │ ATNA events table  │  │
│ └───────────────────────────┘ └────────────────────┘  │
└───────────────────────────────────────────────────────┘
```

- Server fns: `fetch_status` (shared with the shell), `fetch_smart_config`
  (GET `/.well-known/smart-configuration`; 404 = SMART disabled — that IS
  the status, render "disabled"), `fetch_openapi` (the CDR's natively
  served `openapi.json` — rendered as a grouped endpoint list by our own
  component, **not** Swagger-UI-in-an-iframe), `preview_scope` (master08
  grammar — **§7.4's open design point stands**: needs the grammar lifted
  out of `ehrbase-rest` or a CDR debug endpoint; the tile ships only when
  that lands, behind a feature gate until then), `fetch_activity_log`
  (the CDR's system-log read surface, read-only).
- States: SMART-disabled and log-endpoint-absent are first-class rendered
  states, not errors.
- E2E: **J8** (status card content, SMART tile in both enabled and
  disabled compose variants, OpenAPI list renders > 0 endpoint groups).

### 7A.11 Screen → journey traceability

| Screen | Server fns (BFF surface) | Proven by |
|---|---|---|
| Shell | `fetch_status`, `current_session`, `logout` | all, J2 |
| `/login` | `login_basic`, OIDC callback | J1, J6, J7 |
| `/` dashboard | `dashboard_counts`, `query_group_counts`, `commit_trend` | J8 |
| `/templates` | `list_templates`, `upload_template` | J3 |
| `/templates/{id}` | `fetch_template_opt`, `fetch_webtemplate`, `fetch_example` | J3 |
| `/queries` | `list_stored_queries`, `store_query`, `run_stored_query` | J4 |
| `/queries/builder` | `validate_aql`, `run_aql`, `export_*` | J4 |
| `/ehrs` | `find_ehr`, `list_ehrs` | J5 |
| `/ehrs/{id}` | `fetch_ehr`, `fetch_ehr_status`, `fetch_directory`, `list_compositions`, `list_contributions` | J5 |
| `…/compositions/{uid}` | `fetch_composition`, `fetch_versions`, `fetch_at_version` | J5 |
| `/system` | `fetch_smart_config`, `fetch_openapi`, `fetch_activity_log`, (`preview_scope`) | J8 |

Every server fn in this table is the **complete** BFF API — a new fn means
a catalog update in the same change.

---

## 8. Packaging — a third OCI image (follows the shipped compose/Helm/CI pattern)

Follow the existing distroless pattern exactly; the console is a normal Rust
binary with embedded/served assets.

- **Image:** `ghcr.io/rubentalstra/ehrbase-rs-admin-ui` (matches the
  `ehrbase-rs` / `ehrbase-rs-postgres` naming).
- **Build:** `cargo-leptos` produces the server binary + WASM/CSS assets; a
  builder stage runs it, a `gcr.io/distroless/cc-debian12:nonroot` runtime stage
  serves it. Same toolchain pin single-sourced from `rust-toolchain.toml`.
  WASM release profile per the Leptos book (`deployment/binary_size`):
  `[profile.wasm-release]` `opt-level='z'` + `lto` + `codegen-units=1`, wired
  via `lib-profile-release`; the server serves compressed WASM
  (`tower-http` compression is already in the stack).
- **Config** *(rev. 2026-07-17)*: follow the CDR's current convention — **one
  TOML file** (the CDR moved from the `EHRBASE_*` env matrix to a single
  `ehrbase.toml` in the 2026-07 configuration redesign), with env overrides
  for container use. Minimally: CDR base URL, auth mode/credentials or OIDC
  issuer, bind address. **No DB** — it is stateless bar its own session store.
- **Healthcheck:** a `healthcheck` subcommand (no shell in distroless), as the
  server binary already does.
- **Compose:** add an `ehrbase-admin-ui` service to the quickstart, `depends_on`
  the CDR healthy; document the CDR-URL env.
- **CI:** a matrix binary build + COPY-only multi-arch packaging, cloned from the
  app-image job in `containers.yml`.

---

## 8b. Engineering standards & testing (added 2026-07-13, from the full Leptos-book read)

The complete Leptos book (leptos-rs/book `main`, ~60 chapters) was read
2026-07-13 and distilled into standing project governance, so UI work follows
best practice from the first line of code:

- **`.claude/rules/leptos-ui.md`** — the hard-rules file (owner mandates,
  feature/crate discipline, reactivity, components, views/`<For>` keys, forms,
  async/BFF, server functions, **hydration hard rules**, router, testing
  gates, deferred-islands policy). Every UI-touching session reads it.
- **Agents:** `ui-implementer` (bounded UI tasks, done-gate = both-target
  clippy + nextest + leptosfmt + cargo-leptos build) and `leptos-reviewer`
  (read-only diff review against the rule file — the UI counterpart of
  `spec-conformance-reviewer`).
- **Skills:** `/leptos-lookup` (route a question to the owning book chapter,
  cached clone — the UI counterpart of `/spec-lookup`; the book is the oracle,
  never memory) and `/ui-gates` (the full quality-gate battery, including the
  wasm32 clippy pass that catches server-only deps leaking past the `ssr`
  feature gate).

**Testing approach** (book `testing` chapter + the no-JS mandate):
business logic — query-builder AST assembly, criteria validation, the OPT
path catalog — lives in plain, component-free types with ordinary unit tests;
components stay thin. Browser-level tests use `wasm-bindgen-test`
(`mount_to` + `tick().await`); end-to-end tests are **Rust-native and
merge-gating** — the full design is **§8d** — not Playwright, which would
put JavaScript in the repo against the mandate.

## 8c. CLAUDE.md edits (docs-verified 2026-07-13)

Verified against the official Claude Code memory documentation
(code.claude.com/docs/en/memory) before acting — the load mechanics matter:

- **Nested `CLAUDE.md` in subdirectories is supported and loads on demand**
  ("included when Claude reads files in those subdirectories"), not at
  launch. Ancestor files load in full at launch. After `/compact`, only the
  root is re-injected automatically; nested files reload on next file-read.
- **Consequence:** repo-wide hard rules stay in the root `CLAUDE.md` (always
  loaded, compaction-safe); crate-local discipline goes in per-crate files.
  Official size guidance: under ~200 lines per file.
- `.claude/rules/*.md` supports `paths:` frontmatter (this repo already uses
  it); `.claude/rules/leptos-ui.md` is scoped
  `paths: ["app/ehrbase-admin-ui/**"]` so the UI rulebook loads only when UI
  files are touched.

**Executed 2026-07-13 (repo-wide, not only the console):** every existing
crate got a nested `CLAUDE.md` — `app/{ehrbase,ehrbase-rest,ehrbase-sm}`
*(historical note: `ehrbase-sm` was deleted in the 2026-07-16 consolidation)*,
`crates/openehr-{base,rm,am,term,lang,its,query,flat,codegen,derive}`,
`tools/{conformance,benchmark}` — each ~20–35 lines: crate role,
generated-vs-hand-written split, never-do rules, gates, pointers (backticked
paths only — `@` would import-at-launch and defeat the on-demand scoping).
The root `CLAUDE.md` gained a "Layered memory" section defining the split
and now names the new UI agents.

**Still to do when the console lands (same PR as the scaffold):**

1. Create `app/ehrbase-admin-ui/CLAUDE.md` (~25 lines): crate role (BFF over
   ITS-REST), the three §0 mandates (no-JS, REST boundary, server-fns-are-
   public-auth), pointer to `.claude/rules/leptos-ui.md` + `/ui-gates` +
   `/leptos-lookup`, and the both-targets gate battery. (The directory
   cannot be created before the crate: `app/*` is a workspace glob — a
   manifest-less dir under it breaks `cargo metadata`.)
2. Root `CLAUDE.md`: extend the repo-map `app/*` bullet from "three crates"
   to four, adding one line for `ehrbase-admin-ui`; add the UI gate battery
   to "Build and test" if the console joins the standard workspace gates.
3. **Root-trim follow-up (owner review required):** with per-crate files in
   place, the root's crate-by-crate detail (compile-status paragraph, parts
   of the codegen section) can migrate down to shrink the root toward the
   official ~200-line guidance. Not done unilaterally — the root is the
   always-loaded safety net, and every removal must be verified as covered
   by a nested file first.

## 8d. End-to-end testing — Rust-native, merge-gating (owner mandate 2026-07-13)

The test pyramid's top layer: drive the **real composed stack** with a **real
browser** and assert full user journeys. Unit tests can't see hydration; wasm
tests can't see the BFF↔CDR path; only E2E sees the product. It is a
**gate**, not a nice-to-have.

### Stack (all Rust / declarative — zero JS in the test suite)

| Concern | Choice | Why |
|---|---|---|
| Browser driver | **`thirtyfour`** (0.37.2, 2026-07-05 — verified live; earlier drafts said "0.31 built on fantoccini", both stale: current releases carry no fantoccini dependency) | High-level Rust WebDriver client with Selenium-style ergonomics; same W3C WebDriver protocol family as the `fantoccini` the Leptos book's own e2e example uses. |
| Browser | Headless Chromium + chromedriver (geckodriver as the cross-check, non-gating) | Standard WebDriver endpoints; no vendored browser tooling. |
| Test runner | `cargo nextest` — journeys are plain `#[tokio::test]`s in `app/ehrbase-admin-ui/tests/e2e_*.rs` | Follows `testing.md` (tests live in the owning crate); no new runner. `cucumber-rs` (Gherkin journeys) is an optional later layer if owner-readable specs are wanted — pure Rust too. |
| Stack under test | `scripts/ui-e2e.sh`: docker compose up **postgres + ehrbase + ehrbase-admin-ui + Keycloak** → chromedriver → nextest → teardown | Mirrors the proven `scripts/conformance.sh` pattern exactly. Keycloak is in the compose because auth v1 is dual (§10) — the OIDC journey is gated, not deferred. |
| Screenshots | `thirtyfour` `screenshot()` per journey step → `target/ui-e2e/screenshots/j{NN}-{step}-{slug}.png`, uploaded as the `ui-e2e-screenshots` CI artifact | **Owner decision 2026-07-17:** every journey saves step screenshots for human review; **no pixel-diff assertions** (they flake). On failure, an additional full-page capture + the DOM snapshot land next to it. |

**Skip-with-reason seam** (the B4 `--tx-server-url` precedent): the e2e tests
read `UI_E2E_BASE_URL` (+ `UI_E2E_WEBDRIVER_URL`); when unset they skip with
a printed reason, so a plain `cargo nextest run --workspace` without Docker
stays green. The CI job always sets them — skipping is impossible in the
gate.

### The v1 journey matrix (each row merge-gating) *(expanded 2026-07-17b)*

Journey ids (J1–J8) are the same ones the §7A screen catalog traces to.
Fixture setup is **REST-only** (the boundary, §5.2): the harness seeds the
CDR through its own API in a `#[ctor]`-free explicit setup fn per journey
file — never through the database.

| # | Journey | Key assertions | Fixtures |
|---|---|---|---|
| J1 | **Login, both modes**: Basic form → dashboard; OIDC → Keycloak redirect → code exchange → session | dashboard URL reached; session cookie `HttpOnly`+`SameSite`; user menu shows identity; wrong password → MessageBar error, no session | Keycloak realm import `tests/fixtures/keycloak-realm.json` (one admin user, the console client, the CDR audience); Basic creds from the compose env |
| J2 | **Hydration proof**: after first paint, toggle the theme + open/close the nav drawer | DOM class actually flips post-interaction (proves WASM attached, not just SSR HTML); zero console errors | none (shell only) |
| J3 | **Template Manager**: upload OPT → list row appears → open detail → path catalog | upload succeeds (201); row visible without manual reload (resource refetch); WT tree renders; node inspector shows the known `aqlPath` + `rmType` of a fixture node; duplicate re-upload → 409 surfaced in-dialog | 2 corpus OPTs in `app/ehrbase-admin-ui/tests/fixtures/opt/`: one single-archetype vitals, one multi-archetype (IPS-class) — taken from the vendored conformance template set |
| J4 | **Query Builder end-to-end**: pick template → tick path → DV_QUANTITY range criterion → run → rows; save as stored query; re-run from `/queries`; AQL mode validate | AQL preview contains `CONTAINS COMPOSITION` + the path; RESULT_SET table shows the seeded row count; stored query GET-able via the CDR after save (asserted over REST, not just UI); invalid AQL in raw mode → 400 diagnostic rendered | J3's templates + 3 seeded compositions with known magnitudes (committed over REST in setup) |
| J5 | **EHR + composition browser**: find seeded EHR → status tab → compositions tab → open composition → **JSON ⇄ XML ⇄ FLAT toggle** → version dropdown | all format tabs render content-bearing output (JSON has `"_type": "COMPOSITION"`, XML has the `http://schemas.openehr.org/v1` namespace, FLAT has the flat-key prefix); v1→v2 switch changes displayed content; audit card shows committer | 1 seeded EHR, 1 composition committed then updated (2 versions) over REST |
| J6 | **Progressive enhancement**: fresh profile with JavaScript disabled → Basic login via plain form POST → a `<ActionForm>` mutation (template upload) still works | login + upload succeed with JS off (server-rendered redirect flow) — the Leptos degradation contract (§5.1) holds | J1 + J3 fixtures |
| J7 | **Auth discipline**: unauthenticated deep link → login redirect with `next=`; post-login lands on the deep link; a low-scope OIDC user hits a 403 surface | redirect chain exact; forbidden state renders the "insufficient scope" surface (§7A states), not a blank page or raw error | second Keycloak user with reduced scopes in the same realm fixture |
| J8 | **Dashboard + system panel**: stat tiles numeric; trend chart SVG present; `/system` status card, SMART tile (both compose variants: enabled + disabled), served-OpenAPI list | tile values match the seeded corpus counts; SMART tile renders "disabled" on the 404 variant and endpoint links on the enabled one; OpenAPI component lists > 0 endpoint groups | J4/J5 seeds; a second compose profile flag flipping the CDR's SMART config |

**Standing assertions on every journey:**

- read the browser console log (WebDriver `goog:loggingPrefs`) and **fail on
  any hydration error or panic** — the cheapest possible detector for the
  §8-class bugs, applied everywhere;
- screenshot each numbered step (`j{NN}-{step}-{slug}.png`, the §8d stack
  table row) — CI uploads the folder as the `ui-e2e-screenshots` artifact
  on success *and* failure;
- explicit waits on elements/conditions only — a bare `sleep` in a journey
  is a review-rejected defect (flake discipline below).

### Gating (how it's enforced)

- **CI job `ui-e2e`** in the main workflow: path-filtered to
  `app/ehrbase-admin-ui/**` (plus the compose/e2e script paths), **required
  for merge** on PRs touching them, and always run on release tags. Runs
  `scripts/ui-e2e.sh` on the standard runner (Docker + chromium +
  chromedriver are stock on `ubuntu-latest`-class runners). Uploads
  `target/ui-e2e/screenshots/` as the **`ui-e2e-screenshots` artifact on
  every run** (success and failure) for human review — the owner-decided
  substitute for pixel-diff assertions (2026-07-17). Chromium gates;
  the geckodriver cross-check job is `continue-on-error` (informational).
- **`/ui-gates` skill** gains the E2E battery as its final stage (runs when
  Docker is available; reports SKIPPED(no docker) locally, never in CI).
- **`.claude/rules/leptos-ui.md` §10** lists the e2e gate; the
  `ui-implementer` agent's done-definition includes it for journeys its
  change touches.
- **Flake discipline = `testing.md`:** a flaky journey is fixed (explicit
  waits on elements/conditions — never `sleep`), **never** `#[ignore]`d,
  retried-by-default, or deleted to get green.

## 8e. Published page screenshots — the website shows every screen *(owner mandate 2026-07-17)*

People evaluating the console want to **see** it. The operator docs
(`website/book`) therefore carry **one canonical screenshot per §7A route
screen**, and a machine-enforced rule keeps them from rotting.

### Capture (a deterministic harness pass, not hand-made)

- After J1–J8, `scripts/ui-e2e.sh --docs-shots` runs a dedicated capture
  pass over the same seeded compose stack: **fixed viewport 1440×900, light
  theme, the same corpus fixtures** — one full-page PNG per §7A screen
  (login, dashboard, templates, template-detail, queries, query-builder,
  ehrs, ehr-detail, composition-viewer, system — 10 shots; the shell is
  visible in all of them), slug-named after the route.
- Output is **committed** at `website/book/src/admin-ui/img/{slug}.png` —
  never hand-cropped, never mocked up; what ships is what the harness saw.
- The book gains an **Admin console** chapter: one page per screen embedding
  its screenshot with operator-facing text derived from the §7A entry.

### The refresh rule (machine-enforced — the changelog-guard pattern)

- **CI job `ui-screenshot-guard`** (lands with the console in the §12 PR,
  same shape as `changelog-guard`): a PR that touches
  `app/ehrbase-admin-ui/{src,style}/**` must **also** touch
  `website/book/src/admin-ui/img/**` — i.e. UI change ⇒ screenshots
  re-captured in the same PR (`--docs-shots` + commit). Escape hatch: the
  **`no-ui-visual-change`** label for genuinely invisible changes (BFF-only
  logic, comments, tests) — mirroring `no-changelog`.
- The guard checks **freshness, not pixels** — consistent with owner
  decision 6 (§10): screenshots are for humans; pixel-diff assertions flake
  and stay banned. Review of the refreshed images happens on the PR (they
  render inline in the GitHub diff).
- Rule recorded in `.claude/rules/leptos-ui.md` §10 (testing gates) and in
  the `ui-implementer` done-definition: a visual change ships with its
  re-captured screenshots.

## 9. Risks & honest tradeoffs

- **Pre-1.0 ecosystem.** Leptos (→1.0), `thaw`, `leptos-struct-table`,
  `leptos-chartistry` all still ship breaking changes across minors. Pin every
  version, expect an occasional upgrade tax. This is the real cost of the
  Rust-only mandate — accepted deliberately.
- **`thaw` is pinned to the 0.5 beta** (`0.5.0-beta`, targets Leptos 0.8.5) —
  a deliberate owner decision (2026-07-13), because stable 0.4.8 targets Leptos
  0.7.7 and cannot run on our 0.8 stack. Accepted risk: a pre-release UI kit may
  carry breaking changes / rough edges. Mitigation: pin an **exact** beta build,
  and be ready to vendor the handful of components we depend on if a beta
  regresses. Re-pin to stable 0.5 once it ships.
- **Thin widgets for rich interactions.** The **Query Builder** is the hard part:
  no off-the-shelf Rust "visual query builder" exists, so its drag/click surface
  is bespoke (on top of `openehr-query` for correctness). Budget real UI work
  here — it is the feature the owner most wants and the one with the least
  ecosystem support.
- **SSR complexity (hydration).** Full-stack Leptos + hydration is more moving
  parts than a plain SPA, and hydration mismatches are the classic failure mode
  (invalid HTML, `cfg!`-branched views, browser APIs at render time).
  Mitigation: the hydration hard-rules in `.claude/rules/leptos-ui.md` §8,
  enforced by the `leptos-reviewer` agent. Note: Leptos's `#[island]` mode (a
  distinct compilation mode, not a figure of speech) could cut the WASM bundle
  ~50%, but v1 uses **standard full hydration** — the beta widget kit plus a
  highly interactive console make islands the riskier start. Islands is a
  recorded, measured-later optimization (rules §11).
- **Not a JS-React admin.** If build speed and mature component/charting
  libraries ever outweigh the language-unity + `openehr-*` reuse benefits, a
  TypeScript admin over the same REST API would be lower-risk. The owner has
  chosen Rust-only; this doc honours that and records the tradeoff.

---

## 10. Decisions (confirmed by owner, 2026-07-13)

1. **Rendering:** **full-stack SSR** — one Leptos + `axum` 0.8 binary serves the
   UI and proxies to the CDR (§5); CDR tokens stay server-side; one OCI image.
2. **Location + name:** **in-workspace `app/ehrbase-admin-ui`**, reusing the
   `openehr-*` crates directly; one CI, one product release line.
3. **Auth (v1):** **Basic *and* OAuth2/OIDC from day one** — both ship in the
   first version (§5.4), not phased.
4. **Templates (v1):** **ADL 1.4 only**, matching the CDR's current Definition
   surface; ADL2 added when the CDR's ADL2 path lands.
5. *(2026-07-17)* **View specs:** full wireframe-level screen catalog (§7A) — routes,
   wireframes, component trees, server-fn → endpoint data tables, states;
   subagents build to it without layout guesses.
6. *(2026-07-17)* **E2E gating:** Chromium/chromedriver journeys merge-gate; **every
   journey saves step screenshots as CI artifacts** (human review — no
   pixel-diff assertions); geckodriver cross-check stays non-gating.
7. *(2026-07-17)* **Delivery:** **one big-bang PR** on `claude/admin-ui` — the owner's
   standing single-convergence style; the E2E harness lands inside that
   same PR (§12).
8. *(2026-07-17)* **Tracking + prior art:** registered as the `ADMIN-UI` row in
   `docs/plans/WORKLIST.md` with this file as the governing plan (deleted
   in the implementing PR); Cabolabs EHRServer is an **idea source only —
   never a 1:1 port** (revision block item 6).
9. *(2026-07-17)* **Website screenshots:** every §7A screen gets a canonical,
   harness-captured screenshot published in the `website/book` admin-console
   chapter, with the **`ui-screenshot-guard`** CI job enforcing re-capture on
   any UI-touching PR (§8e; escape label `no-ui-visual-change`).

---

## 11. Same-PR obligations (land inside the §12 PR, not after)

*(rev. 2026-07-17b — the ADR bullet is gone: the ADR layer was abolished
2026-07-17; this doc is the plan and is deleted at close.)*

- `CHANGELOG.md` `[Unreleased]` entry + the `website/book` **admin-console
  chapter**: an operator page (deploy, config, auth setup) plus **one page
  per §7A screen embedding its harness-captured screenshot** (§8e) — all
  standing same-PR rules for user-visible surfaces.
- `containers.yml` + the quickstart compose gain the third image/service
  (§8); Helm chart addition if the owner wants it charted at v1 —
  compose-only otherwise (ask at convergence).
- The §8c CLAUDE.md edits: `app/ehrbase-admin-ui/CLAUDE.md` + the root
  repo-map bullet ("three crates" → four) + the gate battery mention.
- **Acknowledgement in the root `README.md`.** Add a credit that the admin
  console's feature set (Template Manager, point-and-click Query Builder,
  saved/grouped/cohort queries) is **inspired by
  [Cabolabs EHRServer](https://github.com/ppazos/cabolabs-ehrserver)** by Pablo
  Pazos / CaboLabs Health Informatics (Apache-2.0). We reimplement the *UX*
  fresh in Rust over our own AQL engine — **no code is copied, nothing is a
  1:1 port** (owner rule, revision block item 6) — but the design lineage is
  credited.
- Worklist close: `docs/PROGRESS.md` entry, `ADMIN-UI` row → Closed table
  with the PR link, **this file deleted** — all in the same PR.

---

## 12. Build plan — Fable 5 orchestrates, subagents implement *(added 2026-07-17b)*

**Delivery shape (owner decision 7, §10): ONE PR** on branch
`claude/admin-ui`, single convergence at the end — no intermediate stubs,
nothing deferred. The plan below is the work order *inside* that one PR.

### 12.1 Roles (per the root `CLAUDE.md` orchestration section)

| Role | Who | Owns |
|---|---|---|
| **Orchestrator** | Fable 5 (this session tier), effort `high` | Architecture + all design judgement; the **BFF auth core** (session, Basic + OIDC flows, the server-fn auth guard — §5.4); the **query-builder state → `openehr-query` AST lowering** (§7A.6, the correctness core); screen-catalog conformance review of everything the workers return; **every cargo invocation** (one `./target`, owner rule — subagents never build); the single convergence. |
| **`ui-implementer`** (Opus) | max **2 concurrent** (owner cap) | Bounded screen/component tasks from the §7A catalog: each prompt carries the catalog section, `.claude/rules/leptos-ui.md`, the relevant spec paths, and the fixture list. Delivers code only; the orchestrator runs the gates. |
| **`leptos-reviewer`** (read-only) | after each subsystem lands on the branch | Diff review against `.claude/rules/leptos-ui.md` (no-JS, REST boundary, server-fn auth, hydration safety, `<For>` keys, form/async idioms). Findings fixed before the next subsystem starts. |
| **`spec-researcher`** | on demand | Any "what does ITS-REST say" question (negotiation, status codes, versioned-object semantics) — answered from `docs/specs/openehr/` with citations, never memory. |

Standing discipline for every subagent prompt: hard rules travel with the
task (no re-exports, no `use X as Y`, deny-tier lints, TODO-form comments,
`urlencoding` for percent codecs, **Cabolabs = ideas only, never code**);
`/leptos-lookup` for any Leptos question the rule file doesn't settle;
`/spec-lookup` before touching any wire-facing format.

### 12.2 Work order (W0 → W6, inside the one PR)

| Stage | Work | Executor |
|---|---|---|
| **W0 — scaffold + risk retirement** | `/crate-scaffold` for `app/ehrbase-admin-ui` (+ nested `CLAUDE.md`, §8c); cargo-leptos config (Tailwind v4.3.3 pin, `wasm-release` profile §8); **thaw `=0.5.0-beta` smoke test against leptos 0.8.20** — a page with the components §7A actually uses (Layout, NavDrawer, Table, Tree, Tabs, Upload, MessageBar, Skeleton). Beta fails → pin the git rev of main **now**, before any screen work. | Orchestrator |
| **W1 — BFF core** | Session store (`tower-sessions`), Basic + OIDC login flows, the auth guard every server fn calls, the CDR `reqwest` client (base-URL config via the console's TOML, §8), error normalization, content-negotiation helper (§5.3 `Accept` matrix). | Orchestrator (auth is the risk center) |
| **W2 — shell + login + system panel** | §7A.0 shell, §7A.1 login, §7A.10 system panel (minus scope-previewer, feature-gated per §7.4's open point). | 2 × `ui-implementer` in parallel (shell+login / system) |
| **W3 — browse surfaces** | §7A.3/7A.4 Template Manager + detail; §7A.7/7A.8/7A.9 EHR finder, EHR detail, composition viewer (the shared format-viewer component built once, in the template-detail task, reused). | 2 × `ui-implementer`, two sequential pairs |
| **W4 — Query Builder + queries + dashboard** | Orchestrator: the builder-state model + AST lowering + its exhaustive unit tests (component-free, §8b). Workers: the §7A.6 widget surface (criteria widgets per `DV_*`, stepper, result pane), §7A.5 stored queries/groups, §7A.2 dashboard tiles. | Orchestrator core + 2 × `ui-implementer` |
| **W5 — E2E harness + journeys** | `scripts/ui-e2e.sh` (compose: postgres + ehrbase + console + Keycloak), realm + OPT fixtures, J1–J8 (§8d matrix), screenshot plumbing, the `--docs-shots` capture pass (§8e), the `ui-e2e` CI job + `ui-e2e-screenshots` artifact upload, and the `ui-screenshot-guard` CI job. | Orchestrator (harness pattern = `conformance.sh`); journey bodies may fan to `implementer` |
| **W6 — convergence** | The full battery in order: `/ui-gates` (both-target clippy, nextest, leptosfmt+fmt, cargo-leptos build) → workspace gates (`clippy --workspace --all-targets`, `nextest run --workspace`, fmt, audit/deny) → `scripts/ui-e2e.sh` full J1–J8 → `leptos-reviewer` whole-crate pass → §11 obligations (changelog, book page, compose/CI, README credit, CLAUDE.md edits) → PROGRESS entry + worklist close + **delete this file** → PR. | Orchestrator |

Reviewer cadence: `leptos-reviewer` after W2, W3, W4 (scoped diffs), and the
whole crate at W6 — findings block the next stage, mirroring how
`spec-conformance-reviewer` gates CDR subsystems.

### 12.3 Done-definition (the PR merges when ALL hold)

1. `/ui-gates` green: clippy native **and** wasm32, nextest, leptosfmt +
   cargo fmt, cargo-leptos release build.
2. Workspace gates green (the console is a member: `--workspace` clippy +
   nextest + fmt + audit/deny pass with it in).
3. **J1–J8 all green under `scripts/ui-e2e.sh`** with zero
   console-log-detected hydration errors and the screenshot artifact
   populated.
4. `leptos-reviewer` whole-crate pass with no unresolved findings;
   spot-check that no Cabolabs code/markup was ported (idea-source rule).
5. Every §7A server fn implemented, auth-guarded, and listed in the §7A.11
   catalog (catalog drift = a finding).
6. §11 obligations all landed in the PR (changelog, book chapter, compose +
   `containers.yml`, README credit, CLAUDE.md edits, PROGRESS + worklist
   close, this file deleted).
7. The §8e screenshot set captured by `--docs-shots` and committed under
   `website/book/src/admin-ui/img/` (all 10 screens), embedded in the book
   chapter, with the `ui-screenshot-guard` job live in CI.

---

## Appendix A — Cabolabs EHRServer feature inventory (read from source, 2026-07-13)

Read directly from the Grails/Groovy source (`grails-app/{controllers,domain,services,views,jobs}`).
Grouped by area; **bold = the features the owner called out**. The right column
is the §7 disposition.

### A.1 Clinical data management
| Feature | Source | Disposition |
|---|---|---|
| openEHR versioned compositions (CONTRIBUTION / VERSIONED_COMPOSITION / VERSION / AUDIT_DETAILS) | `change_control/*`, `VersionedCompositionController`, `ContributionController` | **Adopt** (browse/audit views over REST) |
| EHR management (create/list/show, EHR_STATUS) | `Ehr`, `EhrController` | Adopt |
| Directory / folders + **folder templates** | `Folder`, `FolderTemplate*`, `FolderController` | Adopt (browser) |
| Composition UI viewer (`showCompositionUI`, `showComposition`) | `EhrController` | Adopt (JSON/XML viewer) |
| Data-value indexing — per-datatype index tables | `DataValueIndex` + 18 `Dv*Index` classes, `DataIndexerService`, `IndexDataJob` | **Defer** — our node store + AQL engine replaces it |

### A.2 Templates
| Feature | Source | Disposition |
|---|---|---|
| **Template Manager**: upload OPT, activate/deactivate, list, show | `OperationalTemplateController` (`upload/activate/deactivate/items/archetypeItems/generate`) | **Adopt** |
| OPT → queryable **path index** (archetypeId + path + rmType) | `OperationalTemplateIndex*`, `ArchetypeIndexItem`, `OperationalTemplateIndexerService/Job` | **Adapt** — we walk the OPT with `openehr-am` + BMM RM model (no index table) |
| OPT storage backends (filesystem / S3) | `OptFSService`, `OptS3Service` | Defer (CDR storage) |

### A.3 Query Builder (the headline)
| Feature | Source | Disposition |
|---|---|---|
| **Point-and-click builder**, no programming | `QueryController`, `views/query`, `QueryTagLib`, `OptGuiTagLib` | **Adapt → emits AQL** |
| Builder AJAX flow: template → archetypes → paths → criteria spec | `getTemplateJson`, `getArchetypesInTemplate`, `getArchetypePaths`, `getCriteriaSpec` | **Adapt** (§7.2) |
| Typed criteria per RM datatype | `DataCriteria*` (one `DataCriteriaDV_*` per DV type) | **Adapt** → typed AQL WHERE widgets |
| Complex boolean WHERE (AND/OR tree) | `DataCriteriaExpression` (binary-tree encoding) | Adapt → `openehr-query` AST |
| Query type: compositions vs data points | `Query.type` (`composition`/`datavalue`), `DataGet` (SELECT) | Adapt → AQL SELECT shape |
| Grouping: none / composition / path | `Query.group` | Adapt → table vs chart series |
| Save / name (i18n) / edit / delete / **export** queries | `Query`, `QueryController` | **Adopt** (stored queries) |
| **Query Groups** — dashboard of match counts | `QueryGroup.executeCount` (async parallel) | **Adopt** |
| **EHR / cohort queries** — match a set of conditions; boolean or EHR set | `EhrQuery`, `EhrQueryController` | **Adopt** → AQL over EHRs |
| Query sharing | `QueryShare` | Adopt (nice-to-have) |
| SNOMED-CT expression constraints | `QuerySnomedService`, `validateSnomedExpression` (external SNQUERY) | **Defer** → our AQL `TERMINOLOGY()` family (CDR-side, no open worklist row) |
| Raw query escape hatch | `QueryController.hql` | Adopt → raw AQL editor |

### A.4 Platform / operations (mostly CDR- or Stage-2-side)
| Feature | Source | Disposition |
|---|---|---|
| **XML + JSON** everywhere, XML-schema validation | `XmlService`, `JsonService`, `XmlValidationService` | **Adopt** (via `openehr-its`) |
| REST API + interceptor + token auth | `RestController`, `RestInterceptor`, `RestAuthController`, `api/*_Insomnia.json` | We *consume* the CDR's ITS-REST instead |
| **Full audit / activity log** + error log | `ActivityLog*`, `ErrorLog`, `LogService` | Adopt (read-only view over ATNA System Log) |
| Dashboard + usage stats (repo usage, templates loaded) | `StatsController`, `views/stats` | **Adopt** |
| RBAC: users / roles / request-map URL perms | `User`/`Role`/`UserRole`/`RequestMap`, `Auth*` | CDR auth (Stage-1 authn) + Stage-2 RBAC |
| Multitenancy: organizations / accounts / plans / API keys / billing | `Organization`/`Account`/`Plan`/`PlanAssociation`/`ApiKey` | **Defer** (Stage-2 enterprise) |
| Instance sync / replication (VNA master-replica) | `SyncController`, `SyncLog`, `Sync*Service`, `SyncJob` | Defer (CDR extension, no spec) |
| Notifications / remote webhooks | `Notification*`, `RemoteNotificationsService`, `NotificationJob` | Defer (CDR eventing extension) |
| Commit-log & version blob repos (FS/S3) | `CommitLogger{FS,S3}Service`, `Version{FS,S3}RepoService` | Defer (CDR storage) |
| Runtime configuration + terminology ids | `ConfigurationService`, `ConfigurationItem`, `TerminologyId` | Adopt (read-only config view) |
| Multitenant vendor-neutral archive | README "Vendor Neutral Archive" | Defer (CDR) |

**Net:** the console's own scope is A.1–A.3 (browse, template manager, query
builder + saved/grouped/cohort queries, audit views, XML/JSON) — all over
ITS-REST. A.4's platform machinery is CDR-side or Stage-2 and is *consumed*
(auth, audit, stats, config), not rebuilt.

---

*Sources for the version/tooling facts above (first verified 2026-07-13;
**all crate pins re-verified 2026-07-17** against the crates.io API
(`max_stable_version` + per-version dependency reqs) and Tailwind against
the `tailwindlabs/tailwindcss` GitHub releases API — v4.3.3, 2026-07-16):*
[Leptos vs Dioxus 2026](https://rustify.rs/articles/leptos-vs-dioxus-rust-frontend-2026) ·
[Leptos 0.8](https://github.com/leptos-rs/leptos/releases/tag/v0.8.0) ·
[Dioxus 0.7](https://dioxuslabs.com/blog/release-070/) ·
[cargo-leptos](https://github.com/leptos-rs/cargo-leptos) ·
[thaw](https://github.com/thaw-ui/thaw) ·
[leptos-struct-table](https://docs.rs/leptos-struct-table) ·
[leptos-chartistry](https://github.com/feral-dot-io/leptos-chartistry) ·
[wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) ·
[Cabolabs EHRServer](https://github.com/ppazos/cabolabs-ehrserver)
