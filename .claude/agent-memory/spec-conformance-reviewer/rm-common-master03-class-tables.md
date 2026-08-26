---
name: rm-common-master03-class-tables
description: Verified findings for RM common master03 §3.2 (the six PATHABLE/LOCATABLE/ARCHETYPED/LINK/FEEDER_AUDIT(_DETAILS) class tables) — where invariant enforcement is partial and where the invariant tripartition register lies
metadata:
  type: feedback
---

Verified 2026-08-01 against RM 1.2.0
`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.{pathable,locatable,
archetyped,link,feeder_audit,feeder_audit_details}.adoc`.

**Archetype_node_id_valid is enforced on only ~17 of 39 LOCATABLE subtypes.**
`crates/openehr-rm/src/validate/generated.rs:315 archetype_node_id_core` is
called from 8 `*_impl.rs` (element/cluster/section/folder/point_event/
interval_event/item_table/generic_entry) plus the composite cores
(history_basic/composition/entry_root/activity). **ITEM_TREE / ITEM_LIST /
ITEM_SINGLE have NO `*_impl.rs` at all** and are not in the
`openehr-its/src/rm_validate.rs:330-412` hand table → they fall to
`run_structural` (:425), which only decodes. `validate::fast`
(`crates/openehr-rm/src/validate/fast.rs:494`) lists them in `known_class`
(:168-170) but has no match arm → `_ => return false` (:576). Net: a
COMPOSITION whose `data` is `{"_type":"ITEM_TREE","archetype_node_id":""}`
commits 201. Same for every demographic LOCATABLE and the EHR_EXTRACT ones.

**The invariant tripartition register only accounts for the HOOK bucket.**
`tools/openehr-codegen/src/render/emit_validate.rs:93 pending_register`
enumerates `Bucket::RuntimeHookMissing` only; `CORES` (:231) is a fixed
literal set of 17 cores / 21 invariant names, while the classifier reports
**90 emitted**. `tests/it/emitter_invariants.rs:566` checks a HARDCODED name
list, so a classified-"emitted" invariant can be silently unrealized —
demonstrated by `LOCATABLE.Links_valid` (pinned "emitted" at
`emitter_invariants.rs:537`) and `LOCATABLE.Archetyped_valid`, neither of
which appears anywhere in `crates/openehr-rm/`.

**The RM invariant pass runs on the COMPOSITION path ONLY.** Nothing in
`app/` calls `openehr_its::rm_validate::validate_rm_value`; the only entry is
`service/ehr/validation.rs:107` (`flat::validation::validate_rm_and_terminology`,
COMPOSITION). EHR_STATUS (:385), EHR_ACCESS (:517), FOLDER (:591) and the
demographic parties get hand-written structural checks only → on those kinds
`ARCHETYPED.Rm_version_valid`, `FEEDER_AUDIT_DETAILS.System_id_valid`, LINK
structural conformance and nested `Links_valid` are ALL unenforced.

**Archetyped_valid is arguably vacuous — no register entry exists.**
`is_archetype_root()` has no defining formula in the class table (Meaning
only), so `is_archetype_root xor archetype_details = Void` reads as
`X xor not X` under the reference-object-model derivation. The adjudication
lives as a `// NOTE:` at `crates/openehr-its/src/flat/validation/mod.rs:750`
and leans partly on stalled CNF data sets — not in
Veredictum's `artifacts/registers/ambiguities.yaml`.

**Conformant, verified (do not re-check):** all six generated shapes match the
tables (`common/archetyped/*.rs`); XML emits the XSD sequence order for
FEEDER_AUDIT_DETAILS (location/provider/subject — NOT the BMM order) with
`other_details` appended (no ITS-XML lineage declares it, RM 1.2.0 does);
`Link` JSON emits `"type"` not `r#type`; all 5 PATHABLE functions in
`crates/openehr-rm/src/paths.rs`; LINK inside a COMPOSITION IS validated
(links → `declared_concrete_type` = LINK → structural + `target` → DV_EHR_URI
`Scheme_valid`); the FEEDER_AUDIT change_type gap is AMB-176 (upstream #1610).

**Coverage gaps:** no CNF/corpus fixture anywhere carries a NON-EMPTY `links`
(only `minimal_event.links_empty.json`, the refusal twin) — the valid twin is
missing; no case for `Rm_version_valid` or `System_id_valid`;
`flat/map/structures.rs:424 build_link` fabricates empty-string DV_TEXTs for
missing `_link:i|meaning`/`|type` (refused later by `Valid_value`, but with a
misleading message).
