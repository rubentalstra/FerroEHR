---
name: many-issues-per-pr
description: Owner 2026-09-16 wants several issues per pull request during the rewrite; one issue per PR is too slow
metadata:
  type: feedback
---

Batch related issues into one pull request (six to ten per PR is fine), one `Closes #n` line per issue, one worker per batch.

**Why:** the owner on 2026-09-16: "we can definitely do more issues in one PR right? because one issue one PR is very very slow". One PR per issue cost a CI round and a merge wait per item.

**How to apply:** cut briefs by batch (storage follow-ups, hygiene, docs); a batch shares a branch and a worker; each issue's work is still its own commit inside the branch so a revert stays scoped. See [[pr-closes-one-keyword-per-issue]] and [[one-worker-per-phase-hard-fences]].
