# docs/plans/

Running plans and task lists. This is the durable layer: markdown checkboxes
on disk survive `/clear` and `/compact`, while the built-in todo tool is
session-scoped only. If it isn't ticked here, it isn't done.

## Doc lifecycle & citation rules (owner, 2026-07-17)

- **Internal plan/design markdown is deleted in the PR that implements it.**
  A plan or design file tracks work that is not yet done; once its content
  has landed, the stale file causes conflicts and confusion, so it is
  removed. The durable record of what shipped is `docs/PROGRESS.md`,
  `CHANGELOG.md`, git history, and the living reference docs
  (`docs/architecture.md`, `docs/endpoint-map.md`, `docs/VERSIONS.md`).
- **There is no ADR layer** — it was deleted (it caused more confusion than
  value). No file may instruct anyone to read, write, or cite an ADR.
- **The only citable references are the vendored openEHR specs
  (`docs/specs/openehr/`) and official external documentation** (the
  PostgreSQL docs, the Rust book/reference, the docs.rs/crates.io docs of a
  pinned crate). Never cite an internal markdown file as an authority — it
  moves or dies. (Live *pointers* to open-work trackers — `WORKLIST.md`,
  `current-phase.md`, an open plan file — are fine; that is navigation, not
  citation.)

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
- `feature-flat-structured.md` — the FLAT/STRUCTURED interop-depth track.

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
