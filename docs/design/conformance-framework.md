# The ehrbase-rs Conformance Catalogue (ECC) — design v4 (our own framework)

- **Status:** v4 accepted 2026-07-08 (owner directive: complete clean rewrite,
  our own framework); engine core implemented on `claude/cnf-hardening`
  (`docs/plans/s2-phase-05-cnf-engine-rewrite.md`).
- **Supersedes:** v1–v3 of this document. v3.1's "trace every legacy unit"
  model is **retired**: there is no runtime mapping to the legacy CNF corpus
  anywhere in the framework.
- **Related:** `docs/design/version-signing.md`,
  `docs/design/access-control.md`; the consolidated spec-gap surface is
  `docs/blueprint/00-THE-BLUEPRINT.md` §2 (+ `blueprint/07-cnf.md`).

---

## 1. The idea

We build **our own, modern conformance framework** for openEHR CDRs — the
best available testing engine for proving a server conforms to the openEHR
platform specifications. It is not a port, transcription, or mapping of the
official openEHR CNF corpus. That corpus (vendored at
`docs/specs/openehr/CNF/`) is **design-time reference reading**: upstream is
frozen since 2024, five of its chapters were never finished, its executable
layer is a 2019-era EHRbase Robot/Python harness, and its data sets predate
RM 1.2.0. We studied all of it exhaustively (2026-07-08 inventories: 324
schedule headings, ~1,371 truth-table rows, 464 robot cases, fixture corpus),
took what is good — the *ideas*: profile-scoped claims, data-set-driven
validation cases, a certificate-shaped report — and built better, from the
**current pinned specifications** we actually implement.

> A conformance claim is a pure function of a run over **our own enumerated
> catalogue**. Nothing hand-counted, nothing hand-asserted, nothing inherited
> from an unmaintained corpus.

## 2. Principles (what "better" means, concretely)

1. **Spec-first universe.** The case base derives from the living pinned
   specs: every ITS-REST 1.0.3 operation and documented status code, every
   AQL 1.1 language construct, every RM 1.2.0 data-type constraint semantic,
   plus the capabilities the specs imply but never got tests upstream
   (version signing, security behaviour, `ALL_VERSIONS`). Coverage is a
   property of the design, not of what someone typed in 2019.
2. **Our identity system.** Every case is `ECC-<AREA>-<NNN>` (optionally
   `.<VV>` for a data-set variant) — allocated once in a committed catalogue
   file, never reused, grouped in a clean area taxonomy (§4). Industry-style
   test-catalogue numbering; no legacy ids anywhere.
3. **Generated data sets, not copied fixtures.** Validation cases get their
   accept/reject matrices from **generators** (boundary values, cardinality
   grids, presence/absence, type substitution, constraint mutation over our
   own authored OPTs) — systematically more combinations than the old
   hand-written tables, and regenerable when the RM version moves. Vendored
   fixtures are reused only as convenient *input payloads* (`testdata`),
   never as framework structure.
4. **Declarative scenarios.** Cases read as flows, not hand-rolled HTTP
   plumbing: a small typed step API (given/when/expect) over the transport,
   so a failure reports "step 3/5: PUT …/composition expected 412, got 200"
   and a reviewer can read a case top-to-bottom like prose.
5. **Version-aware.** The specification versions under test
   ([`SpecVersions`]: RM, ITS-REST, AQL, TERM) are a first-class dimension of
   the model, the run config, and the claim. **Today exactly one set is
   supported — the latest published of each (RM 1.2.0, ITS-REST 1.0.3,
   AQL 1.1.0, TERM 3.1.0).** Supporting another set later is additive, not a
   rewrite.
6. **Machine-enforced claims.** Profile verdicts (CORE/STANDARD/OPTIONS) are
   computed all-or-nothing per capability from the run; the statement's claim
   line is generated from that verdict only. Failures become findings
   (`F-AA-NN`), never exclusions; skips carry stated reasons and appear in
   the deviations section.
7. **Clean, layered, boring engineering.** One responsibility per layer,
   committed data files for anything durable, guards that break the build on
   inconsistency, zero Python, zero runtime dependence on
   `docs/specs/openehr/CNF/`.

## 3. What we keep from the old CNF (ideas only) — and what we discard

