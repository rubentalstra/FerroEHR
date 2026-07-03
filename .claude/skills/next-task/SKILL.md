---
name: next-task
description: >
  Reads the current phase file, picks the first unchecked task, and restates
  it as a concrete work plan naming the files involved and which agent
  should do the work. Use when the user asks "what's next" or "what should
  I work on".
allowed-tools: [Read, Grep, Glob]
argument-hint: (none)
---

# /next-task

Turns the top of the current phase's task list into an actionable plan —
step 2 of the six-step loop (`CLAUDE.md`). Does not do the work itself; that
is a separate step the caller takes after seeing the plan.

## Steps

1. **Read `docs/plans/current-phase.md`** to find the active phase file.
2. **Read that phase file** and find the first unchecked (`- [ ]`) line
   under `## Tasks`. If every task is checked but `## Exit criteria` has
   unchecked lines, surface those instead and suggest `/phase-done` once
   they are all verified.
3. **Turn the task into a plan**, stating:
   - **What** the task requires, in one or two sentences.
   - **Which files** are involved — search for them (Grep/Glob under
     `crates/`) rather than guessing paths; if the task names a Java module
     or a spec component, resolve it against
     `PORT_MASTER_PLAN.md` Section 9.1 (Maven→crate mapping) or Section 7.1
     (RM class inventory).
   - **Which agent / mechanism** should do it: **openEHR spec layer**
     (`openehr-base`/`openehr-rm`/`openehr-am`) → **the code generator**, not an
     agent — change `openehr-codegen`'s emitter and run
     `cargo run -p openehr-codegen -- emit` (ADR-004; the `rm-transcriber` agent
     is retired). Per-file EHRbase Java→Rust ports (`ehrbase-*`) → `porter`.
     Read-only fidelity check afterward → `port-reviewer`. "Run the tests and
     report" → `test-runner`. Harness/docs scaffolding (not code) → inline.
   - **What "done" looks like** for this task specifically, distinct from
     the phase's overall exit criteria.
4. **Do not tick the checkbox or commit** — that happens after the work is
   actually done (steps 4-5 of the loop), not as part of planning it.
