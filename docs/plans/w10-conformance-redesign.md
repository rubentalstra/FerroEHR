# W-10 — Conformance framework redesign + rewrite (`tools/conformance`)

- Status: in-progress
- Started: 2026-07-13   Owner: Ruben (session: orchestrator)
- Source prompt: `docs/plans/w10-conformance-redesign-PROMPT.md` (owner ruling
  2026-07-13: the incrementally-grown instrument is not trusted; rethink from
  the spec up). W-10 absorbs X1's ECC half (upstream EHRbase through the
  suite, fairness register); the benchmark overhaul stays X1.
- Compile required: yes (normal workspace code; intermediate rewrite steps
  need not compile, ONE fix pass, zero-TODO close)

## Owner rulings captured at session start (2026-07-13)

1. **Third-party SUT:** none by name — the "Cadasto" target is dropped.
   Instead the multi-SUT logic must be **bring-your-own-endpoint**: anyone
   points the runner at their CDR by URL (+ auth) and gets the full
   spec-cited report. Per-SUT adapters exist only for boot/auth quirks of
   the two first-class targets (ehrbase-rs compose, upstream EHRbase Java
   Docker); adding a target is a config entry, never code.
2. **Multi-version spec tolerance (owner, mid-session):** support multiple
   versions of RM/AM/ITS editions per assertion — **start at the highest
   pinned version and step down** when the SUT's wire doesn't match, so one
   run discovers which spec version a CDR actually speaks. Honesty
   constraint: a lower-version match is recorded as an *edition finding*
   (report says which version level passed), never a silent pass; a real
   failure is only when no supported version form matches the normative
   assertion.

3. **Certificate for every SUT (owner, mid-session):** the framework aims
   to be the industry-standard CNF validator — the Conformance Certificate
   artefact is emitted for ANY SUT (ours, upstream, BYO endpoint), always
   self-identifying as a framework assessment (never official openEHR
   certification) with the claim machine-computed from the attached run.

4. **Spine-first suite authoring (owner, mid-session):** every expected
   status/header/body condition traces to the CNF schedule or the vendored
   ITS-REST text — never to observed ehrbase-rs behaviour (our code could be
   wrong; the CNF exists to prove it). Legacy suites are consulted only for
   request mechanics, never expected outcomes; a case our server fails is a
   correct instrument outcome, adjudicated as a defect — assertions are
   never weakened to pass.

## Mission (from the prompt)

1. Spec-first re-derivation of the case catalogue — registers first in
   `docs/design/conformance/` (method = W-3f: spec skeleton → case map →
   G-rows → target design), every case citing its Platform Test Schedule
   section; skips through the adjudication register; baseline re-derived,
   not inherited.
2. Multi-SUT architecture from day one (ehrbase-rs default compose; upstream
   EHRbase Java Dockerised; BYO-endpoint mode; per-SUT edition/version
   profile + the version ladder above; X1 fairness rules absorbed).
3. First-class outputs: per-SUT results.json + report + badges;
   machine-computed profile verdicts (CORE/STANDARD/OPTIONS);
   Statement + Certificate per certificate/master03 (self-assessment — never
   emitted for an upstream/foreign run, X1 rule 4); honest cross-SUT
   COMPARISON matrix; CI-runnable via `scripts/conformance.sh` (re-pointed).
4. Instrument honesty invariants (B5): identity from provenance; a case that
   contradicts the vendored spec is adjudicated with citation — the server is
   never bent to a wrong case; every coverage bound logged.
5. Explicit data-set strategy: resolve the Robot-corpus-as-fixture tension in
   the register (raw material allowed; ownership/generation deliberate).

## Constraints (fixed)

- Server (`app/*`) out of scope except adjudicated real defects (separate,
  spec-cited commits).
- Pure Rust runner, reqwest, Docker-composed SUTs; no Robot/Python/ANTLR.
- ECC law: own numbering/taxonomy (`ECC-<AREA>-<NNN>`), generated data sets,
  latest-versions-first, never a Robot/legacy mapping as machinery.
- Owned-fixture register + generated data sets discipline stands.
- Max 2 concurrent workers; spec citations only (CNF file + §); files ≤ ~700
  lines; official CLIs; no import renaming.
- Ancestor baseline to explain every delta against: **341 executed · 315
  passed · 0 failed · 26 adjudicated skips; CORE PASS · STANDARD PASS ·
  OPTIONS OBTAINED** (W-3f close).
- CI jobs bind the crate, updated not deleted: `cargo nextest run -p
  conformance` + the `cnf coverage guard` job.

## Tasks

### A — Setup + oracle
- [x] A1. Plan file authored from the prompt; branch
      `claude/w10-conformance-redesign`; owner SUT-list + version-ladder
      rulings recorded above.
- [x] A2. Orchestrator reads the CNF methodology end-to-end: `docs/guide/`
      (master03/04/05), `docs/profiles/master03`, `docs/certificate/
      master03`, `README.adoc`, `PROVENANCE.md`, `manifest.json`.
- [x] A3. Schedule spine: every `platform_test_schedule/master*-*_tc_*.adoc`
      chapter enumerated (normative test condition + citation) into
      `docs/design/conformance/` registers (read-only Opus auditors, ≤2
      concurrent).

### B — Registers (`docs/design/conformance/`)
- [x] B1. Case map: every existing `tools/conformance` case mapped onto the
      spine — conformant / divergent / missing /
      instrument-encodes-server-behaviour — with file:line evidence.
