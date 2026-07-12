# System API (STABLE) — compliance design + gap register

Part of the ITS-REST per-specification audit ([`README.md`](README.md)),
ahead of the `ehrbase-rest` rewrite (W-3e). The System API is the smallest
ITS-REST specification: **one operation** — `OPTIONS /`, "System Options and
Conformance" — returning an `Options` capability/conformance manifest.

**Spec oracle** (read before any change):

- `docs/specs/openehr/ITS-REST/specifications/docs/system/Description.md`
  (the System API purpose — "service endpoints, resources and operations …
  that interact with the openEHR System API in a RESTful manner")
- `docs/specs/openehr/ITS-REST/computable/OAS/system-codegen.openapi.yaml`
  (the machine contract — the single `options` operation, the `Options`
  schema, the `Allow` header, the `200_options` response, `security: []`)
- Cross-refs the Overview spec (`docs/system/` links to `overview.html`); the
  blueprint records this as requirement **R32** (`docs/blueprint/05-its.md`
  §E, line 68).

**Contract facts from the OAS** (system-codegen.openapi.yaml):

- Server URL `https://{baseUrl}/v1`, path `/`, method `OPTIONS`,
  `operationId: options` (lines 41, 52-54).
- `security: []` (line 47) — the endpoint is **public** (no auth).
- Request: `Accept` header parameter constrained to `application/json`
  (`Accept_JSON`, lines 64-65, 71-78).
- Response `200_options` (lines 122-134): `Allow` header + `Content-Type:
  application/json` + an `Options` body.
- `Options` schema (lines 92-121): `solution`, `solution_version`, `vendor`,
  `restapi_specs_version`, `conformance_profile`, `endpoints: [string]`. The
  OAS `example` lists `endpoints: [/ehr, /demographic, /definition, /query,
  /admin]` and `conformance_profile: STANDARD`.
- The `Allow` header (lines 80-85) enumerates supported methods (example
  `GET, POST, PUT, DELETE, OPTIONS`).

The System API is **not** part of the generated ITS-REST contract — the
`emit-rest` groups are `ehr`, `query`, `definition`, `admin`, `demographic`
only (`crates/openehr-its/src/rest/generated/` — no `system.rs`). `OPTIONS /`
is therefore hand-written in `ehrbase-rest`, correctly (it is a standalone
1-operation OAS, not one of the resource-API groups).

**Current implementation** (verified 2026-07-12):

- `OPTIONS /` handler: `app/ehrbase-rest/src/overview/status.rs:60-74`
  (`system_options`), emitting the `Options` struct
  (`status.rs:43-51`) + the `Allow` header (`status.rs:16-17`, constant
  `ALLOW_METHODS = "GET, POST, PUT, DELETE, OPTIONS"`).
- Mounted at the **absolute root** `/`, above the `CorsLayer`:
  `app/ehrbase-rest/src/router.rs:118-121`
  (`.route("/", axum::routing::options(status::system_options))`).
- Wire test: `app/ehrbase-rest/tests/protocol_tail.rs:68-88`
  (`options_root_is_system_options_and_conformance`) — asserts 200, the
  `Allow` header, `restapi_specs_version == "1.0.3"`,
  `conformance_profile == "STANDARD"`, non-empty `endpoints`.
- Adjacent **extension** surfaces (no System-API spec governs them; our own
  design): `/rest/status` (`status.rs:20-35`, `ServerStatus`),
  `/health` + `{rest_root}/status/health` (`status.rs:37-39, 81-86`), and the
  whole management surface `/management/{info,health,env,metrics,prometheus,
  loggers}` (`app/ehrbase-rest/src/management/mod.rs:141-228`), of which
  `/management/info` (`management/info.rs:101-104`, `BuildInfo`) carries the
  richest and most accurate build/spec provenance.
- The conformance runner (`tools/conformance/`) computes CORE/STANDARD/OPTIONS
  **verdicts from catalogued case pass/fail** (`reporting/report.rs:100,
  421-432`; `master03-profiles.adoc`), and does **not** contain a case that
  fetches or schema-validates the `OPTIONS /` manifest.

---

## 1. Gap register (what is not spec-true today)

Every gap cites the governing spec text. G-1..G-3 are substantive; the rest
are minors / honest-residue notes. The endpoint is functionally present and
unit-tested — this is a correctness-and-honesty audit of a manifest whose
whole job is to *tell the truth about the server*.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **`endpoints` is a static, incomplete list.** The manifest advertises `["/ehr", "/definition", "/query"]` but the server also mounts the demographic and admin groups (generated `demographic.rs`/`admin.rs`, dispatched in `dispatch/demographic.rs`, `dispatch/admin.rs`). The OAS example itself lists `/demographic` and `/admin`. A conformance client reading the manifest to discover the API is told less than the server serves. | `system-codegen.openapi.yaml` `Options.endpoints` + `example` (lines 106-121) | Hardcoded `vec!["/ehr", "/definition", "/query"]` (`status.rs:68`); never derived from the mounted route set. |
| G-2 | **`conformance_profile` is a static assertion, not machine-derived.** The manifest hardcodes `"STANDARD"`, while the conformance instrument computes CORE/STANDARD/OPTIONS verdicts from actual case results (`report.rs:100, 421-432`). Today the two can drift silently — the manifest could claim STANDARD while the ECC verdict differs — and CORE (which also passes, blueprint §2) is not surfaced. The manifest's stated purpose is "exposing service capabilities for a conformance manifest". | `system-codegen.openapi.yaml` `options.description` (lines 56-61); `Options.conformance_profile` | `conformance_profile: "STANDARD"` literal (`status.rs:66-67`); not reconciled with the runner's computed profile. |
| G-3 | **`OPTIONS /` is mounted at the absolute root `/`, not at the API base-path root.** The OAS server is `https://{baseUrl}/v1` with path `/` — i.e. the manifest lives at the API root the resource endpoints hang off. Our resource API is nested under `/ehrbase/rest/openehr/v1` (`config.rs:211-213`), but `OPTIONS` is bound to bare `/` (`router.rs:121`). A client that `OPTIONS`es the documented API root (`…/openehr/v1/` or `…/openehr/v1`) does not reach the manifest. | `system-codegen.openapi.yaml` `servers` + `paths` `/` (lines 41-54) | Absolute-root mount only; the base-path root has no `OPTIONS` handler. |
| G-4 | **No conformance/ECC case exercises the manifest.** The instrument never fetches `OPTIONS /` nor validates the body against the `Options` schema; the only coverage is one unit test (`protocol_tail.rs:68-88`) asserting three fields. The manifest's schema-conformance (all six properties, types) and its accuracy (G-1/G-2) are therefore unverified by the acceptance instrument. | `system-codegen.openapi.yaml` (the whole `Options` schema, lines 92-134) | No `tools/conformance` case for the System API; profile math ignores the manifest. |
| G-5 | **`Accept` request parameter unhandled.** The OAS constrains `Accept` to `application/json`; the handler ignores `Accept` entirely (returns `Json(..)` unconditionally, `status.rs:70`). A client sending an unsatisfiable `Accept` gets JSON rather than a negotiated response. (Minor — the API is JSON-only, so a 406 is arguably over-engineering; recorded for completeness.) | `system-codegen.openapi.yaml` `Accept_JSON` (lines 71-78) | No `Accept` inspection. |
| G-6 | **`solution`/`vendor` are the same placeholder.** Both are `"ehrbase-rs"` (`status.rs:62,64`); the schema intends `vendor` = the organisation and `solution` = the product name. Cosmetic, but the manifest is public identity. | `system-codegen.openapi.yaml` `Options.{solution,vendor}` | `solution: "ehrbase-rs"`, `vendor: "ehrbase-rs"`. |
| G-7 | **Provenance duplicated across three shapes.** `OPTIONS /` (`Options`), `/rest/status` (`ServerStatus`, `status.rs:20-35`), and `/management/info` (`BuildInfo`, `info.rs:29-44`) each carry overlapping version/provenance data, and the version constant `OPENEHR_REST_API_VERSION = "1.0.3"` is defined twice (`status.rs:15`, `info.rs:13`). The spec-defined discovery surface (`Options`) is the *thinnest*; the accurate, complete data lives in the extension endpoints. | No System-API spec governs `/status` or `/management/*` — our own design/extensions | Three parallel structs; the manifest is not fed from the shared `BuildInfo`. |

---

## 2. Target design

The endpoint is small and already works; the redesign is about making the
manifest **truthful and self-describing**, mounting it where the spec puts it,
and giving the conformance instrument a case over it. Wire location:
`app/ehrbase-rest/src/overview/status.rs` (or a dedicated
`overview/system.rs` when the crate rewrite splits `status.rs`).

### 2.1 A single provenance source of truth (G-6/G-7)

Derive the `Options` manifest from the same `BuildInfo`/spec-version constants
that feed `/management/info` and `/rest/status`, so the three surfaces cannot
drift and `1.0.3` is declared once:

- Lift `OPENEHR_REST_API_VERSION` (and the product name/vendor) to one place
  (e.g. `management::info` already owns `OPENEHR_REST_API_VERSION` +
  `SpecVersions`); `status.rs` consumes it rather than re-declaring.
- `solution` = product ("EHRbase-RS"), `vendor` = the organisation,
  `solution_version` / `restapi_specs_version` from the shared constants
  (`restapi_specs_version = "1.0.3"` is spec-correct — the OAS `example` value
  `1.1.0` is illustrative only, not normative).

### 2.2 A live `endpoints` list (G-1)

Advertise exactly the groups the server mounts. Build the list from the
mounted API groups (the generated `ROUTES` group set — `ehr`, `query`,
`definition`, plus `demographic`/`admin` whenever their dispatch is wired, and
config-gated extension surfaces like `/terminology` only if we choose to
expose extensions here). Deriving it from the router (or a single
`enabled_groups()` helper the router and the manifest share) keeps the
manifest honest as groups are gated on/off.

### 2.3 A machine-derived `conformance_profile` (G-2)

The manifest should not out-claim the instrument. Options, in order of
preference:

1. **Report the highest profile the last committed ECC run obtained**
   (CORE/STANDARD), read from the committed `docs/conformance/results.json`
   badge data at build time (a `build.rs` constant, like `git_sha`), so the
   manifest states a *measured* profile, never a hand-asserted one. Surface
   CORE too when both pass (e.g. `conformance_profile: "STANDARD"` with the
   full obtained set available via `/management/info`).
2. If build-time coupling is undesirable, keep the literal but add a PORT NOTE
   that it is a *target* profile and the authoritative verdict is the ECC
   Statement/Certificate (`tools/conformance` `reporting/statement.rs`).

Either way the value must be reconciled with, not independent of, the runner's
`master03-profiles` computation.

### 2.4 Mount at the API root (G-3)

Bind `OPTIONS` to the base-path root as well as (or instead of) bare `/`, so
`OPTIONS /ehrbase/rest/openehr/v1` reaches the manifest — that is the root the
OAS `servers`/`paths` describe. Keep it **above** the `CorsLayer` (the layer
treats every `OPTIONS` as a CORS preflight and short-circuits it — the reason
for the current placement, `router.rs:118-120`). Retaining bare `/` as well is
harmless and helps naive probes; document both.

### 2.5 Content type (G-5)

Optionally honour `Accept`: return `406` when a client demands a media type
other than `application/json`/`*/*`. Low value (JSON-only API); acceptable to
leave with a PORT NOTE that the endpoint is unconditionally JSON per the OAS's
single-enum `Accept`.

### 2.6 Conformance coverage (G-4)

Add a System-API case to `tools/conformance` (a `SYS`/Options area, or fold
into an existing structural area) that:

- issues `OPTIONS` at the API root, asserts `200` + the `Allow` header;
- validates the body against the `Options` schema (all six properties + types);
- asserts `endpoints` is non-empty and each entry is a mounted group (cross-
  check against the runner's own capability set);
- asserts `conformance_profile` does not exceed the run's computed verdict.

This turns the manifest from unit-tested-only into instrument-verified, and
makes G-1/G-2 regressions fail the suite.

---

## 3. Work plan

1. Single provenance source: consume shared `BuildInfo`/version constants in
   `system_options`; fix `solution`/`vendor`; de-duplicate the `1.0.3`
   constant. (G-6, G-7)
2. Live `endpoints` from the mounted group set (shared helper with the
   router). (G-1)
3. Machine-derived `conformance_profile` (build-time ECC badge or a cited
   target-profile PORT NOTE). (G-2)
4. Mount `OPTIONS` at the base-path root (keep it above CORS; retain bare `/`).
   (G-3)
5. Optional `Accept` negotiation or a PORT NOTE. (G-5)
6. `tools/conformance` System-API case (schema-validate the manifest, cross-
   check endpoints + profile). (G-4)

Exit: every G-row closed in code or a re-verified cited PORT NOTE; the
manifest is truthful and instrument-verified; workspace suites + ECC
zero-drift green; the website book's API/discovery page reflects the
`OPTIONS /` location and body (same-PR docs rule).

---

## 4. Standing PORT NOTEs after the redesign (the honest residue)

- **`restapi_specs_version = "1.0.3"`** is our conformance target; the OAS
  `example` value `1.1.0` is illustrative and not tracked
  (`system-codegen.openapi.yaml` line 114).
- **`/rest/status`, `/health`, `/management/*`** are extensions — no
  System-API (or any ITS-REST) spec governs them; they are our own operational
  design (probe + provenance surfaces), distinct from the spec-defined
  `Options` manifest.
- If `Accept` negotiation is not implemented, note the endpoint is
  unconditionally `application/json` per the single-enum `Accept_JSON`
  parameter — a deliberate simplification, not a defect.
- The System API is intentionally **hand-written**, not generated: it is a
  standalone one-operation OAS, outside the `emit-rest` resource-group set
  (`crates/openehr-its/src/rest/generated/` has no `system` group).
</content>
</invoke>
