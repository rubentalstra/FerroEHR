---
name: owner-work-style
description: "Durable owner rulings on HOW to work: defer nothing, no quick fixes (proper rewrites welcome), orchestrator codes context-heavy work itself, rerun the CNF pipeline after runner/validation merges"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 58d2e09d-1858-4a52-a5b7-e494a0472505
---

Standing owner rulings on how work is done (2026-07-11 onward), binding in
every phase:

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
- **Rerun the CNF pipeline (`scripts/conformance.sh`) after merging anything
  that touches the runner or validation** — the committed baseline is only
  trustworthy immediately after a rerun (a skipped rerun once left main
  claiming 92 phantom failures).
- **Never copy a number forward.** Any figure quoted in a doc, issue, PR, or
  status update is re-derived from the committed artifacts at the moment of
  writing — "stalled information is very very bad" (owner 2026-07-20).
- **"Rewrite" means FRESH FILES FROM THE SPEC, not in-place refactor
  (owner, 2026-07-16, service-rewrite escalation).** When the owner orders a
  "nuking complete rewrite" of a subsystem: design it anew from the governing
  spec (module tree, call inventory, types), WRITE NEW FILES, port the
  behaviour in, DELETE the old files. Parameter-threading, renames and
  incremental edits to the existing shape do not qualify and will be
  rejected. No cargo/clippy runs between steps — they are the "wild detour";
  the compiler is consulted once, at the single convergence.
- **Big-bang rewrites, converge ONCE at the end (owner, 2026-07-16, W-14
  ruling).** For structural
  rewrites: land ALL code moves first, then one compile/test convergence
  pass. NEVER stabilize intermediate steps — "then we create stubs to sort
  of make it work"; compatibility shims between steps are banned. The
  orchestrator (Fable) executes such rewrites in-session, not via workers.
  Re-confirmed 2026-07-17 (Simplified Formats rewrite): this includes NOT
  running nextest/clippy on freshly-authored foundation modules mid-rewrite
  and NOT fixing lint/test fallout per-file — author ALL the code first,
  then resolve everything in the single convergence pass.
- **No ADRs, no kept design docs (owner, 2026-07-17 — "causing more
  confusion than good"); plan/design markdowns are
  deleted in the PR that implements them.** The only citable references are
  docs/specs/openehr/ and OFFICIAL external docs (PostgreSQL, Rust,
  docs.rs) — never internal markdown (it moves or dies). Decisions live in
  root CLAUDE.md, docs/architecture.md, and the code.
- **Specs over decision docs, always re-verify (owner, 2026-07-16).** When
  an analysis leans on a doc's characterization of a spec, re-read the
  vendored spec text first-hand — a doc-flavoured claim ("C contradicts
  SM") was retracted after reading SM master02 directly (packaging is
  implementer-free; conformance = tested call semantics).

**How to apply:** treat these as standing defaults in every phase.
Related: [[autonomous-phase-flow]], [[merge-on-local-gates]].
