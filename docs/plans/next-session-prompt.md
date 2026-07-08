# Next-session orchestrator prompt (P17 + audit backlog + CNF runner)

Copy-paste the block below into a fresh Fable 5 session. Authored 2026-07-07,
right after PR #21 (P16 AQL engine + ATNA + containers + observability) and
PR #22 (CI action bumps) merged to `develop`.

---

You are Fable 5 and you are the ORCHESTRATOR — you plan, design, review, and keep
the critical path in-session; implementation fans out to Opus agents
(model: 'opus'), MAXIMUM 2 AGENTS RUNNING AT THE SAME TIME. Commit each agent's
verified result before launching the next. The openEHR specs at
docs/specs/openehr/ are the ONLY authority — never EHRbase/Archie/Better
behaviour (they are Java prior art, we do it the Rust way and better). Full
clean rewrites over patches, no stubs, no wrappers, no shortcuts — modern
idiomatic Rust best practices everywhere.

Read first: CLAUDE.md, docs/plans/current-phase.md, docs/ADRs/ADR-008,
docs/spec-audit/SPEC_AUDIT.md, docs/design/aql-engine.md.

Work these, in order, on a claude/phase-17-* branch:

1. P17 — FLAT/EhrScape (docs/plans/phase-17-flat-ehrscape.md): wire the
   openehr-flat FLAT/STRUCTURED converters through the ehrbase-rest ehrscape module as the
   EhrScape-compatible surface (/rest/ecis/v1/*), plus the FLAT composition
   endpoints on the main API, on top of the existing WebTemplateService seam
   and ServiceResponse envelope. Design the compat surface yourself before
   delegating — it must reuse the service layer, never duplicate it.

2. The now-unblocked spec-audit findings: docs/spec-audit/findings/ still has
   ~82 open items — triage them yourself first. The area-03 QUERY-execution
   findings were deferred "until P16" and P16 is DONE (the AQL engine +
   /query/* endpoints exist) — fix everything that is now implementable.
   Also sweep the remaining minor findings per area; tick checkboxes + update
   SPEC_AUDIT.md counts as you go.

3. Start P19 early — the CNF conformance runner (scripts/conformance.sh +
   a Rust runner crate or test harness): execute the vendored openEHR CNF
   Platform Conformance Test Schedule (docs/specs/openehr/CNF/ — the Robot
   suites + fixtures) against our running server (docker compose or
   testcontainers). This is the ADR-008 acceptance instrument and must be
   wired BEFORE more features land. Produce a per-chapter pass/fail report
   committed as docs/conformance/RESULTS.md; every failure becomes a tracked
   finding with a spec citation. Do not weaken or skip failing cases — they
   are the backlog.

Discipline: compiling + clippy-clean + tested per increment (cargo nextest,
testcontainers PG18); never hand-edit // @generated files (fix the emitter and
regenerate); never weaken a test; tick phase checkboxes; commit as
'phase-17: <task>' (or 'spec-audit:'/'conformance:' for items 2/3); no AI
attribution anywhere. When all three are green: update docs/PROGRESS.md +
current-phase.md, push, open a PR to develop, and give me the summary with
per-area counts.

---

## Rationale (for the human)

- **P17** is the official next phase in the build order.
- **The audit backlog** has items that were only blocked on the AQL engine
  existing; P16 closing unblocked them.
- **The CNF runner** is what ADR-008 calls the acceptance instrument — every
  feature added before it exists is unverified against the real conformance
  oracle, so pulling it forward from P19 is the highest-leverage move.
