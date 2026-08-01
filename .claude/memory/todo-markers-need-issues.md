---
name: todo-markers-need-issues
description: "Owner rule 2026-08-01 — every code // TODO must reference a GitHub issue (TODO(#NNNN)); a bare TODO is not allowed"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 39213bee-2a89-4d7c-8dca-5e502effeeab
  modified: 2026-08-01T10:14:55.786Z
---

Owner rule (2026-08-01, stated mid-session with emphasis): a `// TODO` in code
WITHOUT a GitHub issue behind it is not allowed.

**Why:** pending work must be visible in the tracker, not only in a grep —
the tracker is the worklist ([[tracker-is-github-issues]]); an unlinked TODO
is invisible planning debt.

**How to apply:** when writing a TODO, create/find the issue first and write
`// TODO(#NNNN): …`. A comment describing a SETTLED design is a `// NOTE:`
(with spec citation or spec-silence flag), never a TODO — mislabeling happened
in rm_validate.rs (fixed with #1431). The pre-existing 19-marker sweep is
issue #1432. Extends [[todo-only-markers]].
