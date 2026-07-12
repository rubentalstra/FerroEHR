---
name: a1-audit-cadence
description: "A1 spec audit execution cadence (owner rulings 2026-07-11): extract-only workflow done; then ONE agent at a time, per chapter file, fix EVERYTHING in the file"
metadata: 
  node_type: memory
  type: project
  originSessionId: 638fb34b-f9d0-4f7b-9da8-b5fa3ba9a9e9
---

A1 full spec audit (branch `claude/a1-spec-audit`, phase file
`docs/plans/a1-full-spec-audit.md`) — owner rulings 2026-07-11 that
supersede parts of the brief (`docs/plans/a1-spec-audit-PROMPT.md`):

1. **No monolithic 3-phase workflow** ("I will run out of tokens and is
   also wasteful") — only the 24 extract agents ran as a workflow
   (done: 1,126 requirements, 418 high-risk, in
   `docs/spec-audit/<chapter>/requirements.md`).
2. **Verify+fix cadence:** ONE active agent at a time, chapter by chapter
   in README order; each agent verifies and FIXES **everything** in that
   chapter's requirements.md (all requirements, not only high-risk), with
   regression/negative test per fix, then writes verification.md; the
   orchestrator reviews the diff, commits, ticks the row, then starts the
   next chapter. No parallel fix agents.
2b. **DEFER NOTHING (owner ruling 2026-07-11, angry — "we will never
   solve the issue of non compliance if we keep deferring things"):** a
   chapter is not done while any requirement is classified deferred.
   Architectural items (version branching, terminology-group checks,
   invariant helpers) get DESIGNED AND IMPLEMENTED in the pass, spec-cited;
   the orchestrator records an ADR afterwards if the design is
   ADR-worthy. "Deferred: needs owner" is banned from verification.md.
3. The brief's "pause after FINDINGS.md before fix wave" is overridden by
   ruling 2 (fixes proceed per chapter); FINDINGS.md is maintained
   incrementally as chapters close.

2c. **Fable codes directly (owner ruling 2026-07-11):** no delegation to
   Opus subagents for the A1 fixes — "opus needs every time all the
   context again and it's not doing anything"; the main session (Fable)
   writes the code itself. Greenfield discipline: MAJOR rewrites are
   expected and welcome — "no quick fixes, always proper".

**Why:** token discipline + reviewability (one diff at a time).
**How to apply:** never fan out parallel verify/fix agents for A1; keep
the sequential per-chapter loop until chapter 24 closes, then the ECC gate
(must only improve from 341/315/0). Related: [[autonomous-phase-flow]],
[[spec-adherence-mandate]].

2d. **No `use X as Y` aliasing (owner, 2026-07-11 — also encoded in
   CLAUDE.md + .claude/rules/rust-style.md):** always import types under
   their direct names; an alias is a quick fix hiding a naming problem —
   "if the name is not good, change the NAME, don't `as` around it".
   A genuine collision gets a qualified path at the use site. Alias only
   in highly exceptional cases with no other solution (trait `as _`
   imports are fine). Scrub aliases from files you touch.
