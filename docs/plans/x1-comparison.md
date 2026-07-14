# Phase X1 — The honest comparison: EHRbase (Java) vs EHRbase-rs

- Status: not-started
- Started: —   Owner: Ruben
- Consumes: W1 (the live docs website — comparison page joins it), the ECC
  conformance framework (`tools/conformance`, B5/B6), the benchmark harness +
  its methodology design (`tools/benchmark`, `docs/design/benchmarking.md`),
  the dual-stack compose environments (`docker/benchmark/`,
  `docker/conformance/`). Upstream facts live-verified **2026-07-11** (§2c).
- Compile required: yes (harness changes are normal workspace code — nextest +
  clippy green; the published page is website content under the W1 gates)

## Objectives

A public **comparison page** on the docs website comparing upstream **EHRbase**
(Java, `ehrbase/ehrbase`) with **EHRbase-rs** on: features, spec
compliance/conformance, performance, and deployment footprint (image size,
startup time, memory/CPU, storage) — every number **measured by us,
reproducible from committed scripts**, with the methodology published next to
the results. To get real (not estimated) conformance values, upstream EHRbase
is actually **run through our ECC suite**; to get real performance values, the
benchmark tooling is **overhauled to the standard its own design doc already
demands** and run against both servers on the same host.

The owner's #1 rule for this phase, verbatim: **"keep this honest okay no
false claims!!!"** — §1 is the constitution; everything else serves it.

---

## 1. Honesty rules (non-negotiable — restated at the top of the page itself)

1. **Measured, not asserted.** Every number on the page traces to a committed
   `results.json` produced by a run whose exact commands, images (by digest),
   host, and date are recorded. No number is copied from upstream marketing,
   estimated, or extrapolated. A claim that cannot be reproduced from the
   repo does not ship.
2. **Upstream fairness adjudication.** The ECC catalogue is **our own
   instrument**, authored against our reading of the pinned specs (RM 1.2.0,
   ITS-REST development@e8a093e) with adjudicated skips for *our* server. An
   unmodified run would unfairly fail upstream on version skew and on our
   extensions. Before any upstream result is published, every upstream
   failure goes through a **fairness triage** (§3a.4) into a committed,
   cited **upstream adjudication register**; only failures that survive
   triage as genuine spec defects are reported as failures.
3. **Extension routes are N/A, not failures.** ECC areas exercising
   ehrbase-rs extensions (Demographic wire, TERMINOLOGY() AQL family,
   version signing, `/terminology`) are reported **"extension — not
   applicable upstream"**, never as upstream failures (§3a.3).
4. **No certification claims.** ECC results are presented as "under the ECC
   catalogue" — our spec-cited instrument — never as official openEHR
   certification (no official program was run, for either server). The
   Conformance **Certificate** artifact is a self-assessment of *our*
   product and is **never emitted for an upstream run** (§3a.2).
5. **Where EHRbase wins, the page says so plainly.** The "Where EHRbase
   wins" sections (already mandatory in the benchmark report,
   `tools/benchmark/src/report.rs:188-203`) carry through to the page, for
   both performance and features. A feature upstream has and we lack is a
   plain "not supported" row on our side.
6. **Every number is date- and version-stamped.** Each data block on the
   page carries: run date, both server versions, both image digests, host
   spec, harness commit. Numbers from different runs are never mixed in one
   table without per-cell stamps.
7. **Methodology published next to results.** The page links the full
   methodology (`docs/design/benchmarking.md`, the ECC design, the
   adjudication register) and the raw `results.json` artifacts, plus a
   reproduce-it block with the exact commands.
8. **Rerun per release; stale numbers marked.** Data is refreshed on every
   ehrbase-rs release; a scheduled check watches upstream releases and flags
   the page when the compared upstream version is no longer their latest
   (§3d). A stale block is labelled, never silently left to imply currency.
9. **Fair configuration.** Both servers run their vendor-published images,
   default-recommended stacks, equal container resource pins, same
   credential path, warmed identically (JVM warmup honoured — the design's
   anti-strawman table, `docs/design/benchmarking.md:15-33`, governs).
   Config asymmetries are eliminated or listed in the parity table.
10. **Standing rule 3 applies to the instrument:** never weaken an ECC case
    or benchmark scenario to make either server look better; the upstream
    register only *reclassifies with citation*, it never edits a case.

---

## 2. Research findings (all verified 2026-07-11 against the working tree / live sources)

### 2a. ECC runner (`tools/conformance/`) — external-SUT readiness

**Transport is already SUT-agnostic; the honesty seams are missing.**

