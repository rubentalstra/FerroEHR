---
name: phase-status
description: >
  Prints the current phase pointer, the current phase file's unchecked
  tasks, and a short git status. Use when the user asks "where are we",
  "what phase are we on", or at the start of a work session to orient.
allowed-tools: [Read, Bash]
argument-hint: (none)
---

# /phase-status

A fast orientation dump — step 1 of the six-step loop
(`CLAUDE.md` "Phase workflow"). Read-only; makes no changes.

## Steps

1. **Read `docs/plans/current-phase.md`** and print its three lines
   (phase file path, session goal, next action) verbatim.
2. **Read the referenced phase file** (e.g.
   `docs/plans/phase-00-scaffolding.md`) and list:
   - Its `Status` and `Compile required` header fields.
   - Every unchecked (`- [ ]`) line under `## Tasks`, in order.
   - Every unchecked line under `## Exit criteria`, so it is clear whether
     the phase is close to done.
3. **Run `git status --short`** and `git log --oneline -5` to show
   uncommitted work and recent history at a glance.
4. **Do not** modify `current-phase.md`, the phase file, or make any commit
   — this is a read-only status check. If the user wants the next task
   turned into a work plan, point them at `/next-task` instead.
