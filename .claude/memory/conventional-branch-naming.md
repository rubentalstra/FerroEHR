---
name: conventional-branch-naming
description: Owner hard rule 2026-07-19 — branches are <type>/<kebab-slug> (feat/fix/chore/docs/refactor/perf/test/ci/build/release); claude/* is retired
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9491391b-2993-4fd4-a58b-26e1d0724da8
  modified: 2026-07-19T12:16:32.368Z
---

Owner hard rule (2026-07-19, codified in root CLAUDE.md + hooks): branch
names use the industry-standard conventional types —
`<type>/<kebab-case-slug>`, type ∈ feat | fix | chore | docs | refactor |
perf | test | ci | build | release (mirrors the Conventional Commits type
set). The former `claude/*` scheme is RETIRED — never create a new
`claude/*` branch; existing ones remain as historical facts. A tracker
issue's phase branch is normally `feat/<issue-slug>` (see
[[tracker-is-github-issues]]).

**Why:** match industry convention, not an AI-specific namespace.

**How to apply:** pick the type by the dominant change; the
`block_dangerous` hook's force-push allowlist now keys on these prefixes.
See [[autonomous-phase-flow]] for the merge-before-branch ordering.
