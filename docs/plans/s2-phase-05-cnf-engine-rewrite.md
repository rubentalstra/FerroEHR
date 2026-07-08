# Phase S2-05 — The ehrbase-rs Conformance Catalogue (ECC): full clean rewrite

- Status: in-progress (engine core v4 landed)
- Started: 2026-07-08   Owner: —
- Branch: `claude/cnf-hardening` (owner instruction: stay on this branch)
- Design: `docs/design/conformance-framework.md` (**v4** — read first)
- Compile required: yes — compiling, clippy-clean, tested increments.

## Mission (owner directives, 2026-07-08)

Build **our own, modern conformance framework** — the ECC — as the project's
acceptance instrument. The official openEHR CNF corpus is *reference reading
only* (frozen upstream, unfinished chapters, 2019 Robot/Python harness): we
studied it exhaustively, keep its good ideas (profile claims, data-set-driven
validation, certificate-shaped reports), and build better from the current
pinned specs. Our numbering (`ECC-<AREA>-<NNN>[.VV]`), our taxonomy, our
generated data sets, version-aware (latest-only today), ≥2,000 executable
tests at build-out, enterprise-clean layering. **No mapping machinery to the
legacy corpus, no Robot, no Python.**

## Tasks

- [x] Recon: exhaustive inventories of the old corpus (schedule 324 headings /
      1,371 truth rows; robot 464 cases; fixtures; upstream freshness) —
      2026-07-08. Retained as design-review knowledge.
- [x] Vendor the ISO 18308 Conformance Statement
      (`docs/specs/openehr/REQUIREMENTS/`) as the requirements-level lens.
- [x] Design v4 (`docs/design/conformance-framework.md`): own framework,
      own catalogue, spec-first universe, generated data sets, version
      dimension, machine profile verdicts.
- [x] **Engine core v4** — layered crate (`model/` `testdata/` `engine/`
      `reporting/` `suites/` + facade), ECC catalogue (committed TSV,
      allocation guard, 310 cases numbered), `SpecVersions` (latest-only),
      catalogue-driven runner + reports (RESULTS/CATALOG/STATEMENT/badge);
      all legacy-corpus mapping machinery deleted. 29/29 tests, clippy-clean.
- [x] Re-title + re-key the existing ~310 cases as native ECC cases —
      done (c885d2be0): CaseMeta v4 (`id` slug + `title` + explicit `area` +
      `citation`; Chapter/Provenance/schedule_ref/upstream_tags deleted),
      all 310 TSV lines re-titled in place with ECC numbers byte-identical;
      32/32, clippy-clean. Design-review checklist:
      `docs/design/ecc-coverage-review.md`.
- [x] `engine/flow.rs` — the declarative step API (numbered, named steps;
      step-stamped assertion failures). EHR-area migration folded into the
      re-key pass below.
- [x] `model/profile.rs` — capability→profile matrix (design §8) +
      all-or-nothing machine verdict; statement §4 renders per-capability
      verdict tables for CORE/STANDARD/OPTIONS. 32/32 tests, clippy-clean.
- [ ] `testdata/generate.rs` — the VAL generators (cardinality grids,
      presence/absence, boundary values, type substitution over authored
      OPTs): 1,000+ variants with per-variant `ECC-VAL-nnn.vv` outcomes.
- [ ] `REST` area — the ITS-REST operation × documented-status matrix from
      the pinned contract.
- [ ] `QRY` build-out — AQL 1.1 construct checklist + corpus goldens with a
      rule-named normalizer.
- [x] Badge set rewrite (owner): four generated badges — total
      (`badge.json`) + per-profile `badge-core/standard/options.json`,
      profile badges driven by the machine verdict (PASS only when the
      all-or-nothing verdict holds); artifact-set test extended.
- [ ] `SEC` sweeps (RBAC 401/403), JUnit/CTRF output, CI tiers, first
      generated STANDARD-profile statement.
- [x] Regenerate `docs/conformance/` with the v4 artifact set — first ECC
      run 2026-07-08 (self-host, JSON+XML): 318 executed · 211 passed ·
      106 failed (the s2-phase-04 hardening backlog, VAL-dominated);
      consolidated CONFORMANCE_REPORT.md + CATALOG.md + 4 badges; stale v2
      RESULTS.md/CONFORMANCE_STATEMENT.md removed; README carries the four
      ECC badges.

## Exit criteria

- [ ] Catalogue ≥2,000 executable tests across the areas (design §4.3),
      every case with a human title and spec citation.
- [ ] Profile verdict machine-computed; statement generated; failures are
      findings; zero legacy-corpus machinery.
- [ ] Full run against the compose stack produces the v4 `docs/conformance/`
      set; CI tiers wired.

## Decisions made this phase

- v4 ownership: our framework, legacy CNF demoted to reference reading
  (design doc v4 §1–§3).
- Version dimension modeled, latest-only supported (design §2.5).

## Handoff for next session

Engine core v4 is on the branch: layered crate, ECC catalogue live
(310 numbered cases), catalogue-driven runner/reports, clippy-clean, 29/29.
Next: the re-title/re-key pass (task 5) and `engine/flow.rs` — both are
delegable area-by-area with the design doc §5/§7 as the spec.
