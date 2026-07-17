---
name: served-openapi-is-native
description: "ehrbase-rest serves ONLY its own utoipa-generated OpenAPI; the vendored ITS-REST OAS is codegen input + behavioural oracle, never imported/served — owner corrected agents on this repeatedly"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7aa6f5cf-558f-4e08-aa70-143c486e1b76
---

**The served OpenAPI is OURS, generated from OUR code.** Owner correction
(2026-07-17, angrily, after stale decision-doc framing misled a worker): `ehrbase-rest` never imports, serves, renders, or drift-gates
the vendored ITS-REST OAS yaml. Every handler carries `#[utoipa::path]`;
the complete document is composed natively in
`app/ehrbase-rest/src/extensions/openapi.rs` (single-sourced with the
router; one `ehrbase-rest` spec entry; "serve only what we generate").

**Why:** the vendored OAS is exactly two things — the `emit-rest` codegen
input for the generated `openehr-its` contract, and the behavioural/
conformance oracle (verified by the ECC, not by a document diff). The two
documents legitimately differ (ours includes the own-design extensions).

**How to apply:** when changing anything wire-visible (media types,
headers, params, status codes), read the vendored OAS for the required
BEHAVIOUR, then update OUR `#[utoipa::path]` declarations in the same PR so
the served document advertises it. Root CLAUDE.md's OpenAPI note carries
the rule (the ADR layer itself was deleted 2026-07-17). Never
resurrect the "code→OAS drift-check" idea. Related: [[owner-work-style]].
