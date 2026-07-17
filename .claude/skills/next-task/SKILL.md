---
name: next-task
description: >
  Reads the worklist, picks the top open row (or the row the user names),
  and restates it as a concrete in-session work plan naming the files and
  crates involved. Use when the user asks "what's next" or "what should I
  work on".
allowed-tools: [Read, Grep, Glob]
argument-hint: (none)
---

# /next-task

Turns the top open worklist row into an actionable plan — the planning step
of the worklist workflow (`CLAUDE.md`). Does not do the work itself; that
is a separate step the caller takes after seeing the plan.

## Steps

1. **Read `docs/plans/WORKLIST.md`** and take the first row of the `## Open`
   table (or the row the user named). If the row points at an open plan
   file, read that file too; its unchecked (`- [ ]`) tasks are the queue.
3. **Turn the task into a plan**, stating:
   - **What** the task requires, in one or two sentences.
   - **Which files** are involved — search for them (Grep/Glob under
     `crates/` and `app/`) rather than guessing paths; if the task names a
     spec component, resolve it against `docs/architecture.md` (the crate map)
     and `docs/specs/openehr/README.md`.
   - **Which mechanism** applies:
     **openEHR spec/ITS layer** (`openehr-base`/`openehr-rm`/`openehr-am`/
     `openehr-its`) → **the code generator** — change `openehr-codegen`'s
     emitter and regenerate (`/regen-codegen`).
     **Application** (`ehrbase`/`ehrbase-rest`) → idiomatic
     modern Rust of our own design on the generated crates, the openEHR
     specs as the authority (EHRbase = prior art only). Build
     compiling + tested. Note whether the task suits fanning out to an
     `implementer`/`ui-implementer` subagent per the CLAUDE.md
     Model-orchestration section (max 2 concurrent), or belongs in-session
     (architecture, the AQL IR/codec core, spec-conformance judgement).
   - **Which spec sections govern it** — for any spec-facing task, name the
     `docs/specs/openehr/...` files (and CNF test-schedule chapters) the
     implementation must be read against, per `spec-adherence.md` /
     `/spec-lookup`. Doing the work starts by reading those.
   - **What "done" looks like** for this task specifically, distinct from
     the phase's overall exit criteria — including which ECC cases, fidelity
     gates, or corpus tests prove it.
4. **Do not update the worklist row or commit** — that happens after the
   work is actually done, not as part of planning it.