- The runner is **external-only, pure HTTP** — the in-process mode was removed
  2026-07-09 (`tools/conformance/src/engine/sut.rs:1-13`); the only
  constructor is `Sut::external(base_url, regular, admin)` (`sut.rs:56-64`).
  The base URL is **fully arbitrary**: `SutClient::new` stores any string
  (`engine/client.rs:64-79`) and every request is `base_url + relative path`
  (`client.rs:104`). CLI: `--base-url` is required
  (`src/bin/conformance.rs:52-54`, enforced at `conformance.rs:145-148`).
- `scripts/conformance.sh` wraps it: `CONF_BASE_URL` (line 24, default
  `http://localhost:8080/ehrbase/rest/openehr/v1`), `CONF_NO_COMPOSE=1`
  skips compose management for a pre-deployed SUT (lines 20, 38-39, 63),
  `CONF_AUTH`/`CONF_ADMIN_AUTH` default `basic:ehrbase:ehrbase` /
  `basic:ehrbase-admin:ehrbase` (lines 26-29). Auth supports
  `basic:<u>:<p>` and `bearer:<t>` with a regular + admin slot
  (`client.rs:24-46, 81-89, 109-114`).
- **A dual-stack conformance compose already exists**:
  `docker/conformance/docker-compose.yml` defines `ehrbase-rs` (profile
  `rs`, port 8090) and `ehrbase-java` (**pinned `ehrbase/ehrbase:2.33.0` —
  stale**, upstream is 2.34.0, §2c) each with its own PostgreSQL;
  `docker/conformance/run.sh` takes `rs|java` and writes per-edition output.
  **However** the `CNF_COMPARISON.md` and `docs/conformance/<edition>/`
  artifacts its README promises **do not exist on disk** — the java path
  has never been executed and persisted. X1.2 makes that real.
- **Case areas** (`Area`, `model/catalog.rs:30-63`, 16 areas; suites wired
  in `suites/mod.rs:33-49`). Upstream-applicability split:
  - Core ITS-REST (upstream implements): `Ehr, Sta, Com, Ctb, Dir, Tpl,
    Sqr, Qry, Val, Rest`.
  - **Extensions / N-A upstream:** `Dem` (upstream has **no** demographic
    API — `suites/demographic.rs:243,267,285` would 404), `Ts`
    openEHR-bundle cases (assume our `TERMINOLOGY('expand','openehr',…)`
    AQL feature, `suites/terminology.rs:12-36`), `Sig` (version signing is
    ours, `suites/signing.rs`), `Msg` (already self-skips —
    every case `SKIPPED(NativeApiOnly)`, `suites/message.rs:8,140-166`),
    `Sec` (self-adjudicates on probed auth mode,
    `suites/security.rs:96-138`).
  - **Path mismatch:** `Adm` cases build `/admin/ehr/{id}` **relative to
    the openehr base** (`suites/admin.rs:104,159,207`) — matches our nested
    mount (`app/ehrbase-rest/src/config.rs:196-211`) but upstream serves
    admin at the **sibling** `/ehrbase/rest/admin` (§2c) → 404s, not
    honest failures. `sec/forbidden-role-403` also hits `/admin/ehr`
    (`security.rs:120`).
- **results.json / report**: written by `reporting/report.rs:55-92`
  (results + report + catalog + statement + certificate + 4 badges;
  `report --from results.json` regenerates). Top-level `RunResults`
  (`reporting/results.rs:11-27`) already carries a `SutIdentity` — but it
  is only `{base_url, versions, auth_mode}` (`results.rs:68-77`): **no
  product name/version/digest field**. Worse, the Certificate **hard-codes**
  `Solution/Vendor = ehrbase-rs` (`reporting/statement.rs:138-142`) — an
  unmodified upstream run would emit a certificate falsely labelled
  ehrbase-rs. **Honesty blocker #1; fixed in X1.1.**
- **Profile verdicts** (CORE/STANDARD/OPTIONS) are machine-computed:
  `model/profile.rs:151-187` over the capability matrix
  (`profile.rs:46-99`); a capability passes iff ≥1 passed and 0
  failed/errored (`profile.rs:172`).
- **Adjudication today is inline code, not data.** Skips are
  `CaseError::Skipped(String)` decided per suite
  (`engine/harness.rs:197`, mapped at `engine/run.rs:96`); there is **no
  central register and no SUT-keyed mechanism**. RM-version sensitivity is
  explicitly a future-additive dimension that does not exist yet
  (`model/version.rs:1-9,40-48` — hard-pinned RM 1.2.0). The natural hook
  is the executor loop (`engine/run.rs:76-120`), before/after each case
  runs (§3a.4).
