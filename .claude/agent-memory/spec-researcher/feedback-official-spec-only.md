---
name: feedback-official-spec-only
description: Answer only from the vendored official openEHR spec text — never base findings on internal plan/design docs
metadata:
  type: feedback
---

Base every answer strictly on the vendored official openEHR spec text under
`docs/specs/openehr/`. Do NOT consult or cite any internal markdown (plan or
design docs) as authority for spec questions — even when a task prompt asks to
read them. (The ADR layer has been deleted; plan/design markdown is deleted in
the PR that implements it — so no internal doc is ever a spec authority.)

**Why:** owner correction (2026-07-13, emphatic). The openEHR vendored spec is
the sole oracle (the spec-adherence rule); internal plan/design docs are
project decisions that can be wrong or stale, and treating them as spec
authority contaminates a spec-research answer.

**How to apply:** route to `docs/specs/openehr/` only. If a question references
an internal design decision, I may note that it exists as an internal decision
to be verified, but the verdict and every load-bearing citation must come from
the official openEHR text. If the spec is silent, say so explicitly (a valid
spec-silence `// NOTE:` decision point) rather than filling the gap from prior
art or memory.
