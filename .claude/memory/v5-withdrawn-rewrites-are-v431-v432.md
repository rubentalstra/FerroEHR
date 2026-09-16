---
name: v5-withdrawn-rewrites-are-v431-v432
description: Owner 2026-09-16: the v5 milestones are withdrawn; every greenfield rework lands in v4.3.1 or v4.3.2, and delegated decisions are made and recorded, not deferred
metadata:
  type: project
---

On 2026-09-16 the owner withdrew v5 ("v5 will be removed maybe because we will do something else there") and ruled that all greenfield rewrites are v4.3.1 (the storage rewrite, #3337) and v4.3.2 (the platform seams #3377 and the lean application layer #3086 with its children). Milestone v5.1.0 is closed; v5.0.0 holds only closed issues and awaits the owner's deletion. The owner also delegated the open design decisions ("decide things, check the docs, delete and close what makes no sense"): decisions are made in-session, recorded on the issue, in the plan and in the #3337 design comment the same turn.

**Why:** the owner wants the rewrite closed as one program, not spread across future majors.

**How to apply:** never file or move a rework issue to a v5.x milestone; when a decision is open, decide and record it rather than ask. Related: [[new-issues-go-to-next-patch-milestone]], [[design-record-updated-in-the-same-turn]], [[rewrite-breaks-everything-shipped-means-released]].
