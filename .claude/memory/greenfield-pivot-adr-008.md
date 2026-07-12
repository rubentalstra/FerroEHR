---
name: greenfield-pivot-adr-008
description: 2026-07-05 pivot — greenfield internals incl. own PG18 storage design; openEHR spec conformance (not EHRbase parity) is the compatibility target
metadata: 
  node_type: memory
  type: project
  originSessionId: f34b6e9d-07db-4e26-bbb0-53471ae7eb9f
---

On 2026-07-05 Ruben pivoted the project (during P10): **full greenfield internals including our own storage design**, and the compatibility target changed from "bug-for-bug EHRbase REST parity" to **openEHR spec conformance** (ITS-REST 1.0.3 contract + AQL spec; acceptance = conformance suite, not diff-vs-EHRbase).

**Why:** Ruben repeatedly objected to "copying old Java shit" — he wants the best possible modern Rust codebase and algorithms, exploiting PG 18, not a port. EHRbase is reference/inspiration only now.

**How to apply:**
- ADR-008 (branch `claude/adr-008-greenfield-storage`) records the pivot; supersedes ADR-006's "follow EHRbase's algorithm/schema" core and the P19 EHRbase-parity harness. CLAUDE.md/roadmap prose predating it is stale until rewritten.
- The EHRbase-schema P09 work (merged PR #12) keeps its infrastructure (pool/settings/migrator/testcontainers gate) but the schema content gets replaced by our own design.
- The P10 rm-db-format port is archived unmerged on `claude/phase-10-rm-db-format`.
- Storage + AQL execution get designed together (single-JSONB-doc vs own decomposition vs hybrid — evaluate against AQL requirements).
- Related: [[official-cli-tooling-first]] (his standing demand for official tooling/modern practice).
