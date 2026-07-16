# docs/plans/

Running plans and task lists. This is the durable layer: markdown checkboxes
on disk survive `/clear` and `/compact`, while the built-in todo tool is
session-scoped only. If it isn't ticked here, it isn't done.

## What lives here now

Completed phase/plan files are pruned once their close is recorded in
`docs/PROGRESS.md` (their content lives in git history). The former
blueprint + design-doc layer was deleted 2026-07-16 (owner): implemented or
stale — **the spec oracle is `docs/specs/openehr/`**, the product roadmap is
the root `ROADMAP.md`, and this folder carries only the live work:

- `current-phase.md` — the live pointer: which work is active, the session
  goal, and the next action. Update whenever the active work changes.
- `WORKLIST.md` — the single open-items tracker (owner-mandated): one row
  per open item; close a row by linking the merged PR.
- `w14-audit.md` + `w14-service-rewrite.md` — the active W-14 register
  (full endpoint/path audit) and rewrite tracker.
- `feature-flat-structured.md` — the FLAT/STRUCTURED interop-depth +
  EhrScape feature track (renamed from the old phase-17 file).

### Phase-file template

New phase files follow this skeleton:

```markdown
# Phase NN — <title>

- Status: not-started | in-progress | blocked | done
- Started: <date>   Owner: <name>
- Consumes: <spec/layer or prior phase(s) + governing docs/specs paths>
- Compile required: yes (application phases build as compiling, tested increments)

## Objectives

## Tasks

- [ ] ...

## Exit criteria

- [ ] ...
```
