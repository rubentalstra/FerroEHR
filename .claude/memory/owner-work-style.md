---
name: owner-work-style
description: "Durable owner rulings on HOW to work: defer nothing, no quick fixes (proper rewrites welcome), orchestrator codes context-heavy work itself, rerun ECC after runner/validation merges"
metadata:
  type: feedback
---

Durable owner rulings distilled from the A1 audit era (2026-07-11/12), still
binding after A1 closed:

- **DEFER NOTHING.** "We will never solve the issue of non-compliance if we
  keep deferring things." A task is not done while any part is classified
  deferred; architectural items get designed and implemented in the pass,
  spec-cited. "Deferred: needs owner" is banned language.
- **No quick fixes — always proper.** Major rewrites are expected and
  welcome in this greenfield codebase; never patch around a design problem.
- **Context-heavy work: code it directly.** Delegating to Opus subagents
  failed when each agent needed the full context re-fed ("opus needs every
  time all the context again and it's not doing anything"). Delegate only
  bounded tasks with a tight, self-contained spec; the orchestrator writes
  the hard, context-rich code itself.
- **Rerun ECC after merging anything that touches the runner or
  validation.** PR #69 left develop with a stale ECC baseline (92 phantom
  failures); the baseline is only trustworthy immediately after a rerun.

**How to apply:** treat these as standing defaults in every phase, not
A1-specific. Related: [[autonomous-phase-flow]],
[[ecc-own-conformance-framework]].
