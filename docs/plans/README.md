# docs/plans/

Deep working plans for open tracker issues. **The tracker itself is GitHub
Issues** (owner 2026-07-20; root `CLAUDE.md` §Issue workflow): an issue's
body carries the contract + exit-criteria checklist; a plan file here holds
the deep working material (research, task breakdowns, design registers) that
would overload the issue body. Every plan file is linked from its issue and
deleted in the PR that implements it.

## Doc lifecycle & citation rules (owner, 2026-07-17)

- **Internal plan/design markdown is deleted in the PR that implements it.**
  A plan or design file tracks work that is not yet done; once its content
  has landed, the stale file causes conflicts and confusion, so it is
  removed. (If two open issues consume the same plan file, it is deleted
  when the LAST of them closes.) The durable record of what shipped is
  the closed issues + PR descriptions, `CHANGELOG.md`, git history, and the
  living reference docs (`docs/architecture.md`, `docs/VERSIONS.md`).
- **There is no ADR layer** (owner ruling 2026-07-17). No file may instruct
  anyone to read, write, or cite an ADR.
- **The only citable references are the vendored openEHR specs
  (`docs/specs/openehr/`) and official external documentation** (the
  PostgreSQL docs, the Rust book/reference, the docs.rs/crates.io docs of a
  pinned crate). Never cite an internal markdown file as an authority — it
  moves or dies. (Live *pointers* to open work — a tracker issue, an open
  plan file — are fine; that is navigation, not citation.)

## What lives here now

- `WORKLIST.md` — a pointer stub to the tracker (GitHub Issues; this stub
  and this README are delete-protected).
- One `*.md` working plan per open issue that needs one; nothing else.
  Completed plan files are pruned in the PR that lands them, with the close
  recorded in the PR description + the issue's handoff comment.

### Plan-file template

New plan files follow this skeleton (linked from their tracker issue):

```markdown
# <title> (tracker issue #NN)

- Status: in-progress | blocked
- Started: <date>
- Consumes: <spec/layer or prior work + governing docs/specs paths>

## Objectives

## Tasks

- [ ] ...

## Exit criteria

- [ ] ...
```