- **Seeding is API-only and SUT-portable** (`suites/support.rs:15-33`
  creates EHRs via `POST /ehr`; `support.rs:51-84` uploads OPTs
  idempotently, 409-tolerant). Fixtures come from the vendored CNF corpus
  (`testdata/fixtures.rs:38-41`) **adapted up to RM 1.2.0** (`_type`
  injection, `fixtures.rs:23-31`) — so we *send* RM 1.2.0 canonical JSON
  and compare responses via `Compare::Exact/Superset/IgnoreSet`
  (`model/case.rs:191-202`). Upstream (archie, RM 1.1.0-era wire, §2c) may
  legitimately diverge on request acceptance *and* response shape — the
  `rm-version-sensitive` adjudication category exists for exactly this.

### 2b. Benchmark harness (`tools/benchmark/`) — current vs its own design

**The multi-SUT plumbing exists and is honestly structured; what's missing is
what the design doc (`docs/design/benchmarking.md`) already specifies.**

Already built (reuse, don't rebuild):
- 24 scenarios across 8 REST groups, a flat `enum Scenario`
  (`src/workload.rs:48-82`; ids/groups/gates at `workload.rs:114-216`;
  measured op at `workload.rs:328-475`).
- Identical-client guarantee: `Target` wraps the conformance `SutClient`
  (`src/target.rs:45-81`) — the same code path drives both servers, by
  construction. `--base-url` is arbitrary; `--implementation
  ehrbase-rs|ehrbase-java` labels results (`src/bin/bench.rs:40-47`,
  `target.rs:11-42`); `--merge` stitches a two-server report
  (`bin/bench.rs:54-57,152-160`).
- Warmup discarded before measurement (`src/driver.rs:128-141`; defaults
  200 warmup / 2000 measure / 5 runs, `driver.rs:29-37`); HdrHistogram
  percentiles p50/p90/p99/p99.9/max (`src/measure.rs:121-137`); inter-run
  CoV (`measure.rs:142-154`); pre-flight correctness gate — a wrong answer
  is never timed (`driver.rs:114-119`).
- Report already renders the Java column, head-to-head with a ±10% dead
  band, and a mandatory "Where EHRbase wins" section
  (`src/report.rs:63,148-203,341-362`).
- `docker/benchmark/docker-compose.yml` + `run.sh`: honest one-at-a-time
  dual-stack protocol, Basic auth both sides (also pinned **2.33.0** —
  stale).

Gaps vs the design doc (the "way better" list — each cited to the design):
1. **No concurrency/throughput sweep** — strictly closed-loop single client
   (`driver.rs:19-27,133-147`); the open-loop ramp {1..128} + saturation
   profile (`benchmarking.md:80-89`) is unimplemented. The
   coordinated-omission correction **exists but is dead code**
   (`measure.rs:45-57` — the driver calls plain `record`,
   `driver.rs:140`); wiring it in comes with the open-loop driver.
2. **No resource sampling at all** — the design's `stats.rs` (container
   CPU/RSS at 1 Hz, `benchmarking.md:181,206`) does not exist; no
   cold-start, no image size/digest capture, no
   `pg_total_relation_size` storage footprint (`benchmarking.md:150-156`).
   The only host data captured is the **load-generator** box
   (`src/host.rs:43-69`).
3. **Env block cannot carry comparison provenance** — `EnvBlock`
   (`src/report.rs:13-32`) has no image-digest / PG-version / resource-pin
   fields (`benchmarking.md:243-244` requires them); the compose has no
   `cpus:`/`memory:` limits (design §3.1).
