---
name: lang-bmm3-two-schema-chimera
description: RESOLVED — openehr-lang's bmm3 tree once carried v2 class shapes at bmm3 paths; verified fixed 2026-08-11, both schemas now emit fully side by side
metadata:
  type: project
---

**This defect is FIXED. Do not re-report it; re-verify before citing it.**

History: `openehr-lang` composes two vendored LANG schemas
(`tools/openehr-codegen/src/plan/composition.rs`, `LANG_BMM` + `LANG_BMM3`),
and the merge used to be name-keyed first-wins, so 18 colliding class names
emitted the RELEASED-v2 shape at `bmm3/` paths and `BMM_MODEL_TYPE` /
`BMM_MODULE` were never emitted at all.

**Verified 2026-08-11** (post generation-module refactor, paths are now
`crates/openehr-lang/src/v1_1/bmm3/…`):
- `crates/openehr-lang/src/v1_1/bmm3/core/entity/bmm_model_type.rs` and
  `…/bmm_module.rs` both EXIST.
- `…/bmm3/core/entity/bmm_simple_class.rs` carries the true v3 shape:
  `ancestors: BTreeMap<String, BmmModelType>` plus `features`,
  `feature_groups`, `functions`, `procedures`, `static_properties`,
  `invariants`, `creators`, `converters`, `is_primitive` (not
  `is_primitive_type`).
- `bmm_persistence/create_model.rs`'s "no destination" boundaries are now
  GENERATION-SCOPED in prose ("has no destination in the generation this
  materialisation targets … this module materialises the **v2.x**
  `BMM_MODEL`"), and `create_bmm3_model.rs` lands the v3 destinations.

**How to apply:** the merge policy is still worth a glance on any LANG audit
(`load/bmm.rs`), but start from the assumption that both units emit fully.
Related: [[bmm3-unimplemented-function-surface]].
