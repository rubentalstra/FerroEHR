---
name: phase-done
description: >
  Closes a worklist row: verifies the row's work is genuinely done (gates,
  docs, changelog), records the close in docs/PROGRESS.md, moves the row to
  the Closed table with its PR link, and deletes the implemented plan file.
  Use when the user says a work item is complete or asks to close it out.
allowed-tools: [Read, Edit, Grep, Glob]
argument-hint: (none)
---

# /phase-done

The closing step of the worklist workflow (`CLAUDE.md`). Only run this once
the row's work is actually finished — this skill verifies and records, it
does not decide the work is done on your behalf.

## Steps

1. **Read `docs/plans/WORKLIST.md`** and identify the row being closed (the
   user names it, or it is the row whose plan file this branch implements).
   If the row points at a plan file, that file's `## Exit criteria` is the
   checklist.
2. **Verify every `## Exit criteria` checkbox is `- [x]`.** If any remain
   `- [ ]`, stop and list them — do not tick a criterion yourself just to
   proceed; that must reflect real, verified state (e.g. "workspace builds"
   means someone actually ran `cargo build --workspace` and it succeeded).
3. **Spec-adherence check:** for a phase that shipped spec-facing behaviour,
   confirm a conformance pass happened (`/spec-audit` findings addressed or
   filed as tasks). If it never happened, stop and say so — that is an
   unmet exit criterion in spirit.
3a. **ECC zero-drift gate:** confirm a full ECC run
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
3c. **Living-reference-doc maintenance:** confirm the living reference docs
   (`docs/architecture.md`, `docs/endpoint-map.md`, `docs/VERSIONS.md`) were
   refreshed to verified reality for anything this phase changed, and the
   phase's close note is recorded. (There is no blueprint/design-doc layer —
   internal plan/design files are deleted in the PR that implements them; the
   durable record is `docs/PROGRESS.md`, `CHANGELOG.md`, git history, and
   these living reference docs.)
4. **Update `docs/PROGRESS.md`** with one line for this phase: phase number,
   title, completion date, and a short note (mirroring the phase file's
   `## Decisions made this phase`, if any). Append; never rewrite prior
   phase rows.
5. **Write the phase file's `## Handoff for next session`** paragraph: where
   things stand at close, and what the next phase should do first. Replace
   any placeholder text that is there.
6. **Set the phase file's `Status` header to `done`.**
7. **Move the row to the WORKLIST `## Closed` table** with the merged-PR
   link, and **delete the implemented plan file** in the same PR (the
   delete-on-implementation lifecycle; `docs/plans/README.md`).
8. **Remind the user to commit** the close on the current
   conventional-type branch (`feat/…` etc., per the CLAUDE.md branch hard
   rule) — this skill edits files but does not run git commands itself.

## What this skill does not do

It does not run `cargo build`, the test suite, or the ECC suite to "check"
the exit criteria for you — those must already have been run and have
genuinely passed before this skill is invoked. If in doubt, run
`/run-conformance` or the relevant `cargo` command first.
