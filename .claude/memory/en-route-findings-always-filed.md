---
name: en-route-findings-always-filed
description: "Owner hard rules 2026-08-02 + 2026-08-04 — every en-route finding gets a tracker issue, AND a tractable one is fixed in the same branch (file = record, never a parking lot)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-04T17:01:45.814Z
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

**Sharpened (owner, 2026-08-04, the #1899–#1903 correction):** filing is
the RECORD, not a deferral. A finding whose fix is tractable in the
current branch is implemented THERE, and the PR closes its issue
(`Closes #N`) — consistent with [[owner-work-style]] "defer nothing".
Only a finding that genuinely cannot land now (needs an idle box, an
upstream answer, a separate exclusive run) stays open past the branch,
with the blocker stated on the issue. "It needs its own adjudication" is
NOT a blocker — adjudicate it now.

**Verify NEGATIVE premises before filing (self-correction, 2026-08-22, the
#2568 lesson):** a worker's "no API offers X" claim is an existence claim
about the whole surface, and the worker only probed the routes IT knew.
Before filing an issue on a negative premise, grep the route
tables/wire_surface first-hand — #2568 was filed on "an ADL2 template can
never be removed through any API" while `DELETE
/definition/artefact/adl2/{artefact_id}` existed, Admin-classed and fully
CNF-cased, and a duplicate route got half-built before the check that
should have preceded the filing.

**Sharpened again (owner, 2026-08-20, the #2441–#2454 correction):**
within an audit/QA program, the fix-first cadence covers SELF-FILED
issues too — verification-pass findings, guard gaps, en-route defects I
file while auditing a chapter are all FIXED AND MERGED before the next
chapter/unit starts, exactly like the chapter's own section findings.
Filing them and moving on to the next chapter is the violation the owner
called out on 2026-08-20 ("solve all these issues you create before you
are allowed to go to the next chapter"). A program-wide drain issue
(e.g. a register sweep) is no exception: it is worked NOW, not parked
behind a blocked-by edge on the program parent.
