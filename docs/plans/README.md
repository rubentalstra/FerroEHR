# docs/plans/

Running plans and task lists for the port. This is the durable layer: markdown
checkboxes on disk survive `/clear` and `/compact`, while the built-in todo
tool is session-scoped only. If it isn't ticked here, it isn't done.

## Files

- `current-phase.md` — a 3-line pointer: which phase file is active, the
  session goal, and the next action. Update this whenever the active phase
  changes.
- `phase-NN-<title>.md` — one file per phase (P0-P19, P99), each following the
  template in `PORT_MASTER_PLAN.md` Section 13.1: header (Status/Started/
  Owner/Consumes/Compile required), Objectives, Preconditions, Scope,
  Tasks, Exit criteria, Decisions made this phase, Handoff for next session.

## The six-step loop

1. Read `docs/plans/current-phase.md`.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work (delegate per-file ports to the `porter` subagent, spec
   transcription to `rm-transcriber`; prefer subagents over inline work when
   it would otherwise burn context).
4. Tick the task `- [ ]` to `- [x]` and add a one-line note.
5. Commit as `phase-NN: <task>` on a `claude/phase-NN-*` branch.
6. When the phase's exit criteria are all met, run `/phase-done`, update
   `docs/PROGRESS.md`, and advance `current-phase.md` to the next phase file.

## Discipline

- Every ticked box gets a one-line note (what changed, where). A bare
  `- [x]` with no note is not enough to hand off to the next session.
- Never tick a box for work that wasn't actually done, and never delete or
  weaken a task to make a phase look finished.
- Phase files in this directory are never deleted, not even after a phase is
  done — they are the permanent record of how the port proceeded. A
  `PreToolUse` hook blocks deletion of anything under `docs/plans/`.
- Status transitions are `not-started -> in-progress -> blocked | done`.
  Update the `Status` line in a phase file's header as work proceeds.
