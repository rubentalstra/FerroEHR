# `ehrbase-admin-ui` — a pure-Rust admin console over the ITS-REST API

- **Status:** design — framework + ecosystem selection, architecture, feature
  map. **Not an ADR** (owner instruction 2026-07-13); an ADR follows only after
  this design is approved.
- **Date:** 2026-07-13 · **revised 2026-07-17** (see the revision block below)
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
3. **The served OpenAPI is the server's own** (ADR-005 amendment,
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

| Concern | Choice | Version (verified live 2026-07-13) | No-JS status |
|---|---|---|---|
| UI framework | `leptos` (SSR/full-stack) | 0.8.20 | Rust → WASM |
| Build / bundler | `cargo-leptos` | 0.3.7 (2026-07-03) | Builds both binaries (native server + WASM client); **bundles the Tailwind standalone binary — no Node/npm**. ([cargo-leptos](https://github.com/leptos-rs/cargo-leptos)) |
| Styling | Tailwind CSS v4 (via `cargo-leptos` standalone) | v4.2.x (pin with `LEPTOS_TAILWIND_VERSION`) | CSS, no JS |
| Component kit | [`thaw`](https://github.com/thaw-ui/thaw) (Fluent-design) | **`0.5.0-beta`** — leptos req `^0.8.0` (crates.io index) | Rust → WASM. **Owner decision (2026-07-13): use the 0.5 beta.** The "stable" 0.4.8 is pinned to Leptos **^0.7.7** and is *not* 0.8-compatible, so the beta is the only 0.8 line. Published 2025-08-03 — nearly a year old; the main branch is the active 0.8 line. Pin the newest `0.5.0-beta.N` published at scaffold time; fall back to a pinned git rev of main only if the beta fails against Leptos 0.8.20. (Leptonic is stale — last release Feb 2024 — **do not use**.) |
| Data grid / tables | [`leptos-struct-table`](https://docs.rs/leptos-struct-table) | 0.19.0 (2026-06-23), leptos `^0.8`, leptos-use `^0.19` | Rust → WASM. Async data from a REST source, virtualization, pagination, multi-column sort, column hide/reorder, headless (our CSS). Exactly the RESULT_SET / EHR-list widget. |
| Charts | [`leptos-chartistry`](https://github.com/feral-dot-io/leptos-chartistry) | 0.2.3 (2026-01-23), leptos `^0.8` | **Pure Rust + SVG — "no JS, no canvas."** Has an SSR feature. Dashboard tiles. Note: depends on leptos-use `^0.18` while struct-table wants `^0.19` — cargo resolves both (pre-1.0 minors are distinct), at the cost of a duplicated leptos-use in the WASM bundle until chartistry bumps. Accepted. |
| Reactive utils | `leptos-use` | 0.19.0 (2026-06-22) | Rust → WASM (storage, debounce, clipboard, etc.) |
| Server→CDR HTTP | `reqwest` 0.13 (rustls) | workspace pin | Server-side only; the BFF's call into the CDR (§5). |

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
> server's own generated `openapi.json`** (ADR-005 as amended 2026-07-17: the
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
> **terminology family** (`TERMINOLOGY()` / `matches {uri}`), which is a **CDR**
> capability (blueprint row 12) — the builder gains a terminology-constraint
> widget only once/if that lands CDR-side. Recorded, not built at v1.

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
(`app/ehrbase-rest/src/smart/`; spec
`docs/specs/openehr/ITS-REST/docs/smart_app_launch/` master02–09; design
`docs/design/its-rest/smart.md`). What exists today, verified in source:

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

## 8. Packaging — a third OCI image (extends `container-images.md`)

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
crate got a nested `CLAUDE.md` — `app/{ehrbase,ehrbase-rest,ehrbase-sm}`,
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

**Skip-with-reason seam** (the B4 `--tx-server-url` precedent): the e2e tests
read `UI_E2E_BASE_URL` (+ `UI_E2E_WEBDRIVER_URL`); when unset they skip with
a printed reason, so a plain `cargo nextest run --workspace` without Docker
stays green. The CI job always sets them — skipping is impossible in the
gate.

### The v1 journey set (each one merge-gating)

1. **Login (Basic)** → dashboard renders; **login (OIDC)** → Keycloak
   redirect → code exchange → session established.
2. **Hydration proof:** after first paint, interact (click a counter/filter)
   and assert the DOM updated — proves WASM loaded and hydration attached,
   not just that SSR emitted HTML.
3. **Template Manager:** upload a corpus OPT → appears in the list →
   inspect its path catalog.
4. **Query Builder:** point-and-click template → archetype → path → typed
   criterion → run → RESULT_SET rows render in the table; save as stored
   query; re-run from saved.
5. **Composition browser:** open an EHR → composition → **JSON ⇄ XML
   toggle** round-trip renders both canonical forms.
6. **Progressive enhancement:** one journey with JavaScript disabled in the
   browser profile — `<ActionForm>` mutation still works via plain form
   POST + redirect (the Leptos degradation contract, §5.1).
7. **Auth discipline:** unauthenticated request → login redirect;
   insufficient scope → 403 surface renders.

**Standing assertion on every journey:** read the browser console log
(WebDriver `goog:loggingPrefs`) and **fail on any hydration error or panic**
— the cheapest possible detector for the §8-class bugs, applied everywhere.

### Gating (how it's enforced)

- **CI job `ui-e2e`** in the main workflow: path-filtered to
  `app/ehrbase-admin-ui/**` (plus the compose/e2e script paths), **required
  for merge** on PRs touching them, and always run on release tags. Runs
  `scripts/ui-e2e.sh` on the standard runner (Docker + chromium +
  chromedriver are stock on `ubuntu-latest`-class runners).
- **`/ui-gates` skill** gains the E2E battery as its final stage (runs when
  Docker is available; reports SKIPPED(no docker) locally, never in CI).
- **`.claude/rules/leptos-ui.md` §10** lists the e2e gate; the
  `ui-implementer` agent's done-definition includes it for journeys its
  change touches.
- **Flake discipline = `testing.md`:** a flaky journey is fixed (explicit
  waits on elements/conditions — never `sleep`), **never** `#[ignore]`d,
  retried-by-default, or deleted to get green.

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

---

## 11. Follow-ups (after approval — not started here)

- Write the ADR (owner said not yet) recording the framework + BFF decision.
- Scaffold `app/ehrbase-admin-ui` (workspace member, `openehr-*` deps only).
- Extend `container-images.md` + `containers.yml` + the quickstart compose.
- Build the E2E harness (§8d): `scripts/ui-e2e.sh` (compose stack incl.
  Keycloak + chromedriver), the `e2e_*.rs` journey tests, and the
  merge-gating `ui-e2e` CI job — landing with the first UI feature PR, not
  after.
- `CHANGELOG.md` `[Unreleased]` entry + a `website/book` operator page (both are
  standing same-PR rules for user-visible surfaces).
- **Acknowledgement in the root `README.md`.** Add a credit that the admin
  console's feature set (Template Manager, point-and-click Query Builder,
  saved/grouped/cohort queries) is **inspired by
  [Cabolabs EHRServer](https://github.com/ppazos/cabolabs-ehrserver)** by Pablo
  Pazos / CaboLabs Health Informatics (Apache-2.0). We reimplement the *UX*
  fresh in Rust over our own AQL engine — no code is copied — but the design
  lineage is credited. Land this in the same PR that introduces the console.

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
| SNOMED-CT expression constraints | `QuerySnomedService`, `validateSnomedExpression` (external SNQUERY) | **Defer** → our AQL `TERMINOLOGY()` family (CDR, blueprint row 12) |
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

*Sources for the version/tooling facts above (verified July 2026):*
[Leptos vs Dioxus 2026](https://rustify.rs/articles/leptos-vs-dioxus-rust-frontend-2026) ·
[Leptos 0.8](https://github.com/leptos-rs/leptos/releases/tag/v0.8.0) ·
[Dioxus 0.7](https://dioxuslabs.com/blog/release-070/) ·
[cargo-leptos](https://github.com/leptos-rs/cargo-leptos) ·
[thaw](https://github.com/thaw-ui/thaw) ·
[leptos-struct-table](https://docs.rs/leptos-struct-table) ·
[leptos-chartistry](https://github.com/feral-dot-io/leptos-chartistry) ·
[wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) ·
[Cabolabs EHRServer](https://github.com/ppazos/cabolabs-ehrserver)
