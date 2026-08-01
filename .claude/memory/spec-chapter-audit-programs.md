---
name: spec-chapter-audit-programs
description: "Owner's preferred compliance method — one milestone per spec component, one issue per chapter, whole-codebase sweep, fix-first cadence"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-08-01T06:30:12.656Z
---

Owner directive 2026-07-29: systematic code compliance is driven as **one
milestone = one spec component, one issue per spec CHAPTER**, each chapter
issue sweeping the WHOLE codebase via `/spec-audit` — the same per-unit
discipline as the ITS-REST per-endpoint audit (#373). First instance: the
BASE 1.3.0 program (#686, 29 chapter children) = v3.13.0, **completed and
released 2026-07-30** (8 findings, all fixed under the cadence; zero-drift
close run 809/809; tag v3.13.0). v3.14.0 = QUERY, **completed and released 2026-07-30** (9 findings incl. six
wire-visible engine defects the green suite had never caught; catalogue
28→45 query cases; AMB-174/175). v3.15.x = AM, completed. v3.16.0 = LANG
(41 chapters, 198 issues), **completed and released 2026-08-01** — the
big catch was the two-schema merge chimera (v2 shapes emitted at bmm3
paths; fixed by per-generation emission + attribute-level emitter
invariants, PR #1410); read-only spec-conformance-reviewer agents in
parallel worked well for bulk chapter audits (evidence-verified
checklists pasted from their report files), and posting GH comment
bodies MUST use heredocs (backticks in double-quoted `--body` get
shell-executed and silently mangle the comment). v3.17.0 = TERM (5 chapters + 5 section
children), **completed and released 2026-08-01** — asset fidelity was
exact (all 7 code sets + 17 vocabulary groups x 5 languages, verified by
scripted table<->XML diff; ch.5/ch.6 closed honestly as stubs master.adoc
excludes); the real catches were extract_content_type unenforced on
EXTRACT_SPEC.extract_type (#1416) and — flushed out by the program's four
new envelope wire cases — the UNTAGGED-NODE ESCAPE CLASS (#1431): every
validation pass dispatched on the wire _type tag while canonical JSON
legally omits it on concretely-declared slots, so untagged nodes skipped
ALL RM invariants (JSON committed what XML refused); fixed by
effective-type resolution over the BMM-generated RM model + a corpus-wide
tag-independence property. Lesson: adding SMALL spec-cited wire cases is
what finds application escape classes — the in-crate suites were
example-based and blind to whole input shapes; the property-test hardening
program for openehr-its is #1434 (v3.17.1). Queue: v3.18.0 = RM (47),
v3.19.0 = SM (20), then v4.0.0 = the admin-UI program.
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

**Refinement (owner directive 2026-07-30, applied to RM/LANG/TERM/SM —
AM excluded as the then-active program, kept at its owner-adjudicated
pre-split granularity):** each chapter issue decomposes further into one
sub-issue per spec §X.Y section (a chapter with 0–1 `== ` sections stays
the audit unit itself) — auditing a whole chapter at once risks skipping
sections like RM common §6.3 Versioning Semantics. And every audit body is
behaviour-explicit: a requirement is the WHOLE normative content —
structural (classes/invariants) AND behavioural (state machines,
procedures, identifier rules, prose semantics — normative even when no
invariant encodes them) AND wire-visible — and the audit question is
whether the RUNTIME BEHAVIOUR matches the text, never merely "the types
exist" (the owner rejected wording focused on "testable code" only).
Section counts derive from `grep '^== ' <master file>` in the vendored
spec tree; totals filed 2026-07-30: RM 122, LANG 136, TERM 5, SM 60.
