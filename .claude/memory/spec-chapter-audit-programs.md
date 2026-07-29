---
name: spec-chapter-audit-programs
description: "Owner's preferred compliance method — one milestone per spec component, one issue per chapter, whole-codebase sweep, fix-first cadence"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-07-29T12:41:43.440Z
---

Owner directive 2026-07-29: systematic code compliance is driven as **one
milestone = one spec component, one issue per spec CHAPTER**, each chapter
issue sweeping the WHOLE codebase via `/spec-audit` — the same per-unit
discipline as the ITS-REST per-endpoint audit (#373). First instance: v3.14.0 = the BASE 1.3.0 program (#686, 29 chapter
children). Release sequence (owner 2026-07-29): v3.13.0 = the CDS 2.0.1 /
GDL2 component program (#716 — a NET-NEW component, GDL2 only, retired
GDL never implemented); v3.14.0 = BASE audit; v4.0.0 = the admin-UI
program.

**Why:** the owner considers this the best approach to reach real code
compliance ("proper and systematically"), mirroring what worked for
ITS-REST.

**How to apply:** when a new compliance push starts (RM, AM, TERM, QUERY
likely next), propose the same shape: parent program issue + per-chapter
children (normative chapters only, front matter excluded by adjudication),
findings as sub-issue fix issues with the [[its-audit-fix-first-cadence]]
(all fixes from a chapter merged before the next chapter's audit starts),
zero-drift pipeline at program close.
