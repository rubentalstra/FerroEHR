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
- [ ] Phase 1 — Extract: 24 chapters × requirements.md (numbered normative
      requirements + citations; rejection duties prioritized)
- [ ] Phase 2 — Verify: per-requirement verdict table (verification.md:
      classification, file:line evidence, severity, negative-test-exists,
      fix sketch)
- [ ] Commit after phases 1+2 (pipelined per chapter)
- [ ] Phase 3 — Skeptic: adversarial refute pass per chapter-with-findings
      (skeptic.md: confirmed / refuted / uncertain + exact runtime probe)
- [ ] Commit after phase 3
- [ ] FINDINGS.md — consolidated register (confirmed defects by severity +
      uncertain findings with probes) + README.md status table
- [ ] **PAUSE for owner review** (per brief: after FINDINGS.md, before fixes)
- [ ] Fix wave: confirmed defects in severity order (implementer agents,
      spec citations in every prompt, regression/negative test per fix;
      derive/emitter fixes get full workspace suite + fidelity gates)
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