4. **Missing pre-registered scenarios** — W3 (large composition), W7
   (multi-version contribution), W9 (AQL CONTAINS chain), W10 (AQL
   aggregate/ORDER BY), and **W13 mixed 70/30** ("the number that actually
   matters", `benchmarking.md:64-78`) are absent; the `ContributionGet`
   scenario is a placeholder read (`workload.rs:415-425`).
5. **No scale ladder** — `seed.rs` is a flat bulk-create; the deterministic
   empty/10k/100k/1M rungs (`benchmarking.md:91-100`) don't exist, and
   subject ids come from a process-local counter (`workload.rs:40-45`).
6. **No error-rate metric** — mid-run statuses are discarded
   (`driver.rs:138`).
7. Cosmetic: CLI help still references stale W-ids (`bin/bench.rs:48,141`);
   the committed `docs/benchmarks/REPORT.md` is a smoke-config run
   (20/100/2, `REPORT.md:20`) with an empty Java column — not publishable.

### 2c. Upstream EHRbase (live-verified 2026-07-11; every fact from a fetched source)

| Fact | Value | Source |
|---|---|---|
| Latest release | **v2.34.0**, published 2026-07-08 (develop = 2.35.0-SNAPSHOT) | `api.github.com/repos/ehrbase/ehrbase/releases/latest`, root `pom.xml` |
| Docker image | `ehrbase/ehrbase` — tags `latest`(→2.34.0), `next`, per-version; compressed size ≈ **170 MB** (170,687,684 B) | Docker Hub v2 tags API |
| Database | custom `ehrbase/ehrbase-v2-postgres:16.2` (PostgreSQL 16.2) | their `docker-compose.yml` (develop) |
| REST base path | **`/ehrbase/rest/openehr/v1`** (contextPath `/ehrbase` + `openehr-api.context-path /rest/openehr` + `v1`) | `configuration/src/main/resources/application.yml` |
| Admin API | context path **`/rest/admin`** (sibling of `/rest/openehr`); `admin-api.active: false` by default — must be enabled by env for ADM cases | same `application.yml` |
| Auth default | `security.authType: BASIC`; users `ehrbase-user`/`SuperSecretPassword`, admin `ehrbase-admin`/`EvenMoreSecretPassword`; compose bundles Keycloak 24.0.3 for OAuth2 | `application.yml` + `.env.ehrbase` |
| RM / spec versions | archie **3.13.0** (`bom/pom.xml`) → RM 1.1.0-era wire (strongly indicated by archie docs; treat as indicated, not byte-pinned); their docs state **ITS-REST 1.0.2** supported (we target 1.0.3/dev) | `bom/pom.xml`, nedap/archie, docs.ehrbase.org |
| Misc | `SYSTEM_ALLOWTEMPLATEOVERWRITE` available (idempotent re-runs); Java 25 on develop; management endpoints under `/management` | compose + `application.yml` |

Consequences: both in-repo composes must bump 2.33.0 → **2.34.0 and pin the
digest**; the version-skew facts (RM 1.1.0-era wire, ITS-REST 1.0.2 vs our
1.0.3/dev catalogue) are the backbone of the upstream adjudication register
and must be stated on the page.

### 2d. Website integration (W1 is live; the page is one more chapter)

- Chapter slot: `website/book/src/SUMMARY.md:28` has the Conformance
  chapter; **Comparison** slots directly after it
  (`website/book/src/comparison.md`). The book builds with mdBook 0.5.4 +
  mermaid/toc preprocessors (`website/book/book.toml`); a new chapter +
  static assets needs **no `docs.yml` change** — it's just more markdown in
  `src/`.
- Precedent for data on the page: `website/book/src/conformance.md` states
  the published numbers in prose/tables (hand-written against
  `docs/conformance/`). For the comparison page we go one step better:
  **generated markdown fragments + static SVG charts, checked in and
  drift-gated**, following the `scripts/assemble-oas.sh --check` pattern
  from W1 (§5a of `docs/design/docs-website.md`) — regenerate from the
  committed `results.json` artifacts and `git diff --exit-code` in CI, so
  the page can never silently disagree with the raw data.
- mdBook's built-in `{{#include}}` link-preprocessor inlines the generated
  fragments; inline SVG inherits page CSS so light/dark both work. No JS,
  no CDN (W1 rules hold).
- W1's "never publish" list is unaffected: the page publishes only
  generated artifacts + cited public facts, no internal docs.

---

## 3. Design decisions

### 3a. ECC upstream mode

1. **SUT identity becomes first-class.** Extend `SutIdentity`
   (`reporting/results.rs:68-77`) with a `product` block: `{ name, version,
   image_digest: Option<String> }`, fed by new CLI flags `--sut-name` /
   `--sut-version` / `--sut-image-digest` (defaulted to
   `ehrbase-rs @ <workspace version>` so existing runs stay stable). All
   report artifacts render the identity; `docs/conformance/` (our baseline)
   remains byte-compatible modulo the new field.
2. **The Certificate is self-assessment only.** `statement.rs:138-142`
   stops hard-coding `ehrbase-rs`; Statement/Certificate are emitted **only
   when the SUT product is ehrbase-rs**. An upstream run produces
   `results.json` + `CONFORMANCE_REPORT.md` + `CATALOG.md` only — we do not
   manufacture certification artifacts for someone else's product (honesty
   rule 4).
3. **A `NotApplicable` outcome, distinct from `Skipped`.** New
   `CaseStatus::NotApplicable` (`results.rs:140-151`), excluded from both
   pass and fail counts and from capability computation
   (`profile.rs:172`), rendered in its own report section
   ("Extensions — not applicable to this SUT"). This is how "extension —
   N/A upstream" appears in data, unambiguous to a machine reader.
