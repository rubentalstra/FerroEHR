---
name: rm-class-defs-location
description: RM class attribute tables (with existence multiplicities) live in docs/UML/classes/*.adoc, included by the master chapters
metadata:
  type: reference
---

RM class attribute definition tables (the `1..1`/`0..1` existence column) are in
`RM/docs/UML/classes/org.openehr.rm.<pkg>.<class>.adoc`, `include::`d at the end
of the prose chapter (e.g. `master08-entry_package.adoc` ends with
`include::{uml_export_dir}/classes/{pkg}action.adoc`). Grep the class file
directly for the attribute table — the prose chapters do NOT carry the
multiplicity table.

Confirmed: `org.openehr.rm.composition.action.adoc` — `ACTION.description :
ITEM_STRUCTURE` existence `*1..1*` (mandatory). Same for ACTION.time (1..1),
ism_transition (1..1); instruction_details (0..1).
