---
name: lang-bmm3-two-schema-chimera
description: Verified openehr-lang defect — the two vendored LANG schemas name-merge first-wins so src/bmm3/ carries RELEASED-v2 shapes at bmm3 paths; check this first on any LANG/BMM audit
metadata:
  type: project
---

`openehr-lang` composes TWO vendored LANG schemas into one crate
(`tools/openehr-codegen/src/plan/composition.rs:98-107`, `own: &[LANG_BMM,
LANG_BMM3]`), and the merge is **asymmetric**: class *definitions* are
first-wins (`load/bmm.rs:275-283`, `entry().or_insert_with`) while class
*package paths* are last-wins (`load/bmm.rs:288-304`, `out.insert(c,
p.name)`). Verified 2026-07-31 during the BMM3 ch.10–14 audit.

**Why:** 18 class names exist in both `openehr_lang_1.1.0.bmm.json`
(`org.openehr.lang.bmm`, RELEASED/STABLE) and
`openehr_lang_1.1.0-bmm3.bmm.json` (DEVELOPMENT). For those 18 the emitted
Rust **shape** comes from v2 while the emitted **file path** comes from the
bmm3 package tree — so `src/bmm3/core/entity/bmm_class.rs` is labelled bmm3
but carries the v2 attribute set. Cascading: 2 bmm3 classes
(`BMM_MODEL_TYPE`, `BMM_MODULE`) lose all descendants post-merge and are
skipped entirely as "abstract-unused".

**How to apply:** before auditing ANY LANG/BMM chapter, diff the two vendored
schemas for the classes in scope — never assume a `src/bmm3/…` file reflects
`org.openehr.lang.bmm3.*.adoc`. Attributes verified missing:
`BMM_CLASS.{invariants,features,functions,procedures,static_properties,
creators,converters}`, `BMM_MODEL_ELEMENT.{extensions,scope}`,
`BMM_MODEL.{modules,used_models}`, `BMM_PROPERTY.is_composition`,
`BMM_PACKAGE.members`, `BMM_PACKAGE_CONTAINER.scope`,
`BMM_CONTAINER_TYPE.{container_class,is_ordered,is_unique,item_type}`,
`BMM_ENUMERATION_{INTEGER,STRING}.item_values`. Types diverged:
`BMM_CLASS.ancestors` (`BMM_MODEL_TYPE`→`BmmClass` — breaks bmm3 ch.13
§Simple Inheritance "a list of _types_ rather than classes"),
`immediate_descendants`, `BMM_MODEL_ELEMENT.documentation`
(`Hash<String,Any>`→`Option<String>`), `BMM_GENERIC_{CLASS,TYPE}.generic_parameters`,
`BMM_SIMPLE_TYPE.base_class`.

Two `bmm_persistence/create_model.rs` honest-boundary NOTEs (:302-306
functions, :1020-1026 generic ancestors) justify the drops by citing
`org.openehr.lang.bmm.bmm_class.adoc` — true of v2, FALSE of
`…bmm3.bmm_class.adoc`. Do not read them as "the spec provides no
destination".

The emitter's completeness gate cannot see any of this: it is class-NAME
level over the ALREADY-MERGED map (`testsupport.rs:94-127` +
`tests/it/emitter_invariants.rs:17-37`), and it actively sanctions the two
non-emissions. See [[codegen-degrade-and-optionality-conventions]].