4. **The upstream adjudication register — data, not code.** A committed
   TOML file `tools/conformance/adjudications/ehrbase-java-2.34.toml`,
   loaded via `--adjudications <file>`, consulted in the executor loop
   (`engine/run.rs:76-120`). Entry shape: ECC id (or an area-wide rule) +
   `disposition` + `reason` + citation. Dispositions:
   - `extension` → `NotApplicable` (DEM wire, TS openEHR-bundle cases, SIG;
     MSG/SEC already self-handle, §2a);
   - `rm-version-sensitive` → `NotApplicable` with the RM/ITS-version
     citation (cases whose request payload or response comparison depends
     on RM 1.2.0 shapes upstream's archie 3.13.0 / ITS-REST 1.0.2 surface
     cannot be expected to produce);
   - `defect` → stays a **failure**, with the spec citation (a genuine
     upstream spec gap survives triage and is reported plainly — e.g.
     upstream rejects `ALL_VERSIONS`, ADR-008 §2).
   Rule: the register **only reclassifies**; it never edits a case
   (standing rule 3 / honesty rule 10). Running with no register = today's
   behaviour, byte-for-byte (zero-drift gate on our own baseline).
5. **Admin runs for real instead of being adjudicated away.** New optional
   `--admin-base-url` (upstream: `http://host:8091/ehrbase/rest/admin`,
   with `ADMINAPI_ACTIVE=true` env); `Adm` suite paths (and
   `sec/forbidden-role-403`) resolve against it when set, else against the
   openehr base as today. Real upstream admin data beats an N/A row.
6. **The fairness-triage process (X1.2)** is itself recorded: first run →
   every failure gets a triage note (runner-tolerance fix / register entry
   / genuine defect) committed with the register; second run is the
   published one. Profile verdicts for upstream are computed by the same
   machinery and labelled "under the ECC catalogue" (honesty rule 4).

### 3b. Benchmark overhaul (implementing the harness's own design)

The design doc is the spec; X1.3 closes the design-vs-code gap (§2b), in
dependency order:

1. **Provenance first** — extend `EnvBlock` (`report.rs:13-32`) with per-SUT
   `{ image, image_digest, pg_image, pg_version, cpu_limit, mem_limit }`;
   captured via `docker image inspect` / `docker compose ps` at run start
   by `run.sh`, passed via new `bench run` flags. Add `cpus:`/`memory:`
   pins to `docker/benchmark/docker-compose.yml` (equal both sides, design
   §3.1) and bump/pin the Java images to 2.34.0 by digest.
2. **Resource sampling** — new `stats.rs` per the design
   (`benchmarking.md:206`): a sampler task shelling out to the official
   CLI, `docker stats --format '{{json .}}'` (streaming, ~1 Hz), parsed
   per container (CPU%, RSS); attached to each scenario result as
   idle/loaded summaries. Storage footprint via
   `docker exec <db> psql -c "SELECT pg_total_relation_size(...)"` per
   rung (design §3.5). **Cold-start**: `run.sh` times compose-up → first
   200 on `/rest/status`, N=5, reported as its own labelled metric (never
   mixed into warm latencies). **Image size**: from `docker image inspect`
   (plus the registry compressed size, labelled separately). Official-CLI
   parsing over a Docker-API crate: zero new dependencies, and the repo's
   official-CLI-first rule.
3. **Open-loop concurrency sweep** — a second driver mode
   (`DriverConfig` gains a concurrency dimension): fixed-rate open-loop at
   ramping concurrency {1,2,4,8,16,32,64}, fixed duration per step,
   **finally wiring `record_corrected`** (`measure.rs:45-57`) for
   coordinated-omission-honest tails; capture the throughput-vs-latency
   knee and an **error rate** (statuses counted during the measured
   window, fixing `driver.rs:138`).
4. **Workload completion** — add W3 (large/nested composition), W7 (real
   multi-version CONTRIBUTION commit — replacing the placeholder read,
   `workload.rs:415-425`), W9 (AQL CONTAINS chain), W10 (AQL
   aggregate/ORDER BY), W13 (mixed 70/30 blend). Fixtures from the CNF
   corpus as today. **Re-freeze `workload.lock` before the first measured
   run** (`workload.rs:492-511`) — pre-registration discipline.
5. **Scale ladder** — deterministic seeder rungs per the design
   (`benchmarking.md:91-100`): `empty` and `10k` are mandatory for the
   published run; `100k` runs if the run host can seed it in reasonable
   time; `1M` is explicitly labelled future work on dedicated hardware
   (an unmeasured rung is *absent*, never extrapolated). Deterministic
   ids replace the `AtomicU64` counter (`workload.rs:40-45`).
