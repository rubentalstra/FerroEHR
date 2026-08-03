---
name: en-route-findings-always-filed
description: "Owner hard rule 2026-08-02 — anything strange found outside a task's scope gets a tracker issue, never ignored because \"it was already there\""
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-02T09:41:50.660Z
---

Owner hard rule 2026-08-02: when work (mine or a subagent's) encounters
something wrong or misplaced OUTSIDE the task at hand — code in the wrong
crate, duplicated definitions, stale claims, missing coverage — it is FILED
as a tracker issue so it can be taken on later. "It was already there" /
"not on the task list" is never a reason to ignore it.

**Why:** unreported observations are lost work; the owner wants every
discovered defect visible on the tracker even if deferred.

**How to apply:** all six .claude/agents/*.md defs now carry an
"En-route findings" reporting section (workers report with file:line
evidence, never fix out-of-scope themselves); the orchestrator FILES an
issue for every reported finding — adjudicating one away requires stating
why in the record, silence is not an option. This generalizes CLAUDE.md
§Issue workflow's "new work discovered en route gets its own issue".
Related: [[foundation-first-sequencing]] (systemic classes escalate to a
sweep phase).
