---
name: next-milestone-in-fresh-session
description: After a release cut, stop; the owner starts the next milestone in a clean session rather than continuing in the release session
metadata:
  type: feedback
---

After cutting a release (tag pushed, milestone closed, board update posted), do not pick up the next milestone's issues in the same session. Finish watching the release run, report, and stop.

**Why:** owner instruction 2026-09-12 after the v4.2.2 cut ("i do not want you to start with 4.3.0 … i will restart the session clean session"): a long release session carries compacted context, and the next milestone deserves a fresh orientation from the tracker.

**How to apply:** the release procedure ends at the board update + `sync-dates` + the release-run watch. Never branch for the next milestone or move its issues to In Progress in the release session; the earlier standing "start the next without asking" flow ([[autonomous-phase-flow]]) applies within a milestone, not across a release cut.
