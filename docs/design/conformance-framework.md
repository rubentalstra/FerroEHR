# openEHR CNF Conformance Framework — design

- **Status:** designed, **v2 (2026-07-07)** — re-verified against the official
  GitHub repository and updated for the now-implemented version-signing and
  access-control subsystems. Ready to implement.
- **Stage:** the ADR-008 acceptance instrument (P19, deliberately pulled forward —
  every feature landed before this exists is unverified against the real oracle)
- **Owner:** —
- **Sources (all verified 2026-07-07):** the vendored CNF corpus at
  `docs/specs/openehr/CNF/` (upstream `openEHR/specifications-CNF` @ `33251d2a`,
  946 files) — **confirmed identical to the live upstream `master` HEAD**
  (compare API: 0 ahead / 0 behind; last upstream commit 2024-08-06; the three
  open issues/PRs are 2017–2023 stale). The CNF material is a *frozen* target:
  no re-vendoring needed, and the coverage guard (§4.2) is keyed to a stable
  corpus. Also: the P19 phase file, the `/run-conformance` skill contract, the
  existing e2e/compose harnesses, and the excluded upstream `.py` helpers
  (fetched and assessed 2026-07-07 — see §2.2a).
- **Related:** `docs/enterprise/atna-audit.md` + `docs/enterprise/access-control.md`
  (implemented) + `docs/design/version-signing.md` (implemented — closes the
  STANDARD Signing gap this design's v1 flagged), `docs/spec-audit/SPEC_AUDIT.md`
  (the `F-AA-NN` findings register failures feed).

---

## 1. Goal

Make the sentence *"EHRbase-rs is conformant to the openEHR platform
specifications"* *provable, reproducible, and public*: a runner that executes
the official **openEHR Platform Conformance Test Schedule** against a running
server, a committed per-test-case results matrix, and a published **Conformance
Statement** scoped exactly the way the CNF framework defines claims (profile ×
capability × protocol × data format). No hand-waving: the claim is generated
from the run, and a failing case is a tracked finding — never a skipped test.

## 2. What the CNF corpus actually is (inventory summary)

Full inventory in the 2026-07-07 investigation; the load-bearing facts:

### 2.1 The normative layer — `CNF/docs/` (this is the oracle)

- **Conformance Guide** (`docs/guide/`): conformance is assessed against a
  *deployed real system* (the SUT) at a concrete technology binding (REST +
  JSON/XML). Two aspects: **API conformance** (call-in test cases vs reference
  results) and **data-validation conformance** (variable data sets vs reference
  validity). The result artefacts are a **Test Execution Report**, a
  **Conformance Statement** (vendor-published), and a **Conformance
  Certificate** (issued by an assessment agency). The guide's
  Statement/Report/Tooling sections are literally `TBD` — the only concrete
  template is the Certificate.
- **Profiles** (`docs/profiles/master03-profiles.adoc`): claims are made per
  **profile**, composed of **capabilities**:
  - **CORE** — ADL 1.4 archetype + OPT 1.4 provisioning; EHR operations,
    EHR_STATUS, COMPOSITION operations, change sets, versioning, archetype
    validation; DEFINITION + EHR REST APIs; anonymous EHRs. *All capabilities
    must pass — all-or-nothing.*
  - **STANDARD** — CORE **plus** query provisioning, directory operations,
    AQL basic, the QUERY API, and **Signing**.
  - **OPTIONS** — any optional capability (demographic, admin, messaging,
    ADL2/OPT2, AQL advanced, terminology-integrated AQL) reported
    individually.
- **Certificate template** (`docs/certificate/master03-certificate.adoc`):
  the claim tables — SUT identity; scope (profiles, security, data formats);
  a **Detailed Test Report** (`conformance point × test case × protocol →
  pass/FAIL`); a **Profile Report** (`capability × required-in-profile →
  result`). This is the schema our generated report mirrors.
- **Platform Conformance Test Schedule** (`docs/platform_test_schedule/`):
  **322 identified test cases** across two families:

  | Chapters | Family | Cases | Test-case id form |
  |---|---|---|---|
  | master04–13 | functional (API) | 203 | `I_<SERVICE>.<operation>-<variant>` |
  | master15–17.x | content (data validation) | 119 | `CONT-<CLASS>-<variant>` |

  Functional chapters carry **normative test-data-set tables** (e.g. the 16
  valid EHR_STATUS combinations for `create_ehr`); a "test" = one case × one
  data set. Content chapters embed truth tables (`value → accepted/rejected +
  constraint violated`). Expected results are prose ("positive/negative
  response"); exact status codes come from the ITS-REST spec (which we already
  treat as the wire oracle). Known holes **in the schedule itself**: the QUERY
  chapter (master11) is mostly `TBD` stubs, master17.5 (time specification) is
  empty, and there is no master14.
- **RM version rule** (schedule overview): minimum RM 1.0.2; *"the supported
  RM version(s) … should be stated in the Conformance Statement, because this
  will determine some variations on the data sets used for testing."* We run
  RM 1.2.0 — a declared property of the claim, not a deviation.

### 2.2 The executable layer — `CNF/tests/platform/robot/` (prior art + fixtures, NOT the oracle)

~207 Robot Framework suites, one per functional case, named by case id. Facts
that disqualify it as our primary instrument:

1. It is **EHRbase's own harness** re-hosted (`_resources/README.txt`: "From
   ehrbase commit 157a0607"; vitasystems copyright): direct Postgres backdoors
   into EHRbase's schema (`db_keywords.robot`), `java -jar` server lifecycle,
   EHRbase-specific error-message assertions and node names.
2. It is **not executable as vendored**: our vendoring excludes `.py`, so
   `variables/sut_config.py` and all four Python helper libraries are absent;
   upstream pins a 2021-era stack (Robot 4.0.3, pyjwt 1.7, psycopg2).
3. Its **coverage doesn't match the schedule**: no robots for demographic
   (24 cases), messaging (14), or any content chapter (119 — upstream validated
   those through a missing Python lib); conversely `I_QUERY_SERVICE` has real
   robots + a huge fixture corpus backing a schedule chapter that is `TBD`.

What it *is* good for — **fixtures and expected-behaviour prior art**, directly
reusable by our runner:

- 52 `.opt` templates (valid + invalid classes: alien tags, removed mandatory
  elements, removed template id, empty file …)
- compositions in 6 formats (canonical JSON 10, canonical XML 7, FLAT 27,
  STRUCTURED 4, TDD 6, valid 4), contributions, EHRs, directory trees
- the **AQL corpus**: ~119 valid queries in groups A–D, invalid queries, data
  loads, and **golden `expected_results` for empty and loaded DBs**
- suite-layout YAML maps and the keyword files as a readable record of the
  exact HTTP sequences EHRbase's harness performed per case.

### 2.2a The excluded upstream `.py` helpers (fetched from GitHub, assessed — do NOT vendor)

Our vendoring excludes `*.py`; the five missing files were fetched from the
official repo and assessed. Verdict: **keep them un-vendored** — three are pure
EHRbase/Keycloak/docker plumbing (`sut_config.py`, `dockerlib.py`,
`token_decoder.py`), but two carry ideas this design adopts natively:

- **`jsonlib.py` (the response-assertion engine, ~340 LoC, DeepDiff):** its
  comparison modes are the semantic our `assert.rs` mirrors — (a)
  **exact match**, (b) **superset match** (the response may carry more than the
  expected fixture — used where servers legitimately add fields), and (c)
  **ignore-sets** for RM `_type`/metadata/path keys when diffing. Each
  registry case declares which comparison mode its payload assertion uses.
- **`composition_validation_lib.py` (~90 LoC):** the content-chapter fixtures
  are generated by two mutators — set a high-level field
  (`language`/`territory`/`category`/`composer`) to `exist` / `not_exist` /
  `invalid` (e.g. corrupt its `_type`), and pad/trim array items to a target
  count. Our content suites (§4.1 `suites/content/`) implement these as
  **typed Rust fixture mutators** over the vendored base compositions instead
  of hand-maintaining hundreds of static variants.

Also confirmed upstream: the Robot invocation (`run_local_tests.sh`,
`Taskfile.yml`) is an EHRbase harness artifact (RF 4.0.3, `ehrbase/ehrbase:13.3`
docker), **not** a spec-defined runner. The one convention worth honoring is the
**tag taxonomy**: upstream excludes `future`, `obsolete`, `TODO`, `not-ready`
suites from scoring — our registry records the upstream tags per transcribed
case (in `CaseMeta`) so provenance-aware filtering matches upstream's own
notion of "stable".

### 2.3 Design consequence

**The Test Schedule is normative; the Robot suite is data.** Our runner
implements the schedule's identified cases natively in Rust, reuses the
fixture corpus, and cites the schedule (not EHRbase's harness) as the
authority — exactly the ADR-008 posture. Running upstream's Robot verbatim is
neither possible (missing files) nor desirable (EHRbase-specific), and is
**not** what a conformance claim requires: the guide requires executing the
*test schedule* against the SUT and publishing the results.

## 3. The claim we are building toward (honest scoping)

Target public claim, generated — never hand-written — from a run:

> **EHRbase-rs `<version>` conforms to the openEHR STANDARD profile**
> (REST API binding; canonical JSON and XML; RM 1.2.0), evidenced by the
> attached Test Execution Report over the openEHR Platform Conformance Test
> Schedule (`specifications-CNF` @ `33251d2a`), with the deviations register
> below.

Scoping decisions (each visible in the generated report):

1. **Profile target: STANDARD** (= CORE + query provisioning + directory +
   AQL basic + QUERY API + Signing). **Signing is now implemented**
   (`docs/design/version-signing.md`: `VERSION.signature`, digest default-on +
   OpenPGP RFC 4880 mode, `canonical_form()` per RFC 8785) — the full
   STANDARD claim is reachable. Upstream ships **zero** test material for the
   Signing capability (verified against HEAD: the only signature hits are
   Keycloak crypto config and a clinical coded-text value), so *our*
   runner-defined `SIGN-*` cases (§4.6) are the capability's entire evidence
   base, declared as such in the statement.
2. **OPTIONS capabilities we run anyway** (we implement them): ADMIN API
   (master12 subset), DEMOGRAPHIC API (master10 — we mount the generated
   demographic group). Reported as OPTIONS passes; never blended into the
   CORE/STANDARD claim.
3. **Excluded, with reasons in the deviations register**: ADL2/OPT2 (explicit
   501, OPTIONS-only per profiles), MESSAGING (master13 — not implemented,
   OPTIONS-only), FLAT/STRUCTURED fixtures (EhrScape interop layer —
   explicitly not CNF-gated; they stay in the `openehr-flat` test suite).
4. **Schedule holes handled honestly**: master11 (QUERY) being `TBD` prose is
   supplemented by the **AQL fixture corpus as runner-defined cases**
   (`QUERY-FIXTURE-<group>-<name>` ids, provenance-tagged as
   "fixture-derived, schedule chapter TBD upstream"); master17.5 (0 cases)
   reported as "no normative cases published". Re-verified 2026-07-07: both
   remain stubs at upstream HEAD, and upstream is dormant — these fills are
   long-lived, not temporary.
5. **Security scope — now claimable.** With RBAC + ABAC implemented
   (`docs/enterprise/access-control.md`), the CNF `SECURITY_TESTS` intent
   (Basic + OAuth2/Keycloak flows, 401/403 behaviour, role-gated admin) is in
   scope: the runner parameterizes auth (Basic user + admin credentials;
   Bearer via the compose stack's existing Keycloak service + realm import)
   and runs the functional chapters under **RBAC enabled** — master12 admin
   cases require the ADMIN role, which is itself part of what's being proven.
   The declared run configuration in the statement: **RBAC on, ABAC off**
   (ABAC models deployment-specific policy, which CNF does not test; it stays
   config-off exactly as a fresh install ships).
6. **RM 1.2.0 declared** in the statement. Fixture payloads authored in the
   RM 1.0.x era are adapted where the wire shape legitimately changed, each
   adaptation recorded in the fixtures' provenance file (see §6).

## 4. Architecture

One new workspace crate + one thin shell entrypoint + one CI job + one
generated report set.

### 4.1 Crate: `crates/ehrbase-conformance`

An application-layer crate (test harness; never a dependency of the server —
no crate depends on it). Library + CLI binary:

```
crates/ehrbase-conformance/src/
├── lib.rs
├── case.rs        # TestCase model: id, chapter, capability, profile, protocol,
│                  #   format, provenance (Schedule | FixtureDerived), run fn
├── registry.rs    # the static registry of all cases, keyed by CNF id
├── client.rs      # SUT client: reqwest (rustls) + auth (Basic/Bearer) + the
│                  #   canonical JSON/XML codecs from openehr-its for assertions
├── sut.rs         # SUT lifecycle: External (BASE_URL) | SelfHosted (in-process
│                  #   serve_with + testcontainers PG18)
├── assert.rs      # response assertions: status, headers (ETag/Location/…),
│                  #   payload comparison in the upstream jsonlib semantics —
│                  #   Exact | Superset | IgnoreSet(RM _type/meta/path keys) —
│                  #   declared per case; RESULT_SET diffing against goldens
├── fixtures.rs    # typed access to docs/specs/openehr/CNF/tests/…/_resources
│                  #   + our adapted fixture overlay (see §6)
├── suites/        # the transcribed cases, one module per schedule chapter
│   ├── ehr.rs             # master06 → I_EHR_SERVICE.* + I_EHR_STATUS.*
│   ├── composition.rs     # master07
│   ├── contribution.rs    # master08
│   ├── directory.rs       # master09
│   ├── definition_adl14.rs# master04 (ADL 1.4 half)
│   ├── definition_query.rs# master05
│   ├── query.rs           # master11 stubs + QUERY-FIXTURE-* corpus cases
│   ├── admin.rs           # master12 (OPTIONS)
│   ├── demographic.rs     # master10 (OPTIONS)
│   └── content/           # master15/16/17.x — validation truth tables
│       ├── mutate.rs      #   typed fixture mutators (the upstream
│       │                  #   composition_validation_lib catalogue: field →
│       │                  #   Exist|NotExist|Invalid, array count pad/trim)
│       ├── composition.rs #   (commit mutated variant → expect accepted/rejected)
│       ├── entry.rs
│       └── data_types.rs  #   17.1–17.7, table-driven
├── sign.rs        # the runner-defined SIGN-* capability cases (§4.6)
└── report.rs      # results.json + RESULTS.md + CONFORMANCE_STATEMENT.md +
                   #   badge JSON (shields endpoint schema)
└── bin/conformance.rs     # the CLI (clap)
```

Dependencies (all already in `[workspace.dependencies]`): `reqwest`, `serde`/
`serde_json`, `quick-xml` (via `openehr-its` codecs), `openehr-its` +
`openehr-rm` (typed payload assertions), `openehr-query` (AQL corpus parse
checks), `clap`, `jiff`, `thiserror`, `tracing`; dev/self-host mode:
`testcontainers`, `ehrbase` + `ehrbase-rest` (to boot the real app
in-process). The self-host path lives behind a `self-host` cargo feature so
the CLI can also be built lean for external-SUT-only use.

### 4.2 The case model (the heart of the design)

```rust
pub struct CaseMeta {
    pub id: &'static str,            // "I_EHR_SERVICE.create_ehr-main" | "CONT-DV_ORDINAL-validate_open"
    pub chapter: Chapter,            // Master06, … Master17_7 — book provenance
    pub capability: Capability,      // EhrOperations | CompositionOps | AqlBasic | … (profiles doc)
    pub profiles: &'static [Profile],// which profiles require this capability (Core/Standard/Options)
    pub formats: &'static [Format],  // Json, Xml — a case runs once per claimed format where applicable
    pub provenance: Provenance,      // Schedule | FixtureDerived | RunnerDefined (§3.4, §4.6)
    pub schedule_ref: &'static str,  // "master06-func_tc_ehr.adoc §Test Case I_EHR_SERVICE.create_ehr-main"
    pub upstream_tags: &'static [&'static str], // the Robot suite's tags where one exists
                                     // ("refactor", "not-ready", "future", …) — upstream's own
                                     // stability signal, reportable and filterable (§2.2a)
    pub compare: Compare,            // Exact | Superset | IgnoreSet — the jsonlib semantics (§2.2a)
}
```

- Each schedule case becomes one registry entry whose run function executes
  the case's Flow steps over the SUT client and asserts per the ITS-REST spec
  (status codes, headers, payload shapes) — the schedule's prose
  ("positive/negative response") is concretized by citing the ITS-REST
  section in the assertion message, the same dual-citation discipline the
  spec-audit uses.
- **Data-set expansion**: the normative data-set tables (e.g. the 16
  EHR_STATUS combinations) are encoded as const tables; the case iterates
  them, so the report can say "case passed, 16/16 data sets".
- **Total-coverage guard** (the house pattern, third use): a unit test parses
  `docs/specs/openehr/CNF/docs/platform_test_schedule/*.adoc` for the two
  test-case heading regexes and asserts every extracted id is either in the
  registry or in the explicit `EXCLUDED` list with a reason enum
  (`NotImplemented(Messaging)`, `Adl2Returns501`, `UpstreamTbd`,
  `UpstreamEmpty`) — a schedule change on re-vendor breaks the build until
  triaged. The 322-case inventory above is thereby *enforced*, not aspirational.

### 4.3 SUT modes (both required)

- **External** (`--base-url … --auth basic:user:pass|bearer:… [--admin-auth …]`):
  the guide's own model — assess a *deployed real system*. This is what runs
  against the compose stack / published GHCR images and what a
  certification-grade report uses. Two credential slots: a regular clinical
  user and an ADMIN-role credential (master12 admin cases run under the
  latter; a deliberate USER-role attempt asserting 403 is part of the
  security evidence). Bearer mode targets the compose stack's existing
  Keycloak service (realm import already in `docker/keycloak/`). The SUT is
  expected to run with **RBAC enabled, ABAC off** (§3.5) — the runner records
  the SUT's auth mode in `results.json`. No DB access, no lifecycle control:
  the runner is a pure API client (unlike EHRbase's harness, we do not reach
  into the database — cases are written to be self-contained through the API,
  using fresh EHRs per case rather than DB cleans).
- **SelfHosted** (`--self-host`, feature-gated): boots testcontainers PG18 +
  `EhrbaseService::new(pool)` + `ehrbase_rest::serve_with` on an ephemeral
  port — the fast inner loop for development and the PR-time CI subset.
  Reuses the `Pg` helper pattern from `crates/ehrbase/tests/service_ehr.rs`
  (extracted into the crate, not duplicated a seventh time).

### 4.4 CLI + `scripts/conformance.sh` (the `/run-conformance` contract)

```
conformance run   [--base-url URL | --self-host] [--filter SUBSTR] [--profile core|standard|options]
                  [--format json|xml|both] [--out docs/conformance/]
conformance list  [--filter …]        # print registry with metadata
conformance report --from results.json # regenerate MD artifacts without a run
```

Exit codes: `0` all selected cases pass · `1` failures (report still written)
· `2` runner/SUT error. `scripts/conformance.sh` is a thin wrapper satisfying
the skill contract (one optional filter arg): brings up the compose stack
(reusing `docker/smoke-test.sh`'s wait-healthy logic), runs
`cargo run -p ehrbase-conformance --features self-host -- run …` against it,
tears down. Filter argument maps to `--filter`.

### 4.5 Reports (committed, generated — the public face)

Written to `docs/conformance/`:

- **`results.json`** — machine-readable: per case id → pass/fail/excluded ×
  format, data-set counts, durations, SUT identity (version, git sha, RM
  version), corpus pin (`specifications-CNF` commit).
- **`RESULTS.md`** — the per-chapter matrix (the P19/next-session contract):
  chapter → cases passed/failed/excluded, with failure links.
- **`CONFORMANCE_STATEMENT.md`** — generated following the **Certificate
  template's** table structure (SUT identity, Scope of Test, Detailed Test
  Report, Profile Report) + the RM-version declaration and the deviations
  register (§3's exclusions, each with reason + spec citation). Regenerated
  per release; committed so the README can link it.
- **`badge.json`** — shields.io endpoint schema (e.g.
  `openEHR CNF: 289/301 · CORE ✓`), so the README badge the owner wants is
  data-driven: `![CNF](https://img.shields.io/endpoint?url=…badge.json)` via
  raw.githubusercontent. The badge never says "100%" unless the run does.

Failure workflow (binding): every failing case gets a spec-audit-style finding
(`F-AA-NN` in `docs/spec-audit/findings/`, citing both the CNF case id and the
ITS-REST/RM clause) before or alongside the fix; the runner's failure output
prints the template to make that a copy-paste. **Never** move a failing case
to `EXCLUDED` to green a run — exclusion reasons are structural
(not-implemented/upstream-TBD), not "currently failing".

### 4.6 The `SIGN-*` capability cases (runner-defined; the STANDARD Signing evidence)

Upstream has zero Signing test material (§3.1), so these runner-defined cases
— specified against the implemented behaviour in
`docs/design/version-signing.md` — are the capability's entire evidence base
(`provenance: RunnerDefined`, declared in the statement):

| Id | Asserts |
|---|---|
| `SIGN-digest-present` | committed composition's VERSION read (JSON + XML) carries `signature` matching `sha256:<base64>` |
| `SIGN-digest-recomputes` | the digest recomputes from the **served** VERSION's canonical form (RFC 8785 over the signature-voided object) — commit-time and read-time object identity |
| `SIGN-all-kinds` | EHR_STATUS update, FOLDER write, and a multi-version CONTRIBUTION all yield signed versions |
| `SIGN-client-verbatim` | a CONTRIBUTION `UPDATE_VERSION` with a client-supplied signature is stored and served verbatim (never re-signed) |
| `SIGN-pgp-verifies` | with the SUT in `pgp` mode (self-hosted tier only — needs key config), the served ASCII-armored RFC 4880 detached signature verifies against the server's public key |

`SIGN-pgp-verifies` runs only where the runner controls SUT config (self-host /
compose with a test key); external-SUT runs report it `SKIPPED(SutConfig)` with
the digest cases still proving the capability.

## 5. CI integration

Two tiers (mirroring how cheap/expensive the modes are):

1. **PR tier (ci.yml, new `conformance-smoke` job)**: self-hosted mode,
   `--profile core --format json`, the functional chapters only (minutes, same
   PG18 service-container pattern as the `test` job). Required check — a PR
   cannot regress CORE.
2. **Full tier (containers.yml, after `smoke`)**: external mode against the
   freshly pushed GHCR images via compose — the full registry, both formats,
   all profiles. Uploads `docs/conformance/` artifacts; on `develop` pushes a
   commit updating `docs/conformance/` is proposed (or artifact-only, owner's
   choice at implementation; recommend artifact + weekly refresh commit to
   avoid badge churn per push).

## 6. Fixture + RM-version policy

- The vendored `_resources/test_data_sets/**` are consumed **read-only in
  place** (path-resolved from the workspace root). Where a payload's wire
  shape is RM-version-sensitive (RM 1.0.x-era fixtures vs our RM 1.2.0 — e.g.
  DV_SCALE availability, `_type` sets, number formatting), the runner uses an
  **overlay directory** `crates/ehrbase-conformance/fixtures/` containing the
  adapted copy plus a `PROVENANCE.md` line per file: source fixture, what
  changed, why (spec citation). The overlay is consulted first; unmodified
  fixtures come from the vendored tree. Never edit `docs/specs/openehr/**`.
- AQL golden results (`expected_results/{empty_db,loaded_db}/{A–D}`): diffed
  through a documented normalizer (RESULT_SET envelope fields that are
  legitimately SUT-specific: generator ids, timestamps; RM-version formatting
  differences), with the normalizer's rules unit-tested — a diff suppressed by
  the normalizer must name its rule.
- **Signature-aware normalization:** our SUT signs versions by default
  (`docs/design/version-signing.md` §3.4), so VERSION payloads carry a
  `signature` the RM-1.0.x-era fixtures lack. The comparison layer treats
  `signature` as a named ignore-set entry when diffing against upstream
  fixtures (rule `SignatureDefaultOn`), while the `SIGN-*` cases assert it
  positively — never both on the same field silently.
- Content-chapter truth tables (master15–17) are transcribed as const tables
  with the schedule file+row cited per entry; they run against the validation
  OPTs from the fixture set.

## 7. What this is not

- **Not a Robot Framework port.** No Python enters the repo. If a future
  certification agency insists on upstream's harness, the external-SUT mode +
  a re-fetched upstream checkout can host that exercise out-of-tree; nothing
  in this design blocks it.
- **Not the EhrScape/FLAT test bed** — that is `openehr-flat`'s suite and the
  P17 work; CNF does not gate it.
- **Not a benchmark** — performance is P20; the runner records durations as
  telemetry only.

## 8. Implementation plan (for the implementer, after access control)

Ordered, compiling+tested increments on `claude/s2-conformance` (or the P19
branch when reached); each step cites its section. The registry grows
chapter-by-chapter — the framework is valuable from step 3 onward, long before
all 322 cases are in.

1. **Crate scaffold + case model + registry + coverage guard** (§4.1, §4.2):
   the guard initially maps every schedule id to `EXCLUDED(NotYetTranscribed)`
   — the honest zero state; the report generator works from day one and shows
   0/322, which is the point: the backlog is now enforced and visible.
2. **SUT client + modes + CLI + `scripts/conformance.sh`** (§4.3, §4.4):
   prove both modes with one hand-picked case (`I_EHR_SERVICE.create_ehr-main`)
   end-to-end incl. RESULTS.md/badge output.
3. **master06 (EHR + EHR_STATUS, 21 cases)** — the CORE heart; then
   **master07 (COMPOSITION, 31)** with both formats.
4. **master04/05 (DEFINITION, 22)** and **master08 (CONTRIBUTION, 31)**.
5. **master09 (DIRECTORY, 37)** — completes the CORE+directory surface.
6. **Query** (§3.4): master11's real cases + the `QUERY-FIXTURE-*` corpus
   with golden-result diffing (§6).
7. **Content chapters (master15–17, 119)** — table-driven; large but
   mechanical against the validation service.
8. **OPTIONS chapters we implement** (master12 admin subset — run under the
   ADMIN credential with a USER-role 403 assertion, master10 demographic) +
   the **`SIGN-*` capability cases** (§4.6); wire the two CI tiers (§5);
   first committed `docs/conformance/` + README badge.
9. **The first STANDARD-profile statement** — with Signing implemented and
   evidenced by §4.6, generate and commit the first
   `CONFORMANCE_STATEMENT.md` claiming the full STANDARD profile (REST,
   JSON + XML, RM 1.2.0; RBAC on, ABAC off; deviations register per §3).

Discipline throughout: failures are findings, not skips (§4.5); fixture
adaptations carry provenance (§6); every case cites schedule + ITS-REST
sections; no test weakening, ever — the phase-19 exit criteria ("CNF schedule
passes with documented exceptions only; deviation register complete") are the
finish line.
