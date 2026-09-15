---
name: design-record-updated-in-the-same-turn
description: Owner rule 2026-09-15 — every decision that changes a design is written into the plan file, the design comment on the parent issue (#3337 comment 5658324709 for storage) and every affected issue IN THE SAME TURN it is made; a stale record is legacy
metadata:
  type: feedback
---

The owner asked whether the storage design comment (#3337, comment id 5658324709) had been updated after the role and immutability decisions; it had not, and the answer must always be yes: "it's very very important that all the issues and all the decisions are kept up to date", otherwise "things get old and then it's getting legacy".

**Why:** the tracker and the plan ARE the design record (no ADR layer); a decision that lives only in a PR or in chat is invisible to the next worker and rots into a contradiction.

**How to apply:** a design-changing decision (an owner ruling, a review finding that changes the shape, a rename, a rule change) is applied in ONE turn to: (1) `docs/plans/<plan>.md` (the sections it touches, the decomposition table), (2) the design comment on the parent issue, edited in place (one comment, never a new one, per [[public-comments-one-and-short]]), (3) every sub-issue whose contract it changes (body edited, criteria ticked or reworded), (4) memory when it is a standing rule. When the checkout is held by a worker, the comment and the issues are updated immediately through `gh` and the plan edit is queued for the very next free window, never later. Related: [[rewrite-breaks-everything-shipped-means-released]], [[en-route-findings-always-filed]].
