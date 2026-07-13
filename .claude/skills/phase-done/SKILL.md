---
name: phase-done
description: >
  Verifies every exit-criteria checkbox in the current phase file is ticked,
  updates docs/PROGRESS.md, writes the phase's Handoff section, and advances
  docs/plans/current-phase.md to the next phase file. Use when the user says
  a phase is complete or asks to close out / wrap up the current phase.
allowed-tools: [Read, Edit, Grep, Glob]
argument-hint: (none)
---

# /phase-done

Step 6 of the six-step loop (`CLAUDE.md`). Only run this once the phase's
work is actually finished — this skill verifies and records, it does not
decide the phase is done on your behalf.

## Steps

1. **Read `docs/plans/current-phase.md`** to find the active phase file.
2. **Verify every `## Exit criteria` checkbox is `- [x]`.** If any remain
   `- [ ]`, stop and list them — do not tick a criterion yourself just to
   proceed; that must reflect real, verified state (e.g. "workspace builds"
   means someone actually ran `cargo build --workspace` and it succeeded).
3. **Spec-adherence check:** for a phase that shipped spec-facing behaviour,
   confirm a conformance pass happened (`/spec-audit` findings addressed or
   filed as tasks). If it never happened, stop and say so — that is an
   unmet exit criterion in spirit.
3a. **ECC zero-drift gate (blueprint §4 rule 4):** confirm a full ECC run
   (`/run-conformance`) happened at close and shows zero drift vs the
   committed baseline, and that the ratcheted `docs/conformance/` artifacts
   (results.json + report + badges) are in-branch. No green ECC run → the
   phase is not closable.
3b. **User docs + changelog updated?** If this phase changed a user-visible
   surface (REST, configuration, CLI, deployment artifacts), confirm BOTH:
   the matching `website/book/src` page was updated in-branch (and
   `scripts/assemble-oas.sh` re-run if the REST contract changed —
   `.claude/rules/docs-website.md`), AND a `CHANGELOG.md [Unreleased]` entry
   exists (`.claude/rules/changelog.md`; CI `changelog-guard` enforces it).
   If not, stop: both are part of the phase's deliverable.
3c. **Blueprint maintenance (blueprint §4 rule 7):** confirm the affected
   blueprint chapter + `00-THE-BLUEPRINT.md` §2 state rows were refreshed to
   verified reality, and the phase's close note is recorded.
4. **Update `docs/PROGRESS.md`** with one line for this phase: phase number,
   title, completion date, and a short note (mirroring the phase file's
   `## Decisions made this phase`, if any). Append; never rewrite prior
   phase rows.
5. **Write the phase file's `## Handoff for next session`** paragraph: where
   things stand at close, and what the next phase should do first. Replace
   any placeholder text that is there.
6. **Set the phase file's `Status` header to `done`.**
7. **Advance `docs/plans/current-phase.md`** to point at the next phase file
   in sequence (per the build order in `docs/blueprint/00-THE-BLUEPRINT.md` §3),
   with a fresh session goal and next action for that phase's first task.
8. **Remind the user to commit** as `phase-NN: phase complete` on the
   current `claude/phase-NN-*` branch — this skill edits files but does not
   run git commands itself.

## What this skill does not do

It does not run `cargo build`, the test suite, or the ECC suite to "check"
the exit criteria for you — those must already have been run and have
genuinely passed before this skill is invoked. If in doubt, run
`/run-conformance` or the relevant `cargo` command first.
