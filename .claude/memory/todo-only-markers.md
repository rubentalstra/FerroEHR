---
name: todo-only-markers
description: "Owner hard rule 2026-07-17: pending work uses ONLY official TODO(...) markers; the bespoke PORT vocabulary (PORT NOTE/TODO(port)/PERF(port)/PORT STATUS) is deleted, banned, and CI-guarded"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7aa6f5cf-558f-4e08-aa70-143c486e1b76
---

Owner hard rule (2026-07-17, emphatic): **only the official `TODO` marker
family marks pending work** — `// TODO:`, `// TODO(perf):`, etc. — because
tools recognize it (IDE highlighting, grep, CI) and it can be enforced. The
bespoke `(port)` vocabulary (`// TODO(port):`, `// PERF(port):`,
`// PORT NOTE:`, PORT STATUS trailers) is **deleted workspace-wide and
banned**: it was unenforceable and its notes went stale.

**Why:** an annotation only works if a linter/CI can surface it; custom
markers rot silently. Same logic as the reliability rules ("a rule without
a failing check is a wish").

**How to apply:** pending/deferred work → `// TODO(scope):` with the
reason. A deliberate spec-silent design decision is NOT pending work → a
plain `// NOTE:` comment with the spec citation or the "no openEHR spec
governs this — our own design/extension" flag. Never reintroduce a bespoke
marker. Enforcement: the `comment-markers` CI guard in
`.github/workflows/ci.yml` greps the banned forms and fails; the rule is
registered in `.claude/rules/reliability.md` and root CLAUDE.md.
Related: [[owner-work-style]], [[no-task-ids-in-code]].
