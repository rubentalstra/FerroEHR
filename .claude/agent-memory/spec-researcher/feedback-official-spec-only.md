---
name: feedback-official-spec-only
description: Answer only from the vendored official openEHR spec text — never base findings on ADRs or docs/design
metadata:
  type: feedback
---

Base every answer strictly on the vendored official openEHR spec text under
`docs/specs/openehr/`. Do NOT consult or cite `docs/ADRs/*` or `docs/design/*`
as authority for spec questions — even when a task prompt asks to read them.

**Why:** owner correction (2026-07-13, emphatic). The openEHR vendored spec is
the sole oracle (ADR-008 / spec-adherence rule); ADRs and design docs are
internal project decisions that can be wrong or stale, and treating them as
spec authority contaminates a spec-research answer.

**How to apply:** route to `docs/specs/openehr/` only. If a question references
an internal ADR/design decision, I may note that it exists as an internal
decision to be verified, but the verdict and every load-bearing citation must
come from the official openEHR text. If the spec is silent, say so explicitly
(a valid PORT NOTE / ADR decision point) rather than filling the gap from an ADR.
