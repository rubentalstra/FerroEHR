---
name: svg-text-owner-wording
description: "Owner feedback 2026-07-23: never reword/rewrap approved SVG text based on px-per-char estimates — the owner checks real rendering; iterate placement, keep their wording verbatim"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 0a07a7e8-293e-4d9d-b946-943fab0fcdba
  modified: 2026-07-23T07:40:24.972Z
---

During the class-ladder SVG iterations (2026-07-23) I repeatedly rewrapped
and REWORDED caption text because my ~6.3px-per-character overflow
estimates said it wouldn't fit — the owner had to interrupt three times:
the approved two-line wording DID fit at real glyph widths, and the
rewording/wrapping churn was the annoyance, not the layout.

**Why:** heuristic text-width estimates for 11px sans are pessimistic
(real average ≈ 5.5px/char); the owner verifies the actual render in the
IDE and their judgement of "it fits" is ground truth.

**How to apply:** when the owner approves chart/SVG text, treat the
wording as fixed — adjust geometry (canvas, columns, anchors) around it,
never the words; only flag a genuine measured overflow they can see.
Related: [[session-workflow-gotchas]].