6. **PG-version confound, stated not hidden** — the design's "controlled"
   both-on-PG16 run (`benchmarking.md:119-129`) is **impossible for us**
   (ADR-008 storage is PG18-native: `uuidv7()`, temporal PKs). We attempt
   the inverse control (both on PG 18) — if upstream runs on PG18, publish
   it as the controlled pair; if not, the recommended run (each on its
   vendor stack: us PG18, them PG16.2) stands alone **with the confound
   stated in the limitations**, exactly as the design prescribes
   (`benchmarking.md:128-129`).

### 3c. The comparison page (`website/book/src/comparison.md`)

Structure (top to bottom):
1. **The honesty header** — what this page is (our measurements, our
   instrument, both servers' versions + digests + run dates) and is not
   (official certification, vendor-neutral lab work). Links: methodology,
   raw artifacts, adjudication register, reproduce-it commands.
2. **Feature matrix** — hand-curated, one row per capability (REST APIs,
   AQL envelope incl. `ALL_VERSIONS`, canonical JSON+XML, versioning
   semantics, templates/validation depth, demographic, EHR Extract/TDD,
   terminology integration, signing, auth modes, multi-tenancy, ATNA,
   FHIR/AMQP/S3, admin). **Every cell is verifiable**: our side cites the
   docs-site chapter / conformance artifact; upstream's cites their docs
   or release notes (URL + date checked). Upstream-only capabilities
   (e.g. their plugin system) get a plain "not supported" on our side.
3. **Conformance table** — generated from the two `results.json` files:
   per-area executed / passed / failed / N-A (extension) / skipped for
   both servers + the profile verdicts, with the adjudication register
   linked inline. Upstream's genuine-defect failures listed with
   citations; ours (currently zero) the same rule.
4. **Performance** — generated tables + static SVG charts (latency
   distributions per scenario, throughput-vs-concurrency curves, the
   scale-ladder AQL series): both servers side-by-side, winner marked only
   outside the ±10% noise band (`report.rs:341-362`), CoV shown.
5. **Footprint** — image size (compressed + on-disk), cold-start time,
   idle/loaded RSS, CPU at fixed load, storage bytes/composition per rung.
6. **"Where EHRbase wins"** — mandatory, populated from the generated
   data + the feature matrix.
7. **Methodology & limitations** — the PG confound, JVM-tuning
   sensitivity, single-host caveat, catalogue-authorship bias (rule 2),
   RM-version skew; reproduce-it block.

**Data pipeline:** a generator (`tools/comparison` bin or a `bench
compare`/`conformance compare` subcommand — decide at X1.5 by whichever
avoids a new crate) reads the committed
`docs/conformance/results.json` + `docs/conformance/upstream-ehrbase/results.json`
+ `docs/benchmarks/results.json` and emits
`website/book/src/comparison/_generated/*.{md,svg}` (tables + hand-rolled
static SVG charts, ~200 LoC emitter, dataviz-convention palette readable on
light+dark, no JS/CDN). `scripts/assemble-comparison.sh --check` regenerates
and `git diff --exit-code`s — wired into `docs.yml` beside the OAS gate, so
**page ≠ raw data is structurally impossible**. `SUMMARY.md` gains the
chapter after Conformance; the landing page's conformance strip gains a
"Compare with EHRbase" link (one line, no other landing changes).

### 3d. Refresh discipline

- **Per-release rerun:** the `/phase-done`-adjacent release checklist (and
  the release checklist) gains: rerun ECC-upstream + the
  benchmark suite, regenerate `_generated/`, before tagging. Benchmarks
  stay **manual/dispatch on consistent hardware** — never shared CI
  runners for absolute numbers (`benchmarking.md:274-282`).
- **Upstream watch:** a scheduled monthly workflow hits
  `api.github.com/repos/ehrbase/ehrbase/releases/latest`; if newer than
  the version stamped in the committed comparison data, it opens an issue
  ("comparison data stale: upstream released X"). The page's data blocks
  carry their stamps regardless (honesty rule 6), so even an unrefreshed
  page never lies about what it measured.
- **Drift gate:** `assemble-comparison.sh --check` in `docs.yml` (§3c)
  keeps page and raw data in lockstep on every PR.

---

## Preconditions

- [x] W1 done — the docs site is live (develop `6742d8710`, PR #64).
- [x] ECC green baseline — 341 executed · 315 passed · 0 failed (B6).
- [x] Benchmark harness + design doc + dual-stack composes exist (§2b).

## Scope

In: ECC upstream mode + register + upstream run; benchmark overhaul
(§3b 1–6) + dual runs; the comparison page + generator + gates; refresh
discipline.
Out: official openEHR certification (no program exists to run); benchmarking
on dedicated server hardware (the run host is recorded; a bare-metal rerun is
future work); the 1M scale rung (labelled future work unless the host
allows); upstream bug-fixing (defects are reported, cited, and optionally
filed upstream — not worked around).

## Tasks

- [x] **X1.1 ECC upstream-SUT mode + adjudication register.** *(2026-07-11:
  `SutIdentity.product{name,version,image_digest}` + `--sut-*` flags (default
  `ehrbase-rs @ <workspace ver>`); `CaseStatus::NotApplicable` excluded from
  pass/fail + capability math with its own report section + N/A columns;
  `AdjudicationRegister` TOML loader (`extension`/`rm-version-sensitive` →
  N/A, `defect` stays a failure) applied at the `engine/run.rs` executor seam
  **only for non-ehrbase-rs SUTs**; Statement+Certificate suppressed (log
  line) for non-self SUTs and their Solution/Vendor de-hard-coded;
  `--admin-base-url` routes `/admin/*` to a sibling mount via
  `SutClient::with_admin_base_url` (no case edits); seed register
  `adjudications/ehrbase-java-2.34.toml` (DEM/SIG extensions, cited) + README.
  67/67 nextest, conformance clippy-clean, no-register ehrbase-rs run is
  zero-drift bar the new `product` field.)*
  - [x] `SutIdentity.product {name, version, image_digest}` + CLI flags;
        threaded through report/statement; Certificate/Statement emitted
        only for ehrbase-rs SUTs (`statement.rs` de-hard-coded).
  - [x] `CaseStatus::NotApplicable` (excluded from pass/fail + capability
        math, own report section).
  - [x] `--adjudications <file>` TOML register + loader, hooked in
        `engine/run.rs`; dispositions `extension` /
        `rm-version-sensitive` / `defect` per §3a.4, every entry cited.
  - [x] `--admin-base-url` for sibling-mounted admin APIs (`suites/admin.rs`
        + `security.rs:120`).
  - **Acceptance:** nextest + clippy green; a no-register ehrbase-rs run
    reproduces the standing baseline with **zero drift** (only the new
    identity field added); unit tests cover register reclassification and
    certificate suppression.
- [ ] **X1.2 Run ECC against upstream EHRbase 2.34.0; publish.**
  - [ ] Bump `docker/conformance/docker-compose.yml` to `ehrbase/ehrbase:2.34.0`
        (+ digest pin recorded), `ADMINAPI_ACTIVE=true`, admin base URL wired
        in `run.sh`.
  - [ ] Triage run: every upstream failure dispositioned (runner-tolerance
        fix / register entry with citation / genuine `defect`); register
        committed as `adjudications/ehrbase-java-2.34.toml`.
  - [ ] Published run → `docs/conformance/upstream-ehrbase/` (results.json +
        report + catalog; **no certificate**); our own baseline re-run for
        the same-day pair; both identity-stamped.
  - **Acceptance:** zero unexplained upstream failures (each is a cited
    defect or a register entry); our baseline shows zero drift; the two
    results.json files carry distinct product identities + digests.
- [ ] **X1.3 Benchmark overhaul** (§3b, in order).
  - [ ] Provenance: EnvBlock per-SUT image/digest/PG/pins; compose resource
        pins; images bumped to 2.34.0 by digest; CLI help W-id fix
        (`bin/bench.rs:48,141`).
  - [ ] `stats.rs`: docker-stats CPU/RSS sampling, cold-start timing, image
        size, `pg_total_relation_size` per rung.
  - [ ] Open-loop concurrency sweep + `record_corrected` wired + error-rate
        capture.
  - [ ] Scenarios W3/W7/W9/W10/W13 (real CONTRIBUTION commit replaces the
        placeholder); deterministic seeder + scale-ladder rungs;
        `workload.lock` re-frozen **before** any measured run.
  - **Acceptance:** nextest + clippy green; `--smoke` run against the rs
    stack emits every new field (resources, cold-start, image, storage,
    error rate, concurrency series); pre-registration commit lands before
    X1.4 measurements.
- [ ] **X1.4 Run the benchmarks against both servers; publish.**
  - [ ] Recommended run (us on PG18, them on PG16.2), ≥5 runs/scenario,
        full profiles (latency, sweep, ladder rungs feasible on host);
        attempt the both-PG18 controlled pair, keep or drop with the
        outcome stated.
  - [ ] Publish `docs/benchmarks/REPORT.md` + `results.json` with both
        targets, full env/provenance block, populated "Where EHRbase wins",
        limitations incl. the PG confound.
  - **Acceptance:** the committed report is a full-config (not smoke) run;
    every table cell traces to results.json; no unexplained empty cells;
    win/loss marked only outside the noise band.
- [ ] **X1.5 The comparison page.**
  - [ ] Generator + `scripts/assemble-comparison.sh --check`;
        `_generated/` tables + SVG charts from the three committed
        results.json artifacts; gate wired into `docs.yml`.
  - [ ] `comparison.md` per §3c (honesty header, cited feature matrix,
        conformance table, performance, footprint, "Where EHRbase wins",
        methodology/reproduce-it); `SUMMARY.md` entry; landing link;
        CHANGELOG `[Unreleased]` entry (user-visible).
  - **Acceptance:** `mdbook-lint` + lychee green; `--check` red on a
    hand-edited fragment (negative-tested once, reverted — the W1.7
    discipline); every feature-matrix cell carries a citation; every data
    block carries date + versions + digests.
- [ ] **X1.6 Refresh discipline.**
  - [ ] Upstream-release watch workflow (monthly, opens a staleness issue).
  - [ ] Release-checklist hook: rerun both suites + regenerate before a tag
        (release-checklist note + `/phase-done` item).
  - [ ] Stale-marking rule documented on the page ("data measured against
        EHRbase X on DATE; newer upstream releases are flagged, not
        silently assumed comparable").
  - **Acceptance:** the watch workflow dry-runs green; the checklist items
    exist; the page states the staleness policy.

## Exit criteria

- [ ] Upstream EHRbase 2.34.0 actually executed through ECC; published under
      `docs/conformance/upstream-ehrbase/` with SUT identity, the cited
      adjudication register, and no certificate artifact; our baseline
      untouched (zero drift).
- [ ] The overhauled benchmark ran both servers full-config on one host;
      `docs/benchmarks/` carries the dual-target report with provenance,
      resources, cold-start, image size, storage, concurrency sweep, and a
      populated "Where EHRbase wins".
- [ ] The comparison page is live on the docs site, drift-gated against the
      raw artifacts, every number date/version/digest-stamped, methodology +
      raw data linked, extension routes shown as N/A, upstream wins stated.
- [ ] Refresh discipline installed (watch workflow + release checklist +
      staleness policy).
- [ ] Workspace green (nextest, clippy) and our own ECC baseline ratchet
      intact (blueprint §4 rule 4).

## Decisions made this phase

- (at planning) Certificate/Statement are self-assessment artifacts — never
  emitted for an upstream SUT (§3a.2).
- (at planning) `NotApplicable` is a first-class outcome, not a skip flavour
  (§3a.3).
- (at planning) The adjudication register is committed data with citations,
  applied at the executor seam — cases are never edited (§3a.4).
- (at planning) Admin is exercised for real via `--admin-base-url`, not
  adjudicated away (§3a.5).
- (at planning) Resource sampling shells out to the official docker CLI —
  no Docker-API crate (§3b.2).
- (at planning) The design's PG16 controlled run is impossible (PG18-native
  storage); the confound is stated, with a both-PG18 attempt (§3b.6).
- (execution decisions — generator home, SVG emitter shape, ladder rungs
  actually run — recorded here as they land.)

## Open items needing owner decision

1. **Benchmark hardware:** publish first numbers from the dev laptop
   (env-stamped, honest but noisy) or wait for a dedicated box? Plan
   assumes laptop-first with the host recorded; a bare-metal rerun replaces
   it later.
2. **Upstream defects:** file surviving `defect` findings as upstream GitHub
   issues (good citizenship, but publicizes the comparison early) or only
   publish with citations?
3. **Page tone ruling** on the feature matrix rows for upstream-only
   features (e.g. plugin system): confirm "not supported" plain-cell style.

### Owner rulings (2026-07-11)

1. **Benchmark environment:** publish the first numbers from the dev machine,
   fully env-stamped (hardware, OS, versions, digests) with a visible caveat;
   re-measure on dedicated hardware later.
2. **Upstream defects:** publish with spec citations only — do **not** file
   upstream GitHub issues.
3. **Feature matrix:** plain "not supported" in both directions — identical
   wording for our gaps and theirs; partial support gets ⚠ with a note; no
   roadmap speculation in any cell.

## Handoff for next session

Plan drafted and committed on `claude/x1-comparison-plan`; nothing
implemented. Research is embedded above with file:line — re-verify §2c's
upstream facts (releases move) and the two compose pins (2.33.0 → 2.34.0)
before X1.2/X1.4 runs. Start with **X1.1** (the ECC identity/adjudication
seams) — it unblocks the only genuinely new machinery; the benchmark work
(X1.3) is closing the gap to `docs/design/benchmarking.md`, which remains
the governing spec. `current-phase.md` still points at W1/tail work; repoint
it to this file when X1 is activated.