| Kept (as an idea, reimplemented)                                                                                        | Discarded                                            |
|-------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------|
| Profile-scoped claims (CORE / STANDARD / OPTIONS) and the capability matrix (§8)                                        | The masterNN chapter structure and its ids           |
| "A test = case × data set" and accept/reject truth matrices                                                             | The hand-written 2019 truth tables (we generate)     |
| Certificate-shaped generated statement + execution report                                                               | The Robot/Python harness, entirely                   |
| The functional service walk (EHR → status → composition → contribution → directory → templates → queries) as area seeds | Runtime mapping/tracing to legacy case ids           |
| Reusable input payloads (OPTs, compositions, AQL queries + goldens) as plain test data                                  | Schedule/robot/OAS extractors as framework machinery |
| The ISO 18308 requirements lens (vendored statement, `docs/specs/openehr/REQUIREMENTS/`) as a reporting rollup          | Legacy inventory snapshots and classification enums  |

The one-time human-reviewed completeness check ("did our catalogue cover
every behaviour the old corpus tested?") lives in the phase plan as a design
review step — a checklist for authors, not machinery.

## 4. The catalogue

### 4.1 Identity

```
ECC-<AREA>-<NNN>        a case            (ECC-EHR-005, ECC-QRY-118)
ECC-<AREA>-<NNN>.<VV>   a data-set variant (ECC-VAL-042.07)
```

Allocation lives in `tools/conformance/inventory/ecc-catalog.tsv`
(committed): `ecc_id · area · status(active|retired|planned) · registration
key · title`. Numbers are allocated once (next-free per area, in registry
order via `REGEN_CATALOG=1`) and never reused — a removed case is `retired`,
keeping its number burned. The coverage guard (`tests/coverage.rs`) enforces:
every registered case has a number; every `active` line has a live case; the
area derivation is stable.

### 4.2 Areas (the category taxonomy)

| Area   | Scope                                                   |
|--------|---------------------------------------------------------|
| `EHR`  | EHR service operations                                  |
| `STA`  | EHR_STATUS operations                                   |
| `COM`  | COMPOSITION operations                                  |
| `CTB`  | CONTRIBUTION change sets                                |
| `DIR`  | Directory (FOLDER) operations                           |
| `TPL`  | Template / OPT provisioning                             |
| `SQR`  | Stored-query provisioning                               |
| `QRY`  | AQL execution                                           |
| `VAL`  | Content / archetype validation (data types, structures) |
| `REST` | ITS-REST operation × status matrix                      |
| `DEM`  | Demographic service                                     |
| `ADM`  | Admin service                                           |
| `SEC`  | Security / authorization                                |
| `SIG`  | Version signing                                         |
| `MSG`  | Messaging (when implemented)                            |

### 4.3 The case universe (build-out plan, ≥2,000 executable tests)

| Area group                                        | Source of truth                                                  | Target size                                                                         |
|---------------------------------------------------|------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| Functional services (EHR/STA/COM/CTB/DIR/TPL/SQR) | SM interfaces + ITS-REST 1.0.3 semantics                         | ~300 cases (positive, negative, versioning, time-travel, concurrency preconditions) |
| `REST` matrix                                     | every ITS-REST operation × documented status                     | ~350–400                                                                            |
| `VAL` generated matrices                          | RM 1.2.0 data-type + structure constraints × generator grids     | **1,000+** variants (systematically exceeds the old 1,371 hand rows)                |
| `QRY`                                             | AQL 1.1 construct checklist + our corpus with golden result sets | ~250                                                                                |
| `SEC`/`SIG`/`ADM`/`DEM`                           | our implemented capabilities (access-control, signing designs)   | ~100                                                                                |

## 5. Architecture (crate `conformance`)

```
src/
├── lib.rs           # facade: flat public paths over the layered tree
├── model/           # the domain
│   ├── case.rs      #   CaseMeta, Capability, Profile, Format
│   ├── catalog.rs   #   Area, EccEntry, allocation, TSV persistence
│   └── version.rs   #   SpecVersions (latest-only today; additive later)
├── testdata/        # typed access to input payloads
│   └── fixtures.rs  #   vendored OPTs/compositions as data + RM-1.2.0 adaptation
├── engine/          # execution
│   ├── harness.rs   #   Transport, HttpRequest/Response, CaseError, RunContext
│   ├── client.rs    #   reqwest SUT client (Basic/Bearer, admin slot)
│   ├── sut.rs       #   External | SelfHosted (testcontainers PG18 + in-process app)
│   ├── assert.rs    #   status/header/payload assertions (exact/superset/ignore-set)
│   ├── registry.rs  #   the registered ECC case set
│   └── run.rs       #   RunConfig (+SpecVersions), executor → RunResults
├── reporting/
│   ├── results.rs   #   serializable outcomes (ECC ids first-class)
│   └── report.rs    #   RESULTS.md, CATALOG.md, CONFORMANCE_STATEMENT.md, badge
└── suites/          # the case implementations, one module per area
```

Planned additions in this layout (build-out steps, §7): `engine/flow.rs`
(the declarative step API), `testdata/generate.rs` (the VAL generators),
`model/profile.rs` (the capability matrix + machine verdict), JUnit/CTRF
output in `reporting`.

### 5.1 SUT mode

- **External only** (`--base-url`, `--auth basic:u:p|bearer:t`,
  `--admin-auth`): a deployed real system; pure API client, no DB access,
  self-contained cases (fresh EHR per case). The standard SUT is the
  Docker-composed server built from the current sources
  (`scripts/conformance.sh` — compose `up --build`, run, tear down), so the
  wire under test is always the production binary/stack.
- The former in-process `self-host` mode (testcontainers PG18 + a re-wired
  axum app) was **removed 2026-07-09** (owner ruling): it duplicated the
  binary's wiring and drifted from the production `serve_full` stack during
  the ADR-011 rebuild. One mode, real artefact.

### 5.2 CLI (contract for `scripts/conformance.sh` and `/run-conformance`)

```
conformance run    --base-url URL [--filter S]
                   [--profile core|standard|options] [--format json|xml|both]
                   [--out docs/conformance/] [--auth …] [--admin-auth …]
conformance list   [--filter S]          # catalogue with per-area totals
conformance report --from results.json   # regenerate artifacts without a run
```

Exit codes: `0` pass · `1` failures (report still written) · `2` runner/SUT
error.

### 5.3 Owned fixture register (`tools/conformance/testdata/fixtures/`)

The vendored CNF corpus at `docs/specs/openehr/` is **read-only and is never
edited** (hard rule) — it is the oracle. But some vendored fixtures are
**internally inconsistent** with their own operational template when read
against the vendored spec text (e.g. `all_types.composition.json` carries a full
`DV_DATE` at the `at0003` leaf that its OPT constrains with the `yyyy-??-XX`
`C_DATE` pattern — day *disallowed* per AOM 1.4 `c_date.adoc`; a spec-correct
validator must reject it, though EHRbase/archie leniently accepts it).

When a vendored fixture is **proven defective against the vendored spec text**, a
corrected copy lives under `tools/conformance/testdata/fixtures/` as a reviewed
file, organised by validity then kind (`valid/<kind>/`, `invalid/<kind>/`).
Every correction is documented in that directory's
[`REGISTER.md`](../../tools/conformance/testdata/fixtures/REGISTER.md): the
vendored source path, the exact leaf changed (old → new value), and the spec
citation for why it is a defect. The discipline:

1. **Vendored corpus is never mutated** — neither on disk nor in code.
2. **Corrected copies** (`valid/<kind>/`) are what the positive cases commit,
   loaded through the owned-fixture loader (`crate::fixtures::owned_fixture`) —
   never by mutating vendored data in code (this replaced the former in-code
   `adapt_all_types_date` mutation).
3. **Each corrected fixture has a companion negative ECC case** that commits the
   defective **original** (a byte-faithful `invalid/<kind>/` copy, pinned
   byte-identical to the vendored source by a `fixtures` guard test) and asserts
   the SUT rejects it (`val/dv-date-day-disallowed-pattern`, `ECC-VAL-119`), so
   the defect itself stays under test.

Policy origin: owner ruling 2026-07-09 (B2 — validation-depth).

## 6. Reports (`docs/conformance/`, regenerated per run)

Deliberately few artifacts — one machine record, two markdown documents
with distinct jobs, four badges (owner directive: no report sprawl):

- `results.json` — the single machine record: outcomes (ECC id, title,
  capability, profiles, format, status, data sets, duration, citation), SUT
  identity incl. `SpecVersions`, run selection.
- `CONFORMANCE_REPORT.md` — **the run**, one document: identity + scope,
  per-area execution matrix, detailed per-case table, the machine profile
  verdicts (per-capability tables for CORE/STANDARD/OPTIONS), failures
  (each → finding), deviations (skips by reason).
- `CATALOG.md` — **the catalogue**: the full per-category test list (every
  case, status, title, last outcome) — kept separate because it grows to
  2,000+ rows.
- **Badges (four)** — shields endpoint schema, all generated from the run:
  - `badge.json` — the total: `ECC conformance: <passed>/<active catalogue>`
    (red on any failure, brightgreen only at full pass, else yellow);
  - `badge-core.json`, `badge-standard.json`, `badge-options.json` — one per
    profile, driven by the **machine profile verdict** (`model/profile.rs`):
    message `PASS (n/n capabilities)` when the all-or-nothing verdict holds,
    else `k/n capabilities`; brightgreen on pass, red if any required
    capability has failures/errors, yellow while unevidenced.
  The README embeds all four, so the public face shows the total *and* the
  per-profile claim state at a glance — and a badge can never say PASS
  unless the machine verdict does.

## 7. Build-out plan (compiling, tested increments)

1. ✅ **Engine core v4** (this change): layered layout, ECC catalogue +
   guards, version dimension, catalogue-driven runner/reports; legacy
   mapping machinery deleted.
2. **Re-title + re-key the existing ~310 cases** as native ECC cases (proper
   titles in the catalogue; registration keys become `own:` descriptive
   slugs), area by area — plus the one-time design-review checklist against
   the old corpus (nothing it tested left uncovered by design).
3. **`engine/flow.rs`** (declarative steps) and migrate one area (EHR) to it
   as the pattern.
4. **`model/profile.rs`** — the capability matrix (§8) + all-or-nothing
   machine verdict wired into the statement.
5. **`VAL` generators** (`testdata/generate.rs`): cardinality grids,
   presence/absence, boundary values, type substitution over authored OPTs —
   the 1,000+ variant build-out with per-variant outcomes (`ECC-VAL-nnn.vv`).
6. **`REST` matrix area** from the pinned ITS-REST contract (same source
   `emit-rest` consumes).
7. **`QRY` build-out**: AQL 1.1 construct checklist + corpus goldens with a
   rule-named normalizer.
8. **`SEC` sweeps** (401/403 under RBAC), JUnit/CTRF report output, CI tiers
   (PR: compose-stack CORE smoke · full: compose stack, both formats), first
   generated STANDARD-profile statement.

## 8. Appendix — capability → profile matrix (ours)

Adopted from the openEHR profiles idea (reference:
`docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc`), curated to
our capability names; `model/profile.rs` encodes this table.

| Capability                               | Areas       |          CORE           | STANDARD | OPTIONS |
|------------------------------------------|-------------|:-----------------------:|:--------:|:-------:|
| OPT 1.4 provisioning                     | TPL         |            ✔            |    ✔     |         |
| EHR operations                           | EHR         |            ✔            |    ✔     |         |
| EHR_STATUS                               | STA         |            ✔            |    ✔     |         |
| Composition operations                   | COM         |            ✔            |    ✔     |         |
| Change sets                              | CTB         |            ✔            |    ✔     |         |
| Versioning (incl. `ALL_VERSIONS`)        | EHR/COM/DIR |            ✔            |    ✔     |         |
| Archetype validation                     | VAL         |            ✔            |    ✔     |         |
| Anonymous EHRs                           | EHR         |            ✔            |    ✔     |         |
| REST contract (DEFINITION + EHR APIs)    | REST        |            ✔            |    ✔     |         |
| Directory operations                     | DIR         |                         |    ✔     |         |
| Query provisioning                       | SQR         |                         |    ✔     |         |
| AQL basic + QUERY API                    | QRY         |                         |    ✔     |         |
| Version signing                          | SIG         |                         |    ✔     |         |
| Data formats: JSON + XML                 | all         |            ✔            |    ✔     |         |
| Demographic / Admin / Messaging          | DEM/ADM/MSG |                         |          |    ✔    |
| ADL2/OPT2, AQL advanced, AQL×terminology | TPL/QRY     |                         |          |    ✔    |
| Security behaviour (RBAC 401/403)        | SEC         | reported with the claim |          |         |

ISO 18308 rollup: cases may cite `iso18308:<section>`
(`docs/specs/openehr/REQUIREMENTS/iso18308_conformance.pdf`) so reports can
also present a requirements-level view (structure, privacy & security,
medico-legal, version control).

## 9. What this is not

- **Not a Robot/Python port and not a mapper** — no legacy corpus machinery
  exists at runtime; the vendored CNF is reading material and input payloads.
- **Not the EhrScape/FLAT test bed** (that is `openehr-flat`'s suite, P17).
- **Not a benchmark** (`benchmark` owns performance); durations are
  telemetry only.
