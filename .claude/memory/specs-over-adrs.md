---
name: specs-over-adrs
description: "Owner ruling 2026-07-11: the openEHR specs are LEADING — ignore/override any ADR that conflicts; ADRs could be totally wrong"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 638fb34b-f9d0-4f7b-9da8-b5fa3ba9a9e9
---

Owner ruling (2026-07-11, during A1, verbatim intent): **"ignore all ADRs
because they could be totally wrong — the openEHR specs are leading."**

**Why:** ADRs (and blueprint/plan text, ADR-013 §7 trunk-only etc.) recorded
design decisions made BEFORE the full spec audit; several encoded
convenience limitations (trunk-only versioning, deferred terminology
checks) as if they were settled. They are not authority — the vendored
openEHR spec text at `docs/specs/openehr/` is the only authority.

**How to apply:** when an ADR/PORT NOTE/blueprint row conflicts with the
vendored spec text, implement the SPEC and update/supersede the ADR note —
never cite an ADR to justify not implementing spec-mandated behaviour.
ADRs remain useful as records of storage/tooling choices the spec doesn't
govern (PG18 internals, crate layout), but any spec-facing claim in them
must be re-verified against the spec text before being relied on.

**Citation rule (owner, 2026-07-11, "at most importance"):** code comments,
SQL schema comments, and doc comments must cite ONLY the openEHR specs —
never "ADR-NNN §x" as the justification. Where the openEHR specs are SILENT
on a decision (storage mechanics, index choices, infra), FLAG it explicitly
(e.g. "no openEHR spec governs this — our own storage design") instead of
citing an ADR. Applies to new code immediately; scrub existing citations
as files are touched.
Related: [[spec-adherence-mandate]], [[a1-audit-cadence]].
