# Phase A1 — Full spec audit

- Status: in-progress
- Started: 2026-07-11   Owner: Ruben
- Brief: `docs/plans/a1-spec-audit-PROMPT.md` (executed verbatim)
- Branch: `claude/a1-spec-audit` (cut from develop @ 717585c85, which includes
  the merged PARTY_SELF fix PR #69 — upstream-diff findings B1/B2)
- Compile required: audit phases are read-only on code; the fix wave is normal
  workspace code (nextest + clippy green, ECC must only improve from 341/315/0)

## Objective

Chapter-by-chapter audit of the ENTIRE vendored spec surface
(`docs/specs/openehr/`) against the actual Rust code, hunting three defect
classes: **more lenient** than the spec (accepts what must be rejected),
**less capable** (mandated behaviour missing), **wrong** (different
semantics). Durable logging under `docs/spec-audit/` — every agent's findings
persisted the moment they exist; commits after each phase.

## Tasks

- [x] Setup: branch from up-to-date develop (post PR #69), phase file,
      `docs/spec-audit/` skeleton, PARTY_SELF background read
      (`docs/conformance/upstream-ehrbase/TRIAGE.md` — B1/B2/R1/R2 already
      fixed + merged, do not duplicate)
- [x] Phase 1 — Extract: 24 chapters × requirements.md (numbered normative
      requirements + citations; rejection duties prioritized) — done
      2026-07-11: 1,126 requirements, 418 high-risk, committed per batch
- [ ] Verify+fix, per chapter (owner ruling 2026-07-11, supersedes the
      brief's separate verify/skeptic/fix phases): **ONE active agent at a
      time**, working through the chapters in order. Per chapter: verify
      EVERY requirement in requirements.md (all of them, not only
      high-risk), fix every confirmed defect/missing/partial in the same
      pass (idiomatic Rust, spec citation + regression/negative test per
      fix), write verification.md (verdict + fix status per requirement).
      The orchestrator reviews the diff, commits, ticks the chapter row,
      then launches the next chapter's agent. Architectural/ADR-needing
      fixes are recorded as deferred-with-reason, not improvised.
      - [x] 1 rm-common-change-control (54ae8384a — branching+merge,
        all deferrals closed, verification.md final) · [x] 2 rm-ehr ·
        [x] 3 rm-composition · [x] 4 rm-data-structures ·
        [x] 5 rm-data-types-text-quantity · [x] 6 rm-data-types-rest ·
        [x] 7 rm-support · [x] 8 rm-demographic · [x] 9 rm-ehr-extract ·
        [x] 10 rm-integration · [x] 11 base-foundation ·
        [x] 12 base-base-types · [x] 13 am-aom14-opt ·
        [x] 14 am-aom2-adl2 · [x] 15 term · [x] 16 query-aql ·
        [x] 17 sm-platform · [x] 18 sm-tdd · [x] 19 its-rest-general ·
        [x] 20 its-rest-ehr-composition ·
        [x] 21 its-rest-query-definition-admin · [x] 22 its-json ·
        [x] 23 its-xml · [ ] 24 cnf-cross-check
- [ ] FINDINGS.md — consolidated register maintained as chapters close
      (confirmed defects + fix status + uncertain findings with probes)
- [ ] Uncertain findings: write + run the named runtime probes
      (testcontainers PG18), reclassify
- [ ] Gates: `cargo nextest run --workspace` green; clippy clean; full ECC
      (`scripts/conformance.sh`) — pass/fail must only IMPROVE from 341/315/0
- [ ] PRs per logical chunk to develop, merged; `docs/PROGRESS.md` +
      `CHANGELOG.md` [Unreleased] updated (stricter validation is
      user-visible)

## Chapter map

See `docs/spec-audit/README.md` (the 24-chapter status table) and the brief's
chapter map for spec-path ↔ code-map assignments.

## Decisions made this phase

- (record ADR-worthy decisions here as the fix wave surfaces them)
