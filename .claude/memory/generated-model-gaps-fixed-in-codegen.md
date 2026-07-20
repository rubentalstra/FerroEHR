---
name: generated-model-gaps-fixed-in-codegen
description: "Owner hard rule — a generated openehr-* model gap is fixed in the codegen emitter + regen, never worked around in a consumer"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9491391b-2993-4fd4-a58b-26e1d0724da8
  modified: 2026-07-19T10:31:33.044Z
---

Owner hard rule (2026-07-19, codified in root CLAUDE.md + codegen.md):
when consuming code hits a generated `openehr-*` shape that is wrong or
insufficient vs the vendored spec/BMM, fix `openehr-codegen` (emitter/
override) + regenerate — NEVER a shadow type, duplicate model, adapter,
placeholder, or "temporary" local representation in the consumer.
Cross-component subtype extension (AM extending LANG's closed expression
enums) is re-opened by the emitter at the DOWNSTREAM crate boundary;
upstream crates never gain downstream variants.

**Why:** the generated crates exist so implementation consumes the spec
model directly and is spec-conformant by construction; the owner
suspected (correctly) that consumer-side workarounds had crept in and
silently forked the spec model.

**How to apply:** on hitting such a gap, go straight to the emitter; if
the fix is large, register a tracker issue (`gh issue create`) — the
workaround is still forbidden. On DISCOVERING an existing workaround,
register a removal issue. See [[owner-work-style]] (no quick fixes) and
[[tracker-is-github-issues]].
