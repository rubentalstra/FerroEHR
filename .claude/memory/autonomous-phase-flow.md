---
name: autonomous-phase-flow
description: "Standing owner instruction — auto PR+merge each phase and start the next without asking (E1→E5 enterprise stage, 2026-07-10)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: df0992bc-dba7-497a-b3b2-8614868c0600
---

Owner standing instruction (2026-07-10, during the enterprise E-stage): when
a phase finishes — push, create the PR, **merge it right away**, checkout a
fresh branch from develop, and **start the next phase immediately** without
waiting for a "yes continue".

**Why:** the owner wants uninterrupted autonomous progression through the
roadmap (docs/enterprise/product-roadmap.md §3: E1 eventing → E2
multi-tenancy → E3 FHIR connectors → E4 S3/SeaweedFS multimedia → E5 K8s).

**Hard ordering rule (owner correction 2026-07-11, angry):** the sequence is
strictly commit → push → **create PR → merge** → `git fetch` → checkout the
next branch **from the updated develop**. NEVER cut a new working branch from
develop while finished work sits unmerged on a feature branch — the new
branch silently misses that work ("otherwise we are missing data"). If work
was just committed on any `claude/*` branch, merge it to develop first, then
branch.

**How to apply:** each phase still closes behind the standing gates
(workspace suites green + full ECC zero drift, run centrally via
scripts/conformance.sh; phase file ticked; blueprint/roadmap updated). Only
genuinely new design decisions (spec-silent seams needing an ADR choice the
owner hasn't made) still warrant an AskUserQuestion — mechanical
continuation never does. Related: [[verify-crate-versions-live]],
[[concurrent-sessions-shared-tree]].
