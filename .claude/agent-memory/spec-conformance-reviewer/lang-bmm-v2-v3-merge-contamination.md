---
name: lang-bmm-v2-v3-merge-contamination
description: openehr-lang's bmm3/ module tree carries BMM-v2 class shapes for all 11 colliding class names, and drops BMM_MODEL_TYPE + BMM_MODULE entirely — verify before auditing any BMM3 chapter
metadata:
  type: project
---

`openehr-lang` is generated from TWO vendored LANG BMM files merged **first-wins
with the v2 file first** (`tools/openehr-codegen/src/plan/composition.rs`
`own: &[LANG_BMM, LANG_BMM3]` + `tools/openehr-codegen/src/load/bmm.rs`
`combined()` — `or_insert_with`, so `self` wins). 18 class names exist in both
vendored files; 11 differ materially, and in every one the **v2 shape lands in
the `bmm3/` module path**.

**Why:** LANG's model genuinely spans two files (persisted-BMM/`EXPR_*` +
BMM-object-model/`EL_*`) and neither alone resolves every AM reference, so the
merge is necessary — but the collision policy was never adjudicated against the
bmm3 chapter text.

**How to apply:** before auditing any BMM3 chapter, re-derive the collision set
by loading both `tools/openehr-codegen/vendor/bmm/components/LANG/json/openehr_lang_1.1.0*.bmm.json`
and diffing `ancestors`/`is_abstract`/`properties` per class. Confirmed
contaminated (2026-07-31): `BMM_TYPE`, `BMM_SIMPLE_TYPE`, `BMM_GENERIC_TYPE`,
`BMM_CONTAINER_TYPE` (loses `is_ordered`/`is_unique`, keeps v2
`container_type`/`base_type`), `BMM_CLASS` (concrete instead of abstract; loses
`features`/`static_properties`/`functions`/`procedures`/`invariants`/`creators`/
`converters`; `is_primitive_type` not `is_primitive`; `ancestors` typed as
classes not `BMM_MODEL_TYPE`), `BMM_ENUMERATION`(+`_INTEGER`/`_STRING`),
`BMM_PROPERTY` (concrete; parented on `BMM_MODEL_ELEMENT` not
`BMM_INSTANTIABLE_FEATURE`, so properties are NOT `BMM_FEATURE`s),
`BMM_MODEL_ELEMENT`, `BMM_PACKAGE`, `BMM_PACKAGE_CONTAINER`, `BMM_MODEL`.

**Two hard consequences to check every time:**
1. **`BMM_MODEL_TYPE` and `BMM_MODULE` are emitted NOWHERE** (187 declared, 185
   emitted). Both are v3-only abstract classes whose only descendants got v2
   ancestries. This is a codegen-completeness violation, and it orphans
   `BmmValueSetSpec` (emitted, zero referents).
2. **Hand-written `*_impl.rs` "honest boundaries" cite the v2 class docs.**
   `bmm_persistence/create_model.rs` records `value_constraint` and
   `P_BMM_CLASS.functions` as having "no destination" — bmm3 PROVIDES both
   destinations, so those adjudications (incl. #1391's) are stale, not boundaries.
   Treat a boundary attributed to the spec when the spec provides the destination
   as a misattribution finding.

Chapters unaffected by the merge (all v3-only, structurally faithful): ch.8
features (29/30 includes clean — only `BMM_PROPERTY` and the enums that contain
it are hit) and ch.9 literal values (9/9 clean).

Related: [[bmm3-unimplemented-function-surface]].
