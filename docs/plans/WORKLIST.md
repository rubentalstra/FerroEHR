# Worklist — the single open-items tracker

Owner-requested (2026-07-12): every open work item lives HERE, one row each,
with status + the governing plan/branch. `current-phase.md` points at this
file; close a row by linking the merged PR. Never track work only in chat.

| # | Item | Status | Where |
|---|------|--------|-------|
| W-1 | **H1 — legacy ADR-citation sweep**: ~1,000 mentions (generated headers via the emitters + regen; ~500 hand-written rewritten to spec citations / explicit spec-silence flags / plain prose) | in review | branch `claude/h1-adr-citation-sweep` |
| W-2 | **ECC skip elimination** (owner ruling 2026-07-12: an ECC case may pass, fail, error, or be N/A — NEVER "skipped"). The 26 skips, inventoried: (a) 11 × SM ops with no ITS-REST binding (`list_contributions` ×5, `delete_opt` ×4, `list_queries` ×2) → expose via the existing admin/extension wire and execute, else N/A with citation; (b) 11 × `NativeApiOnly` MSG cases (EHR-extract export/import, TDD) → wire an extract extension API or reclassify N/A pointing at the platform-suite evidence; (c) 4 × `SutConfig` terminology cases → wire the compose SUT to the runner's wiremock TS (host-reachable) + FHIR provider env so they execute; (d) 1 × `sig/pgp-verifies` → a pgp-keyed compose profile. Zero `skipped` outcomes in the final report. | next | this worklist |
| W-3 | **X1 — honest EHRbase vs EHRbase-rs comparison** (owner approved 2026-07-12): upstream ECC run + fairness adjudication register, `tools/benchmark` overhaul (multi-SUT, percentiles, resource footprint), measured comparison page on the docs site. **No false claims — measured numbers only.** | queued (after W-2) | `docs/plans/x1-comparison.md` |
| W-3a | **Architecture Overview study + checklist** (owner directive 2026-07-12, GATES W-4): fully read the vendored BASE Architecture Overview (`docs/specs/openehr/BASE/docs/architecture_overview/`, masters 00–12) and produce a per-chapter/per-subsection checklist (1 → 1.1 → 1.1.1 granularity) of every load-bearing statement, each verified against the codebase (verified / gap / informative). Prompt: `docs/plans/arch-overview-PROMPT.md`. | next session | `docs/spec-audit/architecture-overview/` |
| W-4 | **ADL2 — full implementation, spec-exact** (owner directive 2026-07-12: "following only the official specs, no deviation"): ADL2 source parser (ADL2 syntax + cADL2 + ODIN sections), AOM2 semantic validation (the full master08 catalogue on parsed artefacts), specialisation flattening, OPT2, template semantics (master10) — replacing the registration-surface-only enforcement from A1 ch14. Oracle: `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/`. | queued (after W-3a) | plan to be authored as `docs/plans/adl2.md` |
| W-5 | **P20 — re-plan, then execute** (owner ruling 2026-07-12: the existing P20 plan is stale — re-evaluate from scratch what optimization work is actually warranted now: measure first, then decide; the old AIO/`JSON_TABLE`/pipelining list is input, not the plan). P99 is REMOVED (owner ruling 2026-07-12: no longer exists — the release machinery already shipped with v3.0.0). | queued | rewrite `docs/plans/phase-20-optimization.md` before starting |

## Standing rules picked up along the way (enforced, not tracked)

- CI must be green on develop at all times (audit/deny/machete fixed in PR #71;
  new advisories get dated, documented ignores only when no fix is reachable).
- Spec-only citations in code (never ADRs); scrub on touch.
- No `use X as Y` import renaming; `urlencoding` for all percent codecs.
- Every phase ends with a zero-drift ECC run; the baseline only ratchets up.