- [x] B2. G-rows: gaps + rulings per area (incl. the W-3f ETag lesson: any
      client-side wire parsing centralized).
- [x] B3. Data-set strategy register: Robot corpus as raw material vs owned
      generated fixtures; owned-fixture register design.
- [x] B4. Target design register (orchestrator): crate layout, multi-SUT
      core + adapter seam + BYO-endpoint config, edition/version ladder
      model, spec-grade client layer (single header/ETag/id-extraction
      surface), outputs, CI bindings, comparison matrix, fairness register
      schema (X1 absorption).

### C — Rewrite
- [x] C1. Fresh authoring of the new framework per B4 (workers ≤2, disjoint
      file ownership; orchestrator owns lib.rs/engine seams + fix pass).
- [x] C2. ONE fix pass → `cargo nextest run -p conformance` green, clippy
      clean, coverage guard green; zero TODOs.

### D — Runs + baseline
- [ ] D1. Full run vs ehrbase-rs (compose): re-derived baseline committed
      (results.json + report + badges + Statement/Certificate); every delta
      vs the 341/315/0/26 ancestor explained (new coverage /
      re-adjudication / real defect).
- [ ] D2. Full run vs upstream EHRbase (Java, official image): recorded as
      DATA with the fairness adjudication register; comparison matrix
      emitted; no Certificate for the foreign run.
- [ ] D3. Any adjudicated real server defects fixed in separate spec-cited
      commits (only if found).

### E — Close
- [ ] E1. Workspace gates: `cargo nextest run --workspace`, clippy, fmt.
- [ ] E2. CI: both conformance jobs updated + green;
      `scripts/conformance.sh` re-pointed as the entry point.
- [ ] E3. Docs: changelog entry; website book conformance page updated
      same-PR; blueprint ch07 + §2 refreshed; WORKLIST W-10 row closed.
- [ ] E4. PR opened + merged; PROGRESS.md updated.

## HANDOFF — remaining work (written 2026-07-13, all prior work committed + pushed)

State: framework rewritten and green (crate 0 clippy warnings, 86/86 own
tests, workspace 1459/1459); first D1 run done (368/313/20/35); all 20
failures adjudicated against the full vendored specs — 9 instrument-side
fixed (ladder inversion, cross-format fixture compare, regressed fixture
pointer, store-time overreach), 6 server defects fixed (contribution status
mapping ×2, concrete VERSIONED_* wire types, demographic full-OVID If-Match,
relationship-delete If-Match, demographic weak ETags), rest shared root
causes. Affected suites re-tested 152/152. Remaining, in order:

- [ ] H1. **D1 rerun**: `bash scripts/conformance.sh` (composes + builds our
      server). Expect ~0 failures; every remaining failure gets the same
      triage (fact-check vs docs/specs/openehr — case OR server, never
      assume). Commit `docs/conformance/ehrbase-rs/` (results.json + report
      + Statement + Certificate + badges) as the re-derived baseline; write
      the delta explanation vs the 341/315/0/26 ancestor (new coverage: +27
      executions incl. relationship family, dialect cases, has_composition;
      re-adjudications; fixed defects) into this file + blueprint ch07.
      NOTE: old baseline path was docs/conformance/*.json — update anything
      referencing docs/conformance/badge.json (README badges?) to the
      per-SUT path, and prune/redirect the stale root-level artefacts.
- [ ] H2. **D2 upstream run**: `CONF_SUT=ehrbase-java bash
      scripts/conformance.sh` (images pre-pulled: ehrbase/ehrbase:2.34.0 +
      ehrbase-v2-postgres:16.2; set EHRBASE_JAVA_IMAGE=ehrbase/ehrbase:2.34.0
      to match the fairness register). Results are DATA, never a gate.
      Triage upstream failures into adjudications/ehrbase-java-2.34.toml
      (X1 fairness: extension→N/A, rm-version-sensitive→N/A, defect→stays,
      each cited). Then `conformance compare --from
      docs/conformance/ehrbase-rs/results.json --from
      docs/conformance/ehrbase-java/results.json` → commit COMPARISON.md.
- [ ] H3. **Workspace gates**: cargo nextest run --workspace (fast now —
      Gatekeeper Developer-Tools exemption enabled 2026-07-13), clippy
      --workspace --all-targets, fmt, cargo audit/deny if touched deps.
- [ ] H4. **Docs**: website/book/src/conformance.md rewrite (multi-SUT +
      BYO endpoint + Certificate-for-any-SUT + edition ladder + new
      baseline numbers — user-facing voice); blueprint 00 §2 + ch07 refresh
      (new instrument, new baseline); register 13 prose fix (27→28
      interval ids — worker-flagged off-by-one); PROGRESS.md entry.
- [ ] H5. **Close**: WORKLIST W-10 row → closed with the PR; changelog
      entry exists (verify it still matches reality); tick D1/D2/D3/E1-E4
      boxes above; PR from claude/w10-conformance-redesign → develop
      (NO AI attribution anywhere), merge after CI green.
- [ ] H6. (If D1 rerun still shows VAL-107/temporal failures: the register
      13 temporal family G-3 flagged the validator's temporal-range
      enforcement as an open finding — triage against AOM 1.4 §C_DATE_TIME
      before touching anything; a real gap becomes a spec-cited server fix,
      possibly deferred to a named WORKLIST row with the owner's consent.)

## Decisions made this phase

- (recorded as they land; edition-ladder + BYO-endpoint rulings above)
