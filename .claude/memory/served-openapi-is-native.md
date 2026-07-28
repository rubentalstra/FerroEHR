---
name: served-openapi-is-native
description: "ehrbase-rest serves ONLY its own utoipa-generated OpenAPI; the vendored ITS-REST OAS is codegen input ONLY (stalled — NOT a behavioural oracle; the ITS-REST docs text is the oracle), never imported/served — owner corrected agents on this repeatedly"
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

**Why:** the vendored OAS is the `emit-rest` codegen input for the generated
`openehr-its` contract and a SUBORDINATE wire source — never a served
document. Owner rulings 2026-07-24 + 2026-07-28: the conformance oracle is
the ITS-REST **docs text** (`docs/specs/openehr/ITS-REST/`), which wins
every conflict with the OAS; where the docs text is SILENT the released OAS
grounds the behaviour (the release's own overview `Specifications.md`
presents the OAS files as its computable artifacts). The served document
legitimately differs from the OAS (ours includes the own-design
extensions). See [[cnf-spec-oracle-attribution]].

**How to apply:** when changing anything wire-visible (media types,
headers, params, status codes), read the ITS-REST **docs text**
(`docs/specs/openehr/ITS-REST/`) for the required BEHAVIOUR — NOT the stalled
OAS — then update OUR `#[utoipa::path]` declarations in the same PR so
the served document advertises it. Root CLAUDE.md's OpenAPI note carries
the rule (the ADR layer itself was deleted 2026-07-17). Never
resurrect the "code→OAS drift-check" idea. Related: [[owner-work-style]].
