---
name: codegen-emits-complete-model
description: Owner hard rule — codegen emits EVERYTHING the vendored inputs define; never trim/prune/suppress to shrink a diff or scope
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9491391b-2993-4fd4-a58b-26e1d0724da8
  modified: 2026-07-19T10:57:12.606Z
---

Owner hard rule (2026-07-19, codified in root CLAUDE.md + codegen.md,
after catching an agent narrowing a schema merge to shrink an emission
closure): the generator emits the COMPLETE model from the vendored
inputs — every class a legitimate closure yields, in full, mirrored to
its source package path, including classes nothing consumes yet.
Forbidden: narrowing schema merges, pruning "unrelated" classes,
suppressing generated files to quiet a diff, or restoring-around a
generator defect instead of fixing it in the same change.

**Why:** completeness IS the point of generation ("we may need it in the
future"); minimizing hides code that should exist and silently
under-models the spec.

**How to apply:** when an emission change pulls in a big new class set,
emit it all and let the diff be big; a generation defect found en route
gets fixed in openehr-codegen immediately. Pair with
[[generated-model-gaps-fixed-in-codegen]].
