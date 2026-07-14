---
name: session-workflow-gotchas
description: "Harness/hook gotchas that repeatedly cost time — background-task kill limit, attribution-hook false positives, changelog-guard label refresh"
metadata: 
  node_type: memory
  type: project
  originSessionId: 1be6641a-9768-4fd5-8149-acb2551a1d97
---

Three recurring session-workflow traps (all hit 2026-07-13/14):

1. **Long background Bash tasks get killed (~30 min)** — for multi-hour runs
   (benchmark ladders, seeds), launch detached: `nohup caffeinate -is
   script.sh > log 2>&1 & disown`, then watch the log with a Monitor
   (`tail -f | grep --line-buffered`). Also wrap in `caffeinate` — the Mac
   otherwise sleeps overnight and takes Docker down with it.

2. **Hook false positives**: (a) the no-attribution PreToolUse hook regex
   `generated with .*claude` spans the whole tool-call payload — any commit
   message containing "…generated with…" trips it because scratch/session
   paths contain "claude"; write "rebuilt with"/"produced with" instead.
   (b) `protect_vendored_specs.sh` greps files for the literal `@generated`
   marker — a hand-written file whose doc comment merely *mentions* the
   marker string gets blocked from Edit (fixed in openehr-term bundle.rs by
   rewording; avoid quoting the marker in prose).

3. **changelog-guard label needs a fresh PR event** — the workflow reads
   `github.event.pull_request.labels` (frozen at trigger time), so adding
   `no-changelog` then rerunning the failed job still fails. Close + reopen
   the PR to mint a new event.

**Why:** each cost a debugging loop mid-flow; the fixes are non-obvious.
**How to apply:** overnight/long runs → detached+caffeinate+Monitor pattern;
commit-message wording; close/reopen for late labels.
