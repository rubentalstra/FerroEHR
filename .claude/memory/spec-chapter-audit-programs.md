---
name: spec-chapter-audit-programs
description: "Owner's preferred compliance method — one milestone per spec component, one issue per chapter, whole-codebase sweep, fix-first cadence"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-07-29T23:18:28.454Z
---

Owner directive 2026-07-29: systematic code compliance is driven as **one
milestone = one spec component, one issue per spec CHAPTER**, each chapter
issue sweeping the WHOLE codebase via `/spec-audit` — the same per-unit
discipline as the ITS-REST per-endpoint audit (#373). First instance: the
BASE 1.3.0 program (#686, 29 chapter children) = v3.13.0, **completed and
released 2026-07-30** (8 findings, all fixed under the cadence; zero-drift
close run 809/809; tag v3.13.0). Next: v4.0.0 = the admin-UI program.
(CDS/GDL2 was briefly v3.13.0 but adjudicated OUT after a first-hand spec
read: CDS is a separate application layer consuming the CDR, not a CDR
component — #716 closed with the citation.) Per-chapter mechanics that
worked: post the walked checklist as an issue comment (the record), tick
body checkboxes via `gh issue edit`, close finding-free chapters by hand
with the record; findings merge on green local gates before the next
chapter's audit starts.

**Why:** the owner considers this the best approach to reach real code
compliance ("proper and systematically"), mirroring what worked for
ITS-REST.

**How to apply:** when a new compliance push starts (RM, AM, TERM, QUERY
likely next), propose the same shape: parent program issue + per-chapter
children (normative chapters only, front matter excluded by adjudication),
findings as sub-issue fix issues with the [[its-audit-fix-first-cadence]]
(all fixes from a chapter merged before the next chapter's audit starts),
zero-drift pipeline at program close.
