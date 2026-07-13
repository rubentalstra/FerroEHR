# W-10 session prompt — full redesign + rewrite of `tools/conformance`

Execute **W-10: the complete redesign, re-architecture, and rewrite of the
conformance framework** (`tools/conformance`, ~18k lines). Owner ruling
(2026-07-13): the current instrument is **not trusted anymore** — it grew
incrementally (B1–B6, B5 adjudications, W-3f re-baseline) and encodes
pre-rewrite server behaviour in places; it must be rethought from the spec
up, not patched. The goal is **the best possible openEHR CNF testing
framework**, able to certify not just our server but ANY openEHR CDR.

## Fresh evidence (W-3f close, 2026-07-13) — why the rewrite is due

The W-3f endgame caught the instrument being wrong in exactly the feared
way: its ETag handling still expected the deprecated bare form (the
ITS-REST overview made ETags weak-type, `W/"…"`), and because case setups
scrape ids out of response headers with ad-hoc helpers
(`suites/support.rs::version_uid`, `suites/contribution.rs::contribution_uid`),
that single client-side spec drift silently corrupted 22 case setups into
empty-body 404s that looked like server failures. One real server defect
(CNF master08 E.2) was also found — the split was adjudicated honestly, but
the lesson stands: **the runner's HTTP/client layer must itself be
spec-grade and centralized** (header parsing, ETag weak/bare tolerance,
Prefer handling, id extraction in ONE place), so a wire-form change can
never rot dozens of cases invisibly.

## The oracle — read it ALL first (owner mandate)

Before designing anything, read the FULL vendored CNF component at
`docs/specs/openehr/CNF/` so the framework is understood end-to-end:

- `docs/guide/` — the Conformance Guide (methodology: what conformance
  testing IS, SUT/test-client roles, evidence, profiles).
- `docs/platform_test_schedule/` — the Platform Conformance Test Schedule
  masters (the normative test-case catalogue: every `master*-func_tc_*`
  chapter, area by area).
- `docs/profiles/` — the conformance profiles (CORE/STANDARD/…): what a
  claim means, which capabilities each profile requires.
- `docs/certificate/` — Statement + Certificate artefact definitions
  (`master03-certificate.adoc`) — the output the framework must emit.
- `tests/` — the openEHR Robot suite (prior art for case content — we never
  map to it 1:1, but it is evidence of intended coverage).
- `PROVENANCE.md`, `README.adoc`, `manifest.json`, `scripts/` — pinning +
  regeneration.

Also read (project law): the ECC memory rule — **our own numbering/taxonomy,
generated data sets, latest-spec-versions-only, never a Robot/Python/legacy
mapping** (`.claude/memory/ecc-own-conformance-framework.md`); the B5 phase
record (`docs/plans/b5-conformance-instrument.md` — the honesty overhaul:
identity from provenance, adjudication register, machine-computed profile
verdicts); blueprint ch 07 (`docs/blueprint/07-cnf.md`); and the W-3f
platform registers (`docs/design/platform/`) for what the server now is.

## Mission

1. **Spec-first re-derivation of the case catalogue.** Method identical to
   W-3f (proven three times): registers first, in `docs/design/conformance/`
   — enumerate the Platform Test Schedule chapter by chapter (every
   normative test condition, with citation), map the EXISTING cases onto
   that spine (conformant / divergent / missing / instrument-encodes-server-
   behaviour), then rewrite. Every case cites its schedule section; skips
   carry the sanctioned adjudication register; the baseline is re-derived
   honestly, not inherited.
2. **Multi-SUT architecture from day one.** The framework drives ANY
   ITS-REST CDR by URL + capability discovery (OPTIONS), with per-SUT
   adapters ONLY where a target needs auth/boot quirks — never per-SUT case
   forks. Targets to support at close:
   - **ehrbase-rs** (ours; the compose-based boot stays the default);
   - **EHRbase (Java, upstream)** — Dockerised official image;
   - the further CDRs the owner names (owner wrote "ehrbase from Cadasto" —
     **confirm with the owner at session start which product this is**
     (CaboLabs EhrServer? Better CDR? Code24?) and what else "maybe others"
     should include; design the adapter seam so adding one is a config
     entry, not code).
   - Fairness rules from `docs/plans/x1-comparison.md` (the fairness
     adjudication register; measured numbers only, no false claims) — W-10
     ABSORBS X1's ECC half; benchmark overhaul stays X1's.
   - **Spec-edition tolerance (load-bearing for fairness):** our server
     implements the development edition of ITS-REST (weak `W/"…"` ETags,
     lowercase `openehr-version`/`openehr-audit-details` headers, RM 1.2.0
     wire); upstream EHRbase (Java) speaks Release-1.0.3-era behaviour and
     an RM 1.1.0-era wire (`docs/VERSIONS.md` divergence note). Cases must
     therefore separate the NORMATIVE assertion (what every edition
     mandates) from EDITION-SPECIFIC forms, with a per-SUT edition profile
     — otherwise the Java run fails on edition deltas, not defects, and the
     comparison is dishonest. Where an assertion is edition-specific, the
     report must say which edition it tested.
