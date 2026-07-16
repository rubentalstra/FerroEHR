---
name: no-task-ids-in-code
description: "Owner hard rule (2026-07-16): task/register item IDs (F-nn, S-nn, G-nn, W-nn, wave/step markers) must NEVER appear in code or doc comments — only docs/specs/openehr citations are allowed"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 58d2e09d-1858-4a52-a5b7-e494a0472505
---

Owner hard rule (2026-07-16, during the W-14 platform rewrite): **never
reference internal task-tracking identifiers in the codebase** — no `F-13`,
`S-16`, `G-3`, `G-09-05`, `W-14`, wave/step/checklist markers, or any other
plan-register shorthand in code comments, doc comments, SQL comments, or
test names. Those IDs live only in `docs/plans/*` tracker files, which get
deleted after their phase closes — a code comment citing one becomes a
dangling pointer.

**Why:** the tracker files are ephemeral; the owner ruled they pollute the
docs. This extends the existing "cite ONLY the openEHR specs, never an ADR"
rule to tracker IDs: the only legitimate reference in code is
`docs/specs/openehr/...` (spec file + section), or the explicit
"no openEHR spec governs this — our own design" flag.

**How to apply:** when writing or porting a comment that carries a tracker
ID, replace the ID with the underlying spec citation or plain-prose reason
(what the marker stood for), or drop it. When touching a file that still
carries one, scrub it in the same change. Related: [[owner-work-style]].
