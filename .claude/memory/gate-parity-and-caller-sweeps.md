---
name: gate-parity-and-caller-sweeps
description: "Two escape classes from one day: a local gate line missing a CI flag lets defects through workers' green runs; tightening a shared check without sweeping ALL its CI callers breaks the lane you didn't look at"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32d068af-12e7-4654-9ece-124240b2367f
  modified: 2026-08-27T08:52:17.713Z
---

Two gate-discipline escapes, both live on 2026-08-27:

1. **Local gate lines must mirror the CI invocation VERBATIM.** CLAUDE.md's
   rustdoc line lacked `--document-private-items` while CI passes it; the
   #2832 worker ran the CLAUDE.md line green and eight broken intra-doc links
   in PRIVATE helpers' docs reached main (red rustdoc, fixed in #2833).
   Before trusting any "gates green" claim — mine or a worker's — diff the
   command against the CI step's `run:` line, not against CLAUDE.md.

2. **Tightening a shared check obligates a sweep of EVERY CI caller of the
   underlying script.** #2806 made `validate.sh` FAIL under `CI=true` without
   helm-docs and installed the tool in ci.yml's helm-golden job — but
   `build-chart.yml` also runs `validate.sh`, got no install, and the v4.0.6
   release's chart leg died on it (fixed in #2840, recovered by dispatch).
   The mutation proof tested the SCRIPT, not the lanes. Before merging any
   check-tightening: `grep -rn '<script>' .github/workflows/` and satisfy the
   new requirement in every hit.

**Why:** both escapes shipped through fully green local runs and surfaced as
red CI/release lanes hours later; the misses are structural, not carelessness.

**How to apply:** when writing worker prompts, quote the CI step's exact
command for every gate; when tightening any `scripts/checks/*` behaviour,
enumerate its workflow callers in the same change and mutation-prove per
LANE where feasible, not just per script. Link: [[session-workflow-gotchas]].
