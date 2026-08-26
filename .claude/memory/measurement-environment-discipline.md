---
name: measurement-environment-discipline
description: Measured runs need an idle box + compose limits matching the ixit envelope; lane renames must update ixit containers blocks
metadata: 
  node_type: memory
  type: project
  originSessionId: 3369ea8f-21b3-4a04-b632-fa9c6be28562
  modified: 2026-07-25T03:11:49.978Z
---

Two measurement-integrity traps hit on 2026-07-25 during the #261 ladder:

1. A stress knee halved (512→256/s) because (a) the docker-rework compose
   resource limits (2cpu/1Gi) contradicted the party ixit's declared 8-CPU/8GB
   measurement envelope, and (b) a subagent compiled Rust during the measured
   window. Compose core-service limits now default to the envelope
   (FERROEHR_CPUS/MEM overridable); production limits are Helm's concern.
   **Never run agents/builds concurrently with a measured stress/perf window.**
2. Compose project-name changes rename containers: the ixit `containers`
   blocks (docs/conformance/party/*/ixit*.json) pin container names for
   pg_stat_statements attribution — a rename silently degrades attribution to
   "unavailable". Check them after any lane/project change.

**Why:** measured records are environment-bound and honest-publication gated;
a contaminated or env-shifted number would have been committed as a regression.

**How to apply:** before any `veredictum stress|aql-probe|perf` run: idle box
(no agents, no builds), fresh volumes, ixit container names verified against
`docker ps`. Related: [[owner-work-style]].
