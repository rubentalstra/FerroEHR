---
name: bmm3-unimplemented-function-surface
description: What IS and is NOT implemented in openehr-lang's hand-written BMM impls — the naming trio + graph algorithms are real, the type lattice and signature surface are absent and unmarked
metadata:
  type: project
---

Verified 2026-07-31 by reading `crates/openehr-lang/src/bmm3/core/entity/{bmm_type_impl,bmm_class_impl}.rs`,
`.../model/bmm_model_impl.rs`, `.../feature/bmm_property_impl.rs`,
`crates/openehr-lang/src/bmm/core/bmm_generic_parameter_impl.rs`.

**Genuinely implemented, cited, unit-tested — do not re-flag as missing:** the
naming trio (`type_name`/`type_signature`/`conformance_type_name`) across every
type leaf; `flattened_type_list`; `type_substitutions`; on `BMM_CLASS`
`all_ancestors`/`all_descendants`/`suppliers`/`suppliers_non_primitive`/
`supplier_closure`/`package_path`/`class_path`/`flat_properties`; on `BMM_MODEL`
`class_definition` (case-insensitive, underscores significant)/
`enumeration_definition`/`primitive_types`/`enumeration_types`/
`all_ancestor_classes` (with the implicit `Any` top)/`property_definition`/
`property_definition_at_path`/`ms_conformant_property_type`/`type_conforms_to`/
the package-container trio; on `BMM_PROPERTY` `existence`/`display_name`.

**Absent from the whole crate (grep-verified):** `unitary_type`,
`effective_type`, `is_primitive`, `is_abstract` on TYPES (only on `BmmClass`),
`type_base_name`, `is_open`, `is_closed`, `is_partially_closed`,
`effective_base_class`, `name_map`, `generic_parameter_conformance_type`,
`type()` (the class→type generators, 3 forms), `flat_features`, `signature`,
`is_boolean`, `arity`, `has_ancestor_class`. Plus every ch.6–8 invariant
(`Inv_generic_name`, `Inv_constructors`/`Inv_converters`, `Inv_not_nullable`,
`Operator_validity`, `Inv_result_type`, `Inv_signature_*`) and the enumeration
one-ancestor + `item_names`/`item_values` 1:1 rules. **None carries a `// TODO:`
or `// NOTE:`** — indistinguishable from oversight.

**Why the split matters:** `unitary_type()`/`effective_type()` are
*unimplementable as typed* until the merge is fixed — `BmmUnitaryType` and
`BmmEffectiveType` exclude `BmmSimpleType`/`BmmGenericType`, so a simple type has
no effective type to return. Sequence the fix behind
[[lang-bmm-v2-v3-merge-contamination]].

**Two confirmed behavioural divergences in `type_conforms_to`
(`bmm_model_impl.rs`):**
1. Base-class comparison is byte-equality (`descendant == ancestor`,
   `.any(|name| name == ancestor)`) where BMM3 master06 §Type Conformance calls
   `is_case_insensitive_equal` — `type_conforms_to("dv_quantity","DV_QUANTITY")`
   returns false. The crate already uses `eq_ignore_ascii_case` three lines away.
2. Open generic parameters are substituted with `Any` on BOTH sides
   (`substitute_open_parameter`), not with the class's declared constraint, so
   `Interval<String> ⊑ Interval<T>` is true even against `Interval<T:Ordered>`.
   The in-code NOTE claims the owner class is not in scope — it is
   (`self.class_definition(root)` + `BmmGenericParameter::flattened_conforms_to_type`).

**How to apply:** when a LANG issue claims "the v2 function surface was
implemented (107 fns, #1389)", that covers the naming/conformance/flattening
family — NOT the meta-type lattice or the signature/arity surface. Check the
grep list above before accepting a completeness claim.
