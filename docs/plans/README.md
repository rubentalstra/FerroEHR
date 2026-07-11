# docs/plans/

Running plans and task lists. This is the durable layer: markdown checkboxes
on disk survive `/clear` and `/compact`, while the built-in todo tool is
session-scoped only. If it isn't ticked here, it isn't done.

## What lives here now

The historical phase files (00–16, s2-01..05, sm-01..03) were pruned
2026-07-09, and the completed blueprint/enterprise phase files (B1–B8, E1–E5)
were pruned 2026-07-11 — their content lives in git history + `docs/PROGRESS.md`.
What remains is the *live* pointer and the *future* work:

- **`../blueprint/00-THE-BLUEPRINT.md` is the roadmap.** The blueprint (in
  `docs/blueprint/`) is the single source of truth for where the project is
  going and why; these phase files are the task-level execution record under
  it. Read the blueprint first.
- `current-phase.md` — the live pointer: which work is active, the session
  goal, and the next action. Update whenever the active work changes.
- `phase-20-optimization.md`, `phase-99-cutover.md` — the remaining Stage-1
  P-phases (future work). `phase-17`/`phase-18`/`phase-19` are retained for
  their task detail, though their scope was absorbed into the blueprint arc
  (P19 conformance is met; see `docs/PROGRESS.md`).
- The SM phase files are closed: the Terminology + Admin work of `sm-phase-04`
  shipped through the B1 rebuild + B3/B4 waves (recorded in `docs/PROGRESS.md`);
  the file was pruned 2026-07-11 with the other completed phases.

### Phase-file template

New phase files follow this skeleton:

```markdown
# Phase NN — <title>

- Status: not-started | in-progress | blocked | done
- Started: <date>   Owner: <name>
- Consumes: <spec/layer or prior phase(s) + governing docs/specs paths>
- Compile required: yes (application phases build as compiling, tested increments)

## Objectives
<what this phase delivers>

## Preconditions
- [ ] <prior phase(s) complete>

## Scope
In: <...>
Out: <...>

## Tasks
- [ ] <task 1>
- [ ] <task 2>

## Exit criteria
- [ ] <verifiable condition (e.g. suites green + ECC zero drift)>

## Decisions made this phase
- <ADR links, structural choices>

## Handoff for next session
<one paragraph: where things stand, what to do next>
```

## The loop

1. Read `current-phase.md`, then the blueprint section it points at.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work. The spec + ITS layer is **generated** (ADR-004/005) — change
   the emitter and regenerate (`/regen-codegen`), never hand-edit
   `// @generated`. The application layer (`ehrbase`, `ehrbase-rest`,
   `ehrbase-sm`) is **built idiomatically** on the generated `openehr-*`
   crates (ADR-006/008/011); the openEHR specs are the authority. Build
   compiling + tested. The orchestrator keeps the critical path in-session and
   may fan bounded work out to subagents (see `CLAUDE.md` "Model
   orchestration").
4. Tick the task `- [ ]` to `- [x]` and add a one-line note.
5. Commit as `phase-NN: <task>` on a `claude/*` branch.
6. When the phase's exit criteria are all met, run `/phase-done`, update
   `docs/PROGRESS.md`, and advance `current-phase.md`.

## Discipline

- Every ticked box gets a one-line note (what changed, where). A bare `- [x]`
  is not enough to hand off.
- Never tick a box for work that wasn't done; never weaken a task to make a
  phase look finished.
- Status transitions: `not-started -> in-progress -> blocked | done`.
- Completed phase files are pruned once their record is in `PROGRESS.md` + git
  history (the owner lifted the old "never delete docs/plans/" rule
  2026-07-09).