3. **First-class outputs:** per-SUT results.json + report + badges;
   machine-computed profile verdicts (CORE/STANDARD/OPTIONS per profiles/);
   Statement + Certificate artefacts per certificate/master03; an honest
   COMPARISON matrix across SUTs (per capability, with the fairness
   register); CI-runnable against ehrbase-rs on every phase close
   (`scripts/conformance.sh` stays the entry point, re-pointed).
4. **Instrument honesty invariants** (carry from B5, verify in the rewrite):
   spec identity derived from provenance, never hand-asserted; a case that
   contradicts the vendored spec text is adjudicated (spec-cited) — the
   server is never bent to a wrong case; every bound on coverage is logged,
   never silent.
5. **Decide the data-set strategy explicitly.** The ECC law says *generated
   data sets, never a Robot mapping* — yet today's fixtures load straight
   from the vendored Robot corpus (`testdata/fixtures.rs` `CORPUS_ROOT` →
   `CNF/tests/platform/robot/_resources/test_data_sets`). Resolve the
   tension in the register: the Robot corpus may serve as *raw material*
   (it is the schedule's own referenced data), but ownership, generation,
   and the owned-fixture register must be deliberate design, not an
   accident of a path constant.

## Method (the standing loop)

- Register first (`docs/design/conformance/`, mirror
  `docs/design/platform/` — spec skeleton → case map → G-rows → target
  design), fan-out read-only Opus auditors, **max 2 concurrent workers**
  (owner cap, `.claude/memory/max-two-concurrent-workers.md`).
- Then the big-bang rewrite: fresh authoring, never migrate legacy files;
  audited-faithful case logic may carry re-grounded + re-cited.
  Intermediate steps need not compile; ONE fix pass; zero-TODO close.
- Spec citations only (CNF file + §section; schedule case ids); spec-silent
  design flagged; official CLIs; no import renaming; files ≤ ~700 lines.
- Deferred checks last: workspace nextest → clippy → then the full run of
  the NEW framework against ehrbase-rs, then against the Java EHRbase (its
  results are DATA, not a gate). The ancestor baseline to re-derive
  against: **341 executed · 315 passed · 0 failed · 26 adjudicated skips,
  CORE PASS · STANDARD PASS · OPTIONS OBTAINED** (W-3f close, verdicts
  machine-computed) — every delta of the re-derived catalogue is explained
  (new coverage / re-adjudication / real defect), never silently absorbed.
- Two CI jobs bind the crate and must end green (updated, not deleted):
  `cargo nextest run -p conformance` (the runner's own tests) and the
  `cnf coverage guard` job.
- Author `docs/plans/w10-conformance-redesign.md` from this prompt at start;
  tick as you go; changelog + website book (the conformance page) same-PR;
  WORKLIST row W-10 closed with the merged PR.

## Fixed constraints

- The SERVER is not in scope — `app/*` changes only if a case adjudication
  proves a real server defect (separate commit, spec-cited).
- `docs/specs/openehr/CNF/` is vendored + pinned; re-vendor only via
  `scripts/vendor-spec-docs.sh` with provenance.
- Keep the runner pure Rust (`tools/conformance`), reqwest-driven,
  Docker-composed SUTs; no Robot/Python/ANTLR, ever.
- The owned-fixture register + generated data sets discipline stands.

## Exit criteria

- [ ] `docs/design/conformance/` registers complete (schedule fully
      enumerated, every existing case mapped, G-rows cited).
- [ ] Framework rewritten: multi-SUT core, adapter seam, profile verdicts,
      Statement/Certificate + comparison outputs.
- [ ] Full run vs ehrbase-rs: honest re-derived baseline committed
      (results + report + badges), zero unexplained regressions vs the
      341/315/0 ancestor (each delta adjudicated or fixed).
- [ ] Full run vs upstream EHRbase (Java) recorded with the fairness
      register (absorbing X1's ECC half).
- [ ] The named third-party CDR(s) confirmed with the owner and either
      integrated or explicitly deferred with the reason.
- [ ] Zero actionable TODOs; workspace green; changelog + book + WORKLIST
      updated; PR merged.
